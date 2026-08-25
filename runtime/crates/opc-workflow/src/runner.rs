//! 驱动工作流里的 `agent` 节点真正执行。
//!
//! **这一版只驱动 `kind=agent` 且 `assignment=template` 的节点**：
//! - `human`（评审/Gate）节点**永远不会**被这里自动推进——见 `gate` 模块，
//!   必须走人工 `approve_gate`/`reject_gate`。这不是"没做完"，是
//!   评审红线硬约束本身要求的行为。
//! - `condition` 节点（如模板里的 `qa_gate`，表达式判断 Bug 数量）—— 本版
//!   没有表达式求值器，节点会一直停在 `pending`，不自动推进。
//! - `manual` / `auto-claim` 节点（如 `frontend_dev` 手动指派/能力池认领）——
//!   本版不解析 `assigned_agent_id`，`list_ready_agent_tasks` 天然不会选中。

use std::collections::HashSet;
use std::path::Path;

use opc_agent::{run_task, AgentDefinition};
use opc_provider::ProviderRegistry;
use opc_storage::ProjectDb;
use opc_tool::{write_and_register_artifact, RegisterArtifactInput};

use crate::error::{Error, Result};
use crate::template::{TemplateNode, WorkflowTemplate};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TaskRow {
    pub id: String,
    pub workflow_id: String,
    pub node_key: String,
    pub kind: String,
    pub role: Option<String>,
    pub assignment_mode: String,
    pub assigned_agent_id: Option<String>,
    pub status: String,
}

pub async fn load_dag(project_db: &ProjectDb, workflow_id: &str) -> Result<WorkflowTemplate> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT dag FROM workflows WHERE id = ?").bind(workflow_id).fetch_optional(&project_db.pool).await?;
    let (dag_json,) = row.ok_or_else(|| Error::WorkflowNotFound(workflow_id.to_string()))?;
    Ok(serde_json::from_str(&dag_json)?)
}

/// 纯函数：给定 DAG + 已完成节点集合，算出哪些节点的依赖全部满足（不管 kind，
/// 调用方按需再过滤 `kind=agent`）。
pub fn ready_node_keys(dag: &WorkflowTemplate, completed: &HashSet<String>) -> Vec<String> {
    dag.nodes
        .iter()
        .filter(|n| !completed.contains(&n.id))
        .filter(|n| n.depends_on.iter().all(|d| completed.contains(d)))
        .map(|n| n.id.clone())
        .collect()
}

/// 列出当前可执行的 Agent 节点：`kind=agent` · `assignment=template` ·
/// 已在实例化时解析出 `assigned_agent_id` · 依赖节点全部 `completed`。
pub async fn list_ready_agent_tasks(project_db: &ProjectDb, workflow_id: &str) -> Result<Vec<TaskRow>> {
    let dag = load_dag(project_db, workflow_id).await?;

    let completed_rows: Vec<(String,)> =
        sqlx::query_as("SELECT node_key FROM tasks WHERE workflow_id = ? AND status = 'completed'")
            .bind(workflow_id)
            .fetch_all(&project_db.pool)
            .await?;
    let completed: HashSet<String> = completed_rows.into_iter().map(|(k,)| k).collect();

    let ready_keys: HashSet<String> = ready_node_keys(&dag, &completed).into_iter().collect();

    let rows: Vec<TaskRow> = sqlx::query_as(
        "SELECT id, workflow_id, node_key, kind, role, assignment_mode, assigned_agent_id, status \
         FROM tasks WHERE workflow_id = ? AND status = 'pending' AND kind = 'agent' \
         AND assignment_mode = 'template' AND assigned_agent_id IS NOT NULL",
    )
    .bind(workflow_id)
    .fetch_all(&project_db.pool)
    .await?;

    Ok(rows.into_iter().filter(|t| ready_keys.contains(&t.node_key)).collect())
}

pub struct RunTaskNodeOutput {
    pub task_run_id: String,
    pub provider_id: String,
    pub artifact_ids: Vec<String>,
}

/// 真的跑一次 Agent 节点：选 Provider → 执行 → 按节点声明的每个 output 落盘
/// 登记 → 写 `task_runs` → 节点标记 `completed`。
///
/// **P0 简化（诚实标注，不是完整实现）**：
/// 1. **不把上游节点的 Artifact 内容注入 Prompt**——只给一句提到节点名和
///    期望产出的通用指令；"读上游 PRD 写架构"这种真正的上下文传递是下一版
///    的事。
/// 2. 一次 Provider 调用吐出的同一段文本，**原样写进这个节点声明的每一个
///    output 路径**——不会把一段话拆成几份不同内容的文件。
/// 3. 只支持 `output.path` 是单个字符串的节点（模板里 `assignment=template`
///    的节点全部满足这一条）；`path` 是列表的节点（glob 目录）属于
///    manual/auto-claim，`list_ready_agent_tasks` 不会选中，误传进来会报
///    `Error::UnsupportedOutputShape`。
pub async fn run_task_node(
    project_db: &ProjectDb,
    project_root: &Path,
    registry: &ProviderRegistry,
    agent_defs: &[AgentDefinition],
    task: &TaskRow,
    node: &TemplateNode,
) -> Result<RunTaskNodeOutput> {
    let agent_instance_id =
        task.assigned_agent_id.clone().ok_or_else(|| Error::AgentInstanceNotFound(task.node_key.clone()))?;
    let role_id = node.role.clone().ok_or_else(|| Error::AgentInstanceNotFound(task.node_key.clone()))?;
    let agent_def =
        agent_defs.iter().find(|a| a.id == role_id).ok_or_else(|| Error::AgentInstanceNotFound(role_id.clone()))?;

    for node_output in &node.outputs {
        if node_output.path.len() != 1 {
            return Err(Error::UnsupportedOutputShape(task.node_key.clone()));
        }
    }

    sqlx::query("UPDATE tasks SET status = 'running', started_at = datetime('now') WHERE id = ?")
        .bind(&task.id)
        .execute(&project_db.pool)
        .await?;

    let outputs_desc: Vec<String> =
        node.outputs.iter().map(|o| format!("{}（{}）", o.kind, o.path[0])).collect();
    let prompt = format!("请完成工作流节点「{}」，产出：{}。", node.id, outputs_desc.join("、"));

    let run_result = run_task(registry, agent_def, &prompt, 2000).await;
    let output = match run_result {
        Ok(o) => o,
        Err(e) => {
            sqlx::query("UPDATE tasks SET status = 'failed', finished_at = datetime('now') WHERE id = ?")
                .bind(&task.id)
                .execute(&project_db.pool)
                .await?;
            return Err(Error::Agent(e));
        }
    };

    let task_run_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO task_runs (id, task_id, agent_id, provider_id, status, output_summary, finished_at) \
         VALUES (?,?,?,?, 'succeeded', ?, datetime('now'))",
    )
    .bind(&task_run_id)
    .bind(&task.id)
    .bind(&agent_instance_id)
    .bind(&output.provider_id)
    .bind(&output.response.text)
    .execute(&project_db.pool)
    .await?;

    let mut artifact_ids = Vec::new();
    for node_output in &node.outputs {
        let path = &node_output.path[0];
        let record = write_and_register_artifact(
            project_db,
            project_root,
            RegisterArtifactInput {
                kind: &node_output.kind,
                name: path.rsplit('/').next().unwrap_or(path),
                file_path: path,
                mime: Some("text/markdown"),
                producer_agent_id: Some(&agent_instance_id),
                producer_task_id: Some(&task.id),
                task_run_id: Some(&task_run_id),
                change_note: Some(&format!("workflow node {} · {}", node.id, output.provider_id)),
            },
            &output.response.text,
        )
        .await?;
        artifact_ids.push(record.id);
    }

    sqlx::query("UPDATE tasks SET status = 'completed', finished_at = datetime('now') WHERE id = ?")
        .bind(&task.id)
        .execute(&project_db.pool)
        .await?;

    Ok(RunTaskNodeOutput { task_run_id, provider_id: output.provider_id, artifact_ids })
}

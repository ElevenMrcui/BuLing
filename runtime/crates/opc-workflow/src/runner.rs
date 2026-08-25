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

/// `frontend/**` / `backend/**` 这类 output path 是"整个目录"的 glob 契约，
/// 不是字面文件路径——即便 `OneOrMany` 解析后 `path.len() == 1`，也不能当成
/// 一个真实文件名传给 `write_and_register_artifact`（会真的写出一个字面叫
/// `frontend/**` 的文件，语义上是错的）。见 `run_task_node` 里的用法。
fn is_glob_like(path: &str) -> bool {
    path.contains('*') || path.contains('?') || path.contains('[')
}

/// 解析 `node.inputs` 里单个 token（`templates/README.md`："`prid.output.acceptance`
/// 这类，只声明依赖，Runtime 自动注入"）指向的上游节点 + 具体 output kind：
/// - 裸节点 id（如 `"prd"`）——不限定 kind，注入该节点全部 output
/// - `"<node_id>.output.<kind>"`（如 `"prd.output.acceptance"`）——只注入这一个
/// `"__goal__"` 由调用方单独处理（不是节点引用），这里返回 `None`。
fn parse_input_ref(input: &str) -> Option<(&str, Option<&str>)> {
    if input == "__goal__" {
        return None;
    }
    match input.split_once(".output.") {
        Some((node_id, kind)) => Some((node_id, Some(kind))),
        None => Some((input, None)),
    }
}

/// 把一个 input token 解析出来的上游产出读成 Prompt 里能用的文本块。
///
/// 读不到内容不算错误，直接跳过（返回 `None`）——可能是引用的 output 是
/// `frontend/**` 这类 glob 目录契约（没法当单文件读），也可能是文件确实还
/// 没落盘。工作流依赖已经由 `depends_on`（含 `augment_depends_on_from_inputs`
/// 补齐的隐式依赖）保证"引用的节点跑完了才轮到当前节点"，这里只是尽力而为
/// 地把能读到的内容拼进 Prompt，读不到不该炸掉整个节点执行。
async fn read_input_context(project_root: &Path, dag: &WorkflowTemplate, input: &str) -> Option<String> {
    let (node_id, want_kind) = parse_input_ref(input)?;
    let ref_node = dag.nodes.iter().find(|n| n.id == node_id)?;

    let mut chunks = Vec::new();
    for out in &ref_node.outputs {
        if let Some(k) = want_kind {
            if out.kind != k {
                continue;
            }
        }
        let is_single_literal_path = out.path.len() == 1 && !is_glob_like(&out.path[0]);
        if !is_single_literal_path {
            continue;
        }
        let path = &out.path[0];
        if let Ok(content) = tokio::fs::read_to_string(project_root.join(path)).await {
            chunks.push(format!("#### 「{node_id}」· {}（{path}）\n\n{content}", out.kind));
        }
    }

    if chunks.is_empty() {
        None
    } else {
        Some(chunks.join("\n\n"))
    }
}

/// 给 Agent 节点组装带上游上下文的 Prompt：一句指令 + `node.inputs` 解析出的
/// 上游 Artifact 内容（`__goal__` 从 `project_meta.goal` 取用户最初的目标）。
async fn build_prompt(project_db: &ProjectDb, project_root: &Path, dag: &WorkflowTemplate, node: &TemplateNode) -> Result<String> {
    let outputs_desc: Vec<String> =
        node.outputs.iter().map(|o| format!("{}（{}）", o.kind, o.path[0])).collect();
    let mut prompt = format!("请完成工作流节点「{}」，产出：{}。", node.id, outputs_desc.join("、"));

    let mut context_parts = Vec::new();
    if node.inputs.iter().any(|i| i == "__goal__") {
        let goal: Option<(Option<String>,)> =
            sqlx::query_as("SELECT goal FROM project_meta WHERE id = 1").fetch_optional(&project_db.pool).await?;
        if let Some(g) = goal.and_then(|(g,)| g).filter(|g| !g.trim().is_empty()) {
            context_parts.push(format!("#### 用户最初的目标\n\n{g}"));
        }
    }
    for input in &node.inputs {
        if let Some(text) = read_input_context(project_root, dag, input).await {
            context_parts.push(text);
        }
    }

    if !context_parts.is_empty() {
        prompt = format!("{prompt}\n\n以下是你需要参考的上游产出，请基于这些真实内容工作，不要凭空杜撰：\n\n{}", context_parts.join("\n\n---\n\n"));
    }

    Ok(prompt)
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

/// 真的跑一次 Agent 节点：解析 `node.inputs` 拼真实上游上下文 → 选 Provider →
/// 执行 → 按节点声明的每个 output 落盘登记 → 写 `task_runs` → 节点标记
/// `completed`。
///
/// **P0 简化（诚实标注，不是完整实现）**：
/// 1. 一次 Provider 调用吐出的同一段文本，**原样写进这个节点声明的每一个
///    output 路径**——不会把一段话拆成几份不同内容的文件。
/// 2. 只支持 `output.path` 是单个字面文件路径的节点（模板里
///    `assignment=template` 的节点全部满足这一条）；`path` 是列表、或单个
///    但形如 `frontend/**` 的 glob 目录契约（`assignment=manual`/`auto-claim`
///    的开发类节点常是这种——一次 LLM 调用产不出一整个目录的多份源码文件），
///    会报 `Error::UnsupportedOutputShape`，不会把内容错写成一个字面叫
///    `frontend/**` 的文件。`opc-task` crate 复用这个函数驱动
///    manual/auto-claim 节点执行时，同样会撞上这个限制——这是诚实的能力
///    边界，不是遗漏。同样的原因，`node.inputs` 里引用到这类目录契约 output
///    的 token（见 `read_input_context`）在拼 Prompt 时会被跳过，不会报错。
pub async fn run_task_node(
    project_db: &ProjectDb,
    project_root: &Path,
    registry: &ProviderRegistry,
    agent_defs: &[AgentDefinition],
    dag: &WorkflowTemplate,
    task: &TaskRow,
    node: &TemplateNode,
) -> Result<RunTaskNodeOutput> {
    let agent_instance_id =
        task.assigned_agent_id.clone().ok_or_else(|| Error::AgentInstanceNotFound(task.node_key.clone()))?;
    let role_id = node.role.clone().ok_or_else(|| Error::AgentInstanceNotFound(task.node_key.clone()))?;
    let agent_def =
        agent_defs.iter().find(|a| a.id == role_id).ok_or_else(|| Error::AgentInstanceNotFound(role_id.clone()))?;

    for node_output in &node.outputs {
        let is_single_literal_path = node_output.path.len() == 1 && !is_glob_like(&node_output.path[0]);
        if !is_single_literal_path {
            return Err(Error::UnsupportedOutputShape(task.node_key.clone()));
        }
    }

    sqlx::query("UPDATE tasks SET status = 'running', started_at = datetime('now') WHERE id = ?")
        .bind(&task.id)
        .execute(&project_db.pool)
        .await?;

    let prompt = build_prompt(project_db, project_root, dag, node).await?;

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

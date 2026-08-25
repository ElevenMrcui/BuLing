//! 人工 Gate 通过 / 打回——**评审红线的唯一入口，永远由人工调用，
//! `opc-workflow` 自己不会、也不能替用户按下这个按钮**。
//!
//! `templates/*.yaml` 里 `kind: human` 的节点即 Gate 对应的评审任务，节点上
//! 的 `gate:` 字段（如 `gate: requirement-gate`）指向 `template.gates` 里
//! 声明的 Gate id——两者是不同的 key 空间：`gates` 表按 Gate id
//! （`requirement-gate`）建行，`tasks` 表按节点 id（`prd_review`）建行。
//! 这里的两个函数都以 **Gate id** 为入参，内部再从 DAG 快照里找到对应的
//! 评审任务节点。`runner` 模块永远不会自动把评审任务标记完成，必须显式调
//! 这里的 `approve_gate` / `reject_gate`。

use opc_storage::ProjectDb;

use crate::error::{Error, Result};
use crate::runner::load_dag;
use crate::template::TemplateNode;

async fn find_gate_row(project_db: &ProjectDb, workflow_id: &str, gate_id: &str) -> Result<String> {
    let row: Option<(String,)> = sqlx::query_as("SELECT id FROM gates WHERE workflow_id = ? AND node_key = ?")
        .bind(workflow_id)
        .bind(gate_id)
        .fetch_optional(&project_db.pool)
        .await?;
    row.map(|(id,)| id)
        .ok_or_else(|| Error::GateNotFound { workflow_id: workflow_id.to_string(), node_key: gate_id.to_string() })
}

fn find_review_node<'a>(dag_nodes: &'a [TemplateNode], gate_id: &str) -> Option<&'a TemplateNode> {
    dag_nodes.iter().find(|n| n.kind == "human" && n.gate.as_deref() == Some(gate_id))
}

/// 通过一个 Gate（`gate_id` 是 `template.gates[].id`，如 `"requirement-gate"`）：
/// 该 Gate 对应的评审任务节点在 `tasks` 表标记 `completed`（下游依赖它的
/// 节点才会进入 `list_ready_agent_tasks` 的可执行集合）、`gates` 行标记
/// `passed`，并追加一条 `reviews`（append-only，见 `docs/OPC-数据模型.md` §3.10）。
pub async fn approve_gate(project_db: &ProjectDb, workflow_id: &str, gate_id: &str, reviewer: &str, comment: Option<&str>) -> Result<()> {
    let gate_row_id = find_gate_row(project_db, workflow_id, gate_id).await?;

    let dag = load_dag(project_db, workflow_id).await?;
    let review_node_key = find_review_node(&dag.nodes, gate_id).map(|n| n.id.clone());

    sqlx::query("UPDATE gates SET status = 'passed', passed_at = datetime('now') WHERE id = ?")
        .bind(&gate_row_id)
        .execute(&project_db.pool)
        .await?;

    if let Some(node_key) = &review_node_key {
        sqlx::query(
            "UPDATE tasks SET status = 'completed', finished_at = datetime('now') \
             WHERE workflow_id = ? AND node_key = ?",
        )
        .bind(workflow_id)
        .bind(node_key)
        .execute(&project_db.pool)
        .await?;
    }

    sqlx::query("INSERT INTO reviews (subject_kind, subject_id, reviewer, decision, comment) VALUES ('gate', ?, ?, 'approved', ?)")
        .bind(&gate_row_id)
        .bind(reviewer)
        .bind(comment)
        .execute(&project_db.pool)
        .await?;

    Ok(())
}

/// 打回一个 Gate（`gate_id` 同上）：追加一条 `changes-requested` 评审记录，
/// 并把该 Gate 对应评审节点的 `on_reject.goto` 指向的节点重置回
/// `pending`，让它们重新出现在 `list_ready_agent_tasks` 里（重新产出后，
/// 同一 `file_path` 再登记 Artifact 会自然递增版本号，见
/// `opc-tool::write_and_register_artifact`）。Gate 本身与评审任务节点的
/// 状态都不动——仍在等一次新的评审。
///
/// **已知局限**：只重置 `on_reject.goto` 直接点名的节点，不做下游级联
/// 失效——如果被打回的节点之后还有已经跑完、依赖它产出的节点，这些下游
/// 节点不会自动跟着重置。P0 先把"能打回重做"跑通，级联失效留给下一版。
pub async fn reject_gate(project_db: &ProjectDb, workflow_id: &str, gate_id: &str, reviewer: &str, comment: Option<&str>) -> Result<()> {
    let gate_row_id = find_gate_row(project_db, workflow_id, gate_id).await?;

    let dag = load_dag(project_db, workflow_id).await?;
    let goto_targets: Vec<String> = find_review_node(&dag.nodes, gate_id).map(|n| n.on_reject_goto.clone()).unwrap_or_default();

    sqlx::query("INSERT INTO reviews (subject_kind, subject_id, reviewer, decision, comment) VALUES ('gate', ?, ?, 'changes-requested', ?)")
        .bind(&gate_row_id)
        .bind(reviewer)
        .bind(comment)
        .execute(&project_db.pool)
        .await?;

    for target in &goto_targets {
        sqlx::query(
            "UPDATE tasks SET status = 'pending', started_at = NULL, finished_at = NULL \
             WHERE workflow_id = ? AND node_key = ?",
        )
        .bind(workflow_id)
        .bind(target)
        .execute(&project_db.pool)
        .await?;
    }

    Ok(())
}

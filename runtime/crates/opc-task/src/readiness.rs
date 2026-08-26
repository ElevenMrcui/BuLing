//! `claim_task` / `assign_task_manually` 都要在真的写库之前确认节点的依赖
//! 已经满足——`list_claimable_tasks`/`list_manual_tasks` 已经做了这层过滤，
//! 但那只是给 UI 用的"建议列表"，直接调底层函数的调用方不该绕过这个校验，
//! 所以两个入口都会再查一遍。

use std::collections::HashSet;

use opc_storage::ProjectDb;
use opc_workflow::{load_dag, load_resolved_node_keys, ready_node_keys};

use crate::error::Result;

pub async fn is_node_ready(project_db: &ProjectDb, workflow_id: &str, node_key: &str) -> Result<bool> {
    let dag = load_dag(project_db, workflow_id).await?;
    let resolved = load_resolved_node_keys(project_db, workflow_id).await?;
    let ready: HashSet<String> = ready_node_keys(&dag, &resolved).into_iter().collect();
    Ok(ready.contains(node_key))
}

/// `list_claimable_tasks`（`mode="auto-claim"`）/ `list_manual_tasks`
/// （`mode="manual"`）共用的实现：`kind=agent · status=pending · assignment_mode=mode`
/// 且依赖已满足。
pub async fn list_ready_pending_by_mode(
    project_db: &ProjectDb,
    workflow_id: &str,
    mode: &str,
) -> Result<Vec<opc_workflow::TaskRow>> {
    let dag = load_dag(project_db, workflow_id).await?;
    let resolved = load_resolved_node_keys(project_db, workflow_id).await?;
    let ready: HashSet<String> = ready_node_keys(&dag, &resolved).into_iter().collect();

    let rows: Vec<opc_workflow::TaskRow> = sqlx::query_as(
        "SELECT id, workflow_id, node_key, kind, role, assignment_mode, assigned_agent_id, status \
         FROM tasks WHERE workflow_id = ? AND status = 'pending' AND kind = 'agent' AND assignment_mode = ?",
    )
    .bind(workflow_id)
    .bind(mode)
    .fetch_all(&project_db.pool)
    .await?;

    Ok(rows.into_iter().filter(|t| ready.contains(&t.node_key)).collect())
}

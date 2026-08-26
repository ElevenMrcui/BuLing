//! `assignment: manual` 节点——项目负责人（用户）直接指定由哪个
//! `agent_instances` 来干，不走认领打分。

use opc_storage::ProjectDb;

use crate::error::{Error, Result};
use crate::readiness::is_node_ready;

/// 列出当前可手动指派的 `manual` 节点（`kind=agent · status=pending`，
/// 依赖已满足）——给 UI 展示"有哪些活等着你指派"用。
pub async fn list_manual_tasks(project_db: &ProjectDb, workflow_id: &str) -> Result<Vec<opc_workflow::TaskRow>> {
    crate::readiness::list_ready_pending_by_mode(project_db, workflow_id, "manual").await
}

/// 把一个 `manual` 节点指派给指定的 `agent_instance_id`。
///
/// `agent_instance_id` 是否真的存在由 `tasks.assigned_agent_id` 的外键
/// （`REFERENCES agent_instances(id)`）兜底校验，这里不重复查一遍——传错
/// id 会直接得到 `Error::Sqlx`（外键约束失败），不是静默接受。
pub async fn assign_task_manually(
    project_db: &ProjectDb,
    workflow_id: &str,
    node_key: &str,
    agent_instance_id: &str,
) -> Result<()> {
    let dag = opc_workflow::load_dag(project_db, workflow_id).await?;
    let node = dag.nodes.iter().find(|n| n.id == node_key).ok_or_else(|| Error::NodeNotFound(node_key.to_string()))?;

    if node.kind != "agent" || node.assignment.as_deref() != Some("manual") {
        return Err(Error::NotAssignable {
            node_key: node_key.to_string(),
            reason: "节点不是 kind=agent · assignment=manual".to_string(),
        });
    }

    if !is_node_ready(project_db, workflow_id, node_key).await? {
        return Err(Error::NotAssignable { node_key: node_key.to_string(), reason: "依赖节点还没全部完成".to_string() });
    }

    sqlx::query("UPDATE tasks SET assigned_agent_id = ?, status = 'assigned' WHERE workflow_id = ? AND node_key = ?")
        .bind(agent_instance_id)
        .bind(workflow_id)
        .bind(node_key)
        .execute(&project_db.pool)
        .await?;

    Ok(())
}

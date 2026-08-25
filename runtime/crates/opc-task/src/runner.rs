//! 驱动一个已经指派/认领好的 `manual`/`auto-claim` 节点真正执行——复用
//! `opc_workflow::run_task_node`（它不看 `assignment_mode`，只要
//! `tasks.assigned_agent_id` 有值就会跑），不重复实现"选 Provider → 执行 →
//! 落盘登记 Artifact → 写 task_runs"这条链。

use std::path::Path;

use opc_agent::AgentDefinition;
use opc_provider::ProviderRegistry;
use opc_storage::ProjectDb;
use opc_workflow::RunTaskNodeOutput;

use crate::error::{Error, Result};

pub async fn run_assigned_task(
    project_db: &ProjectDb,
    project_root: &Path,
    registry: &ProviderRegistry,
    agent_defs: &[AgentDefinition],
    workflow_id: &str,
    node_key: &str,
) -> Result<RunTaskNodeOutput> {
    let dag = opc_workflow::load_dag(project_db, workflow_id).await?;
    let node = dag.nodes.iter().find(|n| n.id == node_key).ok_or_else(|| Error::NodeNotFound(node_key.to_string()))?;

    let task: opc_workflow::TaskRow = sqlx::query_as(
        "SELECT id, workflow_id, node_key, kind, role, assignment_mode, assigned_agent_id, status \
         FROM tasks WHERE workflow_id = ? AND node_key = ?",
    )
    .bind(workflow_id)
    .bind(node_key)
    .fetch_optional(&project_db.pool)
    .await?
    .ok_or_else(|| Error::NodeNotFound(node_key.to_string()))?;

    if task.assigned_agent_id.is_none() {
        return Err(Error::NotAssignable {
            node_key: node_key.to_string(),
            reason: "还没指派/认领——先调 claim_task 或 assign_task_manually".to_string(),
        });
    }

    Ok(opc_workflow::run_task_node(project_db, project_root, registry, agent_defs, &task, node).await?)
}

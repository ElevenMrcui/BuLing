//! `assignment: auto-claim` 节点的意愿分计算 + 认领。
//!
//! **诚实标注**：这不是真的"Agent 自主投标"（那需要每个 Agent 自己判断
//! 想不想接、算自己的意愿分），P0 是一个**能力匹配分**——节点声明的
//! `role`（如 `frontend_dev` 的 `role: frontend`）当作"这个节点需要什么能力"
//! 的提示，去查 `agents/frontend.yaml` 的 `capabilities` 当作需求集合，
//! 项目团队里每个 `agent_instances` 按自己模板的 `capabilities` 与需求集合
//! 的重合个数打分，分最高者中标。今天团队固定是 9 个不重复预置岗位，
//! 这个算法几乎总是选回 `role` 提示的那个岗位本身；等以后允许用户加自定义/
//! 派生 Agent 参与认领，重合打分才会真正派上用场。

use std::collections::{HashMap, HashSet};

use opc_agent::AgentDefinition;
use opc_storage::ProjectDb;

use crate::error::{Error, Result};
use crate::readiness::is_node_ready;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ClaimScore {
    pub agent_instance_id: String,
    pub template_agent_id: String,
    pub score: i64,
}

/// 纯函数：给定"需求能力提示"（某个 role id）+ 全部 Agent 定义 + 项目团队
/// 名单（`(agent_instance_id, template_agent_id)`），算出每个团队成员的
/// 能力匹配分，按分数降序（同分按 `template_agent_id` 升序，保证确定性）。
pub fn compute_claim_scores(
    hint_role: &str,
    agent_defs: &[AgentDefinition],
    team: &[(String, String)],
) -> Vec<ClaimScore> {
    let hint_caps: HashSet<&str> = agent_defs
        .iter()
        .find(|a| a.id == hint_role)
        .map(|a| a.capabilities.iter().map(String::as_str).collect())
        .unwrap_or_default();

    let mut scores: Vec<ClaimScore> = team
        .iter()
        .filter_map(|(instance_id, template_id)| {
            let def = agent_defs.iter().find(|a| &a.id == template_id)?;
            let score = def.capabilities.iter().filter(|c| hint_caps.contains(c.as_str())).count() as i64;
            Some(ClaimScore { agent_instance_id: instance_id.clone(), template_agent_id: template_id.clone(), score })
        })
        .collect();

    scores.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.template_agent_id.cmp(&b.template_agent_id)));
    scores
}

/// 列出当前可认领的 `auto-claim` 节点（`kind=agent · status=pending`，依赖
/// 已满足）——给 UI 展示"有哪些活可以让团队认领"用。
pub async fn list_claimable_tasks(project_db: &ProjectDb, workflow_id: &str) -> Result<Vec<opc_workflow::TaskRow>> {
    crate::readiness::list_ready_pending_by_mode(project_db, workflow_id, "auto-claim").await
}

/// 真的执行一次认领：算分 → 分最高且 > 0 的中标 → 写 `tasks.assigned_agent_id`
/// + `status='assigned'` + `claim_scores`（JSON 快照，给 UI 展示"谁想干这个
/// 任务、为什么"）。
pub async fn claim_task(
    project_db: &ProjectDb,
    workflow_id: &str,
    node_key: &str,
    agent_defs: &[AgentDefinition],
) -> Result<ClaimScore> {
    let dag = opc_workflow::load_dag(project_db, workflow_id).await?;
    let node = dag.nodes.iter().find(|n| n.id == node_key).ok_or_else(|| Error::NodeNotFound(node_key.to_string()))?;

    if node.kind != "agent" || node.assignment.as_deref() != Some("auto-claim") {
        return Err(Error::NotAssignable {
            node_key: node_key.to_string(),
            reason: "节点不是 kind=agent · assignment=auto-claim".to_string(),
        });
    }

    if !is_node_ready(project_db, workflow_id, node_key).await? {
        return Err(Error::NotAssignable { node_key: node_key.to_string(), reason: "依赖节点还没全部完成".to_string() });
    }

    let hint_role = node.role.clone().unwrap_or_default();
    let team: Vec<(String, String)> = sqlx::query_as("SELECT id, template_agent_id FROM agent_instances")
        .fetch_all(&project_db.pool)
        .await?;

    let scores = compute_claim_scores(&hint_role, agent_defs, &team);
    let winner = scores
        .iter()
        .find(|s| s.score > 0)
        .cloned()
        .ok_or_else(|| Error::NotAssignable { node_key: node_key.to_string(), reason: "没有能力匹配的候选 Agent".to_string() })?;

    let scores_map: HashMap<&str, i64> = scores.iter().map(|s| (s.template_agent_id.as_str(), s.score)).collect();
    let scores_json = serde_json::to_string(&scores_map)?;

    sqlx::query("UPDATE tasks SET assigned_agent_id = ?, status = 'assigned', claim_scores = ? WHERE workflow_id = ? AND node_key = ?")
        .bind(&winner.agent_instance_id)
        .bind(&scores_json)
        .bind(workflow_id)
        .bind(node_key)
        .execute(&project_db.pool)
        .await?;

    Ok(winner)
}

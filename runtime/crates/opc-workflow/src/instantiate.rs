//! 把一份 [`WorkflowTemplate`] 实例化进 `project.sqlite`：
//! 建 `workflows` 行（`dag` 字段存整份模板的 JSON 快照，见
//! `docs/OPC-数据模型.md` §3.6/§3.7）→ 逐节点建 `tasks` 行 → 逐 Gate 建
//! `gates` 行。
//!
//! **`assignment=template` 的 `agent` 节点在实例化时就把 `assigned_agent_id`
//! 解析好**（调用 `opc_project::find_agent_instance_id`），后续
//! `runner::list_ready_agent_tasks` 才能直接按 `assigned_agent_id IS NOT NULL`
//! 过滤，不用每次都重新查团队。`manual` / `auto-claim` 节点这一版不解析
//! （见 `runner` 模块文档），`assigned_agent_id` 留空。

use opc_project::find_agent_instance_id;
use opc_storage::ProjectDb;

use crate::error::Result;
use crate::template::WorkflowTemplate;

pub struct InstantiatedWorkflow {
    pub workflow_id: String,
}

pub async fn instantiate_workflow(project_db: &ProjectDb, template: &WorkflowTemplate) -> Result<InstantiatedWorkflow> {
    let workflow_id = uuid::Uuid::new_v4().to_string();
    let dag_json = serde_json::to_string(template)?;

    sqlx::query(
        "INSERT INTO workflows (id, template_id, name, dag, status, started_at) \
         VALUES (?,?,?,?, 'running', datetime('now'))",
    )
    .bind(&workflow_id)
    .bind(&template.id)
    .bind(&template.name)
    .bind(&dag_json)
    .execute(&project_db.pool)
    .await?;

    for node in &template.nodes {
        let task_id = uuid::Uuid::new_v4().to_string();

        let assigned_agent_id = if node.kind == "agent" && node.assignment.as_deref() == Some("template") {
            match &node.role {
                Some(role) => find_agent_instance_id(project_db, role).await?,
                None => None,
            }
        } else {
            None
        };

        sqlx::query(
            "INSERT INTO tasks (id, workflow_id, node_key, kind, title, role, assignment_mode, \
             assigned_agent_id, inputs, expected_outputs, status) \
             VALUES (?,?,?,?,?,?,?,?,?,?, 'pending')",
        )
        .bind(&task_id)
        .bind(&workflow_id)
        .bind(&node.id)
        .bind(&node.kind)
        .bind(&node.id)
        .bind(&node.role)
        .bind(node.assignment.as_deref().unwrap_or("template"))
        .bind(&assigned_agent_id)
        .bind(serde_json::to_string(&node.inputs)?)
        .bind(serde_json::to_string(&node.outputs)?)
        .execute(&project_db.pool)
        .await?;
    }

    for gate in &template.gates {
        let gate_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO gates (id, workflow_id, node_key, kind, status) VALUES (?,?,?,?, 'pending')")
            .bind(&gate_id)
            .bind(&workflow_id)
            .bind(&gate.id)
            .bind(&gate.id)
            .execute(&project_db.pool)
            .await?;
    }

    Ok(InstantiatedWorkflow { workflow_id })
}

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

use std::collections::{HashMap, HashSet, VecDeque};

use opc_audit::{kind as log_kind, outcome as log_outcome, LogEvent};
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

/// 正向依赖图（谁 `depends_on` 谁）上，从 `seeds` 出发能走到的全部下游节点 id
/// （不含 `seeds` 自己）——打回一个节点时，凡是依赖它（哪怕是间接依赖）的
/// 节点，产出都建立在即将作废的内容之上，理应一起回到 `pending`。
fn cascade_downstream(dag_nodes: &[TemplateNode], seeds: &[String]) -> HashSet<String> {
    let mut forward: HashMap<&str, Vec<&str>> = HashMap::new();
    for n in dag_nodes {
        for dep in &n.depends_on {
            forward.entry(dep.as_str()).or_default().push(n.id.as_str());
        }
    }

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<&str> = seeds.iter().map(|s| s.as_str()).collect();
    while let Some(cur) = queue.pop_front() {
        if let Some(children) = forward.get(cur) {
            for &child in children {
                if visited.insert(child.to_string()) {
                    queue.push_back(child);
                }
            }
        }
    }
    visited
}

/// 通过一个 Gate（`gate_id` 是 `template.gates[].id`，如 `"requirement-gate"`）：
/// 该 Gate 对应的评审任务节点在 `tasks` 表标记 `completed`（下游依赖它的
/// 节点才会进入 `list_ready_agent_tasks` 的可执行集合）、`gates` 行标记
/// `passed`，并追加一条 `reviews`（append-only，见 `docs/OPC-数据模型.md` §3.10）
/// 和一条 `execution_logs`（`kind=review.decision · result=confirmed`）。
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

    opc_audit::record(
        project_db,
        LogEvent { input: Some(gate_id), output: comment, ..LogEvent::new(log_kind::REVIEW_DECISION, log_outcome::CONFIRMED) },
    )
    .await
    .map_err(Error::Audit)?;

    Ok(())
}

/// 打回一个 Gate（`gate_id` 同上）：追加一条 `changes-requested` 评审记录
/// 和一条 `execution_logs`（`kind=review.decision · result=denied`），并把
/// 该 Gate 对应评审节点的 `on_reject.goto` 指向的节点**连同它们的全部下游**
/// 一起重置回 `pending`（重新产出后，同一 `file_path` 再登记 Artifact 会自然
/// 递增版本号，见 `opc-tool::write_and_register_artifact`）。
///
/// **级联失效**：`on_reject.goto` 直接点名的节点只是"要重做的起点"，真正
/// 该失效的是**依赖它们的全部下游**（不管间接多少层，靠 `depends_on` 正向
/// 展开，含 `augment_depends_on_from_inputs` 补的隐式依赖）——它们的产出
/// 建立在即将作废的内容之上。级联集合里如果包含别的 `kind=human` 评审节点
/// （包括这次被打回的 Gate 自己对应的评审节点——只要它结构上依赖某个
/// `on_reject.goto` 目标，通常都会，比如"打回 prd_review"时 `prd_review`
/// 自己就依赖 `prd`），连它对应的 `gates` 行也会一起重置回 `pending`（不然
/// UI 会出现"Gate 显示已通过，但它审的任务又变回 pending"这种自相矛盾的
/// 状态；对已经 `passed` 过一次、这次又被重新打回的 Gate 来说，回到
/// `pending` 才是正确状态）。**已知局限**：级联只重置 `status`/`started_at`/
/// `finished_at`，不清空 `manual`/`auto-claim` 节点已经写好的
/// `assigned_agent_id`——重跑会沿用原来的认领/指派，不会变回"待认领"。
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

    let cascade = cascade_downstream(&dag.nodes, &goto_targets);
    let reset_targets: HashSet<String> = goto_targets.iter().cloned().chain(cascade).collect();

    for target in &reset_targets {
        sqlx::query(
            "UPDATE tasks SET status = 'pending', started_at = NULL, finished_at = NULL \
             WHERE workflow_id = ? AND node_key = ?",
        )
        .bind(workflow_id)
        .bind(target)
        .execute(&project_db.pool)
        .await?;

        if let Some(downstream_gate_id) = dag.nodes.iter().find(|n| &n.id == target).and_then(|n| n.gate.as_deref()) {
            sqlx::query("UPDATE gates SET status = 'pending', passed_at = NULL WHERE workflow_id = ? AND node_key = ?")
                .bind(workflow_id)
                .bind(downstream_gate_id)
                .execute(&project_db.pool)
                .await?;
        }
    }

    opc_audit::record(
        project_db,
        LogEvent { input: Some(gate_id), output: comment, ..LogEvent::new(log_kind::REVIEW_DECISION, log_outcome::DENIED) },
    )
    .await
    .map_err(Error::Audit)?;

    Ok(())
}

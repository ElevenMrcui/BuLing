//! `kind: condition` 节点（如 `standard-software-delivery.yaml` 的 `qa_gate`）
//! 的表达式求值器。
//!
//! **范围（诚实标注，不是完整实现）**：只求值 `condition` 节点自己的
//! `condition:` 表达式，`on_true`/`on_false` 二选一 `goto`。**不**求值
//! `kind: agent` 节点 `on_complete` 上挂的条件分支（如模板里 `regression`
//! 节点的 `on_complete: [{condition, goto}, ...]`）——那是一种不同形状的分支
//! （附着在一个已经在跑的 Agent 节点后面，不是独立的 `condition` 节点），
//! 语义上还牵扯"跳过的分支要不要级联跳过它自己的下游"这类没有先例可循的
//! 设计判断，这一版不猜，留给下一版专门设计。
//!
//! **表达式语法**：只认 `templates/README.md` 里已经出现过的形状——
//! `<node_id>.output.<kind>.<field> <op> <literal>`，恰好 3 个空格分隔的
//! token（`qa_test.output.bug-report.critical_count == 0`）。`<field>` 从哪
//! 读：`<node_id>` 声明的 `output.kind` 对应的 Artifact 文件里，第一个
//! ` ```yaml ` 围栏代码块（"Runtime 契约字段"，`templates/artifacts/qa/Regression-Report.md`
//! 已经是这个格式的范例）当 YAML 解析，取 `<field>` 键。这不是发明新格式——
//! `templates/artifacts/qa/Bug-Report.md` 原本只在 prose 里写"Runtime 契约：
//! `critical_count = P0 数`"没给围栏代码块，属于模板自身没跟上 Regression-Report.md
//! 已有约定，这次一并把围栏代码块补齐（见该文件 diff），不是这里凭空定的规则。

use std::path::Path;

use opc_storage::ProjectDb;

use crate::error::Result;
use crate::runner::{is_glob_like, load_resolved_node_keys, ready_node_keys};
use crate::template::{TemplateNode, WorkflowTemplate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl CmpOp {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "==" => Some(Self::Eq),
            "!=" => Some(Self::Ne),
            "<" => Some(Self::Lt),
            "<=" => Some(Self::Le),
            ">" => Some(Self::Gt),
            ">=" => Some(Self::Ge),
            _ => None,
        }
    }
}

struct ParsedCondition<'a> {
    node_id: &'a str,
    kind: &'a str,
    field: &'a str,
    op: CmpOp,
    rhs: serde_yaml::Value,
}

fn parse_condition(expr: &str) -> Option<ParsedCondition<'_>> {
    let tokens: Vec<&str> = expr.split_whitespace().collect();
    let [lhs, op_str, rhs_str] = tokens[..] else { return None };

    let op = CmpOp::parse(op_str)?;
    let rhs = serde_yaml::from_str(rhs_str).ok()?;

    let (node_id, rest) = lhs.split_once(".output.")?;
    let (kind, field) = rest.split_once('.')?;
    Some(ParsedCondition { node_id, kind, field, op, rhs })
}

/// `node.outputs` 里找 `kind` 对应的单个字面文件路径（跳过 glob/多路径——
/// 那种 output 没法当结构化 Artifact 读，见 `runner::is_glob_like`）。
fn find_output_path<'a>(node: &'a TemplateNode, kind: &str) -> Option<&'a str> {
    node.outputs
        .iter()
        .find(|o| o.kind == kind)
        .filter(|o| o.path.len() == 1 && !is_glob_like(&o.path[0]))
        .map(|o| o.path[0].as_str())
}

/// 从 Markdown 正文里找第一个 ` ```yaml ` 围栏代码块，当 YAML mapping 解析，
/// 取 `field` 键的值。找不到围栏块、解析失败、或没有这个 key，都返回 `None`
/// ——不是错误，是"这份 Artifact 还没写全结构化字段"，调用方按"暂时判断不了，
/// 保持 pending"处理，不猜。
fn extract_contract_field(markdown: &str, field: &str) -> Option<serde_yaml::Value> {
    let start = markdown.find("```yaml")?;
    let after = &markdown[start + "```yaml".len()..];
    let end = after.find("```")?;
    let block = &after[..end];
    let mapping: serde_yaml::Mapping = serde_yaml::from_str(block).ok()?;
    mapping.get(serde_yaml::Value::String(field.to_string())).cloned()
}

fn compare(op: CmpOp, actual: &serde_yaml::Value, expected: &serde_yaml::Value) -> bool {
    match op {
        CmpOp::Eq => actual == expected,
        CmpOp::Ne => actual != expected,
        CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => {
            let (Some(a), Some(b)) = (actual.as_f64(), expected.as_f64()) else { return false };
            match op {
                CmpOp::Lt => a < b,
                CmpOp::Le => a <= b,
                CmpOp::Gt => a > b,
                CmpOp::Ge => a >= b,
                CmpOp::Eq | CmpOp::Ne => unreachable!(),
            }
        }
    }
}

/// 求值一个 `condition` 节点的表达式。`None` = 现在还判断不了（引用的
/// Artifact 没落盘 / 没有契约字段），不是错误；调用方应该把节点留在
/// `pending`，等下一次有更多 Artifact 落盘之后再试。
async fn evaluate(project_root: &Path, dag: &WorkflowTemplate, expr: &str) -> Option<bool> {
    let parsed = parse_condition(expr)?;
    let ref_node = dag.nodes.iter().find(|n| n.id == parsed.node_id)?;
    let path = find_output_path(ref_node, parsed.kind)?;
    let content = tokio::fs::read_to_string(project_root.join(path)).await.ok()?;
    let actual = extract_contract_field(&content, parsed.field)?;
    Some(compare(parsed.op, &actual, &parsed.rhs))
}

/// 找出当前依赖已满足、还没求值过的 `condition` 节点，逐个求值：
/// - 求出结果：自己标 `completed`，未选中的那个分支的 goto 目标标
///   `cancelled`（`load_resolved_node_keys` 把 `completed ∪ cancelled` 都当
///   "已解决，下游可以继续"——一个被跳过的分支不该永远堵住下游）
/// - 求不出结果：跳过，留在 `pending`，不报错、不重试计数
///
/// 在 `run_task_node()` 成功跑完一个 `kind=agent` 节点之后调用——这一版
/// `condition` 节点的唯一已知先例（`qa_gate`）依赖的正是一个 `kind=agent`
/// 节点（`qa_test`）。别的触发点（比如 `approve_gate` 之后）目前没有真实
/// 模板用例，先不接，避免加一段没有测试覆盖的路径。
pub async fn advance_condition_nodes(
    project_db: &ProjectDb,
    project_root: &Path,
    dag: &WorkflowTemplate,
    workflow_id: &str,
) -> Result<Vec<String>> {
    let resolved = load_resolved_node_keys(project_db, workflow_id).await?;
    let ready_keys = ready_node_keys(dag, &resolved);
    let mut advanced = Vec::new();

    for key in ready_keys {
        let Some(node) = dag.nodes.iter().find(|n| n.id == key) else { continue };
        if node.kind != "condition" {
            continue;
        }
        let Some(expr) = node.condition.as_deref() else { continue };
        let Some(outcome) = evaluate(project_root, dag, expr).await else { continue };

        let skipped = if outcome { &node.on_false_goto } else { &node.on_true_goto };

        sqlx::query("UPDATE tasks SET status = 'completed', finished_at = datetime('now') WHERE workflow_id = ? AND node_key = ?")
            .bind(workflow_id)
            .bind(&key)
            .execute(&project_db.pool)
            .await?;

        for skipped_id in skipped {
            sqlx::query(
                "UPDATE tasks SET status = 'cancelled', finished_at = datetime('now') \
                 WHERE workflow_id = ? AND node_key = ? AND status = 'pending'",
            )
            .bind(workflow_id)
            .bind(skipped_id)
            .execute(&project_db.pool)
            .await?;
        }

        advanced.push(key);
    }

    Ok(advanced)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_equality_condition() {
        let p = parse_condition("qa_test.output.bug-report.critical_count == 0").unwrap();
        assert_eq!(p.node_id, "qa_test");
        assert_eq!(p.kind, "bug-report");
        assert_eq!(p.field, "critical_count");
        assert_eq!(p.op, CmpOp::Eq);
        assert_eq!(p.rhs, serde_yaml::Value::Number(0.into()));
    }

    #[test]
    fn parses_boolean_condition() {
        let p = parse_condition("regression.output.regression-report.all_passed == true").unwrap();
        assert_eq!(p.field, "all_passed");
        assert_eq!(p.rhs, serde_yaml::Value::Bool(true));
    }

    #[test]
    fn extracts_field_from_first_yaml_fence() {
        let md = "# 标题\n\n```yaml\nall_passed: true\ncritical_count: 0\n```\n\n## 其它章节\n";
        assert_eq!(extract_contract_field(md, "all_passed"), Some(serde_yaml::Value::Bool(true)));
        assert_eq!(extract_contract_field(md, "critical_count"), Some(serde_yaml::Value::Number(0.into())));
        assert_eq!(extract_contract_field(md, "missing_field"), None);
    }

    #[test]
    fn extracts_none_when_no_yaml_fence_present() {
        assert_eq!(extract_contract_field("# 只有 prose，没有围栏代码块", "critical_count"), None);
    }
}

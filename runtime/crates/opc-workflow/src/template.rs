//! `templates/*.yaml` 的 Rust 形状。
//!
//! **不是全字段忠实解析**——`on_approve` / `on_complete`（含条件分支）/
//! `on_true` / `on_false` 这些字段目前只在 `templates/README.md` 的语义里
//! 存在，本 P0 加载器不认识它们（serde 默认忽略未知字段，不会报错，但
//! 也不会保留）。只强类型解析执行引擎这一版真正用得到的字段：
//! `id` / `kind` / `role` / `assignment` / `inputs` / `outputs` /
//! `depends_on` / `subject` / `on_reject.goto`。`parallel` 分组节点在加载时
//! **就地拍平**成独立节点（组的 `depends_on` 并入每个子节点）。

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// YAML 里 `path:` / `subject:` / `goto:` 既可能是单值也可能是列表，
/// 统一按列表处理。
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    fn into_vec(self) -> Vec<String> {
        match self {
            OneOrMany::One(s) => vec![s],
            OneOrMany::Many(v) => v,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawGoto {
    goto: OneOrMany,
}

#[derive(Debug, Clone, Deserialize)]
struct RawOutput {
    kind: String,
    path: OneOrMany,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct RawNode {
    id: Option<String>,
    kind: Option<String>,
    role: Option<String>,
    assignment: Option<String>,
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    outputs: Vec<RawOutput>,
    #[serde(default)]
    depends_on: Vec<String>,
    reviewer: Option<String>,
    gate: Option<String>,
    subject: Option<OneOrMany>,
    on_reject: Option<RawGoto>,
    /// `kind: condition` 节点专属：`condition` 表达式求值为 true/false 各自
    /// `goto` 到哪些节点，见 `crate::condition`。
    on_true: Option<RawGoto>,
    on_false: Option<RawGoto>,
    condition: Option<String>,
    #[serde(default)]
    parallel: Vec<RawNode>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawGate {
    id: String,
    display_name: String,
    #[serde(default)]
    required: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct RawTemplate {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    version: i64,
    #[serde(default)]
    required_roles: Vec<String>,
    nodes: Vec<RawNode>,
    #[serde(default)]
    gates: Vec<RawGate>,
}

/// 一个 Artifact 输出契约：`{ kind, path }`。P0 只用单路径（`path.len() == 1`）
/// 的节点驱动执行；多路径（glob 目录，如 `frontend/**`）的节点属于
/// manual/auto-claim，本版不驱动，见 `runner::run_task_node` 文档。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOutput {
    pub kind: String,
    pub path: Vec<String>,
}

/// 拍平后的工作流节点——`parallel` 分组已展开成独立节点，组级 `depends_on`
/// 已并入每个子节点。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateNode {
    pub id: String,
    /// "agent" | "human" | "condition" | "sub-workflow"（"parallel" 分组在加载时已拍平，不会出现）
    pub kind: String,
    pub role: Option<String>,
    /// "template" | "manual" | "auto-claim"（human/condition 节点通常为 None）
    pub assignment: Option<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<NodeOutput>,
    /// 依赖的节点 id 列表。显式 `depends_on` 优先；`kind=human` 且未显式声明时
    /// 退化为 `subject`（评审节点天然依赖它评审的对象）。
    pub depends_on: Vec<String>,
    pub gate: Option<String>,
    pub subject: Vec<String>,
    pub reviewer: Option<String>,
    /// `on_reject.goto`——打回后要重置回 pending 的节点 id 列表，见 `gate::reject_gate`。
    pub on_reject_goto: Vec<String>,
    pub condition: Option<String>,
    /// `kind: condition` 节点专属，见 `crate::condition::advance_condition_nodes`。
    pub on_true_goto: Vec<String>,
    pub on_false_goto: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateDef {
    pub id: String,
    pub display_name: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: i64,
    pub required_roles: Vec<String>,
    pub nodes: Vec<TemplateNode>,
    pub gates: Vec<GateDef>,
}

fn to_template_node(raw: RawNode, file: &str) -> Result<TemplateNode> {
    let id = raw.id.ok_or_else(|| Error::Template { file: file.to_string(), reason: "node missing id".to_string() })?;
    let kind = raw
        .kind
        .ok_or_else(|| Error::Template { file: file.to_string(), reason: format!("node {id} missing kind") })?;

    let subject = raw.subject.map(OneOrMany::into_vec).unwrap_or_default();
    let mut depends_on = raw.depends_on;
    if depends_on.is_empty() && kind == "human" && !subject.is_empty() {
        depends_on = subject.clone();
    }
    let on_reject_goto = raw.on_reject.map(|g| g.goto.into_vec()).unwrap_or_default();
    let on_true_goto = raw.on_true.map(|g| g.goto.into_vec()).unwrap_or_default();
    let on_false_goto = raw.on_false.map(|g| g.goto.into_vec()).unwrap_or_default();

    Ok(TemplateNode {
        id,
        kind,
        role: raw.role,
        assignment: raw.assignment,
        inputs: raw.inputs,
        outputs: raw.outputs.into_iter().map(|o| NodeOutput { kind: o.kind, path: o.path.into_vec() }).collect(),
        depends_on,
        gate: raw.gate,
        subject,
        reviewer: raw.reviewer,
        on_reject_goto,
        condition: raw.condition,
        on_true_goto,
        on_false_goto,
    })
}

fn flatten_nodes(raw_nodes: Vec<RawNode>, file: &str) -> Result<Vec<TemplateNode>> {
    let mut out = Vec::new();
    for raw in raw_nodes {
        if !raw.parallel.is_empty() {
            let group_depends_on = raw.depends_on.clone();
            for child in raw.parallel {
                let mut node = to_template_node(child, file)?;
                for d in &group_depends_on {
                    if !node.depends_on.contains(d) {
                        node.depends_on.push(d.clone());
                    }
                }
                out.push(node);
            }
        } else {
            out.push(to_template_node(raw, file)?);
        }
    }
    Ok(out)
}

/// `templates/README.md` 明确说 `inputs` "只声明依赖，Runtime 自动注入"——
/// 所以依赖不能只看 `depends_on`：这一版模板里不少 `agent` 节点（如
/// `regression` / `acceptance_prep`）压根没写 `depends_on`，真正的前置关系
/// 只体现在 `inputs`（`prd` / `prd.output.acceptance` 这类，取第一个 `.`
/// 之前的部分当节点 id）里。这里在拍平之后补一遍：把 `inputs` 里能匹配上
/// 已知节点 id 的 token 并入 `depends_on`（去重，`__goal__` 不算节点）；
/// `condition` 节点同理从 `condition` 表达式的第一个 token 里补一个依赖。
fn augment_depends_on_from_inputs(nodes: &mut [TemplateNode]) {
    let known_ids: std::collections::HashSet<String> = nodes.iter().map(|n| n.id.clone()).collect();
    for node in nodes.iter_mut() {
        let self_id = node.id.clone();
        let mut extra: Vec<String> = Vec::new();
        for input in &node.inputs {
            if input == "__goal__" {
                continue;
            }
            let candidate = input.split('.').next().unwrap_or(input);
            if candidate != self_id && known_ids.contains(candidate) && !node.depends_on.contains(&candidate.to_string()) && !extra.contains(&candidate.to_string()) {
                extra.push(candidate.to_string());
            }
        }
        if node.kind == "condition" && node.depends_on.is_empty() {
            if let Some(candidate) = node.condition.as_deref().and_then(|c| c.split(['.', ' ']).next()) {
                if candidate != self_id && known_ids.contains(candidate) && !extra.contains(&candidate.to_string()) {
                    extra.push(candidate.to_string());
                }
            }
        }
        node.depends_on.extend(extra);
    }
}

impl WorkflowTemplate {
    pub fn from_yaml_str(file: &str, s: &str) -> Result<Self> {
        let raw: RawTemplate =
            serde_yaml::from_str(s).map_err(|e| Error::Template { file: file.to_string(), reason: e.to_string() })?;
        let mut nodes = flatten_nodes(raw.nodes, file)?;
        augment_depends_on_from_inputs(&mut nodes);
        Ok(WorkflowTemplate {
            id: raw.id,
            name: raw.name,
            description: raw.description,
            version: raw.version,
            required_roles: raw.required_roles,
            nodes,
            gates: raw.gates.into_iter().map(|g| GateDef { id: g.id, display_name: g.display_name, required: g.required }).collect(),
        })
    }
}

/// 扫描 `templates/*.yaml`（顶层文件，不递归进 `artifacts/` 子目录）。
pub fn load_templates_from_dir(dir: impl AsRef<Path>) -> Result<Vec<WorkflowTemplate>> {
    let dir = dir.as_ref();
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }
        let content = std::fs::read_to_string(&path)?;
        out.push(WorkflowTemplate::from_yaml_str(&path.display().to_string(), &content)?);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

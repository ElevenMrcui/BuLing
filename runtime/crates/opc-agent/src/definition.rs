//! `AgentDefinition` —— `agents/*.yaml` 的 Rust 形状，字段与
//! `runtime/migrations/app/0001_init.sql` 的 `agents` 表逐一对应
//! （见 `agents/README.md` 的字段规范）。

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExpectedOutput {
    pub path: String,
    pub kind: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionDefaults {
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(default)]
    pub git: Vec<String>,
    #[serde(default)]
    pub docker: Vec<String>,
    #[serde(default)]
    pub mcp: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: String,
    /// "preset" | "user"
    pub kind: String,
    pub role: String,
    pub display_name: String,
    #[serde(default)]
    pub avatar: Option<String>,
    pub system_prompt: String,

    #[serde(default)]
    pub responsibilities: Vec<String>,
    #[serde(default)]
    pub expected_outputs: Vec<ExpectedOutput>,
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// "low" | "medium" | "high" —— high 只能挂 allowed_sensitivity 含 high 的 Provider。
    #[serde(default = "default_sensitivity")]
    pub sensitivity: String,

    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub mcp_servers: Vec<String>,

    /// Provider Router 尝试顺序，见 opc-provider::registry。
    #[serde(default)]
    pub provider_priority: Vec<String>,

    #[serde(default)]
    pub permission_defaults: PermissionDefaults,
}

fn default_sensitivity() -> String {
    "medium".to_string()
}

impl AgentDefinition {
    pub fn from_yaml_str(file: &str, s: &str) -> Result<Self> {
        serde_yaml::from_str(s).map_err(|e| Error::Yaml { file: file.to_string(), reason: e.to_string() })
    }
}

/// 扫描 `agents/*.yaml`，跳过非 `.yaml` 文件（如 `README.md`）。
pub fn load_agents_from_dir(dir: impl AsRef<Path>) -> Result<Vec<AgentDefinition>> {
    let dir = dir.as_ref();
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }
        let content = std::fs::read_to_string(&path)?;
        let def = AgentDefinition::from_yaml_str(&path.display().to_string(), &content)?;
        out.push(def);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

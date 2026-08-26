//! Provider manifest（`providers/*/manifest.toml`）—— 声明式厂商元数据。
//!
//! **设计原则（开闭原则）**：新增一个走已知协议（openai-compatible /
//! anthropic-messages / 已支持的 CLI）的厂商，只需要加一份 TOML，不需要改
//! Rust 代码。Registry（见 `registry.rs`）按 `wire_format` / `cli_adapter`
//! 字段路由到对应实现。

use serde::Deserialize;
use std::path::Path;

use crate::error::{Error, Result};
use crate::types::ProviderKind;

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderManifest {
    pub id: String,
    pub display_name: String,
    pub vendor: String,
    pub kind: ProviderKind,

    /// api / local 用：走哪种线协议。cli 用忽略此字段。
    #[serde(default)]
    pub wire_format: Option<WireFormat>,

    /// cli 用：走哪个 CliAdapter 实现。api / local 忽略此字段。
    #[serde(default)]
    pub cli_adapter: Option<String>,
    /// cli 用：可执行文件名，供本机网关 probe（对齐 `packages/cli-registry`）。
    #[serde(default)]
    pub cli_binary: Option<String>,

    /// api / local 用：默认 base_url（用户可在模型中心覆盖）。
    #[serde(default)]
    pub default_base_url: Option<String>,

    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub models: Vec<String>,

    /// api 用：credential 走 OS Keychain 的哪个 service/account 组合。
    #[serde(default)]
    pub credential_service: Option<String>,

    /// 敏感度白名单（对齐 app.sqlite providers.allowed_sensitivity）。
    #[serde(default = "default_sensitivity")]
    pub allowed_sensitivity: Vec<String>,

    #[serde(default)]
    pub privacy_note: Option<String>,
}

fn default_sensitivity() -> Vec<String> {
    vec!["low".into(), "medium".into()]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum WireFormat {
    #[serde(rename = "anthropic-messages")]
    AnthropicMessages,
    #[serde(rename = "openai-compatible")]
    OpenAiCompatible,
}

impl ProviderManifest {
    pub fn from_toml_str(file: &str, s: &str) -> Result<Self> {
        toml::from_str(s).map_err(|e| Error::Manifest { file: file.to_string(), reason: e.to_string() })
    }
}

/// 扫描 `providers/` 目录，加载每个子目录下的 `manifest.toml`。
///
/// 布局：`providers/<vendor-dir>/manifest.toml`（见 `providers/README.md`）。
pub fn load_manifests_from_dir(dir: impl AsRef<Path>) -> Result<Vec<ProviderManifest>> {
    let dir = dir.as_ref();
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let manifest_path = entry.path().join("manifest.toml");
        if !manifest_path.is_file() {
            continue;
        }
        let content = std::fs::read_to_string(&manifest_path)?;
        let m = ProviderManifest::from_toml_str(&manifest_path.display().to_string(), &content)?;
        out.push(m);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

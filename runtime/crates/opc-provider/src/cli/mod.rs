pub mod adapter;
pub mod claude_code;

use async_trait::async_trait;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use crate::error::{Error, Result};
use crate::manifest::ProviderManifest;
use crate::provider::Provider;
use crate::types::{ProviderKind, ProviderRequest, ProviderResponse, ProviderStatusReport};

use self::adapter::CliAdapter;
use self::claude_code::ClaudeCodeAdapter;

const EXEC_TIMEOUT: Duration = Duration::from_secs(300);

/// CLI Provider —— Adapter 模式的调用方（context）：不关心具体是哪家 CLI，
/// 只持有一个 `Box<dyn CliAdapter>` 负责拼参数 / 解输出，自己只管进程管理。
pub struct CliProvider {
    manifest: ProviderManifest,
    adapter: Box<dyn CliAdapter>,
    binary: String,
}

impl CliProvider {
    pub fn from_manifest(manifest: ProviderManifest) -> Result<Self> {
        let adapter: Box<dyn CliAdapter> = match manifest.cli_adapter.as_deref() {
            Some("claude-code") => Box::new(ClaudeCodeAdapter),
            other => {
                return Err(Error::Manifest {
                    file: manifest.id.clone(),
                    reason: format!(
                        "未知或未实现的 cli_adapter: {:?}（当前仅实现 claude-code；\
                         新增前请先用真实 CLI --help 核实参数，不臆造）",
                        other
                    ),
                })
            }
        };
        let binary = manifest
            .cli_binary
            .clone()
            .ok_or_else(|| Error::Manifest { file: manifest.id.clone(), reason: "manifest 缺少 cli_binary".into() })?;
        Ok(Self { manifest, adapter, binary })
    }

    fn shell_quote_bin(&self) -> Result<&str> {
        // 与 apps/local-gateway/src/scan.ts 的 shellQuote 同一白名单：
        // CLI 名早在 manifest 里写死，不接受用户输入，防注入。
        if !self.binary.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-')) {
            return Err(Error::CliExec { binary: self.binary.clone(), reason: "invalid binary name".into() });
        }
        Ok(&self.binary)
    }
}

#[async_trait]
impl Provider for CliProvider {
    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Cli
    }

    async fn execute(&self, req: &ProviderRequest) -> Result<ProviderResponse> {
        let prompt = req
            .messages
            .last()
            .map(|m| m.content.clone())
            .ok_or_else(|| Error::Parse { context: "cli provider request".into(), reason: "messages 为空".into() })?;

        let model = req.model_override.as_deref().or(self.manifest.default_model.as_deref());
        let args = self.adapter.build_args(model, req.system.as_deref(), &prompt);

        let mut cmd = Command::new(&self.binary);
        cmd.args(&args);
        cmd.kill_on_drop(true);

        let output = timeout(EXEC_TIMEOUT, cmd.output())
            .await
            .map_err(|_| Error::CliExec { binary: self.binary.clone(), reason: format!("timeout after {EXEC_TIMEOUT:?}") })?
            .map_err(|e| Error::CliExec { binary: self.binary.clone(), reason: e.to_string() })?;

        if !output.status.success() {
            return Err(Error::CliNonZero {
                binary: self.binary.clone(),
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).chars().take(1000).collect(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut resp = self.adapter.parse_output(&stdout)?;
        if resp.model.is_empty() {
            resp.model = model.unwrap_or_default().to_string();
        }
        Ok(resp)
    }

    async fn status(&self) -> ProviderStatusReport {
        let bin = match self.shell_quote_bin() {
            Ok(b) => b,
            Err(e) => return ProviderStatusReport::unavailable(e.to_string()),
        };
        // 只 `command -v`，不带 --version（对齐 docs/本地网关.md 的“只发现不执行”不变量）。
        let result = Command::new("/bin/sh").arg("-c").arg(format!("command -v {bin}")).output().await;
        match result {
            Ok(out) if out.status.success() => {
                let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                ProviderStatusReport::ok(format!("found at {path}"))
            }
            Ok(_) => ProviderStatusReport::unavailable(format!("{bin} not found in PATH")),
            Err(e) => ProviderStatusReport::unavailable(e.to_string()),
        }
    }
}

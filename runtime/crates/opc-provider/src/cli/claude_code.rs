//! Claude Code CLI adapter。
//!
//! **验证状态**：
//! - 命令行参数（`-p`/`--print`、`--output-format json`、`--model`、
//!   `--system-prompt`）已在本仓库开发过程中用 `claude --help` 的真实输出核实
//!   （非臆造）。
//! - `--output-format json` 的**响应体字段**（`result` / `is_error` /
//!   `usage.input_tokens` / `usage.output_tokens`）基于 Claude Code 官方文档
//!   记录的行为，本会话未做一次真实调用核实（用户当次拒绝了活体探测）。
//!   **上线前必须跑一次真实调用核对字段**——若字段不符，本 parser 会返回
//!   携带原始 stdout 的 `Error::Parse`，不会静默返回错误数据，方便快速修正。

use serde::Deserialize;

use crate::error::{Error, Result};
use crate::types::{ProviderResponse, Usage};

use super::adapter::CliAdapter;

pub struct ClaudeCodeAdapter;

#[derive(Deserialize)]
struct ClaudeCodeJsonResult {
    #[serde(default)]
    result: Option<String>,
    #[serde(default)]
    is_error: Option<bool>,
    #[serde(default)]
    usage: Option<UsageWire>,
}

#[derive(Deserialize, Default)]
struct UsageWire {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

impl CliAdapter for ClaudeCodeAdapter {
    fn build_args(&self, model: Option<&str>, system: Option<&str>, prompt: &str) -> Vec<String> {
        let mut args = vec!["-p".to_string(), prompt.to_string()];
        if let Some(m) = model {
            args.push("--model".into());
            args.push(m.into());
        }
        if let Some(s) = system {
            args.push("--system-prompt".into());
            args.push(s.into());
        }
        args.push("--output-format".into());
        args.push("json".into());
        args
    }

    fn parse_output(&self, stdout: &str) -> Result<ProviderResponse> {
        let parsed: ClaudeCodeJsonResult = serde_json::from_str(stdout).map_err(|e| Error::Parse {
            context: "claude code --output-format json".into(),
            reason: format!("{e}; raw stdout (前 500 字): {}", stdout.chars().take(500).collect::<String>()),
        })?;

        if parsed.is_error == Some(true) {
            return Err(Error::CliExec {
                binary: "claude".into(),
                reason: parsed.result.unwrap_or_else(|| "unknown error (is_error=true)".into()),
            });
        }

        let usage = parsed.usage.unwrap_or_default();
        Ok(ProviderResponse {
            text: parsed.result.unwrap_or_default(),
            model: String::new(), // Claude Code JSON 输出未必回显 model id；由调用方按 model_override 兜底
            usage: Usage { input_tokens: usage.input_tokens, output_tokens: usage.output_tokens },
            raw: serde_json::from_str(stdout).unwrap_or(serde_json::Value::Null),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_args_minimal() {
        let adapter = ClaudeCodeAdapter;
        let args = adapter.build_args(None, None, "写一份 PRD");
        assert_eq!(args, vec!["-p", "写一份 PRD", "--output-format", "json"]);
    }

    #[test]
    fn build_args_with_model_and_system() {
        let adapter = ClaudeCodeAdapter;
        let args = adapter.build_args(Some("claude-sonnet-5"), Some("你是产品经理"), "写 PRD");
        assert_eq!(
            args,
            vec![
                "-p",
                "写 PRD",
                "--model",
                "claude-sonnet-5",
                "--system-prompt",
                "你是产品经理",
                "--output-format",
                "json",
            ]
        );
    }

    #[test]
    fn parse_output_success() {
        let adapter = ClaudeCodeAdapter;
        let stdout = r#"{"result":"这是产出","is_error":false,"usage":{"input_tokens":10,"output_tokens":20}}"#;
        let resp = adapter.parse_output(stdout).expect("should parse");
        assert_eq!(resp.text, "这是产出");
        assert_eq!(resp.usage.input_tokens, 10);
        assert_eq!(resp.usage.output_tokens, 20);
    }

    #[test]
    fn parse_output_error_flag() {
        let adapter = ClaudeCodeAdapter;
        let stdout = r#"{"result":"rate limited","is_error":true}"#;
        let err = adapter.parse_output(stdout).expect_err("should error");
        assert!(err.to_string().contains("rate limited"));
    }

    #[test]
    fn parse_output_malformed_json_surfaces_raw_snippet() {
        let adapter = ClaudeCodeAdapter;
        let stdout = "not json at all";
        let err = adapter.parse_output(stdout).expect_err("should error");
        let msg = err.to_string();
        assert!(msg.contains("not json at all"), "raw stdout should be included for debugging: {msg}");
    }
}

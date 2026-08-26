//! 与具体 Provider 无关的共享请求/响应形状。
//!
//! 设计边界（见 `docs/OPC-架构决策.md` ADR-005）：Provider 层只做**一次文本
//! 补全**——system + 对话历史进，文本 + usage 出。工具调用循环 / Artifact
//! 写入是 Runtime（`opc-tool` + `opc-workflow`，P0.5+）的职责，不在这里。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: Role::User, content: content.into() }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: Role::Assistant, content: content.into() }
    }
}

/// 一次 Provider 调用的请求。
#[derive(Debug, Clone)]
pub struct ProviderRequest {
    /// Agent.system_prompt —— 定义岗位人格 / 责任 / 输出约束。
    pub system: Option<String>,
    /// 对话历史（含本轮用户输入）。
    pub messages: Vec<ChatMessage>,
    /// 输出上限；各 adapter 按自己协议换算。
    pub max_tokens: u32,
    /// 覆盖 manifest 里的 default_model；为空则用 manifest 默认值。
    pub model_override: Option<String>,
}

impl ProviderRequest {
    pub fn simple(system: impl Into<String>, user: impl Into<String>, max_tokens: u32) -> Self {
        Self {
            system: Some(system.into()),
            messages: vec![ChatMessage::user(user)],
            max_tokens,
            model_override: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// 一次 Provider 调用的响应。
#[derive(Debug, Clone)]
pub struct ProviderResponse {
    pub text: String,
    pub model: String,
    pub usage: Usage,
    /// 原始响应（CLI 的 stdout JSON / API 的 body JSON），供 ExecutionLog 审计。
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    Cli,
    Api,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatusReport {
    pub available: bool,
    pub detail: Option<String>,
}

impl ProviderStatusReport {
    pub fn ok(detail: impl Into<String>) -> Self {
        Self { available: true, detail: Some(detail.into()) }
    }
    pub fn unavailable(detail: impl Into<String>) -> Self {
        Self { available: false, detail: Some(detail.into()) }
    }
}

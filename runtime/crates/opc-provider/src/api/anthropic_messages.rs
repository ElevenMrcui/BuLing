//! Anthropic Messages API wire format —— `POST {base_url}/v1/messages`
//!
//! 字段来源：Anthropic 官方文档 / claude-api skill 权威参考（本 session 内核实，
//! 不臆造）：
//!   - Headers: `Content-Type: application/json` · `x-api-key` · `anthropic-version: 2023-06-01`
//!   - Request: `{ model, max_tokens, system?, messages: [{role, content}] }`
//!   - Response: `{ content: [{type:"text", text}], model, stop_reason,
//!                  usage: { input_tokens, output_tokens } }`
//!   - Error: `{ type:"error", error: { type, message }, request_id }`

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::types::{ChatMessage, ProviderRequest, ProviderResponse, Role, Usage};

use super::wire::WireFormat;

const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

pub struct AnthropicMessagesWire;

#[derive(Serialize)]
struct Req<'a> {
    model: &'a str,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    messages: Vec<WireMessage<'a>>,
}

#[derive(Serialize)]
struct WireMessage<'a> {
    role: &'static str,
    content: &'a str,
}

fn role_str(r: Role) -> &'static str {
    match r {
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

#[derive(Deserialize)]
struct Resp {
    content: Vec<ContentBlock>,
    model: String,
    usage: RespUsage,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text { text: String },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct RespUsage {
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(Deserialize)]
struct ErrBody {
    error: ErrInner,
}

#[derive(Deserialize)]
struct ErrInner {
    #[serde(rename = "type")]
    kind: String,
    message: String,
}

#[async_trait]
impl WireFormat for AnthropicMessagesWire {
    async fn call(
        &self,
        client: &Client,
        base_url: &str,
        model: &str,
        api_key: &str,
        req: &ProviderRequest,
    ) -> Result<ProviderResponse> {
        let base = if base_url.is_empty() { DEFAULT_BASE_URL } else { base_url };
        let url = format!("{}/v1/messages", base.trim_end_matches('/'));

        let messages: Vec<WireMessage> = req
            .messages
            .iter()
            .map(|m: &ChatMessage| WireMessage { role: role_str(m.role), content: &m.content })
            .collect();

        let body = Req {
            model,
            max_tokens: req.max_tokens,
            system: req.system.as_deref(),
            messages,
        };

        let http_resp = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&body)
            .send()
            .await?;

        let status = http_resp.status();
        let raw_text = http_resp.text().await?;
        let raw: serde_json::Value = serde_json::from_str(&raw_text).unwrap_or(serde_json::Value::Null);

        if !status.is_success() {
            let (kind, message) = match serde_json::from_str::<ErrBody>(&raw_text) {
                Ok(e) => (e.error.kind, e.error.message),
                Err(_) => ("unknown".to_string(), raw_text.clone()),
            };
            return Err(Error::Api { status: status.as_u16(), kind, message });
        }

        let parsed: Resp = serde_json::from_str(&raw_text)
            .map_err(|e| Error::Parse { context: "anthropic messages response".into(), reason: e.to_string() })?;

        let text = parsed
            .content
            .into_iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text),
                ContentBlock::Other => None,
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(ProviderResponse {
            text,
            model: parsed.model,
            usage: Usage { input_tokens: parsed.usage.input_tokens, output_tokens: parsed.usage.output_tokens },
            raw,
        })
    }
}

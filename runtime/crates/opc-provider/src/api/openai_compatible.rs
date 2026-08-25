//! OpenAI 兼容 wire format —— `POST {base_url}/chat/completions`
//!
//! 覆盖走这套公开协议的厂商：OpenAI 官方 / GLM（智谱）/ Qwen（通义 DashScope
//! 兼容模式）/ DeepSeek / Ollama / LM Studio ——它们各自的官方文档都声明兼容
//! OpenAI Chat Completions 形状，这里只实现该协议的**公共子集**（不额外加
//! 任何单一厂商的私有扩展字段，避免把厂商专属参数误当通用协议）。
//!
//! - Headers: `Content-Type: application/json` · `Authorization: Bearer <key>`
//!   （本地 Provider 如 Ollama 通常不校验 key，key 为空时跳过该 header）
//! - Request: `{ model, messages: [{role, content}], max_tokens? }`
//! - Response: `{ choices: [{ message: { role, content } }],
//!                usage: { prompt_tokens, completion_tokens } }`
//! - Error：无统一标准，宽松解析：优先 `error.message`，取不到就回退整段 body。

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::types::{ChatMessage, ProviderRequest, ProviderResponse, Role, Usage};

use super::wire::WireFormat;

pub struct OpenAiCompatibleWire;

#[derive(Serialize)]
struct Req<'a> {
    model: &'a str,
    messages: Vec<WireMessage<'a>>,
    max_tokens: u32,
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
    model: Option<String>,
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<RespUsage>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
}

#[derive(Deserialize, Default)]
struct RespUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
}

#[derive(Deserialize)]
struct ErrBody {
    error: ErrInner,
}

#[derive(Deserialize)]
struct ErrInner {
    message: String,
}

#[async_trait]
impl WireFormat for OpenAiCompatibleWire {
    async fn call(
        &self,
        client: &Client,
        base_url: &str,
        model: &str,
        api_key: &str,
        req: &ProviderRequest,
    ) -> Result<ProviderResponse> {
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        // system prompt 在 OpenAI 兼容协议里是 messages[] 里的一条 role="system"
        let mut messages: Vec<WireMessage> = Vec::with_capacity(req.messages.len() + 1);
        if let Some(sys) = req.system.as_deref() {
            messages.push(WireMessage { role: "system", content: sys });
        }
        messages.extend(
            req.messages
                .iter()
                .map(|m: &ChatMessage| WireMessage { role: role_str(m.role), content: &m.content }),
        );

        let body = Req { model, messages, max_tokens: req.max_tokens };

        let mut builder = client.post(&url).header("Content-Type", "application/json");
        if !api_key.is_empty() {
            builder = builder.header("Authorization", format!("Bearer {api_key}"));
        }

        let http_resp = builder.json(&body).send().await?;
        let status = http_resp.status();
        let raw_text = http_resp.text().await?;
        let raw: serde_json::Value = serde_json::from_str(&raw_text).unwrap_or(serde_json::Value::Null);

        if !status.is_success() {
            let message = match serde_json::from_str::<ErrBody>(&raw_text) {
                Ok(e) => e.error.message,
                Err(_) => raw_text.clone(),
            };
            return Err(Error::Api { status: status.as_u16(), kind: "http_error".into(), message });
        }

        let parsed: Resp = serde_json::from_str(&raw_text)
            .map_err(|e| Error::Parse { context: "openai-compatible response".into(), reason: e.to_string() })?;

        let text = parsed
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .unwrap_or_default();

        let usage = parsed.usage.unwrap_or_default();

        Ok(ProviderResponse {
            text,
            model: parsed.model.unwrap_or_else(|| model.to_string()),
            usage: Usage { input_tokens: usage.prompt_tokens, output_tokens: usage.completion_tokens },
            raw,
        })
    }
}

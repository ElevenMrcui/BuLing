pub mod anthropic_messages;
pub mod openai_compatible;
pub mod wire;

use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

use crate::credential;
use crate::error::Result;
use crate::manifest::{ProviderManifest, WireFormat as WireFormatKind};
use crate::provider::Provider;
use crate::types::{ProviderKind, ProviderRequest, ProviderResponse, ProviderStatusReport};

use self::anthropic_messages::AnthropicMessagesWire;
use self::openai_compatible::OpenAiCompatibleWire;
use self::wire::WireFormat;

/// API Provider —— Strategy 模式的调用方（context）：不知道具体协议，只持有
/// 一个 `Box<dyn WireFormat>` 并转发调用。
pub struct ApiProvider {
    manifest: ProviderManifest,
    wire: Box<dyn WireFormat>,
    client: Client,
    /// 用户在「模型中心」覆盖的 base_url；为空则用 manifest 的 default_base_url。
    base_url_override: Option<String>,
}

impl ApiProvider {
    pub fn from_manifest(manifest: ProviderManifest, base_url_override: Option<String>) -> Result<Self> {
        let wire: Box<dyn WireFormat> = match manifest.wire_format {
            Some(WireFormatKind::AnthropicMessages) => Box::new(AnthropicMessagesWire),
            Some(WireFormatKind::OpenAiCompatible) | None => Box::new(OpenAiCompatibleWire),
        };
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()?;
        Ok(Self { manifest, wire, client, base_url_override })
    }

    fn resolve_base_url(&self) -> String {
        self.base_url_override
            .clone()
            .or_else(|| self.manifest.default_base_url.clone())
            .unwrap_or_default()
    }

    fn resolve_model<'a>(&'a self, req: &'a ProviderRequest) -> Option<&'a str> {
        req.model_override
            .as_deref()
            .or(self.manifest.default_model.as_deref())
    }
}

#[async_trait]
impl Provider for ApiProvider {
    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn kind(&self) -> ProviderKind {
        self.manifest.kind
    }

    async fn execute(&self, req: &ProviderRequest) -> Result<ProviderResponse> {
        let base_url = self.resolve_base_url();
        let model = self.resolve_model(req).unwrap_or_default().to_string();

        // Local Provider（Ollama / LM Studio）通常不需要真实 key。
        let api_key = match &self.manifest.credential_service {
            Some(service) => credential::resolve_api_key(service).unwrap_or_default(),
            None => String::new(),
        };

        self.wire.call(&self.client, &base_url, &model, &api_key, req).await
    }

    async fn status(&self) -> ProviderStatusReport {
        let base_url = self.resolve_base_url();
        if base_url.is_empty() {
            return ProviderStatusReport::unavailable("未配置 base_url");
        }
        match &self.manifest.credential_service {
            Some(service) => match credential::resolve_api_key(service) {
                Ok(_) => ProviderStatusReport::ok(format!("base_url={base_url}")),
                Err(e) => ProviderStatusReport::unavailable(e.to_string()),
            },
            None => ProviderStatusReport::ok(format!("base_url={base_url}（无需鉴权）")),
        }
    }
}

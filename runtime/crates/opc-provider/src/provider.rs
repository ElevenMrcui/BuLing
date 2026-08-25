//! `Provider` trait —— Runtime 面对的唯一接口（依赖倒置：Runtime 只认这个
//! trait，不认具体是哪家厂商 / CLI 还是 API）。

use async_trait::async_trait;

use crate::error::Result;
use crate::types::{ProviderKind, ProviderRequest, ProviderResponse, ProviderStatusReport};

#[async_trait]
pub trait Provider: Send + Sync {
    /// 稳定 id，对齐 `providers` 表主键 / manifest 里的 id。
    fn id(&self) -> &str;

    fn kind(&self) -> ProviderKind;

    /// 执行一次文本补全。不做工具调用循环（见模块文档的职责边界）。
    async fn execute(&self, req: &ProviderRequest) -> Result<ProviderResponse>;

    /// 探测可用性：CLI 是否装了 / API Key 是否能取到。不发起真实推理请求。
    async fn status(&self) -> ProviderStatusReport;
}

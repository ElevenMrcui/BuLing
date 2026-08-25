//! `WireFormat` —— Strategy 接口：把"怎么组请求 / 怎么解响应"从 `ApiProvider`
//! 的传输逻辑（HTTP 调用 / 错误处理 / 计时）里拆出来，一个厂商协议一个实现。
//!
//! 新增一种协议只需实现这个 trait；`ApiProvider` 本身不关心具体协议。

use async_trait::async_trait;
use reqwest::Client;

use crate::error::Result;
use crate::types::{ProviderRequest, ProviderResponse};

#[async_trait]
pub trait WireFormat: Send + Sync {
    /// 组好请求并发起 HTTP 调用，返回解析后的响应。
    ///
    /// `base_url` / `model` / `api_key` 由 `ApiProvider` 从 manifest + 用户配置解析后传入，
    /// wire format 实现只管"这个协议的 HTTP 形状长什么样"。
    async fn call(
        &self,
        client: &Client,
        base_url: &str,
        model: &str,
        api_key: &str,
        req: &ProviderRequest,
    ) -> Result<ProviderResponse>;
}

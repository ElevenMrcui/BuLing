//! 不令 OPC · Provider 层
//!
//! 统一抽象 CLI / API / Local 三种 AI 能力来源（见 `docs/OPC-架构决策.md`
//! ADR-005）。设计上用了三个经典模式，各自解决一个变化点：
//!
//! - **依赖倒置**：Runtime 只依赖 [`provider::Provider`] trait，不认具体是
//!   哪家厂商——新增厂商不影响调用方代码。
//! - **Strategy**（`api::wire::WireFormat`）：API Provider 之间的差异只在于
//!   "线协议"（Anthropic Messages vs OpenAI 兼容），把这层差异封装成可替换
//!   的策略对象，[`api::ApiProvider`] 本身只管 HTTP 调用的公共部分。
//! - **Adapter**（`cli::adapter::CliAdapter`）：CLI Provider 之间的差异是
//!   "参数怎么拼 / 输出怎么解析"，同样封装成可替换适配器，
//!   [`cli::CliProvider`] 只管进程管理的公共部分。
//! - **Factory**（[`registry::ProviderRegistry`]）：`providers/*/manifest.toml`
//!   声明式描述每个厂商，新增一个走已知协议的厂商只需要加一份 TOML，不用
//!   碰 Rust 代码（开闭原则）。
//!
//! 职责边界：Provider 只做**一次文本补全**（system + 历史进，文本 + usage
//! 出）。工具调用循环 / Artifact 落盘是 Runtime（`opc-tool` + `opc-workflow`，
//! P0.5+）的职责，不在这里。

pub mod api;
pub mod cli;
pub mod credential;
pub mod error;
pub mod manifest;
pub mod provider;
pub mod registry;
pub mod types;

pub use error::{Error, Result};
pub use manifest::{ProviderManifest, WireFormat};
pub use provider::Provider;
pub use registry::ProviderRegistry;
pub use types::{ChatMessage, ProviderKind, ProviderRequest, ProviderResponse, ProviderStatusReport, Role, Usage};

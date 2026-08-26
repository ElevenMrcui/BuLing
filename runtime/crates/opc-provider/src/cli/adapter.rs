//! `CliAdapter` —— Adapter 模式：把"这家 CLI 的参数怎么拼 / 输出怎么解析"
//! 从 `CliProvider` 的进程管理逻辑（spawn / 超时 / stdout 采集）里拆出来。
//!
//! 新增一家 CLI 只需要实现这个 trait，不用碰 `CliProvider` 本身。

use crate::error::Result;
use crate::types::ProviderResponse;

pub trait CliAdapter: Send + Sync {
    /// 组装命令行参数（不含 binary 本身）。
    ///
    /// P0 范围：单轮补全——`prompt` 是本次要发的用户输入（多轮历史暂不支持，
    /// 见 `docs/OPC-架构决策.md` ADR-005 附注）。
    fn build_args(&self, model: Option<&str>, system: Option<&str>, prompt: &str) -> Vec<String>;

    /// 解析 stdout（`--output-format json` 的单次结果）为统一响应形状。
    fn parse_output(&self, stdout: &str) -> Result<ProviderResponse>;
}

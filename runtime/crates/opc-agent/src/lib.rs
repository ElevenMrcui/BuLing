//! 不令 OPC · Agent 层
//!
//! 三件事：
//! 1. **加载**（[`definition`]）——把 `agents/*.yaml` 解析成 [`definition::AgentDefinition`]
//! 2. **播种**（[`seed`]）——写进 `app.sqlite` 的 `agents` 表（幂等 upsert，
//!    预置岗位受保护，见 `seed.rs` 顶部注释）
//! 3. **驱动**（[`runner`]）——把一个 Agent 真正接到 `opc-provider` 上跑一次
//!    文本补全（按 `provider_priority` 做 fallback 路由）
//!
//! 不做的事：工具调用循环 / Artifact 落盘 / Report.md 强制产出——那些是
//! `opc-tool` + `opc-workflow`（P0.5+）的职责。

pub mod definition;
pub mod error;
pub mod runner;
pub mod seed;

pub use definition::{load_agents_from_dir, AgentDefinition, ExpectedOutput, PermissionDefaults};
pub use error::{Error, Result};
pub use runner::{run_task, select_provider, AgentRunOutput};
pub use seed::seed_agents;

//! 不令 OPC · Storage
//!
//! SQLite 存储层，双库设计（见 `docs/OPC-架构决策.md` ADR-003）：
//! - [`AppDb`]  = APP 级全局库（`~/.opc/db.sqlite`）
//! - [`ProjectDb`] = 项目级独立库（`<project>/.opc/project.sqlite`）
//!
//! 迁移 SQL 来自 `runtime/migrations/{app,project}/*.sql`，由本 crate 的
//! [`migrator`] 模块加载执行；追踪表 `_migrations` 由 migrator 自动创建。

pub mod app;
pub mod error;
pub mod migrator;
pub mod project;

pub use app::AppDb;
pub use error::{Error, Result};
pub use project::ProjectDb;

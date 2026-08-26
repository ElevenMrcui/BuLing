//! 不令 OPC · Tool 层（P0 最小切片）
//!
//! 只做一件事到位：**把 Agent 的产出真正落到项目目录里，并登记进
//! `project.sqlite`**。
//!
//! - [`fs_tool`] —— 沙箱化文件写入（`permission_defaults.file = "project-only"`
//!   的硬拦：拒绝绝对路径 / `..` 穿越出项目根）
//! - [`artifact`] —— 写文件 + 登记 `artifacts` / `artifact_versions`，
//!   一个事务内完成
//!
//! **不做的事**（留给未来 P0.5+ 的其它切片）：`shell.run` / `git.*` /
//! `db.execute` / `http.fetch` 等其它 Tool；权限系统的 prompt 弹窗确认
//! （现在只有 `file: project-only` 这一条硬约束，其余权限位在
//! `agents/*.yaml` 里声明但还没接执行路径）。

pub mod artifact;
pub mod error;
pub mod fs_tool;

pub use artifact::{write_and_register_artifact, ArtifactRecord, RegisterArtifactInput};
pub use error::{Error, Result};
pub use fs_tool::{resolve_project_path, write_text_file, WrittenFile};

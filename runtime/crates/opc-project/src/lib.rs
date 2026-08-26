//! 不令 OPC · Project 层
//!
//! 补上 `opc-tool::artifact` 需要但 `opc-agent` 不管的一步：把预置 Agent
//! **实例化**进具体项目的团队里，产出 `agent_instances` 行——Artifact 的
//! `producer_agent_id` 外键才有真实数据可指。见
//! `docs/OPC-架构决策.md` ADR-005 附注 3 / 附注 4。
//!
//! 四件事：
//! - `create_project()` —— 建目录 + project.sqlite + app.sqlite 注册 +
//!   默认团队 + 把全部预置 Agent 实例化进团队
//! - `open_project()` —— 按 project_id 打开已存在项目，刷新 `last_opened_at`
//! - `list_projects()` —— 「项目中心」列表，最近打开的排前面
//! - `find_agent_instance_id()` —— 按模板 Agent id 查这个项目里对应的
//!   `agent_instances.id`

pub mod error;
pub mod project;

pub use error::{Error, Result};
pub use project::{
    create_project, find_agent_instance_id, list_projects, open_project, CreateProjectInput,
    CreatedProject, ProjectSummary,
};

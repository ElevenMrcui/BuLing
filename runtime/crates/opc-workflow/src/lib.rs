//! 不令 OPC · Workflow 层
//!
//! 四件事：
//! 1. **加载**（[`template`]）——把 `templates/*.yaml` 解析成 [`template::WorkflowTemplate`]
//!    （`parallel` 分组在加载时拍平成独立节点）
//! 2. **实例化**（[`instantiate`]）——把模板落进 `project.sqlite`：建
//!    `workflows` 行（`dag` 存整份模板快照）+ 逐节点建 `tasks` 行（`assignment=template`
//!    的 Agent 节点顺带解析出 `agent_instances.id`）+ 逐 Gate 建 `gates` 行
//! 3. **驱动**（[`runner`]）——只驱动 `kind=agent` 且 `assignment=template` 的
//!    节点：算依赖满足 → 跑一次 Agent → 按节点声明的 output 落盘登记 Artifact
//! 4. **人工 Gate**（[`gate`]）——评审红线的唯一入口，`approve_gate` /
//!    `reject_gate` 永远由外部（用户）显式调用，Runtime 自己不会替用户按下
//!
//! **这一版没做的事**（诚实标注，不是遗漏）：
//! - `human` 节点不自动推进——这是评审红线本身要求的，不是缺口
//! - `condition` 节点（表达式判断）不求值，会一直停在 `pending`
//! - `manual` / `auto-claim` 节点不解析 `assigned_agent_id`，不会被驱动
//! - 不把上游节点的 Artifact 内容注入 Prompt，只给一句通用指令
//! - `reject_gate` 打回不做下游级联失效，只重置 `on_reject.goto` 直接点名的节点
//!
//! 见 `docs/OPC-架构决策.md` ADR-005 附注 5。

pub mod error;
pub mod gate;
pub mod instantiate;
pub mod runner;
pub mod template;

pub use error::{Error, Result};
pub use gate::{approve_gate, reject_gate};
pub use instantiate::{instantiate_workflow, InstantiatedWorkflow};
pub use runner::{list_ready_agent_tasks, load_dag, ready_node_keys, run_task_node, RunTaskNodeOutput, TaskRow};
pub use template::{load_templates_from_dir, GateDef, NodeOutput, TemplateNode, WorkflowTemplate};

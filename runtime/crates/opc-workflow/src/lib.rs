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
//! 5. **条件分支**（[`condition`]）——`kind=condition` 节点（如 `qa_gate`）
//!    的表达式求值，在 `run_task_node()` 跑完一个 Agent 节点后自动尝试推进
//!
//! **这一版没做的事**（诚实标注，不是遗漏）：
//! - `human` 节点不自动推进——这是评审红线本身要求的，不是缺口
//! - `manual` / `auto-claim` 节点不解析 `assigned_agent_id`，不会被驱动
//! - `node.inputs` 引用到 `frontend/**` 这类目录契约 output 时读不到内容会
//!   静默跳过（见 `runner::read_input_context`），不会报错中断执行
//! - `kind=agent` 节点 `on_complete` 上挂的条件分支（如 `regression` 节点）
//!   不求值——跟 `condition` 节点是两种不同形状的分支，见 `condition` 模块文档
//!
//! 见 `docs/OPC-架构决策.md` ADR-005 附注 5、7、8。

pub mod condition;
pub mod error;
pub mod gate;
pub mod instantiate;
pub mod runner;
pub mod template;

pub use condition::advance_condition_nodes;
pub use error::{Error, Result};
pub use gate::{approve_gate, reject_gate};
pub use instantiate::{instantiate_workflow, InstantiatedWorkflow};
pub use runner::{
    list_ready_agent_tasks, load_dag, load_resolved_node_keys, ready_node_keys, run_task_node, RunTaskNodeOutput,
    TaskRow,
};
pub use template::{load_templates_from_dir, GateDef, NodeOutput, TemplateNode, WorkflowTemplate};

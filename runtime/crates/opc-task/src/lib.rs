//! 不令 OPC · Task 层
//!
//! 补上 `opc-workflow` 留的缺口：`assignment=manual`/`auto-claim` 的节点谁
//! 来干、怎么定下来。三件事：
//!
//! 1. **认领**（[`claim`]）——`auto-claim` 节点的能力匹配打分 + 自动指派
//! 2. **手动指派**（[`manual`]）——`manual` 节点由用户直接点名
//! 3. **驱动**（[`runner`]）——指派/认领定下来之后，复用
//!    `opc_workflow::run_task_node` 真正执行，不重复实现执行链
//!
//! **诚实标注**：`opc-workflow` 的唯一真实模板（`standard-software-delivery.yaml`）
//! 里，全部 `manual`/`auto-claim` 节点（`frontend_dev` / `backend_dev` /
//! `bug_fix`）的 output 都是整个目录的 glob 契约（如 `frontend/**`），不是
//! 单个字面文件——这不是 `opc-task` 能解决的问题，是 `write_and_register_artifact`
//! 的单文件模型本身的边界："一次 Agent 调用产出一整个目录的多份具名源码
//! 文件"是比这个 crate 大得多的另一件事，不在这次范围内。`claim_task`/
//! `assign_task_manually` 本身能正常工作（指派机制是独立于"能不能真的跑出
//! 结果"这件事的）；`run_assigned_task` 对这几个节点会正确报
//! `Error::Workflow(UnsupportedOutputShape)`，而不是把内容错写进一个字面叫
//! `frontend/**` 的文件——见 `docs/OPC-架构决策.md` ADR-005 附注 6。

pub mod claim;
pub mod error;
pub mod manual;
pub mod readiness;
pub mod runner;

pub use claim::{claim_task, compute_claim_scores, list_claimable_tasks, ClaimScore};
pub use error::{Error, Result};
pub use manual::{assign_task_manually, list_manual_tasks};
pub use readiness::is_node_ready;
pub use runner::run_assigned_task;

//! 不令 OPC · 审计层
//!
//! 只做一件事：把一次操作追加写进 `project.sqlite` 的 `execution_logs`
//! （append-only，见 `docs/OPC-数据模型.md` §3.12、`runtime/migrations/README.md`
//! "硬约束（append-only 表）"）。这一版只接项目级库——APP 级 `execution_logs`
//! （provider 测试 / 项目创建 / 隐私拦截这类全局操作）还没有调用方接进来，
//! 是诚实的边界，不是遗漏。
//!
//! **不做**日志查询/聚合——那是「日志中心」UI 直接对 `execution_logs` 建索引查询
//! 就够的事，不需要这个 crate 包一层。

pub mod error;

pub use error::{Error, Result};

use opc_storage::ProjectDb;

/// `execution_logs.kind` 已在 schema 注释里枚举过的取值，这里导出成常量避免
/// 调用方手敲字符串拼错。
pub mod kind {
    pub const LLM_CALL: &str = "llm.call";
    pub const TOOL_FILE_WRITE: &str = "tool.file.write";
    pub const TOOL_GIT_COMMIT: &str = "tool.git.commit";
    pub const PRIVACY_BLOCK: &str = "privacy.block";
    pub const REVIEW_DECISION: &str = "review.decision";
    pub const PERMISSION_GRANT: &str = "permission.grant";
}

/// `execution_logs.result` 的 CHECK 约束枚举值。
pub mod outcome {
    pub const OK: &str = "ok";
    pub const ERROR: &str = "error";
    pub const BLOCKED: &str = "blocked";
    pub const CONFIRMED: &str = "confirmed";
    pub const DENIED: &str = "denied";
    pub const SUPERSEDED: &str = "superseded";
}

/// 一条 `execution_logs` 行——字段对齐 `runtime/migrations/project/0001_init.sql`
/// 的 `execution_logs` 表。`kind`/`result` 必填，其余按事件类型按需填。
#[derive(Debug, Clone, Default)]
pub struct LogEvent<'a> {
    pub kind: &'a str,
    pub result: &'a str,
    pub task_id: Option<&'a str>,
    pub task_run_id: Option<&'a str>,
    pub agent_id: Option<&'a str>,
    pub provider_id: Option<&'a str>,
    pub tool: Option<&'a str>,
    pub input: Option<&'a str>,
    pub output: Option<&'a str>,
    pub error: Option<&'a str>,
    pub duration_ms: Option<i64>,
    pub bytes_in: Option<i64>,
    pub bytes_out: Option<i64>,
}

impl<'a> LogEvent<'a> {
    pub fn new(kind: &'a str, result: &'a str) -> Self {
        Self { kind, result, ..Default::default() }
    }
}

/// 追加一条审计日志，返回新行的 `id`。永远只 `INSERT`，不 `UPDATE`/`DELETE`
/// ——`execution_logs` 是 append-only 表，这是这个 crate 存在的唯一理由。
pub async fn record(db: &ProjectDb, event: LogEvent<'_>) -> Result<i64> {
    let id: (i64,) = sqlx::query_as(
        "INSERT INTO execution_logs \
         (task_id, task_run_id, agent_id, provider_id, kind, tool, input, output, result, error, duration_ms, bytes_in, bytes_out) \
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?) RETURNING id",
    )
    .bind(event.task_id)
    .bind(event.task_run_id)
    .bind(event.agent_id)
    .bind(event.provider_id)
    .bind(event.kind)
    .bind(event.tool)
    .bind(event.input)
    .bind(event.output)
    .bind(event.result)
    .bind(event.error)
    .bind(event.duration_ms)
    .bind(event.bytes_in)
    .bind(event.bytes_out)
    .fetch_one(&db.pool)
    .await?;
    Ok(id.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use opc_storage::ProjectDb;

    fn migrations_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("migrations/project")
    }

    async fn open_test_db() -> (tempfile::TempDir, ProjectDb) {
        let dir = tempfile::TempDir::new().unwrap();
        let db = ProjectDb::open(dir.path().join("project.sqlite"), migrations_root()).await.unwrap();
        (dir, db)
    }

    #[tokio::test]
    async fn records_an_llm_call_event_and_reads_it_back() {
        let (_dir, db) = open_test_db().await;
        let id = record(
            &db,
            LogEvent {
                agent_id: Some("agent-1"),
                provider_id: Some("test-api"),
                duration_ms: Some(123),
                ..LogEvent::new(kind::LLM_CALL, outcome::OK)
            },
        )
        .await
        .unwrap();
        assert!(id > 0);

        let (row_kind, row_result, row_agent): (String, String, Option<String>) =
            sqlx::query_as("SELECT kind, result, agent_id FROM execution_logs WHERE id = ?")
                .bind(id)
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert_eq!(row_kind, "llm.call");
        assert_eq!(row_result, "ok");
        assert_eq!(row_agent.as_deref(), Some("agent-1"));
    }

    #[tokio::test]
    async fn records_a_privacy_block_event() {
        let (_dir, db) = open_test_db().await;
        let id = record(
            &db,
            LogEvent {
                agent_id: Some("acceptance-instance"),
                error: Some("provider anthropic 不满足 sensitivity=high"),
                ..LogEvent::new(kind::PRIVACY_BLOCK, outcome::BLOCKED)
            },
        )
        .await
        .unwrap();

        let (row_result,): (String,) =
            sqlx::query_as("SELECT result FROM execution_logs WHERE id = ?").bind(id).fetch_one(&db.pool).await.unwrap();
        assert_eq!(row_result, "blocked");
    }
}

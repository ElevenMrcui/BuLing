//! 简易 SQLite migration 运行器。
//!
//! 语义：
//! - 迁移文件命名 `NNNN_<slug>.sql`，NNNN 为整数版本号（不 padding 也 OK，
//!   按数值排序）
//! - `_migrations` 表由本模块创建，记录 (version, name, applied_at)
//! - 每次 [`migrate`] 幂等：已应用（version <= current）的跳过
//! - 每份 SQL 用 `execute_many` 走 sqlite3 的多语句执行，兼容 CREATE TRIGGER
//!   等含内嵌 `;` 的语句
//! - **不**用 sqlx 的 `migrate!` 宏——避免与项目自定义追踪表冲突，也避免
//!   编译期需要真库的连锁问题

use futures_util::StreamExt;
use sqlx::{Executor, Pool, Sqlite};
use std::path::Path;
use tracing::info;

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct Migration {
    pub version: i64,
    pub name: String,
    pub sql: String,
}

pub async fn ensure_migrations_table(pool: &Pool<Sqlite>) -> Result<()> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS _migrations (
            version     INTEGER PRIMARY KEY,
            name        TEXT NOT NULL,
            applied_at  TEXT NOT NULL DEFAULT (datetime('now'))
        )"#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn current_version(pool: &Pool<Sqlite>) -> Result<i64> {
    let (v,): (i64,) = sqlx::query_as("SELECT COALESCE(MAX(version), 0) FROM _migrations")
        .fetch_one(pool)
        .await?;
    Ok(v)
}

pub fn load_migrations(dir: impl AsRef<Path>) -> Result<Vec<Migration>> {
    let dir = dir.as_ref();
    if !dir.is_dir() {
        return Err(Error::MigrationDirMissing(dir.display().to_string()));
    }
    let mut out: Vec<Migration> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".sql") {
            continue;
        }
        let (num_str, rest) = name.split_once('_').ok_or_else(|| Error::MigrationParse {
            file: name.clone(),
            reason: "expected NNNN_<slug>.sql".into(),
        })?;
        let version: i64 = num_str.parse().map_err(|_| Error::MigrationParse {
            file: name.clone(),
            reason: format!("cannot parse version from prefix {num_str:?}"),
        })?;
        let slug = rest.trim_end_matches(".sql").to_string();
        let sql = std::fs::read_to_string(entry.path())?;
        out.push(Migration { version, name: slug, sql });
    }
    out.sort_by_key(|m| m.version);
    Ok(out)
}

pub async fn migrate(pool: &Pool<Sqlite>, migrations: &[Migration]) -> Result<usize> {
    ensure_migrations_table(pool).await?;
    let current = current_version(pool).await?;
    let mut applied = 0usize;
    for m in migrations {
        if m.version <= current {
            continue;
        }
        info!(version = m.version, name = %m.name, "applying migration");
        // 逐份用同一连接跑，PRAGMA 与 CREATE TRIGGER 都由 execute_many 处理
        let mut conn = pool.acquire().await?;
        let mut stream = conn.execute_many(m.sql.as_str());
        while let Some(res) = stream.next().await {
            res?;
        }
        drop(stream);
        drop(conn);
        sqlx::query("INSERT INTO _migrations (version, name) VALUES (?, ?)")
            .bind(m.version)
            .bind(&m.name)
            .execute(pool)
            .await?;
        applied += 1;
    }
    Ok(applied)
}

/// 便捷入口：从目录加载 + 执行。
pub async fn migrate_from_dir(
    pool: &Pool<Sqlite>,
    dir: impl AsRef<Path>,
) -> Result<usize> {
    let migrations = load_migrations(dir)?;
    migrate(pool, &migrations).await
}

//! PROJECT 级 SQLite（`<project>/.opc/project.sqlite`）—— 每项目一份。

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::error::Result;
use crate::migrator;

/// 项目级数据库连接池。
#[derive(Clone)]
pub struct ProjectDb {
    pub pool: Pool<Sqlite>,
    pub path: PathBuf,
}

impl ProjectDb {
    /// 打开 / 创建项目库，并跑迁移。
    ///
    /// `migrations_dir` 指向 `runtime/migrations/project`。
    pub async fn open(
        path: impl AsRef<Path>,
        migrations_dir: impl AsRef<Path>,
    ) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await?;
        migrator::migrate_from_dir(&pool, migrations_dir).await?;
        Ok(Self { pool, path })
    }

    pub async fn migration_version(&self) -> Result<i64> {
        migrator::current_version(&self.pool).await
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}

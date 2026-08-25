//! APP 级 SQLite（`~/.opc/db.sqlite`）—— 跨项目全局数据。

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::error::Result;
use crate::migrator;

/// APP 级数据库连接池。
#[derive(Clone)]
pub struct AppDb {
    pub pool: Pool<Sqlite>,
    pub path: PathBuf,
}

impl AppDb {
    /// 打开 / 创建 APP 库，并跑迁移。
    ///
    /// `migrations_dir` 指向 `runtime/migrations/app`。P0 阶段调用方直接传路径；
    /// 未来 Tauri App 内嵌时会用 `include_dir!` 嵌进二进制，再转出临时目录。
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

    /// 当前应用到的迁移版本。
    pub async fn migration_version(&self) -> Result<i64> {
        migrator::current_version(&self.pool).await
    }

    /// 关闭连接池（sqlx 通常自动 drop 即可，此方法用于强制回收）。
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

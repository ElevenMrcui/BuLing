//! opc-storage 集成测试：打开两种库 · 跑迁移 · 断言表结构 · FTS5 触发器。

use opc_storage::{AppDb, ProjectDb};
use std::path::PathBuf;
use tempfile::TempDir;

fn migrations_root() -> PathBuf {
    // 相对 crate 根：./runtime/crates/opc-storage → 上溯两层到 runtime/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("migrations")
}

#[tokio::test]
async fn app_db_opens_and_migrates() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("app.sqlite");
    let db = AppDb::open(&db_path, migrations_root().join("app"))
        .await
        .expect("open app db");
    let v = db.migration_version().await.unwrap();
    assert!(v >= 1, "expected migration to have run, got version {v}");

    // 断言业务表存在
    let names: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    let set: std::collections::HashSet<String> = names.into_iter().map(|(n,)| n).collect();
    for t in [
        "_migrations",
        "agents",
        "cross_project_memory",
        "execution_logs",
        "projects",
        "providers",
        "settings",
        "user_profile",
    ] {
        assert!(set.contains(t), "app db missing table {t}, have: {set:?}");
    }

    // 幂等：再跑一次不应报错，也不该重复插入 _migrations
    let db2 = AppDb::open(&db_path, migrations_root().join("app"))
        .await
        .unwrap();
    assert_eq!(db.migration_version().await.unwrap(), db2.migration_version().await.unwrap());

    // 断言 seed 数据（user_profile 单行 + settings 五个默认键）
    let (name,): (String,) = sqlx::query_as("SELECT display_name FROM user_profile WHERE id = 1")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert!(!name.is_empty());
    let (settings_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM settings")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert!(settings_count >= 5, "expected >=5 seeded settings, got {settings_count}");
}

#[tokio::test]
async fn project_db_opens_and_migrates_with_fts5() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join(".opc/project.sqlite");
    let db = ProjectDb::open(&db_path, migrations_root().join("project"))
        .await
        .expect("open project db");

    // 关键表存在
    let names: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    let set: std::collections::HashSet<String> = names.into_iter().map(|(n,)| n).collect();
    for t in [
        "_migrations",
        "agent_instances",
        "artifact_refs",
        "artifact_versions",
        "artifacts",
        "execution_logs",
        "gates",
        "memory_entries",
        "memory_fts",
        "permissions",
        "project_meta",
        "reviews",
        "task_runs",
        "tasks",
        "teams",
        "workflows",
    ] {
        assert!(set.contains(t), "project db missing table {t}");
    }

    // FTS5 触发器：写一条 memory_entries → memory_fts 应命中
    sqlx::query(
        "INSERT INTO memory_entries (scope, key, value) VALUES ('project', 'api_style', 'REST')",
    )
    .execute(&db.pool)
    .await
    .unwrap();
    let hits: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT scope, key, value FROM memory_fts WHERE memory_fts MATCH 'REST'",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].0, "project");
    assert_eq!(hits[0].2, "REST");
}

#[tokio::test]
async fn migration_is_idempotent() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("app.sqlite");
    for _ in 0..3 {
        let db = AppDb::open(&db_path, migrations_root().join("app"))
            .await
            .unwrap();
        drop(db);
    }
    // 三次打开后 _migrations 只应有 1 条 version=1 记录
    let db = AppDb::open(&db_path, migrations_root().join("app"))
        .await
        .unwrap();
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM _migrations")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

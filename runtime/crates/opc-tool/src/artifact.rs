//! 把 [`fs_tool::write_text_file`] 的产物登记进 `project.sqlite` 的
//! `artifacts` + `artifact_versions`（append-only，见
//! `docs/OPC-数据模型.md` §3.8）。

use std::path::Path;

use opc_storage::ProjectDb;

use crate::error::Result;
use crate::fs_tool::{write_text_file, WrittenFile};

pub struct RegisterArtifactInput<'a> {
    pub kind: &'a str,
    pub name: &'a str,
    /// 项目根相对路径，如 `docs/product/PRD.md`。
    pub file_path: &'a str,
    pub mime: Option<&'a str>,
    pub producer_agent_id: Option<&'a str>,
    pub producer_task_id: Option<&'a str>,
    pub task_run_id: Option<&'a str>,
    pub change_note: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct ArtifactRecord {
    pub id: String,
    pub version: i64,
    pub file_hash: String,
    pub file_bytes: i64,
}

/// 写文件 + 登记版本，一个事务内完成（避免"文件写了但库没记"或反过来的半成品状态）。
///
/// 幂等语义：同一 `file_path` 再次写入 = 新版本（`artifacts.latest_version`
/// 递增、`artifact_versions` 追加一行），不覆盖旧版本记录——旧版本仍能从
/// `artifact_versions` 查到，只是不再是 latest。
pub async fn write_and_register_artifact(
    db: &ProjectDb,
    project_root: &Path,
    input: RegisterArtifactInput<'_>,
    content: &str,
) -> Result<ArtifactRecord> {
    let WrittenFile { file_hash, file_bytes, .. } = write_text_file(project_root, input.file_path, content).await?;

    let mut tx = db.pool.begin().await?;

    let existing: Option<(String, i64)> =
        sqlx::query_as("SELECT id, latest_version FROM artifacts WHERE file_path = ?")
            .bind(input.file_path)
            .fetch_optional(&mut *tx)
            .await?;

    let (artifact_id, version) = match existing {
        Some((id, latest_version)) => {
            let next_version = latest_version + 1;
            sqlx::query(
                "UPDATE artifacts SET kind=?, name=?, mime=?, latest_version=?, \
                 producer_agent_id=?, producer_task_id=?, updated_at=datetime('now') WHERE id=?",
            )
            .bind(input.kind)
            .bind(input.name)
            .bind(input.mime)
            .bind(next_version)
            .bind(input.producer_agent_id)
            .bind(input.producer_task_id)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
            (id, next_version)
        }
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO artifacts (id, kind, name, file_path, mime, latest_version, \
                 producer_agent_id, producer_task_id) VALUES (?,?,?,?,?,1,?,?)",
            )
            .bind(&id)
            .bind(input.kind)
            .bind(input.name)
            .bind(input.file_path)
            .bind(input.mime)
            .bind(input.producer_agent_id)
            .bind(input.producer_task_id)
            .execute(&mut *tx)
            .await?;
            (id, 1i64)
        }
    };

    sqlx::query(
        "INSERT INTO artifact_versions (artifact_id, version, file_hash, file_bytes, \
         author_agent_id, task_run_id, change_note) VALUES (?,?,?,?,?,?,?)",
    )
    .bind(&artifact_id)
    .bind(version)
    .bind(&file_hash)
    .bind(file_bytes)
    .bind(input.producer_agent_id)
    .bind(input.task_run_id)
    .bind(input.change_note)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(ArtifactRecord { id: artifact_id, version, file_hash, file_bytes })
}

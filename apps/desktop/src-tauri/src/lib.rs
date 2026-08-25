//! 不令 OPC 桌面应用后端（Tauri 2 · Rust）
//!
//! 这层是**薄命令壳**：接 Tauri IPC 请求 → 转发到 `opc-storage` 等 runtime
//! crate → 返回 serde 可序列化的响应。业务逻辑禁止塞这里，避免壳层变胖。

use std::path::PathBuf;

use opc_storage::AppDb;
use serde::Serialize;
use tauri::{Manager, State};
use tokio::sync::OnceCell;

/// 全局共享状态。
#[derive(Default)]
pub struct OpcState {
    app_db: OnceCell<AppDb>,
}

#[derive(Debug, Serialize)]
pub struct OpcStatus {
    pub app_db_version: i64,
    pub app_db_path: String,
    pub ready: bool,
}

fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("cannot resolve app_data_dir: {e}"))
}

fn migrations_dir_for(kind: &str, app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // 打包后：resource_dir/migrations/<kind>
    if let Ok(res_dir) = app.path().resource_dir() {
        let inside = res_dir.join(format!("migrations/{kind}"));
        if inside.is_dir() {
            return Ok(inside);
        }
    }
    // dev fallback：<crate>/../../../runtime/migrations/<kind>
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_root
        .join(format!("../../../runtime/migrations/{kind}"))
        .canonicalize()
        .map_err(|e| format!("dev migrations dir not found ({kind}): {e}"))
}

async fn get_app_db(
    app: &tauri::AppHandle,
    state: &State<'_, OpcState>,
) -> Result<AppDb, String> {
    let db_ref = state
        .app_db
        .get_or_try_init(|| async {
            let data_dir = app_data_dir(app)?;
            let db_path = data_dir.join("db.sqlite");
            let mig_dir = migrations_dir_for("app", app)?;
            AppDb::open(&db_path, &mig_dir)
                .await
                .map_err(|e| format!("AppDb::open({}): {e}", db_path.display()))
        })
        .await?;
    // AppDb 内部是 Arc<Pool>，克隆开销约等于 Arc::clone。
    Ok(db_ref.clone())
}

#[tauri::command]
async fn opc_status(
    app: tauri::AppHandle,
    state: State<'_, OpcState>,
) -> Result<OpcStatus, String> {
    let db = get_app_db(&app, &state).await?;
    let version = db
        .migration_version()
        .await
        .map_err(|e| format!("migration_version: {e}"))?;
    Ok(OpcStatus {
        app_db_version: version,
        app_db_path: db.path.display().to_string(),
        ready: version >= 1,
    })
}

pub fn run() {
    tauri::Builder::default()
        .manage(OpcState::default())
        .invoke_handler(tauri::generate_handler![opc_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

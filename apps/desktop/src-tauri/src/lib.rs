//! 不令 OPC 桌面应用后端（Tauri 2 · Rust）
//!
//! 这层是**薄命令壳**：接 Tauri IPC 请求 → 转发到 `opc-storage` / `opc-provider`
//! 等 runtime crate → 返回 serde 可序列化的响应。业务逻辑禁止塞这里，避免
//! 壳层变胖。

use std::path::PathBuf;
use std::sync::Arc;

use opc_provider::ProviderRegistry;
use opc_storage::AppDb;
use serde::Serialize;
use tauri::{Manager, State};
use tokio::sync::OnceCell;

/// 全局共享状态。
#[derive(Default)]
pub struct OpcState {
    app_db: OnceCell<AppDb>,
    provider_registry: OnceCell<Arc<ProviderRegistry>>,
}

#[derive(Debug, Serialize)]
pub struct OpcStatus {
    pub app_db_version: i64,
    pub app_db_path: String,
    pub ready: bool,
}

#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub display_name: String,
    pub vendor: String,
    pub kind: String,
    pub wire_format: Option<String>,
    pub available: bool,
    pub detail: Option<String>,
}

fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("cannot resolve app_data_dir: {e}"))
}

/// `resource_dir/<sub>` 优先（打包后）；dev 阶段 fallback 到源码树里的对应目录。
fn resolve_bundled_dir(app: &tauri::AppHandle, sub: &str) -> Result<PathBuf, String> {
    if let Ok(res_dir) = app.path().resource_dir() {
        let inside = res_dir.join(sub);
        if inside.is_dir() {
            return Ok(inside);
        }
    }
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_root
        .join(format!("../../../{sub}"))
        .canonicalize()
        .map_err(|e| format!("dev dir not found ({sub}): {e}"))
}

fn migrations_dir_for(kind: &str, app: &tauri::AppHandle) -> Result<PathBuf, String> {
    resolve_bundled_dir(app, &format!("runtime/migrations/{kind}"))
}

fn providers_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    resolve_bundled_dir(app, "providers")
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

async fn get_provider_registry(
    app: &tauri::AppHandle,
    state: &State<'_, OpcState>,
) -> Result<Arc<ProviderRegistry>, String> {
    let reg = state
        .provider_registry
        .get_or_try_init(|| async {
            let dir = providers_dir(app)?;
            ProviderRegistry::load_from_dir(&dir)
                .map(Arc::new)
                .map_err(|e| format!("ProviderRegistry::load_from_dir({}): {e}", dir.display()))
        })
        .await?;
    Ok(reg.clone())
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

/// 列出所有已知 Provider（`providers/*/manifest.toml`）及其可用性。
///
/// **只发现不执行**：CLI 只做 `command -v`，API/Local 只检查凭证是否能取到，
/// 不发起任何真实推理调用（对齐 `docs/本地网关.md` 的不变量）。
#[tauri::command]
async fn opc_providers(
    app: tauri::AppHandle,
    state: State<'_, OpcState>,
) -> Result<Vec<ProviderInfo>, String> {
    let registry = get_provider_registry(&app, &state).await?;

    let mut out = Vec::new();
    for manifest in registry.manifests() {
        let instance = registry
            .build(&manifest.id, None)
            .map_err(|e| format!("build({}): {e}", manifest.id))?;
        let status = instance.status().await;
        out.push(ProviderInfo {
            id: manifest.id.clone(),
            display_name: manifest.display_name.clone(),
            vendor: manifest.vendor.clone(),
            kind: format!("{:?}", manifest.kind).to_lowercase(),
            wire_format: manifest.wire_format.map(|w| format!("{w:?}")),
            available: status.available,
            detail: status.detail,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

pub fn run() {
    tauri::Builder::default()
        .manage(OpcState::default())
        .invoke_handler(tauri::generate_handler![opc_status, opc_providers])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

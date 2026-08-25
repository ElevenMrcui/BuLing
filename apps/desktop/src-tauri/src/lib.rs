//! 不令 OPC 桌面应用后端（Tauri 2 · Rust）
//!
//! 这层是**薄命令壳**：接 Tauri IPC 请求 → 转发到 `opc-storage` / `opc-provider` /
//! `opc-agent` 等 runtime crate → 返回 serde 可序列化的响应。业务逻辑禁止塞
//! 这里，避免壳层变胖。

use std::path::PathBuf;
use std::sync::Arc;

use opc_agent::{load_agents_from_dir, seed_agents};
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
    pub agents_seeded: i64,
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

#[derive(Debug, Serialize)]
pub struct AgentInfo {
    pub id: String,
    pub kind: String,
    pub role: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub sensitivity: String,
    pub provider_priority: Vec<String>,
    pub version: i64,
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

fn agents_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    resolve_bundled_dir(app, "agents")
}

/// 打开（或复用）APP 库，并在每次冷启动时重新播种预置 Agent
/// （`seed_agents` 幂等 · 不覆盖用户 fork 过的行，见 `opc-agent` 文档）。
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
            let db = AppDb::open(&db_path, &mig_dir)
                .await
                .map_err(|e| format!("AppDb::open({}): {e}", db_path.display()))?;

            let agents_root = agents_dir(app)?;
            let defs = load_agents_from_dir(&agents_root)
                .map_err(|e| format!("load_agents_from_dir({}): {e}", agents_root.display()))?;
            seed_agents(&db, &defs).await.map_err(|e| format!("seed_agents: {e}"))?;

            Ok::<AppDb, String>(db)
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
    let (agents_seeded,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents WHERE kind='preset'")
        .fetch_one(&db.pool)
        .await
        .map_err(|e| format!("count agents: {e}"))?;
    Ok(OpcStatus {
        app_db_version: version,
        app_db_path: db.path.display().to_string(),
        ready: version >= 1,
        agents_seeded,
    })
}

/// 列出所有已知 Provider（`providers/*/manifest.toml`）及其可用性。
///
/// **只发现不执行**：CLI 只做 `command -v`，API/Local 只检查凭证是否能取到，
/// 不发起任何真实推理调用（对齐 `docs/本地网关.md` 的不变量）。
///
/// **单个 Provider 构建失败不拖垮整个列表**——例如 codex-cli / gemini-cli /
/// aider 目前只有 manifest、没有实现 adapter，`registry.build()` 会报错；
/// 这里把这种情况当"该 Provider 不可用"处理，照样把其余 Provider 的状态
/// 返回给前端。
#[tauri::command]
async fn opc_providers(
    app: tauri::AppHandle,
    state: State<'_, OpcState>,
) -> Result<Vec<ProviderInfo>, String> {
    let registry = get_provider_registry(&app, &state).await?;

    let mut out = Vec::new();
    for manifest in registry.manifests() {
        let (available, detail) = match registry.build(&manifest.id, None) {
            Ok(instance) => {
                let status = instance.status().await;
                (status.available, status.detail)
            }
            Err(e) => (false, Some(e.to_string())),
        };
        out.push(ProviderInfo {
            id: manifest.id.clone(),
            display_name: manifest.display_name.clone(),
            vendor: manifest.vendor.clone(),
            kind: format!("{:?}", manifest.kind).to_lowercase(),
            wire_format: manifest.wire_format.map(|w| format!("{w:?}")),
            available,
            detail,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// 列出 `app.sqlite.agents` 里的全部岗位（预置 + 用户自建）。
#[tauri::command]
async fn opc_agents(app: tauri::AppHandle, state: State<'_, OpcState>) -> Result<Vec<AgentInfo>, String> {
    let db = get_app_db(&app, &state).await?;
    let rows: Vec<(String, String, String, String, Option<String>, String, String, i64)> = sqlx::query_as(
        "SELECT id, kind, role, display_name, avatar, sensitivity, provider_priority, version \
         FROM agents ORDER BY id",
    )
    .fetch_all(&db.pool)
    .await
    .map_err(|e| format!("list agents: {e}"))?;

    let out = rows
        .into_iter()
        .map(
            |(id, kind, role, display_name, avatar, sensitivity, provider_priority_json, version)| AgentInfo {
                id,
                kind,
                role,
                display_name,
                avatar,
                sensitivity,
                provider_priority: serde_json::from_str(&provider_priority_json).unwrap_or_default(),
                version,
            },
        )
        .collect();
    Ok(out)
}

pub fn run() {
    tauri::Builder::default()
        .manage(OpcState::default())
        .invoke_handler(tauri::generate_handler![opc_status, opc_providers, opc_agents])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

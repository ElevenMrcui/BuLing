//! 不令 OPC 桌面应用后端（Tauri 2 · Rust）
//!
//! 这层是**薄命令壳**：接 Tauri IPC 请求 → 转发到 `opc-storage` / `opc-provider` /
//! `opc-agent` 等 runtime crate → 返回 serde 可序列化的响应。业务逻辑禁止塞
//! 这里，避免壳层变胖。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use opc_agent::{load_agents_from_dir, seed_agents};
use opc_project::{create_project, find_agent_instance_id, list_projects, open_project, CreateProjectInput};
use opc_provider::ProviderRegistry;
use opc_storage::{AppDb, ProjectDb};
use serde::Serialize;
use tauri::{Manager, State};
use tokio::sync::{Mutex, OnceCell};

/// 全局共享状态。
#[derive(Default)]
pub struct OpcState {
    app_db: OnceCell<AppDb>,
    provider_registry: OnceCell<Arc<ProviderRegistry>>,
    /// 已打开过的项目库缓存（project_id → (ProjectDb, 项目根目录)），避免
    /// 每次工作流相关 IPC 调用都重新开一个 sqlx pool。
    open_projects: Mutex<HashMap<String, (ProjectDb, PathBuf)>>,
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

#[derive(Debug, Serialize)]
pub struct ProjectInfo {
    pub id: String,
    pub slug: String,
    pub display_name: String,
    pub root_path: String,
    pub status: String,
    pub starred: bool,
    pub last_opened_at: Option<String>,
    /// 建项目时如果传了 template_id，这里是同时实例化出的工作流 id。
    pub active_workflow_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TaskInfo {
    pub node_key: String,
    pub kind: String,
    pub role: Option<String>,
    pub assignment_mode: String,
    pub status: String,
}

impl From<opc_workflow::TaskRow> for TaskInfo {
    fn from(t: opc_workflow::TaskRow) -> Self {
        TaskInfo { node_key: t.node_key, kind: t.kind, role: t.role, assignment_mode: t.assignment_mode, status: t.status }
    }
}

#[derive(Debug, Serialize)]
pub struct GateInfo {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct TaskRunInfo {
    pub task_run_id: String,
    pub provider_id: String,
    pub artifact_ids: Vec<String>,
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

fn templates_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    resolve_bundled_dir(app, "templates")
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

/// 打开（或复用缓存的）项目库 + 项目根目录路径——工作流相关 IPC 命令共用。
async fn get_project_db(
    app: &tauri::AppHandle,
    state: &State<'_, OpcState>,
    project_id: &str,
) -> Result<(ProjectDb, PathBuf), String> {
    {
        let cache = state.open_projects.lock().await;
        if let Some((db, root)) = cache.get(project_id) {
            return Ok((db.clone(), root.clone()));
        }
    }

    let app_db = get_app_db(app, state).await?;
    let project_mig_dir = migrations_dir_for("project", app)?;
    let project_db =
        open_project(&app_db, project_id, &project_mig_dir).await.map_err(|e| format!("open_project: {e}"))?;
    let (root_path,): (String,) = sqlx::query_as("SELECT root_path FROM projects WHERE id = ?")
        .bind(project_id)
        .fetch_one(&app_db.pool)
        .await
        .map_err(|e| format!("query project root_path: {e}"))?;
    let root = PathBuf::from(root_path);

    let mut cache = state.open_projects.lock().await;
    cache.insert(project_id.to_string(), (project_db.clone(), root.clone()));
    Ok((project_db, root))
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

/// 建一个新项目：建目录 + project.sqlite + app.sqlite 注册 + 默认团队 +
/// 把全部预置 Agent 实例化进团队（见 `opc-project` 文档）。传 `template_id`
/// 会顺带实例化对应工作流（`opc-workflow`），`active_workflow_id` 带回给
/// 前端，后续工作流 IPC 命令都要传它。
#[tauri::command]
async fn opc_create_project(
    app: tauri::AppHandle,
    state: State<'_, OpcState>,
    slug: String,
    display_name: String,
    root_path: String,
    goal: Option<String>,
    template_id: Option<String>,
) -> Result<ProjectInfo, String> {
    let db = get_app_db(&app, &state).await?;

    let agents_root = agents_dir(&app)?;
    let defs = load_agents_from_dir(&agents_root)
        .map_err(|e| format!("load_agents_from_dir({}): {e}", agents_root.display()))?;

    let project_mig_dir = migrations_dir_for("project", &app)?;
    let created = create_project(
        &db,
        &project_mig_dir,
        &defs,
        CreateProjectInput {
            slug: &slug,
            display_name: &display_name,
            root_path: &PathBuf::from(&root_path),
            goal: goal.as_deref(),
            template_id: template_id.as_deref(),
        },
    )
    .await
    .map_err(|e| format!("create_project: {e}"))?;

    let mut active_workflow_id = None;
    if let Some(tid) = &template_id {
        let templates_root = templates_dir(&app)?;
        let templates = opc_workflow::load_templates_from_dir(&templates_root)
            .map_err(|e| format!("load_templates_from_dir({}): {e}", templates_root.display()))?;
        if let Some(tpl) = templates.into_iter().find(|t| &t.id == tid) {
            let instantiated = opc_workflow::instantiate_workflow(&created.project_db, &tpl)
                .await
                .map_err(|e| format!("instantiate_workflow: {e}"))?;
            active_workflow_id = Some(instantiated.workflow_id);
        }
    }

    {
        let mut cache = state.open_projects.lock().await;
        cache.insert(created.project_id.clone(), (created.project_db.clone(), PathBuf::from(&root_path)));
    }

    Ok(ProjectInfo {
        id: created.project_id,
        slug,
        display_name,
        root_path,
        status: "active".to_string(),
        starred: false,
        last_opened_at: None,
        active_workflow_id,
    })
}

/// 列出「项目中心」——所有已注册项目，最近打开的排前面。
#[tauri::command]
async fn opc_list_projects(app: tauri::AppHandle, state: State<'_, OpcState>) -> Result<Vec<ProjectInfo>, String> {
    let db = get_app_db(&app, &state).await?;
    let list = list_projects(&db).await.map_err(|e| format!("list_projects: {e}"))?;
    Ok(list
        .into_iter()
        .map(|p| ProjectInfo {
            id: p.id,
            slug: p.slug,
            display_name: p.display_name,
            root_path: p.root_path,
            status: p.status,
            starred: p.starred,
            last_opened_at: p.last_opened_at,
            // 列表页不逐个打开每个项目的 project.sqlite 去查——太贵。
            active_workflow_id: None,
        })
        .collect())
}

/// 列出一个工作流的全部任务节点（不筛可执行性，给 UI 画完整任务列表用）。
#[tauri::command]
async fn opc_workflow_tasks(app: tauri::AppHandle, state: State<'_, OpcState>, project_id: String, workflow_id: String) -> Result<Vec<TaskInfo>, String> {
    let (project_db, _root) = get_project_db(&app, &state, &project_id).await?;
    let rows: Vec<opc_workflow::TaskRow> = sqlx::query_as(
        "SELECT id, workflow_id, node_key, kind, role, assignment_mode, assigned_agent_id, status \
         FROM tasks WHERE workflow_id = ? ORDER BY rowid",
    )
    .bind(&workflow_id)
    .fetch_all(&project_db.pool)
    .await
    .map_err(|e| format!("list tasks: {e}"))?;
    Ok(rows.into_iter().map(TaskInfo::from).collect())
}

/// 列出当前可执行的 Agent 节点（依赖已满足 · assignment=template）。
#[tauri::command]
async fn opc_workflow_ready_tasks(app: tauri::AppHandle, state: State<'_, OpcState>, project_id: String, workflow_id: String) -> Result<Vec<TaskInfo>, String> {
    let (project_db, _root) = get_project_db(&app, &state, &project_id).await?;
    let ready = opc_workflow::list_ready_agent_tasks(&project_db, &workflow_id)
        .await
        .map_err(|e| format!("list_ready_agent_tasks: {e}"))?;
    Ok(ready.into_iter().map(TaskInfo::from).collect())
}

/// 真的跑一次可执行的 Agent 节点。
#[tauri::command]
async fn opc_workflow_run_task(
    app: tauri::AppHandle,
    state: State<'_, OpcState>,
    project_id: String,
    workflow_id: String,
    node_key: String,
) -> Result<TaskRunInfo, String> {
    let (project_db, project_root) = get_project_db(&app, &state, &project_id).await?;
    let registry = get_provider_registry(&app, &state).await?;
    let agents_root = agents_dir(&app)?;
    let agent_defs = load_agents_from_dir(&agents_root).map_err(|e| format!("load_agents_from_dir: {e}"))?;

    let ready = opc_workflow::list_ready_agent_tasks(&project_db, &workflow_id)
        .await
        .map_err(|e| format!("list_ready_agent_tasks: {e}"))?;
    let task = ready
        .into_iter()
        .find(|t| t.node_key == node_key)
        .ok_or_else(|| format!("节点「{node_key}」当前不可执行（依赖没满足，或不是 assignment=template 的 agent 节点）"))?;

    let dag = opc_workflow::load_dag(&project_db, &workflow_id).await.map_err(|e| format!("load_dag: {e}"))?;
    let node = dag.nodes.iter().find(|n| n.id == node_key).ok_or_else(|| format!("dag 里找不到节点「{node_key}」"))?;

    let result = opc_workflow::run_task_node(&project_db, &project_root, &registry, &agent_defs, &task, node)
        .await
        .map_err(|e| format!("run_task_node: {e}"))?;
    Ok(TaskRunInfo { task_run_id: result.task_run_id, provider_id: result.provider_id, artifact_ids: result.artifact_ids })
}

/// 列出一个工作流的全部 Gate 及其状态。
#[tauri::command]
async fn opc_workflow_gates(app: tauri::AppHandle, state: State<'_, OpcState>, project_id: String, workflow_id: String) -> Result<Vec<GateInfo>, String> {
    let (project_db, _root) = get_project_db(&app, &state, &project_id).await?;
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT node_key, status FROM gates WHERE workflow_id = ? ORDER BY node_key")
        .bind(&workflow_id)
        .fetch_all(&project_db.pool)
        .await
        .map_err(|e| format!("list gates: {e}"))?;
    Ok(rows.into_iter().map(|(id, status)| GateInfo { id, status }).collect())
}

/// 评审红线的唯一入口——通过一个 Gate。永远由用户在界面上点击触发，
/// Runtime 自己不会调这个命令。
#[tauri::command]
async fn opc_workflow_approve_gate(
    app: tauri::AppHandle,
    state: State<'_, OpcState>,
    project_id: String,
    workflow_id: String,
    gate_id: String,
    comment: Option<String>,
) -> Result<(), String> {
    let (project_db, _root) = get_project_db(&app, &state, &project_id).await?;
    opc_workflow::approve_gate(&project_db, &workflow_id, &gate_id, "user", comment.as_deref())
        .await
        .map_err(|e| format!("approve_gate: {e}"))
}

/// 打回一个 Gate——同样只能由用户显式触发。
#[tauri::command]
async fn opc_workflow_reject_gate(
    app: tauri::AppHandle,
    state: State<'_, OpcState>,
    project_id: String,
    workflow_id: String,
    gate_id: String,
    comment: Option<String>,
) -> Result<(), String> {
    let (project_db, _root) = get_project_db(&app, &state, &project_id).await?;
    opc_workflow::reject_gate(&project_db, &workflow_id, &gate_id, "user", comment.as_deref())
        .await
        .map_err(|e| format!("reject_gate: {e}"))
}

/// 列出当前可认领的 `auto-claim` 节点（依赖已满足）。
#[tauri::command]
async fn opc_task_claimable_tasks(app: tauri::AppHandle, state: State<'_, OpcState>, project_id: String, workflow_id: String) -> Result<Vec<TaskInfo>, String> {
    let (project_db, _root) = get_project_db(&app, &state, &project_id).await?;
    let rows = opc_task::list_claimable_tasks(&project_db, &workflow_id).await.map_err(|e| format!("list_claimable_tasks: {e}"))?;
    Ok(rows.into_iter().map(TaskInfo::from).collect())
}

/// 列出当前等待用户手动指派的 `manual` 节点（依赖已满足）。
#[tauri::command]
async fn opc_task_manual_tasks(app: tauri::AppHandle, state: State<'_, OpcState>, project_id: String, workflow_id: String) -> Result<Vec<TaskInfo>, String> {
    let (project_db, _root) = get_project_db(&app, &state, &project_id).await?;
    let rows = opc_task::list_manual_tasks(&project_db, &workflow_id).await.map_err(|e| format!("list_manual_tasks: {e}"))?;
    Ok(rows.into_iter().map(TaskInfo::from).collect())
}

/// 真的执行一次认领：按能力匹配分选出中标者，写 `assigned_agent_id`。
#[tauri::command]
async fn opc_task_claim(
    app: tauri::AppHandle,
    state: State<'_, OpcState>,
    project_id: String,
    workflow_id: String,
    node_key: String,
) -> Result<opc_task::ClaimScore, String> {
    let (project_db, _root) = get_project_db(&app, &state, &project_id).await?;
    let agents_root = agents_dir(&app)?;
    let agent_defs = load_agents_from_dir(&agents_root).map_err(|e| format!("load_agents_from_dir: {e}"))?;
    opc_task::claim_task(&project_db, &workflow_id, &node_key, &agent_defs).await.map_err(|e| format!("claim_task: {e}"))
}

/// 把一个 `manual` 节点指派给指定的团队成员——前端传预置岗位 id（如
/// `"backend"`，跟 `opc_agents` 返回的 id 一致），这里查这个项目团队里对应
/// 的 `agent_instances.id` 再指派，前端不用先知道内部实例 id。
#[tauri::command]
async fn opc_task_assign_manually(
    app: tauri::AppHandle,
    state: State<'_, OpcState>,
    project_id: String,
    workflow_id: String,
    node_key: String,
    template_agent_id: String,
) -> Result<(), String> {
    let (project_db, _root) = get_project_db(&app, &state, &project_id).await?;
    let agent_instance_id = find_agent_instance_id(&project_db, &template_agent_id)
        .await
        .map_err(|e| format!("find_agent_instance_id: {e}"))?
        .ok_or_else(|| format!("这个项目团队里没有「{template_agent_id}」这个岗位的实例"))?;
    opc_task::assign_task_manually(&project_db, &workflow_id, &node_key, &agent_instance_id)
        .await
        .map_err(|e| format!("assign_task_manually: {e}"))
}

/// 跑一次已经指派/认领好的 `manual`/`auto-claim` 节点。
#[tauri::command]
async fn opc_task_run(
    app: tauri::AppHandle,
    state: State<'_, OpcState>,
    project_id: String,
    workflow_id: String,
    node_key: String,
) -> Result<TaskRunInfo, String> {
    let (project_db, project_root) = get_project_db(&app, &state, &project_id).await?;
    let registry = get_provider_registry(&app, &state).await?;
    let agents_root = agents_dir(&app)?;
    let agent_defs = load_agents_from_dir(&agents_root).map_err(|e| format!("load_agents_from_dir: {e}"))?;

    let result = opc_task::run_assigned_task(&project_db, &project_root, &registry, &agent_defs, &workflow_id, &node_key)
        .await
        .map_err(|e| format!("run_assigned_task: {e}"))?;
    Ok(TaskRunInfo { task_run_id: result.task_run_id, provider_id: result.provider_id, artifact_ids: result.artifact_ids })
}

pub fn run() {
    tauri::Builder::default()
        .manage(OpcState::default())
        .invoke_handler(tauri::generate_handler![
            opc_status,
            opc_providers,
            opc_agents,
            opc_create_project,
            opc_list_projects,
            opc_workflow_tasks,
            opc_workflow_ready_tasks,
            opc_workflow_run_task,
            opc_workflow_gates,
            opc_workflow_approve_gate,
            opc_workflow_reject_gate,
            opc_task_claimable_tasks,
            opc_task_manual_tasks,
            opc_task_claim,
            opc_task_assign_manually,
            opc_task_run
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

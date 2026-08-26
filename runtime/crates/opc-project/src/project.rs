//! 创建 / 打开 / 列出项目，并把预置 Agent"实例化"进项目团队
//! （补上 `opc-tool::artifact` 需要的 `agent_instances` 行——见
//! `docs/OPC-架构决策.md` ADR-005 附注 3）。

use std::path::{Path, PathBuf};

use opc_agent::AgentDefinition;
use opc_storage::{AppDb, ProjectDb};

use crate::error::{Error, Result};

pub struct CreateProjectInput<'a> {
    pub slug: &'a str,
    pub display_name: &'a str,
    pub root_path: &'a Path,
    pub goal: Option<&'a str>,
    pub template_id: Option<&'a str>,
}

pub struct CreatedProject {
    pub project_id: String,
    pub team_id: String,
    pub project_db: ProjectDb,
}

/// 创建一个新项目：
/// 1. 建 `<root>/.opc/` 目录 + `project.sqlite`（跑 migration）
/// 2. 在 `app.sqlite.projects` 注册
/// 3. 在 `project.sqlite.project_meta` 写自描述
/// 4. 建一个默认 `team`
/// 5. 把 `agent_defs`（通常来自 `opc_agent::load_agents_from_dir()`）逐个
///    实例化进 `agent_instances`——这一步做完，`opc-tool` 的
///    `producer_agent_id` 外键才有真实数据可指
pub async fn create_project(
    app_db: &AppDb,
    project_migrations_dir: &Path,
    agent_defs: &[AgentDefinition],
    input: CreateProjectInput<'_>,
) -> Result<CreatedProject> {
    let root_path_str = input.root_path.to_string_lossy().to_string();

    let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM projects WHERE slug = ?")
        .bind(input.slug)
        .fetch_optional(&app_db.pool)
        .await?;
    if existing.is_some() {
        return Err(Error::SlugTaken(input.slug.to_string()));
    }

    tokio::fs::create_dir_all(input.root_path).await?;
    let db_path = input.root_path.join(".opc/project.sqlite");
    let project_db = ProjectDb::open(&db_path, project_migrations_dir).await?;

    let project_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO projects (id, slug, display_name, root_path, template_id, goal, last_opened_at) \
         VALUES (?,?,?,?,?,?, datetime('now'))",
    )
    .bind(&project_id)
    .bind(input.slug)
    .bind(input.display_name)
    .bind(&root_path_str)
    .bind(input.template_id)
    .bind(input.goal)
    .execute(&app_db.pool)
    .await?;

    sqlx::query(
        "INSERT INTO project_meta (id, project_id, slug, display_name, goal, template_id) \
         VALUES (1, ?, ?, ?, ?, ?)",
    )
    .bind(&project_id)
    .bind(input.slug)
    .bind(input.display_name)
    .bind(input.goal)
    .bind(input.template_id)
    .execute(&project_db.pool)
    .await?;

    let team_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO teams (id, name) VALUES (?, '默认团队')")
        .bind(&team_id)
        .execute(&project_db.pool)
        .await?;

    for def in agent_defs {
        let instance_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO agent_instances (id, team_id, template_agent_id, role, display_name, avatar) \
             VALUES (?,?,?,?,?,?)",
        )
        .bind(&instance_id)
        .bind(&team_id)
        .bind(&def.id)
        .bind(&def.role)
        .bind(&def.display_name)
        .bind(&def.avatar)
        .execute(&project_db.pool)
        .await?;
    }

    Ok(CreatedProject { project_id, team_id, project_db })
}

/// 打开一个已存在的项目（更新 `last_opened_at`，返回可用的 `ProjectDb`）。
pub async fn open_project(app_db: &AppDb, project_id: &str, project_migrations_dir: &Path) -> Result<ProjectDb> {
    let row: Option<(String,)> = sqlx::query_as("SELECT root_path FROM projects WHERE id = ?")
        .bind(project_id)
        .fetch_optional(&app_db.pool)
        .await?;
    let root_path = row.ok_or_else(|| Error::NotFound(project_id.to_string()))?.0;

    sqlx::query("UPDATE projects SET last_opened_at = datetime('now') WHERE id = ?")
        .bind(project_id)
        .execute(&app_db.pool)
        .await?;

    let db_path = PathBuf::from(root_path).join(".opc/project.sqlite");
    Ok(ProjectDb::open(&db_path, project_migrations_dir).await?)
}

#[derive(Debug, Clone)]
pub struct ProjectSummary {
    pub id: String,
    pub slug: String,
    pub display_name: String,
    pub root_path: String,
    pub status: String,
    pub starred: bool,
    pub last_opened_at: Option<String>,
}

/// 列出全部项目，最近打开的排前面（对应「项目中心」列表页）。
pub async fn list_projects(app_db: &AppDb) -> Result<Vec<ProjectSummary>> {
    let rows: Vec<(String, String, String, String, String, i64, Option<String>)> = sqlx::query_as(
        "SELECT id, slug, display_name, root_path, status, starred, last_opened_at \
         FROM projects ORDER BY last_opened_at DESC NULLS LAST, created_at DESC",
    )
    .fetch_all(&app_db.pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, slug, display_name, root_path, status, starred, last_opened_at)| ProjectSummary {
            id,
            slug,
            display_name,
            root_path,
            status,
            starred: starred != 0,
            last_opened_at,
        })
        .collect())
}

/// 查某个模板 Agent（`template_agent_id`，如 `"product-manager"`）在这个项目
/// 团队里对应的 `agent_instances.id`——`opc-tool` 登记 Artifact 时要用这个
/// id 当 `producer_agent_id`，不是 `agents/*.yaml` 里的预置 id。
pub async fn find_agent_instance_id(project_db: &ProjectDb, template_agent_id: &str) -> Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT id FROM agent_instances WHERE template_agent_id = ?")
        .bind(template_agent_id)
        .fetch_optional(&project_db.pool)
        .await?;
    Ok(row.map(|(id,)| id))
}

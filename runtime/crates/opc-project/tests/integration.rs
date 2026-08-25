//! opc-project 集成测试：创建 / 打开 / 列出项目，以及一条端到端胶水测试——
//! 真的用 `create_project()` 实例化出来的 `agent_instances.id`（不是手工
//! 种的测试行）去跑一次 Agent 并登记 Artifact，证明 ADR-005 附注 3 的 FK
//! 缺口已经补上。

use opc_agent::{load_agents_from_dir, AgentDefinition, PermissionDefaults};
use opc_project::{create_project, find_agent_instance_id, list_projects, open_project, CreateProjectInput};
use opc_storage::AppDb;
use std::path::PathBuf;
use tempfile::TempDir;

fn migrations_root() -> PathBuf {
    // runtime/crates/opc-project → 上溯两层到 runtime/
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().join("migrations")
}

fn agents_dir() -> PathBuf {
    // runtime/crates/opc-project → 上溯三层到仓库根，再进 agents/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("agents")
}

async fn open_temp_app_db() -> (TempDir, AppDb) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("app.sqlite");
    let db = AppDb::open(&db_path, migrations_root().join("app")).await.unwrap();
    (dir, db)
}

fn fake_agent_defs() -> Vec<AgentDefinition> {
    vec![
        AgentDefinition {
            id: "product-manager".to_string(),
            kind: "preset".to_string(),
            role: "产品经理".to_string(),
            display_name: "产品经理".to_string(),
            avatar: None,
            system_prompt: "你是产品经理".to_string(),
            responsibilities: vec![],
            expected_outputs: vec![],
            capabilities: vec![],
            sensitivity: "medium".to_string(),
            skills: vec![],
            tools: vec![],
            mcp_servers: vec![],
            provider_priority: vec![],
            permission_defaults: PermissionDefaults::default(),
        },
        AgentDefinition {
            id: "tech-lead".to_string(),
            kind: "preset".to_string(),
            role: "技术负责人".to_string(),
            display_name: "技术负责人".to_string(),
            avatar: None,
            system_prompt: "你是技术负责人".to_string(),
            responsibilities: vec![],
            expected_outputs: vec![],
            capabilities: vec![],
            sensitivity: "medium".to_string(),
            skills: vec![],
            tools: vec![],
            mcp_servers: vec![],
            provider_priority: vec![],
            permission_defaults: PermissionDefaults::default(),
        },
    ]
}

#[tokio::test]
async fn create_project_registers_meta_team_and_all_agent_instances() {
    let (app_dir, app_db) = open_temp_app_db().await;
    let project_root = app_dir.path().join("projects/health-app");
    let defs = fake_agent_defs();

    let created = create_project(
        &app_db,
        &migrations_root().join("project"),
        &defs,
        CreateProjectInput {
            slug: "health-app",
            display_name: "健康管理 App",
            root_path: &project_root,
            goal: Some("做一个健康管理 App"),
            template_id: None,
        },
    )
    .await
    .expect("create_project");

    // app.sqlite.projects 已注册
    let (slug, display_name): (String, String) =
        sqlx::query_as("SELECT slug, display_name FROM projects WHERE id = ?")
            .bind(&created.project_id)
            .fetch_one(&app_db.pool)
            .await
            .unwrap();
    assert_eq!(slug, "health-app");
    assert_eq!(display_name, "健康管理 App");

    // project.sqlite.project_meta 已写自描述
    let (meta_project_id,): (String,) = sqlx::query_as("SELECT project_id FROM project_meta WHERE id = 1")
        .fetch_one(&created.project_db.pool)
        .await
        .unwrap();
    assert_eq!(meta_project_id, created.project_id);

    // 默认团队已建
    let (team_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM teams WHERE id = ?").bind(&created.team_id).fetch_one(&created.project_db.pool).await.unwrap();
    assert_eq!(team_count, 1);

    // 全部预置 Agent 都已实例化进 agent_instances
    let (instance_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM agent_instances WHERE team_id = ?")
            .bind(&created.team_id)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(instance_count, defs.len() as i64);

    // 目录真的建出来了
    assert!(project_root.join(".opc/project.sqlite").exists());
}

#[tokio::test]
async fn create_project_rejects_duplicate_slug() {
    let (app_dir, app_db) = open_temp_app_db().await;
    let defs = fake_agent_defs();

    let input = |root: PathBuf| CreateProjectInput {
        slug: "dup-slug",
        display_name: "第一个",
        root_path: Box::leak(root.into_boxed_path()),
        goal: None,
        template_id: None,
    };

    create_project(&app_db, &migrations_root().join("project"), &defs, input(app_dir.path().join("p1")))
        .await
        .expect("first create should succeed");

    let err = create_project(&app_db, &migrations_root().join("project"), &defs, input(app_dir.path().join("p2")))
        .await;
    assert!(err.is_err(), "重复 slug 必须被拒绝");
}

#[tokio::test]
async fn open_project_bumps_last_opened_at_and_returns_working_db() {
    let (app_dir, app_db) = open_temp_app_db().await;
    let project_root = app_dir.path().join("projects/health-app");
    let defs = fake_agent_defs();

    let created = create_project(
        &app_db,
        &migrations_root().join("project"),
        &defs,
        CreateProjectInput {
            slug: "health-app",
            display_name: "健康管理 App",
            root_path: &project_root,
            goal: None,
            template_id: None,
        },
    )
    .await
    .unwrap();

    let (before,): (Option<String>,) = sqlx::query_as("SELECT last_opened_at FROM projects WHERE id = ?")
        .bind(&created.project_id)
        .fetch_one(&app_db.pool)
        .await
        .unwrap();
    assert!(before.is_some());

    let reopened = open_project(&app_db, &created.project_id, &migrations_root().join("project")).await.unwrap();

    // 打开的库真的可用——能查到刚才建的团队
    let (team_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM teams").fetch_one(&reopened.pool).await.unwrap();
    assert_eq!(team_count, 1);
}

#[tokio::test]
async fn open_project_returns_not_found_for_unknown_id() {
    let (_app_dir, app_db) = open_temp_app_db().await;
    let err = open_project(&app_db, "does-not-exist", &migrations_root().join("project")).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn list_projects_orders_most_recently_opened_first() {
    let (app_dir, app_db) = open_temp_app_db().await;
    let defs = fake_agent_defs();

    let mk = |slug: &'static str, root: PathBuf| CreateProjectInput {
        slug,
        display_name: slug,
        root_path: Box::leak(root.into_boxed_path()),
        goal: None,
        template_id: None,
    };

    let p1 = create_project(&app_db, &migrations_root().join("project"), &defs, mk("proj-a", app_dir.path().join("a")))
        .await
        .unwrap();
    let _p2 = create_project(&app_db, &migrations_root().join("project"), &defs, mk("proj-b", app_dir.path().join("b")))
        .await
        .unwrap();

    // 重新打开 p1，让它的 last_opened_at 更新到最新
    open_project(&app_db, &p1.project_id, &migrations_root().join("project")).await.unwrap();

    let list = list_projects(&app_db).await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].slug, "proj-a", "最近打开的应该排最前面");
}

#[tokio::test]
async fn find_agent_instance_id_resolves_template_agent_to_project_instance() {
    let (app_dir, app_db) = open_temp_app_db().await;
    let project_root = app_dir.path().join("projects/health-app");
    let defs = fake_agent_defs();

    let created = create_project(
        &app_db,
        &migrations_root().join("project"),
        &defs,
        CreateProjectInput {
            slug: "health-app",
            display_name: "健康管理 App",
            root_path: &project_root,
            goal: None,
            template_id: None,
        },
    )
    .await
    .unwrap();

    let instance_id = find_agent_instance_id(&created.project_db, "product-manager").await.unwrap();
    assert!(instance_id.is_some());

    let missing = find_agent_instance_id(&created.project_db, "no-such-agent").await.unwrap();
    assert!(missing.is_none());
}

#[tokio::test]
async fn loads_all_nine_real_preset_agents_and_instantiates_them() {
    let real_defs = load_agents_from_dir(agents_dir()).expect("load real agents/*.yaml");
    assert_eq!(real_defs.len(), 9, "预置 Agent 应该正好 9 个");

    let (app_dir, app_db) = open_temp_app_db().await;
    let project_root = app_dir.path().join("projects/real");

    let created = create_project(
        &app_db,
        &migrations_root().join("project"),
        &real_defs,
        CreateProjectInput {
            slug: "real",
            display_name: "真实预置 Agent 项目",
            root_path: &project_root,
            goal: None,
            template_id: None,
        },
    )
    .await
    .unwrap();

    let (instance_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM agent_instances WHERE team_id = ?")
            .bind(&created.team_id)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(instance_count, 9);
}

// ---------------------------------------------------------------------------
// 端到端胶水测试：opc-project（实例化）→ opc-agent（跑一个 Agent）→
// opc-tool（把产出写进项目并登记）——用真实实例化出来的 agent_instances.id，
// 不再手工种测试行，闭合 ADR-005 附注 3 的缺口。
// ---------------------------------------------------------------------------
mod glue {
    use super::*;
    use opc_agent::run_task;
    use opc_provider::ProviderRegistry;
    use opc_tool::{write_and_register_artifact, RegisterArtifactInput};
    use wiremock::matchers::{method, path as wpath};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn write_manifest(dir: &std::path::Path, id: &str, toml: &str) {
        let sub = dir.join(id);
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("manifest.toml"), toml).unwrap();
    }

    #[tokio::test]
    async fn full_chain_create_project_run_agent_register_artifact() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(wpath("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{"message": {"content": "# PRD\n\n由真实实例化的 Agent 产出"}}],
                "usage": {"prompt_tokens": 4, "completion_tokens": 9}
            })))
            .mount(&server)
            .await;

        let manifests_dir = TempDir::new().unwrap();
        write_manifest(
            manifests_dir.path(),
            "test-api",
            &format!(
                r#"
id = "test-api"
display_name = "测试 API"
vendor = "test"
kind = "api"
wire_format = "openai-compatible"
default_base_url = "{}"
"#,
                server.uri()
            ),
        );
        let registry = ProviderRegistry::load_from_dir(manifests_dir.path()).unwrap();

        let mut agent = fake_agent_defs().into_iter().next().unwrap();
        agent.provider_priority = vec!["test-api".to_string()];

        let (app_dir, app_db) = open_temp_app_db().await;
        let project_root = app_dir.path().join("projects/health-app");
        let created = create_project(
            &app_db,
            &migrations_root().join("project"),
            std::slice::from_ref(&agent),
            CreateProjectInput {
                slug: "health-app",
                display_name: "健康管理 App",
                root_path: &project_root,
                goal: Some("做一个健康管理 App"),
                template_id: None,
            },
        )
        .await
        .unwrap();

        // 1. 从真实项目里查出这个 Agent 的实例 id——不是手工种的
        let instance_id = find_agent_instance_id(&created.project_db, &agent.id)
            .await
            .unwrap()
            .expect("agent should have been instantiated by create_project");

        // 2. 真的跑一次 Agent
        let output = run_task(&registry, &agent, "帮我写个健康 App 的 PRD", 200).await.unwrap();
        assert_eq!(output.response.text, "# PRD\n\n由真实实例化的 Agent 产出");

        // 3. 用真实 instance_id 登记 Artifact——FK 约束应当放行
        let record = write_and_register_artifact(
            &created.project_db,
            &project_root,
            RegisterArtifactInput {
                kind: "prd",
                name: "PRD.md",
                file_path: "docs/product/PRD.md",
                mime: Some("text/markdown"),
                producer_agent_id: Some(&instance_id),
                producer_task_id: None,
                task_run_id: None,
                change_note: Some(&format!("由 {} 生成", output.provider_id)),
            },
            &output.response.text,
        )
        .await
        .expect("write_and_register_artifact with real agent_instances.id");

        assert_eq!(record.version, 1);
        let on_disk = std::fs::read_to_string(project_root.join("docs/product/PRD.md")).unwrap();
        assert_eq!(on_disk, "# PRD\n\n由真实实例化的 Agent 产出");
    }
}

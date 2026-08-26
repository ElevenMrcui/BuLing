//! opc-tool 集成测试：写文件 + 登记 artifact，含版本递增；以及一条端到端
//! 胶水测试——真的跑一个 Agent（wiremock 假 Provider）→ 把它的产出写进项目
//! 目录并登记进 project.sqlite，证明 opc-agent 与 opc-tool 能拼起来用。

use opc_storage::ProjectDb;
use opc_tool::{write_and_register_artifact, RegisterArtifactInput};
use std::path::PathBuf;
use tempfile::TempDir;

fn migrations_root() -> PathBuf {
    // runtime/crates/opc-tool → 上溯两层到 runtime/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("migrations")
}

async fn open_temp_project() -> (TempDir, ProjectDb) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join(".opc/project.sqlite");
    let db = ProjectDb::open(&db_path, migrations_root().join("project")).await.unwrap();
    (dir, db)
}

/// `artifacts.producer_agent_id` 外键指向 `agent_instances(id)`（项目内实例，
/// 不是 app.sqlite 里的预置 id）——测 Artifact 登记前先把这条"实例化"
/// 前置条件种下去，不能省，省了就是绕过外键约束而不是真正测通它。
async fn seed_minimal_agent_instance(db: &ProjectDb, instance_id: &str, role: &str) {
    sqlx::query("INSERT INTO teams (id, name) VALUES ('team-1', '默认团队')")
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO agent_instances (id, team_id, template_agent_id, role, display_name) \
         VALUES (?, 'team-1', ?, ?, ?)",
    )
    .bind(instance_id)
    .bind(instance_id)
    .bind(role)
    .bind(role)
    .execute(&db.pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn write_and_register_creates_artifact_and_first_version() {
    let (dir, db) = open_temp_project().await;
    seed_minimal_agent_instance(&db, "product-manager", "产品经理").await;

    let record = write_and_register_artifact(
        &db,
        dir.path(),
        RegisterArtifactInput {
            kind: "prd",
            name: "PRD.md",
            file_path: "docs/product/PRD.md",
            mime: Some("text/markdown"),
            producer_agent_id: Some("product-manager"),
            producer_task_id: None,
            task_run_id: None,
            change_note: Some("首次生成"),
        },
        "# PRD\n\n内容 v1",
    )
    .await
    .expect("write_and_register_artifact");

    assert_eq!(record.version, 1);

    // 文件真的落盘了
    let on_disk = std::fs::read_to_string(dir.path().join("docs/product/PRD.md")).unwrap();
    assert_eq!(on_disk, "# PRD\n\n内容 v1");

    // artifacts 表
    let (kind, latest_version, producer): (String, i64, Option<String>) =
        sqlx::query_as("SELECT kind, latest_version, producer_agent_id FROM artifacts WHERE id = ?")
            .bind(&record.id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(kind, "prd");
    assert_eq!(latest_version, 1);
    assert_eq!(producer.as_deref(), Some("product-manager"));

    // artifact_versions 表
    let (version_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM artifact_versions WHERE artifact_id = ?")
        .bind(&record.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(version_count, 1);
}

#[tokio::test]
async fn rewriting_same_file_path_bumps_version_and_keeps_history() {
    let (dir, db) = open_temp_project().await;
    seed_minimal_agent_instance(&db, "product-manager", "产品经理").await;

    let input = |note: &'static str| RegisterArtifactInput {
        kind: "prd",
        name: "PRD.md",
        file_path: "docs/product/PRD.md",
        mime: None,
        producer_agent_id: Some("product-manager"),
        producer_task_id: None,
        task_run_id: None,
        change_note: Some(note),
    };

    let v1 = write_and_register_artifact(&db, dir.path(), input("v1"), "内容 v1").await.unwrap();
    assert_eq!(v1.version, 1);

    let v2 = write_and_register_artifact(&db, dir.path(), input("v2 · 用户打回后重写"), "内容 v2").await.unwrap();
    assert_eq!(v2.version, 2);
    assert_eq!(v1.id, v2.id, "同一 file_path 应该是同一个 artifact，只是版本号变化");

    // 磁盘上是最新内容
    let on_disk = std::fs::read_to_string(dir.path().join("docs/product/PRD.md")).unwrap();
    assert_eq!(on_disk, "内容 v2");

    // artifacts.latest_version 指向 2
    let (latest_version,): (i64,) = sqlx::query_as("SELECT latest_version FROM artifacts WHERE id = ?")
        .bind(&v1.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(latest_version, 2);

    // 旧版本没被删——append-only，历史两条都在
    let notes: Vec<(i64, String)> =
        sqlx::query_as("SELECT version, change_note FROM artifact_versions WHERE artifact_id = ? ORDER BY version")
            .bind(&v1.id)
            .fetch_all(&db.pool)
            .await
            .unwrap();
    assert_eq!(notes, vec![(1, "v1".to_string()), (2, "v2 · 用户打回后重写".to_string())]);
}

#[tokio::test]
async fn rejects_writing_outside_project_root() {
    let (dir, db) = open_temp_project().await;

    let err = write_and_register_artifact(
        &db,
        dir.path(),
        RegisterArtifactInput {
            kind: "other",
            name: "evil",
            file_path: "../../etc/passwd",
            mime: None,
            producer_agent_id: None,
            producer_task_id: None,
            task_run_id: None,
            change_note: None,
        },
        "pwned",
    )
    .await;
    assert!(err.is_err(), "写到项目根之外必须被拒绝");

    // 且确实什么都没写进库
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM artifacts").fetch_one(&db.pool).await.unwrap();
    assert_eq!(count, 0);
}

// ---------------------------------------------------------------------------
// 端到端胶水测试：opc-agent（跑一个 Agent）→ opc-tool（把产出写进项目并登记）
// ---------------------------------------------------------------------------

mod glue {
    use super::*;
    use opc_agent::{run_task, AgentDefinition, PermissionDefaults};
    use opc_provider::ProviderRegistry;
    use wiremock::matchers::{method, path as wpath};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn write_manifest(dir: &std::path::Path, id: &str, toml: &str) {
        let sub = dir.join(id);
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("manifest.toml"), toml).unwrap();
    }

    #[tokio::test]
    async fn agent_output_lands_as_a_real_artifact_in_the_project() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(wpath("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{"message": {"content": "# PRD\n\n由 Agent 真实产出的内容"}}],
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

        let agent = AgentDefinition {
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
            provider_priority: vec!["test-api".to_string()],
            permission_defaults: PermissionDefaults::default(),
        };

        // 1. 真的跑一次 Agent
        let output = run_task(&registry, &agent, "帮我写个健康 App 的 PRD", 200).await.unwrap();
        assert_eq!(output.response.text, "# PRD\n\n由 Agent 真实产出的内容");

        // 2. 把产出写进项目并登记
        let (dir, db) = open_temp_project().await;
        seed_minimal_agent_instance(&db, &agent.id, &agent.role).await;
        let record = write_and_register_artifact(
            &db,
            dir.path(),
            RegisterArtifactInput {
                kind: "prd",
                name: "PRD.md",
                file_path: "docs/product/PRD.md",
                mime: Some("text/markdown"),
                producer_agent_id: Some(&agent.id),
                producer_task_id: None,
                task_run_id: None,
                change_note: Some(&format!("由 {} 生成", output.provider_id)),
            },
            &output.response.text,
        )
        .await
        .unwrap();

        // 3. 落盘内容与 Agent 产出一致
        let on_disk = std::fs::read_to_string(dir.path().join("docs/product/PRD.md")).unwrap();
        assert_eq!(on_disk, "# PRD\n\n由 Agent 真实产出的内容");
        assert_eq!(record.version, 1);
    }
}

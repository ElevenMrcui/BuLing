//! opc-task 集成测试：`auto-claim` 认领打分、`manual` 手动指派、驱动已指派
//! 节点执行——含一条用真实模板证明"能力边界诚实生效"的测试，和一条用
//! 合成模板证明"指派+执行全链路真的能跑通"的端到端测试。

use std::path::PathBuf;

use opc_agent::{load_agents_from_dir, AgentDefinition};
use opc_project::{create_project, find_agent_instance_id, CreateProjectInput, CreatedProject};
use opc_provider::ProviderRegistry;
use opc_storage::AppDb;
use opc_task::{assign_task_manually, claim_task, compute_claim_scores, list_claimable_tasks, list_manual_tasks, run_assigned_task};
use opc_workflow::{instantiate_workflow, load_templates_from_dir, NodeOutput, TemplateNode, WorkflowTemplate};
use tempfile::TempDir;
use wiremock::matchers::{method, path as wpath};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn runtime_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().to_path_buf()
}

fn repo_root() -> PathBuf {
    runtime_root().parent().unwrap().to_path_buf()
}

fn migrations_root() -> PathBuf {
    runtime_root().join("migrations")
}

fn real_template() -> WorkflowTemplate {
    let templates = load_templates_from_dir(repo_root().join("templates")).expect("load templates/*.yaml");
    templates.into_iter().find(|t| t.id == "standard-software-delivery").expect("standard-software-delivery.yaml")
}

fn real_agents() -> Vec<AgentDefinition> {
    load_agents_from_dir(repo_root().join("agents")).expect("load agents/*.yaml")
}

async fn setup_project() -> (TempDir, AppDb, CreatedProject, PathBuf) {
    let dir = TempDir::new().unwrap();
    let app_db = AppDb::open(dir.path().join("app.sqlite"), migrations_root().join("app")).await.unwrap();
    let defs = real_agents();
    let project_root = dir.path().join("projects/health-app");
    let created = create_project(
        &app_db,
        &migrations_root().join("project"),
        &defs,
        CreateProjectInput {
            slug: "health-app",
            display_name: "健康管理 App",
            root_path: &project_root,
            goal: Some("做一个健康管理 App"),
            template_id: Some("standard-software-delivery"),
        },
    )
    .await
    .unwrap();
    (dir, app_db, created, project_root)
}

async fn mark_completed(project_db: &opc_storage::ProjectDb, workflow_id: &str, node_keys: &[&str]) {
    for key in node_keys {
        sqlx::query("UPDATE tasks SET status = 'completed' WHERE workflow_id = ? AND node_key = ?")
            .bind(workflow_id)
            .bind(key)
            .execute(&project_db.pool)
            .await
            .unwrap();
    }
}

#[test]
fn compute_claim_scores_ranks_by_capability_overlap_with_hint_role() {
    let agent_defs = real_agents();
    let team = vec![
        ("instance-frontend".to_string(), "frontend".to_string()),
        ("instance-backend".to_string(), "backend".to_string()),
        ("instance-qa".to_string(), "qa".to_string()),
    ];

    let scores = compute_claim_scores("frontend", &agent_defs, &team);

    assert_eq!(scores.len(), 3);
    assert_eq!(scores[0].template_agent_id, "frontend");
    assert_eq!(scores[0].score, 4, "frontend 跟自己的能力提示 100% 重合");
    // backend / qa 都只在 "testing" 上重合，同分按 template_agent_id 升序排
    assert_eq!(scores[1].template_agent_id, "backend");
    assert_eq!(scores[1].score, 1);
    assert_eq!(scores[2].template_agent_id, "qa");
    assert_eq!(scores[2].score, 1);
}

#[tokio::test]
async fn claim_task_assigns_highest_scoring_agent_and_records_scores() {
    let (_dir, _app_db, created, _root) = setup_project().await;
    let template = real_template();
    let agent_defs = real_agents();
    let instantiated = instantiate_workflow(&created.project_db, &template).await.unwrap();

    // frontend_dev 依赖 planning + design——直接把这两个前置节点标完成，
    // 不需要真的重跑一遍它们（那是 opc-workflow 自己测过的事）。
    mark_completed(&created.project_db, &instantiated.workflow_id, &["prd", "architecture", "planning", "design"]).await;

    let winner = claim_task(&created.project_db, &instantiated.workflow_id, "frontend_dev", &agent_defs).await.unwrap();
    assert_eq!(winner.template_agent_id, "frontend");

    let (assigned, status, claim_scores): (Option<String>, String, Option<String>) = sqlx::query_as(
        "SELECT assigned_agent_id, status, claim_scores FROM tasks WHERE workflow_id = ? AND node_key = 'frontend_dev'",
    )
    .bind(&instantiated.workflow_id)
    .fetch_one(&created.project_db.pool)
    .await
    .unwrap();
    assert_eq!(assigned, Some(winner.agent_instance_id.clone()));
    assert_eq!(status, "assigned");
    let scores: std::collections::HashMap<String, i64> = serde_json::from_str(&claim_scores.unwrap()).unwrap();
    assert_eq!(scores.get("frontend"), Some(&4));
}

#[tokio::test]
async fn claim_task_rejects_when_dependencies_not_satisfied() {
    let (_dir, _app_db, created, _root) = setup_project().await;
    let template = real_template();
    let agent_defs = real_agents();
    let instantiated = instantiate_workflow(&created.project_db, &template).await.unwrap();

    // 不标记 planning/design 完成——frontend_dev 依赖没满足
    let err = claim_task(&created.project_db, &instantiated.workflow_id, "frontend_dev", &agent_defs).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn claim_task_rejects_non_auto_claim_node() {
    let (_dir, _app_db, created, _root) = setup_project().await;
    let template = real_template();
    let agent_defs = real_agents();
    let instantiated = instantiate_workflow(&created.project_db, &template).await.unwrap();

    // prd 是 assignment=template，不是 auto-claim
    let err = claim_task(&created.project_db, &instantiated.workflow_id, "prd", &agent_defs).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn list_claimable_tasks_only_returns_ready_auto_claim_nodes() {
    let (_dir, _app_db, created, _root) = setup_project().await;
    let template = real_template();
    let instantiated = instantiate_workflow(&created.project_db, &template).await.unwrap();

    assert!(list_claimable_tasks(&created.project_db, &instantiated.workflow_id).await.unwrap().is_empty());

    mark_completed(&created.project_db, &instantiated.workflow_id, &["prd", "architecture", "planning", "design"]).await;
    let claimable = list_claimable_tasks(&created.project_db, &instantiated.workflow_id).await.unwrap();
    let keys: Vec<&str> = claimable.iter().map(|t| t.node_key.as_str()).collect();
    assert!(keys.contains(&"frontend_dev"));
    assert!(keys.contains(&"backend_dev"));
}

#[tokio::test]
async fn assign_task_manually_sets_assignee_and_status() {
    let (_dir, _app_db, created, _root) = setup_project().await;
    let template = real_template();
    let instantiated = instantiate_workflow(&created.project_db, &template).await.unwrap();

    // bug_fix 依赖 qa_test（从 inputs 推导出来的隐式依赖）
    mark_completed(&created.project_db, &instantiated.workflow_id, &["qa_test"]).await;

    let backend_instance = find_agent_instance_id(&created.project_db, "backend").await.unwrap().unwrap();
    assign_task_manually(&created.project_db, &instantiated.workflow_id, "bug_fix", &backend_instance).await.unwrap();

    let (assigned, status): (Option<String>, String) =
        sqlx::query_as("SELECT assigned_agent_id, status FROM tasks WHERE workflow_id = ? AND node_key = 'bug_fix'")
            .bind(&instantiated.workflow_id)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(assigned, Some(backend_instance));
    assert_eq!(status, "assigned");
}

#[tokio::test]
async fn assign_task_manually_rejects_unknown_agent_instance() {
    let (_dir, _app_db, created, _root) = setup_project().await;
    let template = real_template();
    let instantiated = instantiate_workflow(&created.project_db, &template).await.unwrap();
    mark_completed(&created.project_db, &instantiated.workflow_id, &["qa_test"]).await;

    let err = assign_task_manually(&created.project_db, &instantiated.workflow_id, "bug_fix", "does-not-exist").await;
    assert!(err.is_err(), "不存在的 agent_instance_id 应该被外键约束拒绝");
}

#[tokio::test]
async fn list_manual_tasks_only_returns_ready_manual_nodes() {
    let (_dir, _app_db, created, _root) = setup_project().await;
    let template = real_template();
    let instantiated = instantiate_workflow(&created.project_db, &template).await.unwrap();

    assert!(list_manual_tasks(&created.project_db, &instantiated.workflow_id).await.unwrap().is_empty());
    mark_completed(&created.project_db, &instantiated.workflow_id, &["qa_test"]).await;
    let manual = list_manual_tasks(&created.project_db, &instantiated.workflow_id).await.unwrap();
    assert_eq!(manual.len(), 1);
    assert_eq!(manual[0].node_key, "bug_fix");
}

fn write_manifest(dir: &std::path::Path, id: &str, base_url: &str) {
    let sub = dir.join(id);
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(
        sub.join("manifest.toml"),
        format!(
            r#"
id = "{id}"
display_name = "测试 API"
vendor = "test"
kind = "api"
wire_format = "openai-compatible"
default_base_url = "{base_url}"
"#
        ),
    )
    .unwrap();
}

/// **诚实标注的能力边界**：真实模板里唯一能被认领的 `frontend_dev`
/// output 是 `frontend/**`（整个目录的 glob 契约），`run_assigned_task`
/// 复用 `opc_workflow::run_task_node` 的单文件模型，对这种节点必须正确
/// 拒绝——不能把 LLM 吐出的文本错写进一个字面叫 `frontend/**` 的文件。
#[tokio::test]
async fn run_assigned_task_rejects_glob_shaped_output_instead_of_writing_garbage() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wpath("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "// 前端代码"}}],
            "usage": {"prompt_tokens": 4, "completion_tokens": 9}
        })))
        .mount(&server)
        .await;
    let manifests_dir = TempDir::new().unwrap();
    write_manifest(manifests_dir.path(), "test-api", &server.uri());
    let registry = ProviderRegistry::load_from_dir(manifests_dir.path()).unwrap();

    let (_dir, _app_db, created, project_root) = setup_project().await;
    let template = real_template();
    let mut agent_defs = real_agents();
    for a in &mut agent_defs {
        a.provider_priority = vec!["test-api".to_string()];
    }
    let instantiated = instantiate_workflow(&created.project_db, &template).await.unwrap();
    mark_completed(&created.project_db, &instantiated.workflow_id, &["prd", "architecture", "planning", "design"]).await;
    claim_task(&created.project_db, &instantiated.workflow_id, "frontend_dev", &agent_defs).await.unwrap();

    let err = run_assigned_task(&created.project_db, &project_root, &registry, &agent_defs, &instantiated.workflow_id, "frontend_dev")
        .await;
    assert!(err.is_err(), "frontend/** 是整目录 glob，不应该被当成字面文件路径写出去");

    // 且确实什么文件都没写出来
    assert!(!project_root.join("frontend/**").exists());
    assert!(!project_root.join("frontend").exists());
}

/// 端到端：合成一个"正常单文件输出"的 manual 节点（真实模板里的
/// manual/auto-claim 节点全是 glob 输出，没法演示成功路径），证明
/// "手动指派 → 真的跑 → 落盘登记 Artifact"这条链本身是通的。
#[tokio::test]
async fn full_chain_assign_manually_then_run_writes_real_artifact() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wpath("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "# 手动指派任务的真实产出"}}],
            "usage": {"prompt_tokens": 4, "completion_tokens": 9}
        })))
        .mount(&server)
        .await;
    let manifests_dir = TempDir::new().unwrap();
    write_manifest(manifests_dir.path(), "test-api", &server.uri());
    let registry = ProviderRegistry::load_from_dir(manifests_dir.path()).unwrap();

    let (_dir, _app_db, created, project_root) = setup_project().await;
    let mut agent_defs = real_agents();
    for a in &mut agent_defs {
        a.provider_priority = vec!["test-api".to_string()];
    }

    let synthetic = WorkflowTemplate {
        id: "synthetic-manual".to_string(),
        name: "合成测试模板".to_string(),
        description: "只为证明 opc-task 指派+执行全链路能跑通".to_string(),
        version: 1,
        required_roles: vec!["frontend".to_string()],
        nodes: vec![TemplateNode {
            id: "manual_task".to_string(),
            kind: "agent".to_string(),
            role: Some("frontend".to_string()),
            assignment: Some("manual".to_string()),
            inputs: vec![],
            outputs: vec![NodeOutput { kind: "note".to_string(), path: vec!["docs/manual/Note.md".to_string()] }],
            depends_on: vec![],
            gate: None,
            subject: vec![],
            reviewer: None,
            on_reject_goto: vec![],
            condition: None,
            on_true_goto: vec![],
            on_false_goto: vec![],
        }],
        gates: vec![],
    };

    let instantiated = instantiate_workflow(&created.project_db, &synthetic).await.unwrap();
    let frontend_instance = find_agent_instance_id(&created.project_db, "frontend").await.unwrap().unwrap();

    assign_task_manually(&created.project_db, &instantiated.workflow_id, "manual_task", &frontend_instance).await.unwrap();

    let result = run_assigned_task(
        &created.project_db,
        &project_root,
        &registry,
        &agent_defs,
        &instantiated.workflow_id,
        "manual_task",
    )
    .await
    .unwrap();
    assert_eq!(result.artifact_ids.len(), 1);

    let on_disk = std::fs::read_to_string(project_root.join("docs/manual/Note.md")).unwrap();
    assert_eq!(on_disk, "# 手动指派任务的真实产出");

    let (status,): (String,) =
        sqlx::query_as("SELECT status FROM tasks WHERE workflow_id = ? AND node_key = 'manual_task'")
            .bind(&instantiated.workflow_id)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(status, "completed");
}

//! opc-workflow 集成测试：加载真实模板、实例化进项目、驱动可执行的 Agent
//! 节点、人工 Gate 通过/打回。

use std::collections::HashSet;
use std::path::PathBuf;

use opc_agent::{load_agents_from_dir, AgentDefinition};
use opc_project::{create_project, CreateProjectInput, CreatedProject};
use opc_provider::ProviderRegistry;
use opc_storage::AppDb;
use opc_workflow::{
    approve_gate, instantiate_workflow, list_ready_agent_tasks, load_templates_from_dir, reject_gate, ready_node_keys,
    run_task_node, WorkflowTemplate,
};
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

#[test]
fn loads_real_template_with_flattened_parallel_group_and_gates() {
    let template = real_template();
    assert_eq!(template.id, "standard-software-delivery");
    assert_eq!(template.required_roles.len(), 9);
    // parallel 分组（frontend_dev/backend_dev）被拍平成独立节点
    assert_eq!(template.nodes.len(), 16, "16 个拍平后的节点");
    assert_eq!(template.gates.len(), 4);

    let frontend_dev = template.nodes.iter().find(|n| n.id == "frontend_dev").unwrap();
    assert_eq!(frontend_dev.assignment.as_deref(), Some("auto-claim"));
    // parallel 组级 depends_on: [planning, design] 应该并入子节点
    assert!(frontend_dev.depends_on.contains(&"planning".to_string()));
    assert!(frontend_dev.depends_on.contains(&"design".to_string()));

    // human 节点没有显式 depends_on 时，退化为 subject
    let prd_review = template.nodes.iter().find(|n| n.id == "prd_review").unwrap();
    assert_eq!(prd_review.kind, "human");
    assert_eq!(prd_review.depends_on, vec!["prd".to_string()]);
    assert_eq!(prd_review.on_reject_goto, vec!["prd".to_string()]);

    // 入口节点没有依赖
    let prd = template.nodes.iter().find(|n| n.id == "prd").unwrap();
    assert!(prd.depends_on.is_empty());
}

#[test]
fn ready_node_keys_only_returns_entry_node_when_nothing_completed() {
    let template = real_template();
    let ready = ready_node_keys(&template, &HashSet::new());
    assert!(ready.contains(&"prd".to_string()));
    assert!(!ready.contains(&"prd_review".to_string()), "prd_review 依赖 prd，还没就绪");
    assert!(!ready.contains(&"tech_selection".to_string()));
}

#[test]
fn ready_node_keys_unlocks_prd_review_once_prd_completed() {
    let template = real_template();
    let mut completed = HashSet::new();
    completed.insert("prd".to_string());
    let ready = ready_node_keys(&template, &completed);
    assert!(ready.contains(&"prd_review".to_string()));
    assert!(!ready.contains(&"tech_selection".to_string()), "tech_selection 依赖 prd_review 而不是 prd");
}

#[tokio::test]
async fn instantiate_workflow_creates_tasks_and_gates_and_resolves_template_agents() {
    let (_dir, _app_db, created, _project_root) = setup_project().await;
    let template = real_template();

    let instantiated = instantiate_workflow(&created.project_db, &template).await.unwrap();

    let (task_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE workflow_id = ?")
        .bind(&instantiated.workflow_id)
        .fetch_one(&created.project_db.pool)
        .await
        .unwrap();
    assert_eq!(task_count, 16);

    let (gate_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM gates WHERE workflow_id = ?")
        .bind(&instantiated.workflow_id)
        .fetch_one(&created.project_db.pool)
        .await
        .unwrap();
    assert_eq!(gate_count, 4);

    // prd 节点 assignment=template，实例化时应该已经解析出 agent_instances.id
    let (prd_assigned,): (Option<String>,) =
        sqlx::query_as("SELECT assigned_agent_id FROM tasks WHERE workflow_id = ? AND node_key = 'prd'")
            .bind(&instantiated.workflow_id)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert!(prd_assigned.is_some());

    // human 节点不解析 assigned_agent_id
    let (review_assigned,): (Option<String>,) =
        sqlx::query_as("SELECT assigned_agent_id FROM tasks WHERE workflow_id = ? AND node_key = 'prd_review'")
            .bind(&instantiated.workflow_id)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert!(review_assigned.is_none());

    // auto-claim 节点（frontend_dev）也不解析
    let (frontend_assigned,): (Option<String>,) =
        sqlx::query_as("SELECT assigned_agent_id FROM tasks WHERE workflow_id = ? AND node_key = 'frontend_dev'")
            .bind(&instantiated.workflow_id)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert!(frontend_assigned.is_none());
}

#[tokio::test]
async fn list_ready_agent_tasks_only_returns_entry_node_initially() {
    let (_dir, _app_db, created, _project_root) = setup_project().await;
    let template = real_template();
    let instantiated = instantiate_workflow(&created.project_db, &template).await.unwrap();

    let ready = list_ready_agent_tasks(&created.project_db, &instantiated.workflow_id).await.unwrap();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].node_key, "prd");
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

/// 端到端：实例化 → 跑 `prd` 节点（wiremock 假 Provider）→ 产出的 3 个
/// output 都落盘登记 → `prd` 从 ready 集合消失，`prd_review`（human）
/// 依旧不会被自动推进（评审红线）→ 人工 approve_gate 之后 `tech_selection`
/// 才进入可执行集合。
#[tokio::test]
async fn full_chain_run_entry_node_then_gate_unlocks_next_phase() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wpath("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "# PRD\n\n真实产出内容"}}],
            "usage": {"prompt_tokens": 4, "completion_tokens": 9}
        })))
        .mount(&server)
        .await;

    let manifests_dir = TempDir::new().unwrap();
    write_manifest(manifests_dir.path(), "test-api", &server.uri());
    let registry = ProviderRegistry::load_from_dir(manifests_dir.path()).unwrap();

    let (_dir, _app_db, created, project_root) = setup_project().await;
    let mut template = real_template();
    // 把入口节点的 provider 换成假 Provider，绕开"需要真实 CLI/Key"的依赖
    let mut agent_defs = real_agents();
    for a in &mut agent_defs {
        a.provider_priority = vec!["test-api".to_string()];
    }
    // template.rs 不带 provider_priority 字段（那是 AgentDefinition 的字段，不是模板节点的），
    // 上面已经把全部预置 Agent 的 provider_priority 换成测试用 Provider。
    let _ = &mut template; // template 本身不需要改

    let instantiated = instantiate_workflow(&created.project_db, &template).await.unwrap();

    let ready = list_ready_agent_tasks(&created.project_db, &instantiated.workflow_id).await.unwrap();
    let prd_task = ready.into_iter().find(|t| t.node_key == "prd").unwrap();
    let prd_node = template.nodes.iter().find(|n| n.id == "prd").unwrap();

    let result =
        run_task_node(&created.project_db, &project_root, &registry, &agent_defs, &prd_task, prd_node)
            .await
            .unwrap();
    assert_eq!(result.artifact_ids.len(), 3, "prd 节点声明了 3 个 output");

    // 3 份文件都真的落盘且内容一致
    for path in ["docs/product/PRD.md", "docs/product/Acceptance-Criteria.md", "docs/product/User-Stories.md"] {
        let on_disk = std::fs::read_to_string(project_root.join(path)).unwrap();
        assert_eq!(on_disk, "# PRD\n\n真实产出内容");
    }

    // prd 完成后不再出现在 ready 集合里
    let ready_after = list_ready_agent_tasks(&created.project_db, &instantiated.workflow_id).await.unwrap();
    assert!(ready_after.is_empty(), "prd_review 是 human 节点，不会被自动推进");

    // 评审红线：必须显式 approve_gate 才能推进
    approve_gate(&created.project_db, &instantiated.workflow_id, "requirement-gate", "user", Some("需求没问题")).await.unwrap();

    let (gate_status,): (String,) =
        sqlx::query_as("SELECT status FROM gates WHERE workflow_id = ? AND node_key = 'requirement-gate'")
            .bind(&instantiated.workflow_id)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(gate_status, "passed");

    let ready_after_approve = list_ready_agent_tasks(&created.project_db, &instantiated.workflow_id).await.unwrap();
    assert_eq!(ready_after_approve.len(), 1);
    assert_eq!(ready_after_approve[0].node_key, "tech_selection");
}

/// 打回：`reject_gate` 应该把 `on_reject.goto` 指向的节点重置回 pending，
/// 让它重新出现在可执行集合里——不改 Gate 本身的通过状态。
#[tokio::test]
async fn reject_gate_resets_target_node_back_to_ready() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wpath("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "# PRD v1"}}],
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
    let ready = list_ready_agent_tasks(&created.project_db, &instantiated.workflow_id).await.unwrap();
    let prd_task = ready.into_iter().find(|t| t.node_key == "prd").unwrap();
    let prd_node = template.nodes.iter().find(|n| n.id == "prd").unwrap();
    run_task_node(&created.project_db, &project_root, &registry, &agent_defs, &prd_task, prd_node).await.unwrap();

    // 此时 prd 已 completed，不在 ready 集合
    assert!(list_ready_agent_tasks(&created.project_db, &instantiated.workflow_id).await.unwrap().is_empty());

    reject_gate(&created.project_db, &instantiated.workflow_id, "requirement-gate", "user", Some("目标描述不够具体")).await.unwrap();

    let (prd_status,): (String,) =
        sqlx::query_as("SELECT status FROM tasks WHERE workflow_id = ? AND node_key = 'prd'")
            .bind(&instantiated.workflow_id)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(prd_status, "pending", "打回后 prd 应该重置回 pending 等重跑");

    let ready_after_reject = list_ready_agent_tasks(&created.project_db, &instantiated.workflow_id).await.unwrap();
    assert_eq!(ready_after_reject.len(), 1);
    assert_eq!(ready_after_reject[0].node_key, "prd");

    // Gate 本身仍然是 pending（还没通过）
    let (gate_status,): (String,) =
        sqlx::query_as("SELECT status FROM gates WHERE workflow_id = ? AND node_key = 'requirement-gate'")
            .bind(&instantiated.workflow_id)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(gate_status, "pending");

    // reviews 表记录了这次打回（append-only）
    let (rejected_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM reviews WHERE decision = 'changes-requested'")
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(rejected_count, 1);
}

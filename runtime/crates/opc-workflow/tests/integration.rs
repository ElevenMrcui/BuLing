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
    run_task_node, NodeOutput, TemplateNode, WorkflowTemplate,
};
use tempfile::TempDir;
use wiremock::matchers::{body_string_contains, method, path as wpath};
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
        run_task_node(&created.project_db, &project_root, &registry, &agent_defs, &template, &prd_task, prd_node)
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
    run_task_node(&created.project_db, &project_root, &registry, &agent_defs, &template, &prd_task, prd_node).await.unwrap();

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

/// `tech_selection` 节点声明 `inputs: [prd]`（裸节点 id，见 `templates/README.md`：
/// "只声明依赖，Runtime 自动注入"）。这条测试证明它不再是一句通用指令——
/// Provider 真的收到了 `prd` 节点已经落盘的内容。用两条互斥的 body 匹配条件
/// 的 mock 做到：mock A 只认 `prd` 自己的请求；mock B 必须同时看到
/// "节点「tech_selection」"和 `prd` 产出里的真实文本才应答，应答的内容跟
/// mock A 完全不同——最终落盘内容能对上 mock B，就证明命中的是那条要求带
/// 真实上游内容的 mock，而不是巧合命中了别的。
#[tokio::test]
async fn run_task_node_injects_upstream_artifact_content_into_next_node_prompt() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wpath("/chat/completions"))
        .and(body_string_contains("节点「prd」"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "# PRD\n\n真实产出内容"}}],
            "usage": {"prompt_tokens": 4, "completion_tokens": 9}
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(wpath("/chat/completions"))
        .and(body_string_contains("节点「tech_selection」"))
        .and(body_string_contains("真实产出内容"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "# 技术选型\n\n用 Rust + Tauri"}}],
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
    run_task_node(&created.project_db, &project_root, &registry, &agent_defs, &template, &prd_task, prd_node)
        .await
        .unwrap();

    approve_gate(&created.project_db, &instantiated.workflow_id, "requirement-gate", "user", None).await.unwrap();

    let ready = list_ready_agent_tasks(&created.project_db, &instantiated.workflow_id).await.unwrap();
    let tech_task = ready.into_iter().find(|t| t.node_key == "tech_selection").unwrap();
    let tech_node = template.nodes.iter().find(|n| n.id == "tech_selection").unwrap();

    let result =
        run_task_node(&created.project_db, &project_root, &registry, &agent_defs, &template, &tech_task, tech_node)
            .await
            .unwrap();
    assert_eq!(result.artifact_ids.len(), 2, "tech_selection 声明了 2 个 output");

    let tech_stack = std::fs::read_to_string(project_root.join("docs/technical/Technology-Stack.md")).unwrap();
    assert_eq!(
        tech_stack, "# 技术选型\n\n用 Rust + Tauri",
        "命中了要求 body 里带 prd 真实内容的那条 mock，证明上游 Artifact 内容真的注入进了 Prompt"
    );
}

/// `prd` 节点声明 `inputs: [__goal__]`——用户最初的目标要能从 `project_meta.goal`
/// 里被读出来拼进 Prompt，不是空字符串占位。
#[tokio::test]
async fn run_task_node_injects_user_goal_for_entry_node() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wpath("/chat/completions"))
        .and(body_string_contains("做一个健康管理 App"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "# PRD"}}],
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

    // 没匹配到 mock（body 里没带 goal 文本）Provider 调用会失败，`.unwrap()` 直接 panic。
    run_task_node(&created.project_db, &project_root, &registry, &agent_defs, &template, &prd_task, prd_node)
        .await
        .unwrap();
}

/// 一个最小的合成模板：`check`（agent）→ `gate`（condition，读 `check` 落盘
/// 的 Artifact 里 `critical_count` 字段）→ `happy_path`/`fix_path`（两个都只
/// `depends_on: [check]`，跟真实模板里 `qa_gate` 与 `acceptance_prep`/`bug_fix`
/// 的关系一模一样——两个分支节点本来都会在 `check` 完成后同时"结构上就绪"，
/// 要靠 `gate` 求值把没选中的那个取消掉）。
fn synthetic_condition_template() -> WorkflowTemplate {
    let branch_node = |id: &str| TemplateNode {
        id: id.to_string(),
        kind: "agent".to_string(),
        role: Some("qa".to_string()),
        assignment: Some("template".to_string()),
        inputs: vec!["check".to_string()],
        outputs: vec![NodeOutput { kind: "note".to_string(), path: vec![format!("docs/{id}.md")] }],
        depends_on: vec!["check".to_string()],
        gate: None,
        subject: vec![],
        reviewer: None,
        on_reject_goto: vec![],
        condition: None,
        on_true_goto: vec![],
        on_false_goto: vec![],
    };

    WorkflowTemplate {
        id: "synthetic-condition".to_string(),
        name: "合成条件分支模板".to_string(),
        description: "只为证明 condition 节点求值 + 分支取消能真的跑通".to_string(),
        version: 1,
        required_roles: vec!["qa".to_string()],
        nodes: vec![
            TemplateNode {
                id: "check".to_string(),
                kind: "agent".to_string(),
                role: Some("qa".to_string()),
                assignment: Some("template".to_string()),
                inputs: vec![],
                outputs: vec![NodeOutput { kind: "report".to_string(), path: vec!["docs/check/Report.md".to_string()] }],
                depends_on: vec![],
                gate: None,
                subject: vec![],
                reviewer: None,
                on_reject_goto: vec![],
                condition: None,
                on_true_goto: vec![],
                on_false_goto: vec![],
            },
            TemplateNode {
                id: "gate".to_string(),
                kind: "condition".to_string(),
                role: None,
                assignment: None,
                inputs: vec![],
                outputs: vec![],
                depends_on: vec!["check".to_string()],
                gate: None,
                subject: vec![],
                reviewer: None,
                on_reject_goto: vec![],
                condition: Some("check.output.report.critical_count == 0".to_string()),
                on_true_goto: vec!["happy_path".to_string()],
                on_false_goto: vec!["fix_path".to_string()],
            },
            branch_node("happy_path"),
            branch_node("fix_path"),
        ],
        gates: vec![],
    }
}

async fn run_check_node(
    project_db: &opc_storage::ProjectDb,
    project_root: &std::path::Path,
    registry: &ProviderRegistry,
    agent_defs: &[AgentDefinition],
    template: &WorkflowTemplate,
    workflow_id: &str,
) {
    let ready = list_ready_agent_tasks(project_db, workflow_id).await.unwrap();
    let check_task = ready.into_iter().find(|t| t.node_key == "check").unwrap();
    let check_node = template.nodes.iter().find(|n| n.id == "check").unwrap();
    run_task_node(project_db, project_root, registry, agent_defs, template, &check_task, check_node).await.unwrap();
}

/// `check` 落盘 `critical_count: 0` → `gate` 求值为 true → `happy_path` 保持
/// `pending`（马上进入 ready 集合），`fix_path` 被取消（`cancelled`，不会再
/// 出现在 ready 集合，也不会永远堵住依赖它的下游）。
#[tokio::test]
async fn condition_node_true_branch_cancels_the_other_and_unlocks_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wpath("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "# Report\n\n```yaml\ncritical_count: 0\n```\n"}}],
            "usage": {"prompt_tokens": 4, "completion_tokens": 9}
        })))
        .mount(&server)
        .await;

    let manifests_dir = TempDir::new().unwrap();
    write_manifest(manifests_dir.path(), "test-api", &server.uri());
    let registry = ProviderRegistry::load_from_dir(manifests_dir.path()).unwrap();

    let (_dir, _app_db, created, project_root) = setup_project().await;
    let template = synthetic_condition_template();
    let mut agent_defs = real_agents();
    for a in &mut agent_defs {
        a.provider_priority = vec!["test-api".to_string()];
    }

    let instantiated = instantiate_workflow(&created.project_db, &template).await.unwrap();
    run_check_node(&created.project_db, &project_root, &registry, &agent_defs, &template, &instantiated.workflow_id).await;

    let (gate_status,): (String,) =
        sqlx::query_as("SELECT status FROM tasks WHERE workflow_id = ? AND node_key = 'gate'")
            .bind(&instantiated.workflow_id)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(gate_status, "completed", "condition 节点求值出结果后应该标 completed");

    let (fix_status,): (String,) =
        sqlx::query_as("SELECT status FROM tasks WHERE workflow_id = ? AND node_key = 'fix_path'")
            .bind(&instantiated.workflow_id)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(fix_status, "cancelled", "没选中的分支应该被取消，不是一直 pending");

    let ready = list_ready_agent_tasks(&created.project_db, &instantiated.workflow_id).await.unwrap();
    assert_eq!(ready.iter().map(|t| t.node_key.as_str()).collect::<Vec<_>>(), vec!["happy_path"]);

    // 每次 llm.call / tool.file.write 都真的写了一条 execution_logs（opc-audit）。
    let (llm_call_ok_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM execution_logs WHERE kind = 'llm.call' AND result = 'ok'")
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(llm_call_ok_count, 1);
    let (file_write_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM execution_logs WHERE kind = 'tool.file.write' AND result = 'ok'")
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(file_write_count, 1, "check 节点声明了 1 个 output");
}

/// 同一份合成模板，`check` 这次落盘 `critical_count: 3` → `gate` 求值为
/// false → 反过来：`fix_path` 保持可跑，`happy_path` 被取消。
#[tokio::test]
async fn condition_node_false_branch_cancels_the_other_and_unlocks_fix_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wpath("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "# Report\n\n```yaml\ncritical_count: 3\n```\n"}}],
            "usage": {"prompt_tokens": 4, "completion_tokens": 9}
        })))
        .mount(&server)
        .await;

    let manifests_dir = TempDir::new().unwrap();
    write_manifest(manifests_dir.path(), "test-api", &server.uri());
    let registry = ProviderRegistry::load_from_dir(manifests_dir.path()).unwrap();

    let (_dir, _app_db, created, project_root) = setup_project().await;
    let template = synthetic_condition_template();
    let mut agent_defs = real_agents();
    for a in &mut agent_defs {
        a.provider_priority = vec!["test-api".to_string()];
    }

    let instantiated = instantiate_workflow(&created.project_db, &template).await.unwrap();
    run_check_node(&created.project_db, &project_root, &registry, &agent_defs, &template, &instantiated.workflow_id).await;

    let (happy_status,): (String,) =
        sqlx::query_as("SELECT status FROM tasks WHERE workflow_id = ? AND node_key = 'happy_path'")
            .bind(&instantiated.workflow_id)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(happy_status, "cancelled");

    let ready = list_ready_agent_tasks(&created.project_db, &instantiated.workflow_id).await.unwrap();
    assert_eq!(ready.iter().map(|t| t.node_key.as_str()).collect::<Vec<_>>(), vec!["fix_path"]);
}

/// 条件表达式引用的 Artifact 里没有 `critical_count` 这个契约字段（比如
/// Agent 没按格式产出）——`gate` 判断不了，应该老老实实停在 `pending`，
/// 两个分支谁都不会被取消，也不会瞎猜一个结果。
#[tokio::test]
async fn condition_node_stays_pending_when_contract_field_is_missing() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wpath("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "# Report\n\n没有结构化字段，纯 prose。"}}],
            "usage": {"prompt_tokens": 4, "completion_tokens": 9}
        })))
        .mount(&server)
        .await;

    let manifests_dir = TempDir::new().unwrap();
    write_manifest(manifests_dir.path(), "test-api", &server.uri());
    let registry = ProviderRegistry::load_from_dir(manifests_dir.path()).unwrap();

    let (_dir, _app_db, created, project_root) = setup_project().await;
    let template = synthetic_condition_template();
    let mut agent_defs = real_agents();
    for a in &mut agent_defs {
        a.provider_priority = vec!["test-api".to_string()];
    }

    let instantiated = instantiate_workflow(&created.project_db, &template).await.unwrap();
    run_check_node(&created.project_db, &project_root, &registry, &agent_defs, &template, &instantiated.workflow_id).await;

    let (gate_status,): (String,) =
        sqlx::query_as("SELECT status FROM tasks WHERE workflow_id = ? AND node_key = 'gate'")
            .bind(&instantiated.workflow_id)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(gate_status, "pending", "判断不了就该留在 pending，不该猜");

    for key in ["happy_path", "fix_path"] {
        let (status,): (String,) = sqlx::query_as("SELECT status FROM tasks WHERE workflow_id = ? AND node_key = ?")
            .bind(&instantiated.workflow_id)
            .bind(key)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
        assert_eq!(status, "pending", "{key} 不该被瞎猜取消或放行");
    }
}

/// 打回一个**已经通过、且下游已经真的跑完**的 Gate：级联失效要把
/// `on_reject.goto` 指名的节点，以及依赖它的全部下游（`tech_selection`，
/// 就算这次没显式点名它）一起重置回 `pending`——不然 `tech_selection` 已经
/// 落盘的产出还在用被打回之前那份 `prd` 内容，是个悬空的错误状态。
#[tokio::test]
async fn reject_gate_cascades_to_downstream_nodes_that_already_ran() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wpath("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "内容"}}],
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

    let run_node = |key: &'static str| {
        let project_db = &created.project_db;
        let project_root = &project_root;
        let registry = &registry;
        let agent_defs = &agent_defs;
        let template = &template;
        let workflow_id = instantiated.workflow_id.clone();
        async move {
            let ready = list_ready_agent_tasks(project_db, &workflow_id).await.unwrap();
            let task = ready.into_iter().find(|t| t.node_key == key).unwrap();
            let node = template.nodes.iter().find(|n| n.id == key).unwrap();
            run_task_node(project_db, project_root, registry, agent_defs, template, &task, node).await.unwrap();
        }
    };

    run_node("prd").await;
    approve_gate(&created.project_db, &instantiated.workflow_id, "requirement-gate", "user", None).await.unwrap();
    run_node("tech_selection").await;

    // 此时 prd_review / tech_selection 都已经 completed。
    for key in ["prd_review", "tech_selection"] {
        let (status,): (String,) = sqlx::query_as("SELECT status FROM tasks WHERE workflow_id = ? AND node_key = ?")
            .bind(&instantiated.workflow_id)
            .bind(key)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
        assert_eq!(status, "completed");
    }

    // 用户后来发现 prd 有问题，再次打回同一个 Gate（哪怕它已经 passed 过）。
    reject_gate(&created.project_db, &instantiated.workflow_id, "requirement-gate", "user", Some("需求描述有遗漏")).await.unwrap();

    // on_reject.goto 直接点名的 prd，以及级联到的 prd_review / tech_selection 全部回到 pending。
    for key in ["prd", "prd_review", "tech_selection"] {
        let (status,): (String,) = sqlx::query_as("SELECT status FROM tasks WHERE workflow_id = ? AND node_key = ?")
            .bind(&instantiated.workflow_id)
            .bind(key)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
        assert_eq!(status, "pending", "{key} 应该被级联重置");
    }

    // requirement-gate 本身也回到 pending——不然 UI 会显示"已通过"但审的任务又是 pending。
    let (gate_status,): (String,) =
        sqlx::query_as("SELECT status FROM gates WHERE workflow_id = ? AND node_key = 'requirement-gate'")
            .bind(&instantiated.workflow_id)
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(gate_status, "pending");

    // 两条 review.decision 审计日志都在（一条 approved 时的 confirmed，一条这次的 denied）。
    let (confirmed_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM execution_logs WHERE kind = 'review.decision' AND result = 'confirmed'")
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(confirmed_count, 1);
    let (denied_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM execution_logs WHERE kind = 'review.decision' AND result = 'denied'")
            .fetch_one(&created.project_db.pool)
            .await
            .unwrap();
    assert_eq!(denied_count, 1);
}

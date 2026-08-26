//! opc-agent 集成测试。
//!
//! 覆盖三层：加载真实 `agents/*.yaml` · 播种 app.sqlite（含幂等 / 保护
//! 已 fork 用户行两个场景）· 用 wiremock 假 Provider 跑通 select+run 闭环。
//! **不依赖沙箱机器上装了什么 CLI**——select_provider 测试用临时 manifest
//! 目录构造"确定不存在的二进制"+ wiremock API，不读真实 `providers/` 目录，
//! 避免测试结果随机器环境漂移。

use opc_agent::{load_agents_from_dir, run_task, seed_agents, select_provider, AgentDefinition, PermissionDefaults};
use opc_provider::ProviderRegistry;
use opc_storage::AppDb;
use std::path::PathBuf;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn repo_root() -> PathBuf {
    // runtime/crates/opc-agent → 上溯三层到仓库根
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn migrations_root() -> PathBuf {
    repo_root().join("runtime/migrations")
}

fn agents_root() -> PathBuf {
    repo_root().join("agents")
}

async fn open_temp_app_db() -> (TempDir, AppDb) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("app.sqlite");
    let db = AppDb::open(&db_path, migrations_root().join("app")).await.unwrap();
    (dir, db)
}

// ---------------------------------------------------------------------------
// 加载
// ---------------------------------------------------------------------------

#[test]
fn load_agents_from_dir_parses_all_nine() {
    let defs = load_agents_from_dir(agents_root()).expect("load agents/*.yaml");
    let ids: Vec<&str> = defs.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(defs.len(), 9, "expected 9 preset agents, got: {ids:?}");

    for expected in [
        "acceptance",
        "architect",
        "backend",
        "designer",
        "frontend",
        "product-manager",
        "project-manager",
        "qa",
        "tech-lead",
    ] {
        assert!(ids.contains(&expected), "missing agent: {expected}, have {ids:?}");
    }

    for d in &defs {
        assert!(!d.provider_priority.is_empty(), "{} has empty provider_priority", d.id);
        assert!(!d.expected_outputs.is_empty(), "{} has empty expected_outputs", d.id);
        assert!(!d.system_prompt.trim().is_empty(), "{} has empty system_prompt", d.id);
        assert!(
            matches!(d.sensitivity.as_str(), "low" | "medium" | "high"),
            "{} has invalid sensitivity: {}",
            d.id,
            d.sensitivity
        );
    }

    let acceptance = defs.iter().find(|d| d.id == "acceptance").unwrap();
    assert_eq!(acceptance.sensitivity, "high", "验收岗位必须是 high 敏感度（隐私哨兵硬拦）");
}

// ---------------------------------------------------------------------------
// 播种
// ---------------------------------------------------------------------------

#[tokio::test]
async fn seed_agents_inserts_all_and_is_idempotent() {
    let (_dir, db) = open_temp_app_db().await;
    let defs = load_agents_from_dir(agents_root()).unwrap();

    let inserted = seed_agents(&db, &defs).await.unwrap();
    assert_eq!(inserted, 9);

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents WHERE kind='preset'")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 9);

    let (version,): (i64,) = sqlx::query_as("SELECT version FROM agents WHERE id='product-manager'")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(version, 1, "首次插入 version 应为 1");

    // 重新播种（内容未变）：不应重复插入，也不应无意义地涨版本号
    seed_agents(&db, &defs).await.unwrap();
    let (count2,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents WHERE kind='preset'")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count2, 9, "重复播种不应产生重复行");
    let (version2,): (i64,) = sqlx::query_as("SELECT version FROM agents WHERE id='product-manager'")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(version2, 1, "内容未变时 version 不应递增");
}

#[tokio::test]
async fn seed_agents_bumps_version_only_when_system_prompt_changes() {
    let (_dir, db) = open_temp_app_db().await;
    let mut defs = load_agents_from_dir(agents_root()).unwrap();
    seed_agents(&db, &defs).await.unwrap();

    // 改一个 agent 的 system_prompt，其余不变
    let pm = defs.iter_mut().find(|d| d.id == "product-manager").unwrap();
    pm.system_prompt = "全新的人格设定（测试用）".to_string();
    seed_agents(&db, &defs).await.unwrap();

    let (pm_version,): (i64,) = sqlx::query_as("SELECT version FROM agents WHERE id='product-manager'")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(pm_version, 2, "system_prompt 变了应该 +1");

    let (other_version,): (i64,) = sqlx::query_as("SELECT version FROM agents WHERE id='tech-lead'")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(other_version, 1, "没变的 agent 不应被连带涨版本");
}

#[tokio::test]
async fn seed_agents_never_clobbers_a_user_owned_row_with_same_id() {
    let (_dir, db) = open_temp_app_db().await;

    // 模拟"同 id 但 kind='user'"这种理论上不该发生、但 seed 逻辑必须防住的场景
    sqlx::query(
        r#"INSERT INTO agents (id, kind, role, display_name, system_prompt, responsibilities, expected_outputs)
           VALUES ('product-manager', 'user', '用户自定义角色', '用户自定义角色', '用户自己写的 prompt', '[]', '[]')"#,
    )
    .execute(&db.pool)
    .await
    .unwrap();

    let defs = load_agents_from_dir(agents_root()).unwrap();
    seed_agents(&db, &defs).await.unwrap();

    let (kind, role): (String, String) = sqlx::query_as("SELECT kind, role FROM agents WHERE id='product-manager'")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(kind, "user", "预置播种不应把用户行的 kind 改回 preset");
    assert_eq!(role, "用户自定义角色", "预置播种不应覆盖用户行的内容");
}

// ---------------------------------------------------------------------------
// 驱动（select_provider / run_task）—— 用临时 manifest + wiremock，不碰真实机器环境
// ---------------------------------------------------------------------------

fn write_manifest(dir: &std::path::Path, id: &str, toml: &str) {
    let sub = dir.join(id);
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("manifest.toml"), toml).unwrap();
}

fn minimal_agent(id: &str, provider_priority: Vec<String>) -> AgentDefinition {
    AgentDefinition {
        id: id.to_string(),
        kind: "preset".to_string(),
        role: "测试岗位".to_string(),
        display_name: "测试岗位".to_string(),
        avatar: None,
        system_prompt: "你是一个测试用 Agent".to_string(),
        responsibilities: vec![],
        expected_outputs: vec![],
        capabilities: vec![],
        sensitivity: "medium".to_string(),
        skills: vec![],
        tools: vec![],
        mcp_servers: vec![],
        provider_priority,
        permission_defaults: PermissionDefaults::default(),
    }
}

#[tokio::test]
async fn select_provider_skips_unavailable_cli_and_picks_next() {
    let server = MockServer::start().await;
    let manifests_dir = TempDir::new().unwrap();

    write_manifest(
        manifests_dir.path(),
        "definitely-missing-cli",
        r#"
id = "definitely-missing-cli"
display_name = "不存在的 CLI"
vendor = "test"
kind = "cli"
cli_binary = "this-binary-definitely-does-not-exist-xyz123"
"#,
    );
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

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "这是产品经理的回复"}}],
            "usage": {"prompt_tokens": 3, "completion_tokens": 5}
        })))
        .mount(&server)
        .await;

    // "definitely-missing-cli" 因为二进制不存在会拼 args 失败？不——它的
    // Manifest 里没设 cli_adapter，CliProvider::from_manifest 直接构建就报
    // Manifest 错误；select_provider 应该把这当"这一家不可用"跳过，继续试下一家。
    let registry = ProviderRegistry::load_from_dir(manifests_dir.path()).unwrap();
    let agent = minimal_agent(
        "test-agent",
        vec!["definitely-missing-cli".to_string(), "test-api".to_string()],
    );

    let (provider_id, _provider) = select_provider(&registry, &agent.id, &agent.sensitivity, &agent.provider_priority)
        .await
        .expect("should fall through to test-api");
    assert_eq!(provider_id, "test-api");

    let output = run_task(&registry, &agent, "帮我写个 PRD", 100).await.unwrap();
    assert_eq!(output.provider_id, "test-api");
    assert_eq!(output.response.text, "这是产品经理的回复");
    assert_eq!(output.response.usage.input_tokens, 3);
}

#[tokio::test]
async fn select_provider_errors_with_agent_id_and_tried_list_when_none_available() {
    let manifests_dir = TempDir::new().unwrap();
    write_manifest(
        manifests_dir.path(),
        "missing-a",
        r#"
id = "missing-a"
display_name = "missing a"
vendor = "test"
kind = "cli"
cli_binary = "no-such-binary-a"
"#,
    );
    write_manifest(
        manifests_dir.path(),
        "missing-b",
        r#"
id = "missing-b"
display_name = "missing b"
vendor = "test"
kind = "cli"
cli_binary = "no-such-binary-b"
"#,
    );

    let registry = ProviderRegistry::load_from_dir(manifests_dir.path()).unwrap();
    let agent = minimal_agent("lonely-agent", vec!["missing-a".to_string(), "missing-b".to_string()]);

    // Box<dyn Provider> 不是 Debug，不能用 .expect_err()（它要求 Ok 分支也 Debug）
    let err = match select_provider(&registry, &agent.id, &agent.sensitivity, &agent.provider_priority).await {
        Err(e) => e,
        Ok(_) => panic!("both providers should be unavailable"),
    };
    let msg = err.to_string();
    assert!(msg.contains("lonely-agent"), "error should name the agent: {msg}");
    assert!(msg.contains("missing-a") && msg.contains("missing-b"), "error should list tried providers: {msg}");
}

/// 隐私哨兵硬拦：`sensitivity=high` 的 Agent 候选列表里只有一个云 Provider
/// （`allowed_sensitivity` 默认 `["low","medium"]`，不含 `high`）——这家
/// **压根不会被真正调用**（不是"调用了但连不上"），报 `PrivacyBlocked`，
/// 不是 `NoProviderAvailable`。
#[tokio::test]
async fn select_provider_blocks_high_sensitivity_agent_from_cloud_provider() {
    let server = MockServer::start().await;
    let manifests_dir = TempDir::new().unwrap();
    write_manifest(
        manifests_dir.path(),
        "cloud-api",
        &format!(
            r#"
id = "cloud-api"
display_name = "云 API"
vendor = "test"
kind = "api"
wire_format = "openai-compatible"
default_base_url = "{}"
"#,
            server.uri()
        ),
    );
    // 没写 allowed_sensitivity，manifest.rs 默认给 ["low","medium"]，不含 "high"。

    Mock::given(method("POST")).and(path("/chat/completions")).respond_with(ResponseTemplate::new(200)).mount(&server).await;

    let registry = ProviderRegistry::load_from_dir(manifests_dir.path()).unwrap();
    let mut agent = minimal_agent("acceptance-agent", vec!["cloud-api".to_string()]);
    agent.sensitivity = "high".to_string();

    let err = match select_provider(&registry, &agent.id, &agent.sensitivity, &agent.provider_priority).await {
        Err(e) => e,
        Ok(_) => panic!("high sensitivity agent should never reach the cloud provider"),
    };
    match err {
        opc_agent::Error::PrivacyBlocked { agent_id, agent_sensitivity, blocked_providers } => {
            assert_eq!(agent_id, "acceptance-agent");
            assert_eq!(agent_sensitivity, "high");
            assert_eq!(blocked_providers, vec!["cloud-api".to_string()]);
        }
        other => panic!("expected PrivacyBlocked, got: {other}"),
    }

    // 从没真的往 mock server 发过请求——隐私哨兵在网络调用之前就拦下了。
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

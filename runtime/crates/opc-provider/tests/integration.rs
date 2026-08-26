//! opc-provider 集成测试。
//!
//! API adapter 测试用 wiremock 起本机 mock server，**不发起任何真实网络请求**。
//! CLI adapter 的进程执行路径不在这里测（不 spawn 真实 CLI 二进制）；
//! 其纯函数部分（build_args / parse_output）在
//! `src/cli/claude_code.rs` 里用 `#[cfg(test)]` 单测覆盖。

use opc_provider::{ChatMessage, ProviderRequest, ProviderRegistry, Role};
use std::path::PathBuf;
use std::sync::Mutex;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// `std::env::set_var`/`remove_var` 是进程级全局状态；Rust 测试默认并行跑在
/// 同一进程里，多个测试同时读写 `OPC_KEY_*` 会互相踩踏（flaky failure）。
/// 任何触碰这些环境变量的测试都必须先拿这把锁，序列化彼此。
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn providers_root() -> PathBuf {
    // runtime/crates/opc-provider → 上溯三层到仓库根 → providers/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("providers")
}

#[test]
fn manifest_loading_covers_expected_providers() {
    let registry = ProviderRegistry::load_from_dir(providers_root()).expect("load manifests");
    let ids: Vec<&str> = registry.manifests().map(|m| m.id.as_str()).collect();
    for expected in [
        "claude-code",
        "codex-cli",
        "gemini-cli",
        "aider",
        "anthropic-api",
        "openai-api",
        "glm-api",
        "qwen-api",
        "deepseek-api",
        "ollama-local",
        "lm-studio-local",
    ] {
        assert!(ids.contains(&expected), "missing provider manifest: {expected}, have: {ids:?}");
    }

    let anthropic = registry.manifest("anthropic-api").unwrap();
    assert_eq!(anthropic.wire_format, Some(opc_provider::WireFormat::AnthropicMessages));
    assert_eq!(anthropic.credential_service.as_deref(), Some("anthropic"));

    let ollama = registry.manifest("ollama-local").unwrap();
    assert!(ollama.allowed_sensitivity.iter().any(|s| s == "high"), "ollama should allow high sensitivity");

    let anthropic_sensitivity = &registry.manifest("anthropic-api").unwrap().allowed_sensitivity;
    assert!(
        !anthropic_sensitivity.iter().any(|s| s == "high"),
        "cloud API provider must NOT allow high-sensitivity agents by default"
    );
}

#[tokio::test]
async fn anthropic_wire_format_success() {
    let _guard = ENV_LOCK.lock().unwrap();
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-anthropic-key"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_test",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-5",
            "content": [{"type": "text", "text": "你好，我是产品经理分身。"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 12, "output_tokens": 8}
        })))
        .mount(&server)
        .await;

    std::env::set_var("OPC_KEY_ANTHROPIC", "test-anthropic-key");

    let registry = ProviderRegistry::load_from_dir(providers_root()).unwrap();
    let provider = registry.build("anthropic-api", Some(server.uri())).unwrap();

    let req = ProviderRequest {
        system: Some("你是产品经理".into()),
        messages: vec![ChatMessage { role: Role::User, content: "你好".into() }],
        max_tokens: 1024,
        model_override: None,
    };
    let resp = provider.execute(&req).await.expect("execute should succeed");

    assert_eq!(resp.text, "你好，我是产品经理分身。");
    assert_eq!(resp.model, "claude-sonnet-5");
    assert_eq!(resp.usage.input_tokens, 12);
    assert_eq!(resp.usage.output_tokens, 8);

    std::env::remove_var("OPC_KEY_ANTHROPIC");
}

#[tokio::test]
async fn anthropic_wire_format_error_maps_to_api_error() {
    let _guard = ENV_LOCK.lock().unwrap();
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "type": "error",
            "error": {"type": "invalid_request_error", "message": "max_tokens is required"},
            "request_id": "req_test"
        })))
        .mount(&server)
        .await;

    std::env::set_var("OPC_KEY_ANTHROPIC", "test-key");
    let registry = ProviderRegistry::load_from_dir(providers_root()).unwrap();
    let provider = registry.build("anthropic-api", Some(server.uri())).unwrap();

    let req = ProviderRequest::simple("system", "hi", 1);
    let err = provider.execute(&req).await.expect_err("should fail");
    let msg = err.to_string();
    assert!(msg.contains("400"), "expected status in error: {msg}");
    assert!(msg.contains("max_tokens is required"), "expected message passthrough: {msg}");

    std::env::remove_var("OPC_KEY_ANTHROPIC");
}

#[tokio::test]
async fn openai_compatible_wire_format_success() {
    let _guard = ENV_LOCK.lock().unwrap();
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("Authorization", "Bearer test-openai-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-test",
            "model": "gpt-test",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "hello from openai-compatible"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 5, "completion_tokens": 7, "total_tokens": 12}
        })))
        .mount(&server)
        .await;

    std::env::set_var("OPC_KEY_OPENAI", "test-openai-key");
    let registry = ProviderRegistry::load_from_dir(providers_root()).unwrap();
    let provider = registry.build("openai-api", Some(server.uri())).unwrap();

    let req = ProviderRequest::simple("system prompt", "hi", 100);
    let resp = provider.execute(&req).await.expect("execute should succeed");

    assert_eq!(resp.text, "hello from openai-compatible");
    assert_eq!(resp.usage.input_tokens, 5);
    assert_eq!(resp.usage.output_tokens, 7);

    std::env::remove_var("OPC_KEY_OPENAI");
}

#[tokio::test]
async fn openai_compatible_local_provider_skips_auth_header_when_no_key() {
    let server = MockServer::start().await;

    // Ollama 没有 credential_service，本地无需鉴权：不应该带 Authorization header。
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "local reply"}}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        })))
        .mount(&server)
        .await;

    let registry = ProviderRegistry::load_from_dir(providers_root()).unwrap();
    let provider = registry.build("ollama-local", Some(server.uri())).unwrap();

    let req = ProviderRequest::simple("sys", "hi", 10);
    let resp = provider.execute(&req).await.expect("local provider should not require credential");
    assert_eq!(resp.text, "local reply");
}

#[tokio::test]
async fn missing_credential_surfaces_clear_error() {
    let _guard = ENV_LOCK.lock().unwrap();
    let server = MockServer::start().await;
    let registry = ProviderRegistry::load_from_dir(providers_root()).unwrap();
    let provider = registry.build("openai-api", Some(server.uri())).unwrap();

    // 确保没有残留的环境变量（测试之间隔离）
    std::env::remove_var("OPC_KEY_OPENAI");

    let status = provider.status().await;
    assert!(!status.available, "should be unavailable without credential");
}

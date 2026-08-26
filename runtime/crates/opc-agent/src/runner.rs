//! 把一个 `AgentDefinition` 真正接到 Provider 上跑一次。
//!
//! **职责边界**（与 `opc-provider` 的边界一致）：只做一次文本补全。工具
//! 调用循环、Artifact 落盘、Report.md 强制产出是 Runtime 更上层
//! （`opc-tool` + `opc-workflow`，P0.5+）的职责，这里不做。

use opc_provider::{Provider, ProviderRegistry, ProviderRequest, ProviderResponse};

use crate::definition::AgentDefinition;
use crate::error::{Error, Result};

pub struct AgentRunOutput {
    /// 实际命中的 provider id（provider_priority 里第一个可用的）。
    pub provider_id: String,
    pub response: ProviderResponse,
}

/// 按 `provider_priority` 顺序尝试，返回第一个**过了隐私哨兵**又
/// `status().available` 的实例。
///
/// 这就是"没装 CLI 就走 API，都不行再本地兜底"的 fallback 路由
/// （见 `docs/OPC-架构决策.md` ADR-005），外加隐私哨兵的硬拦：`agent_sensitivity`
/// 不在某个候选 Provider 声明的 `allowed_sensitivity` 白名单里，这家
/// **压根不会被构建/调用**，直接跳过（见 `opc-privacy`）。如果全部候选都是
/// 因为这条被跳过（一个都没真正尝试连接），报 `Error::PrivacyBlocked`
/// 而不是笼统的 `NoProviderAvailable`——两种失败原因对用户的意义完全不同
/// （前者是"配置不允许"，后者是"没装/连不上"）。
pub async fn select_provider(
    registry: &ProviderRegistry,
    agent_id: &str,
    agent_sensitivity: &str,
    provider_priority: &[String],
) -> Result<(String, Box<dyn Provider>)> {
    let mut last_error: Option<String> = None;
    let mut blocked_by_privacy: Vec<String> = Vec::new();
    let mut attempted_any = false;

    for id in provider_priority {
        if let Some(manifest) = registry.manifest(id) {
            if !opc_privacy::is_allowed(agent_sensitivity, &manifest.allowed_sensitivity) {
                blocked_by_privacy.push(id.clone());
                continue;
            }
        }
        attempted_any = true;
        match registry.build(id, None) {
            Ok(provider) => {
                let status = provider.status().await;
                if status.available {
                    return Ok((id.clone(), provider));
                }
                last_error = Some(format!("{id}: {}", status.detail.unwrap_or_default()));
            }
            Err(e) => {
                last_error = Some(format!("{id}: {e}"));
            }
        }
    }

    if !attempted_any && !blocked_by_privacy.is_empty() {
        return Err(Error::PrivacyBlocked {
            agent_id: agent_id.to_string(),
            agent_sensitivity: agent_sensitivity.to_string(),
            blocked_providers: blocked_by_privacy,
        });
    }

    Err(Error::NoProviderAvailable {
        agent_id: agent_id.to_string(),
        tried: provider_priority.to_vec(),
        last_error,
    })
}

/// 驱动一次任务：选 Provider（过隐私哨兵）→ 组 system+user → 执行 → 返回结果。
pub async fn run_task(
    registry: &ProviderRegistry,
    agent: &AgentDefinition,
    user_input: &str,
    max_tokens: u32,
) -> Result<AgentRunOutput> {
    let (provider_id, provider) =
        select_provider(registry, &agent.id, &agent.sensitivity, &agent.provider_priority).await?;
    let req = ProviderRequest::simple(agent.system_prompt.clone(), user_input, max_tokens);
    let response = provider.execute(&req).await?;
    Ok(AgentRunOutput { provider_id, response })
}

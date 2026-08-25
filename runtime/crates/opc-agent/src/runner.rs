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

/// 按 `provider_priority` 顺序尝试，返回第一个 `status().available` 的实例。
///
/// 这就是"没装 CLI 就走 API，都不行再本地兜底"的 fallback 路由
/// （见 `docs/OPC-架构决策.md` ADR-005）。
pub async fn select_provider(
    registry: &ProviderRegistry,
    agent_id: &str,
    provider_priority: &[String],
) -> Result<(String, Box<dyn Provider>)> {
    let mut last_error: Option<String> = None;
    for id in provider_priority {
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
    Err(Error::NoProviderAvailable {
        agent_id: agent_id.to_string(),
        tried: provider_priority.to_vec(),
        last_error,
    })
}

/// 驱动一次任务：选 Provider → 组 system+user → 执行 → 返回结果。
pub async fn run_task(
    registry: &ProviderRegistry,
    agent: &AgentDefinition,
    user_input: &str,
    max_tokens: u32,
) -> Result<AgentRunOutput> {
    let (provider_id, provider) = select_provider(registry, &agent.id, &agent.provider_priority).await?;
    let req = ProviderRequest::simple(agent.system_prompt.clone(), user_input, max_tokens);
    let response = provider.execute(&req).await?;
    Ok(AgentRunOutput { provider_id, response })
}

//! 不令 OPC · 隐私哨兵（Privacy Sentinel）
//!
//! **这一版的范围（诚实标注，不是完整实现）**：只做 `docs/OPC-架构决策.md` /
//! `providers/*/manifest.toml` 里已经声明过的那一条硬拦——`AgentDefinition.sensitivity`
//! 必须出现在候选 Provider 的 `ProviderManifest.allowed_sensitivity` 里，否则直接拒绝
//! 调用这个 Provider（`AGENTS.md` 里"自动脱敏或直接拒绝"的后半句）。**不做**
//! "自动脱敏"——把敏感内容从 Prompt 里洗掉再放行，需要真正理解内容语义，
//! 这一版没有，也不该在没有明确脱敏规则的情况下臆造一套。
//!
//! 纯逻辑、零依赖：不碰 Provider/Agent 的具体类型，接收方按自己的字段传值进来，
//! 方便被 `opc-agent`（真正做 Provider 选型的地方）直接调用而不引入循环依赖。

/// `agent_sensitivity`（如 `"high"`）是否被 `provider_allowed_sensitivity`
/// （Provider manifest 里的 `allowed_sensitivity` 白名单，如 `["low","medium"]`）放行。
///
/// 语义：**白名单必须显式包含这个敏感度**，不做"高敏感度自动兼容低白名单"这种
/// 隐含推导——`allowed_sensitivity` 本来就是 Provider 自己声明"我能接住哪些敏感度"，
/// 不是一个可比较大小的等级。
pub fn is_allowed(agent_sensitivity: &str, provider_allowed_sensitivity: &[String]) -> bool {
    provider_allowed_sensitivity.iter().any(|s| s == agent_sensitivity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_sensitivity_blocked_by_cloud_default_whitelist() {
        let cloud_whitelist = vec!["low".to_string(), "medium".to_string()];
        assert!(!is_allowed("high", &cloud_whitelist));
    }

    #[test]
    fn high_sensitivity_allowed_by_local_provider_whitelist() {
        let local_whitelist = vec!["low".to_string(), "medium".to_string(), "high".to_string()];
        assert!(is_allowed("high", &local_whitelist));
    }

    #[test]
    fn low_sensitivity_allowed_everywhere_only_because_it_is_explicitly_listed() {
        let cloud_whitelist = vec!["low".to_string(), "medium".to_string()];
        assert!(is_allowed("low", &cloud_whitelist));
        assert!(is_allowed("medium", &cloud_whitelist));
    }

    #[test]
    fn empty_whitelist_blocks_everything() {
        assert!(!is_allowed("low", &[]));
    }
}

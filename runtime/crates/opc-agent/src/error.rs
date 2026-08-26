use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("yaml parse error ({file}): {reason}")]
    Yaml { file: String, reason: String },

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("provider: {0}")]
    Provider(#[from] opc_provider::Error),

    #[error("no available provider for agent {agent_id}; tried {tried:?}{}", last_error.as_deref().map(|e| format!(" · last error: {e}")).unwrap_or_default())]
    NoProviderAvailable { agent_id: String, tried: Vec<String>, last_error: Option<String> },

    /// 隐私哨兵硬拦：`provider_priority` 里的候选 Provider 一个都没通过
    /// `opc_privacy::is_allowed`（Agent 敏感度不在任何候选的 `allowed_sensitivity`
    /// 白名单里）——这些 Provider 压根没被尝试调用，跟"装了但连不上"的
    /// `NoProviderAvailable` 是两回事，单独报出来方便定位。
    #[error("agent {agent_id}（sensitivity={agent_sensitivity}）的候选 provider 全部被隐私哨兵拦下，一个都没敢调用: {blocked_providers:?}")]
    PrivacyBlocked { agent_id: String, agent_sensitivity: String, blocked_providers: Vec<String> },

    #[error("agent definition not found in registry: {0}")]
    AgentNotFound(String),
}

pub type Result<T> = std::result::Result<T, Error>;

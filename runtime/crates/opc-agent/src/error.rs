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

    #[error("agent definition not found in registry: {0}")]
    AgentNotFound(String),
}

pub type Result<T> = std::result::Result<T, Error>;

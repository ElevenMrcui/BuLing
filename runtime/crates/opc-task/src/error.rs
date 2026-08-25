use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("workflow: {0}")]
    Workflow(#[from] opc_workflow::Error),

    #[error("dag 里找不到节点: {0}")]
    NodeNotFound(String),

    #[error("节点 {node_key} 不能这样指派/执行：{reason}")]
    NotAssignable { node_key: String, reason: String },
}

pub type Result<T> = std::result::Result<T, Error>;

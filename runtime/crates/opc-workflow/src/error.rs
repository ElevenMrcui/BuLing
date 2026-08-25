use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("agent: {0}")]
    Agent(#[from] opc_agent::Error),

    #[error("tool: {0}")]
    Tool(#[from] opc_tool::Error),

    #[error("project: {0}")]
    Project(#[from] opc_project::Error),

    #[error("template {file}: {reason}")]
    Template { file: String, reason: String },

    #[error("workflow not found: {0}")]
    WorkflowNotFound(String),

    #[error("gate not found: workflow={workflow_id} node_key={node_key}")]
    GateNotFound { workflow_id: String, node_key: String },

    #[error("node {0} 没有可解析的 agent_instance（角色未实例化，或 assignment 不是 template）")]
    AgentInstanceNotFound(String),

    #[error("node {0} 的 output.path 是多路径（glob 目录），P0 的 run_task_node 不支持——这类节点应该走 manual/auto-claim，不应该出现在 list_ready_agent_tasks 的结果里")]
    UnsupportedOutputShape(String),
}

pub type Result<T> = std::result::Result<T, Error>;

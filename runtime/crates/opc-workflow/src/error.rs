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

    #[error("audit: {0}")]
    Audit(opc_audit::Error),

    #[error("template {file}: {reason}")]
    Template { file: String, reason: String },

    #[error("workflow not found: {0}")]
    WorkflowNotFound(String),

    #[error("gate not found: workflow={workflow_id} node_key={node_key}")]
    GateNotFound { workflow_id: String, node_key: String },

    #[error("node {0} 没有可解析的 agent_instance（角色未实例化，或 assignment 不是 template）")]
    AgentInstanceNotFound(String),

    #[error("node {0} 的 output.path 是多路径或 glob 目录（如 frontend/**），P0 的 run_task_node 只认单个字面文件路径，不支持——这类节点产出整个目录/多份文件，不是这一版模型能表达的")]
    UnsupportedOutputShape(String),
}

pub type Result<T> = std::result::Result<T, Error>;

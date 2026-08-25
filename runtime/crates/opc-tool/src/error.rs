use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// 权限位 `file: project-only`（见 agents/README.md）的硬拦——路径解析后
    /// 落到了项目根目录之外，一律拒绝，不管是 `..` 穿越还是绝对路径。
    #[error("path escapes project root: {path:?}")]
    PathEscape { path: String },
}

pub type Result<T> = std::result::Result<T, Error>;

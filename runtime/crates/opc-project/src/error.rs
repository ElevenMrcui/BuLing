use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("storage: {0}")]
    Storage(#[from] opc_storage::Error),

    #[error("project not found: {0}")]
    NotFound(String),

    #[error("slug already in use: {0}")]
    SlugTaken(String),
}

pub type Result<T> = std::result::Result<T, Error>;

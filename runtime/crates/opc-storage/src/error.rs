use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("migration parse ({file}): {reason}")]
    MigrationParse { file: String, reason: String },

    #[error("migration missing dir: {0}")]
    MigrationDirMissing(String),
}

pub type Result<T> = std::result::Result<T, Error>;

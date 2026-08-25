use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("provider not available: {0}")]
    NotAvailable(String),

    #[error("credential missing for provider {provider}: {reason}")]
    CredentialMissing { provider: String, reason: String },

    #[error("http: {0}")]
    Http(#[from] reqwest::Error),

    #[error("api error ({status}): {message}")]
    Api { status: u16, kind: String, message: String },

    #[error("cli exec failed ({binary}): {reason}")]
    CliExec { binary: String, reason: String },

    #[error("cli exited non-zero ({binary}, code {code:?}): {stderr}")]
    CliNonZero { binary: String, code: Option<i32>, stderr: String },

    #[error("parse error ({context}): {reason}")]
    Parse { context: String, reason: String },

    #[error("manifest error ({file}): {reason}")]
    Manifest { file: String, reason: String },

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("keychain: {0}")]
    Keychain(#[from] keyring::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

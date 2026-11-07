use thiserror::Error;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("config error: {0}")]
    Config(String),
    #[error("identity error: {0}")]
    Identity(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("trust error: {0}")]
    Trust(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

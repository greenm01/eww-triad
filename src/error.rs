use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("TRIAD_SOCKET is not set and XDG_RUNTIME_DIR is not available")]
    MissingSocketPath,
    #[error("socket path does not exist: {0}")]
    SocketMissing(PathBuf),
    #[error("socket io failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("json parse failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("triad returned an error: {0}")]
    Triad(String),
    #[error("expected triad reply type {expected}, got {actual}")]
    UnexpectedReply { expected: String, actual: String },
    #[error("invalid action payload: {0}")]
    InvalidActionPayload(String),
}

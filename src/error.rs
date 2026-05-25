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
    #[error("unsupported native request: {0}")]
    UnsupportedRequest(String),
    #[error("unsupported native event: {0}")]
    UnsupportedEvent(String),
    #[error("stream disconnected")]
    StreamDisconnected,
    #[error("expected triad reply type {expected}, got {actual}")]
    UnexpectedReply { expected: String, actual: String },
    #[error("invalid action payload: {0}")]
    InvalidActionPayload(String),
    #[error("invalid dispatch-binding request: {0}")]
    InvalidDispatchBinding(String),
}

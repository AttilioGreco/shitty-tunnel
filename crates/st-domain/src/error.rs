use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("tunnel not found for domain: {0}")]
    TunnelNotFound(String),

    #[error("tunnel already exists for domain: {0}")]
    TunnelAlreadyExists(String),

    #[error("request timeout")]
    RequestTimeout,

    #[error("tunnel disconnected")]
    TunnelDisconnected,

    #[error("local service error: {0}")]
    LocalServiceError(String),

    #[error("protocol error: {0}")]
    ProtocolError(String),
}

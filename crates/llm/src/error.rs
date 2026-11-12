use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("missing credentials: {0}")]
    MissingCredentials(String),

    #[error("http {status}: {body}")]
    Http { status: u16, body: String },

    #[error("transport error: {0}")]
    Transport(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("request cancelled")]
    Cancelled,

    #[error("configuration error: {0}")]
    Config(String),
}

impl LlmError {
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Http { status, .. } => *status == 429 || (500..=599).contains(status),
            Self::Transport(_) => true,
            _ => false,
        }
    }
}

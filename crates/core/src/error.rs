use thiserror::Error;

/// Stable machine-readable error kind.
///
/// The string form (`code()`) is part of the CLI contract and must not change
/// without a breaking release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    Config,
    InvalidInput,
    NotFound,
    Io,
}

impl ErrorKind {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Config => "config_error",
            Self::InvalidInput => "invalid_input",
            Self::NotFound => "not_found",
            Self::Io => "io_error",
        }
    }
}

/// The error type shared by library crates.
///
/// Binaries may wrap it in `anyhow`, but every error that crosses a crate
/// boundary carries a stable kind and a human-readable message.
#[derive(Debug, Error)]
pub enum DshrError {
    #[error("{message}")]
    Config { message: String },

    #[error("{message}")]
    InvalidInput { message: String },

    #[error("{message}")]
    NotFound { message: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl DshrError {
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
        }
    }

    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::Config { .. } => ErrorKind::Config,
            Self::InvalidInput { .. } => ErrorKind::InvalidInput,
            Self::NotFound { .. } => ErrorKind::NotFound,
            Self::Io(_) => ErrorKind::Io,
        }
    }

    pub const fn code(&self) -> &'static str {
        self.kind().code()
    }
}

pub type Result<T> = std::result::Result<T, DshrError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stable() {
        assert_eq!(DshrError::config("x").code(), "config_error");
        assert_eq!(DshrError::invalid_input("x").code(), "invalid_input");
        assert_eq!(DshrError::not_found("x").code(), "not_found");
        assert_eq!(DshrError::Io(std::io::Error::other("x")).code(), "io_error");
    }

    #[test]
    fn display_uses_message_only() {
        let err = DshrError::config("bad layer");
        assert_eq!(err.to_string(), "bad layer");
    }
}

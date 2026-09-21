//! The engine's public error type.

use thiserror::Error;

/// Errors the engine returns to its callers.
#[derive(Debug, Error)]
pub enum EngineError {
    /// A match configuration value is out of range.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    /// An input or output operation failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// A tick file is malformed, truncated, or of an unknown schema version.
    #[error("tick file format error: {0}")]
    Format(String),
}

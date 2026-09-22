//! The engine's public error type.

use thiserror::Error;

/// Errors the engine returns to its callers.
#[derive(Debug, Error)]
pub enum EngineError {
    /// A match configuration value is out of range.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    /// An input or output operation failed.
    #[error("io error")]
    Io(#[from] std::io::Error),
    /// A named file could not be read.
    #[error("cannot read {path}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// A tick file is malformed, truncated, or of an unknown schema version.
    #[error("tick file format error: {0}")]
    Format(String),
    /// A tick consumer outside the engine refused a record or went away.
    #[error("the tick consumer stopped: {0}")]
    Sink(String),
    /// A content file failed deserialization or validation (named mechanism: fail-closed
    /// loader). `field` is the path inside the file; `reason` is the rule that failed.
    #[error("content refused: {kind} {path}: {field}: {reason}")]
    Data {
        kind: &'static str,
        path: String,
        field: String,
        reason: String,
    },
    /// A content file carries a schema version this build does not read.
    #[error("content refused: {kind} {path}: schema_version {found}; this build reads {expected}")]
    Version {
        kind: &'static str,
        path: String,
        found: u32,
        expected: u32,
    },
}

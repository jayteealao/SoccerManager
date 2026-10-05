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
    /// A snapshot file was refused. `path` is relative to the data folder or the file name;
    /// `reason` names the check that failed and never quotes the file's bytes.
    #[error("snapshot refused: {path}: {reason}")]
    Snapshot { path: String, reason: String },
    /// A content file carries a schema version this build does not read.
    #[error("content refused: {kind} {path}: schema_version {found}; this build reads {expected}")]
    Version {
        kind: &'static str,
        path: String,
        found: u32,
        expected: u32,
    },

    /// The slot file names a module the engine cannot run in a slot.
    #[error("slot configuration refused: slot {slot}: {problem}; valid: {valid}")]
    SlotRefused {
        slot: String,
        value: String,
        problem: String,
        valid: String,
    },
}

impl EngineError {
    /// The error's kind as the `error.type` record key names it.
    pub fn error_type(&self) -> &'static str {
        match self {
            Self::InvalidConfig(_) => "invalid-config",
            Self::Io(_) | Self::Read { .. } => "io",
            Self::Format(_) => "format",
            Self::Sink(_) => "sink",
            Self::Data { .. } | Self::Version { .. } | Self::SlotRefused { .. } => "content",
            Self::Snapshot { .. } => "snapshot",
        }
    }

    /// One code per variant, finer than `error_type`, as the `error.code` record key names it.
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidConfig(_) => "invalid-config",
            Self::Io(_) => "io",
            Self::Read { .. } => "read",
            Self::Format(_) => "tick-format",
            Self::Sink(_) => "sink-stopped",
            Self::Data { .. } => "content-refused",
            Self::Version { .. } => "content-version",
            Self::SlotRefused { .. } => "slot-refused",
            Self::Snapshot { .. } => "snapshot-refused",
        }
    }

    /// `true` when the same call may succeed if tried again: only a tick consumer that went
    /// away. The engine is deterministic, so every other error repeats on the same input.
    pub fn retriable(&self) -> bool {
        matches!(self, Self::Sink(_))
    }
}

#[cfg(test)]
mod tests {
    use super::EngineError;

    fn io() -> std::io::Error {
        std::io::Error::other("x")
    }

    #[test]
    fn every_variant_has_a_type_a_code_and_a_retry_rule() {
        let cases = [
            (
                EngineError::InvalidConfig("x".into()),
                "invalid-config",
                "invalid-config",
                false,
            ),
            (EngineError::Io(io()), "io", "io", false),
            (
                EngineError::Read {
                    path: "p".into(),
                    source: io(),
                },
                "io",
                "read",
                false,
            ),
            (
                EngineError::Format("x".into()),
                "format",
                "tick-format",
                false,
            ),
            (EngineError::Sink("x".into()), "sink", "sink-stopped", true),
            (
                EngineError::Data {
                    kind: "tuning",
                    path: "p".into(),
                    field: "f".into(),
                    reason: "r".into(),
                },
                "content",
                "content-refused",
                false,
            ),
            (
                EngineError::Snapshot {
                    path: "p".into(),
                    reason: "r".into(),
                },
                "snapshot",
                "snapshot-refused",
                false,
            ),
            (
                EngineError::Version {
                    kind: "tuning",
                    path: "p".into(),
                    found: 9,
                    expected: 1,
                },
                "content",
                "content-version",
                false,
            ),
            (
                EngineError::SlotRefused {
                    slot: "engine.fouls".into(),
                    value: "x".into(),
                    problem: "p".into(),
                    valid: "fouls@1, off".into(),
                },
                "content",
                "slot-refused",
                false,
            ),
        ];
        for (err, ty, code, retriable) in cases {
            assert_eq!(err.error_type(), ty, "{err}");
            assert_eq!(err.error_code(), code, "{err}");
            assert_eq!(err.retriable(), retriable, "{err}");
        }
    }
}

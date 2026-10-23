//! Error types for parsing and analysis.

use thiserror::Error;

/// Errors that can occur while parsing a log line.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParseError {
    #[error("empty line")]
    Empty,

    #[error("missing timestamp")]
    MissingTimestamp,

    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),

    #[error("missing log level")]
    MissingLevel,

    #[error("unknown log level: {0}")]
    UnknownLevel(String),

    #[error("missing message")]
    MissingMessage,
}

/// Result alias used across the crate.
pub type Result<T> = std::result::Result<T, ParseError>;
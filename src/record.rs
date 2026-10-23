//! Log record types: [`Level`], [`LevelFilter`], [`LogRecord`].

use chrono::{DateTime, Utc};
use std::fmt;
use std::str::FromStr;

use crate::error::ParseError;

/// Severity level of a log record.
///
/// Derives `Ord` so filtering can use comparison operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Critical,
}

impl Level {
    /// Numeric weight used for filtering.
    pub fn weight(self) -> u8 {
        match self {
            Level::Trace => 0,
            Level::Debug => 1,
            Level::Info => 2,
            Level::Warn => 3,
            Level::Error => 4,
            Level::Critical => 5,
        }
    }

    /// Canonical uppercase name.
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
            Level::Critical => "CRITICAL",
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Level {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_uppercase().as_str() {
            "TRACE" => Ok(Level::Trace),
            "DEBUG" => Ok(Level::Debug),
            "INFO" | "INFORMATION" => Ok(Level::Info),
            "WARN" | "WARNING" => Ok(Level::Warn),
            "ERROR" | "ERR" => Ok(Level::Error),
            "CRITICAL" | "FATAL" | "CRIT" => Ok(Level::Critical),
            other => Err(ParseError::UnknownLevel(other.to_string())),
        }
    }
}

/// Filter for log records by minimum severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LevelFilter {
    /// No filtering; accept all records.
    #[default]
    All,
    /// Only records at or above the given level.
    AtLeast(Level),
    /// Exactly this level.
    Exact(Level),
}

impl LevelFilter {
    /// Whether a record with `level` passes this filter.
    pub fn matches(self, level: Level) -> bool {
        match self {
            LevelFilter::All => true,
            LevelFilter::AtLeast(min) => level.weight() >= min.weight(),
            LevelFilter::Exact(l) => level == l,
        }
    }
}

impl FromStr for LevelFilter {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Level::from_str(s).map(LevelFilter::AtLeast)
    }
}

/// A single parsed log record.
///
/// The record owns its data so that it can outlive the source buffer,
/// demonstrating ownership of heap-allocated `String` fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    pub timestamp: DateTime<Utc>,
    pub level: Level,
    pub message: String,
    /// Original raw line (useful for search reconstruction).
    pub raw: String,
}

impl LogRecord {
    pub fn new(
        timestamp: DateTime<Utc>,
        level: Level,
        message: impl Into<String>,
        raw: impl Into<String>,
    ) -> Self {
        Self {
            timestamp,
            level,
            message: message.into(),
            raw: raw.into(),
        }
    }
}

impl fmt::Display for LogRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] [{}] {}",
            self.timestamp.to_rfc3339(),
            self.level,
            self.message
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_parse_uppercase() {
        assert_eq!("ERROR".parse::<Level>().unwrap(), Level::Error);
        assert_eq!("warning".parse::<Level>().unwrap(), Level::Warn);
        assert_eq!("Debug".parse::<Level>().unwrap(), Level::Debug);
        assert_eq!("fatal".parse::<Level>().unwrap(), Level::Critical);
    }

    #[test]
    fn level_parse_invalid() {
        assert!("banana".parse::<Level>().is_err());
    }

    #[test]
    fn level_ordering() {
        assert!(Level::Error > Level::Warn);
        assert!(Level::Trace < Level::Info);
        assert!(Level::Critical > Level::Error);
    }

    #[test]
    fn level_filter_matches() {
        assert!(LevelFilter::All.matches(Level::Trace));
        assert!(LevelFilter::AtLeast(Level::Warn).matches(Level::Error));
        assert!(!LevelFilter::AtLeast(Level::Warn).matches(Level::Info));
        assert!(LevelFilter::Exact(Level::Info).matches(Level::Info));
        assert!(!LevelFilter::Exact(Level::Info).matches(Level::Warn));
    }

    #[test]
    fn level_display() {
        assert_eq!(Level::Error.to_string(), "ERROR");
    }
}
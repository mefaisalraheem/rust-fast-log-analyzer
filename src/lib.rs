//! Fast Log Analyzer
//!
//! A high-performance library for parsing, grouping, and searching application logs.
//!
//! # Examples
//!
//! ```
//! use fast_log_analyzer::parse_str;
//!
//! let record = parse_str("[2024-01-15T08:00:00Z] [ERROR] Connection refused").unwrap();
//! assert_eq!(record.level.to_string(), "ERROR");
//! assert_eq!(record.message, "Connection refused");
//! ```

pub mod analyzer;
pub mod error;
pub mod generator;
pub mod parser;
pub mod record;
pub mod search;

pub use analyzer::{Analyzer, Report};
pub use error::ParseError;
pub use parser::{parse_line, parse_str, Parser};
pub use record::{Level, LevelFilter, LogRecord};
pub use search::{search, SearchHit};
//! Line parsing. Zero-copy where possible; allocates only the final `String`s.

use chrono::{DateTime, NaiveDateTime, Utc};

use crate::error::ParseError;
use crate::record::{Level, LogRecord};

/// Parses a single line into a [`LogRecord`].
///
/// Accepts two formats:
///
/// 1. `[TIMESTAMP] [LEVEL] message`
/// 2. `TIMESTAMP LEVEL message`
///
/// Leading/trailing whitespace is trimmed.
pub fn parse_str(line: &str) -> Result<LogRecord, ParseError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }

    let (rest, ts_str) = if let Some(stripped) = trimmed.strip_prefix('[') {
        let end = stripped.find(']').ok_or(ParseError::MissingTimestamp)?;
        let ts = &stripped[..end];
        let rest = stripped[end + 1..].trim_start();
        (rest, ts.to_string())
    } else {
        // Space-delimited: first token is timestamp.
        match trimmed.find(char::is_whitespace) {
            Some(i) => (&trimmed[i..], trimmed[..i].to_string()),
            None => return Err(ParseError::MissingTimestamp),
        }
    };

    let timestamp = parse_timestamp(&ts_str)?;

    // Skip optional `[` before level.
    let rest = rest.trim_start();
    let (level_str, rest) = if let Some(stripped) = rest.strip_prefix('[') {
        let end = stripped.find(']').ok_or(ParseError::MissingLevel)?;
        (&stripped[..end], stripped[end + 1..].trim_start())
    } else {
        match rest.find(char::is_whitespace) {
            Some(i) => (&rest[..i], rest[i..].trim_start()),
            None => return Err(ParseError::MissingLevel),
        }
    };

    let level: Level = level_str.parse()?;

    let message = rest.trim();
    if message.is_empty() {
        return Err(ParseError::MissingMessage);
    }

    Ok(LogRecord::new(
        timestamp,
        level,
        message,
        line.to_string(),
    ))
}

/// Convenience wrapper around [`parse_str`] that maps any error to `None`.
pub fn parse_line(line: &str) -> Option<LogRecord> {
    parse_str(line).ok()
}

fn parse_timestamp(s: &str) -> Result<DateTime<Utc>, ParseError> {
    let s = s.trim();

    // RFC3339 / ISO8601 with timezone.
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Naive formats (assume UTC).
    let naive_formats = [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
    ];
    for fmt in &naive_formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
            return Ok(naive.and_utc());
        }
    }

    // Date only.
    if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        if let Some(naive) = date.and_hms_opt(0, 0, 0) {
            return Ok(naive.and_utc());
        }
    }

    Err(ParseError::InvalidTimestamp(s.to_string()))
}

/// Streaming parser over an iterator of lines.
///
/// Tracks total / parsed / skipped counts without storing every record,
/// demonstrating iterator-based aggregation with minimal memory.
pub struct Parser<I> {
    lines: I,
    parsed: usize,
    skipped: usize,
}

impl<I> Parser<I>
where
    I: Iterator<Item = String>,
{
    pub fn new(lines: I) -> Self {
        Self { lines, parsed: 0, skipped: 0 }
    }

    pub fn parsed(&self) -> usize {
        self.parsed
    }

    pub fn skipped(&self) -> usize {
        self.skipped
    }

    pub fn total(&self) -> usize {
        self.parsed + self.skipped
    }
}

impl<I> Iterator for Parser<I>
where
    I: Iterator<Item = String>,
{
    type Item = LogRecord;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let line = self.lines.next()?;
            match parse_str(&line) {
                Ok(rec) => {
                    self.parsed += 1;
                    return Some(rec);
                }
                Err(_) => {
                    self.skipped += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bracketed_format() {
        let rec = parse_str("[2024-01-15T08:00:00Z] [ERROR] Connection refused").unwrap();
        assert_eq!(rec.level, Level::Error);
        assert_eq!(rec.message, "Connection refused");
        assert_eq!(rec.timestamp.to_rfc3339(), "2024-01-15T08:00:00+00:00");
    }

    #[test]
    fn parse_space_format() {
        let rec = parse_str("2024-01-15T08:00:00Z WARN Cache miss").unwrap();
        assert_eq!(rec.level, Level::Warn);
        assert_eq!(rec.message, "Cache miss");
    }

    #[test]
    fn parse_naive_timestamp() {
        let rec = parse_str("[2024-01-15 08:00:00] [INFO] hello").unwrap();
        assert_eq!(rec.level, Level::Info);
        assert_eq!(rec.message, "hello");
    }

    #[test]
    fn parse_with_fractional_seconds() {
        let rec = parse_str("[2024-01-15T08:00:00.123456] [INFO] x").unwrap();
        assert_eq!(rec.level, Level::Info);
    }

    #[test]
    fn parse_trims_whitespace() {
        let rec = parse_str("   [2024-01-15T08:00:00Z]   [DEBUG]   hi   ").unwrap();
        assert_eq!(rec.message, "hi");
        assert_eq!(rec.level, Level::Debug);
    }

    #[test]
    fn parse_warning_alias() {
        let rec = parse_str("[2024-01-15T08:00:00Z] [WARNING] careful").unwrap();
        assert_eq!(rec.level, Level::Warn);
    }

    #[test]
    fn reject_empty() {
        assert_eq!(parse_str(""), Err(ParseError::Empty));
        assert_eq!(parse_str("   "), Err(ParseError::Empty));
    }

    #[test]
    fn reject_no_timestamp() {
        assert!(parse_str("ERROR something bad").is_err());
    }

    #[test]
    fn reject_bad_timestamp() {
        let e = parse_str("[not-a-date] [INFO] x").unwrap_err();
        assert!(matches!(e, ParseError::InvalidTimestamp(_)));
    }

    #[test]
    fn reject_missing_level() {
        let e = parse_str("[2024-01-15T08:00:00Z]").unwrap_err();
        assert_eq!(e, ParseError::MissingLevel);
    }

    #[test]
    fn reject_unknown_level() {
        let e = parse_str("[2024-01-15T08:00:00Z] [BANANA] msg").unwrap_err();
        assert!(matches!(e, ParseError::UnknownLevel(_)));
    }

    #[test]
    fn reject_missing_message() {
        let e = parse_str("[2024-01-15T08:00:00Z] [INFO]").unwrap_err();
        assert_eq!(e, ParseError::MissingMessage);
    }

    #[test]
    fn streaming_parser_counts() {
        let lines = vec![
            "[2024-01-15T08:00:00Z] [INFO] a".to_string(),
            "garbage".to_string(),
            "[2024-01-15T08:00:01Z] [ERROR] b".to_string(),
            "".to_string(),
        ];
        let mut parser = Parser::new(lines.into_iter());
        let collected: Vec<_> = parser.by_ref().collect();
        assert_eq!(collected.len(), 2);
        assert_eq!(parser.parsed(), 2);
        assert_eq!(parser.skipped(), 2);
        assert_eq!(parser.total(), 4);
    }

    #[test]
    fn parse_line_returns_none_on_error() {
        assert!(parse_line("garbage").is_none());
        assert!(parse_line("[2024-01-15T08:00:00Z] [INFO] ok").is_some());
    }
}
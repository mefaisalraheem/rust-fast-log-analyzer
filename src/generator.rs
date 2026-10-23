//! Deterministic sample log generator for tests and benchmarks.

use std::fmt::Write as _;

use chrono::{Duration, TimeZone, Utc};

const ERROR_MESSAGES: &[&str] = &[
    "Connection refused to upstream service",
    "Timeout after 30s waiting for response",
    "Database pool exhausted",
    "Invalid authentication token",
    "Disk quota exceeded",
    "Null pointer dereference",
];

const WARN_MESSAGES: &[&str] = &[
    "Cache miss for key",
    "Slow query detected",
    "Retry attempt exceeded threshold",
    "Deprecated API endpoint used",
];

const INFO_MESSAGES: &[&str] = &[
    "Request completed",
    "User logged in",
    "Scheduled job started",
    "Configuration reloaded",
];

/// Generates a deterministic sample log with `lines` records.
///
/// Timestamps are spaced one second apart starting 2024-01-15T08:00:00Z.
pub fn generate(lines: usize) -> String {
    let mut out = String::with_capacity(lines * 80);
    let start = Utc.with_ymd_and_hms(2024, 1, 15, 8, 0, 0).unwrap();

    // Simple LCG for deterministic pseudo-randomness without external crates.
    let mut state: u64 = 0x9E3779B97F4A7C15;

    for i in 0..lines {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let roll = (state >> 33) as usize % 100;

        let (level, msg) = if roll < 40 {
            ("INFO", INFO_MESSAGES[i % INFO_MESSAGES.len()])
        } else if roll < 70 {
            ("WARN", WARN_MESSAGES[i % WARN_MESSAGES.len()])
        } else if roll < 95 {
            ("ERROR", ERROR_MESSAGES[i % ERROR_MESSAGES.len()])
        } else {
            ("DEBUG", "internal trace")
        };

        let ts = start + Duration::seconds(i as i64);
        writeln!(
            out,
            "[{}] [{}] {}",
            ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            level,
            msg
        )
        .unwrap();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_str;

    #[test]
    fn generates_requested_line_count() {
        let s = generate(500);
        assert_eq!(s.lines().count(), 500);
    }

    #[test]
    fn generated_lines_are_parseable() {
        let s = generate(1000);
        for line in s.lines() {
            assert!(parse_str(line).is_ok(), "failed on: {line}");
        }
    }

    #[test]
    fn deterministic() {
        assert_eq!(generate(50), generate(50));
    }

    #[test]
    fn zero_lines() {
        assert_eq!(generate(0), "");
    }
}
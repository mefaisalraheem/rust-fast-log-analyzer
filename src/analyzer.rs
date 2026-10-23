//! Aggregation and frequency statistics.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::parser::parse_str;
use crate::record::{Level, LevelFilter, LogRecord};

/// Aggregated analysis result.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub total_lines: usize,
    pub parsed_lines: usize,
    pub skipped_lines: usize,
    pub level_counts: Vec<(Level, usize)>,
    pub error_count: usize,
    /// Top error messages sorted by frequency, then alphabetically.
    pub top_errors: Vec<(String, usize)>,
    pub earliest: Option<DateTime<Utc>>,
    pub latest: Option<DateTime<Utc>>,
}

/// Streaming aggregator. Feeds records one at a time and produces a [`Report`].
#[derive(Debug, Default)]
pub struct Analyzer {
    total_lines: usize,
    parsed_lines: usize,
    skipped_lines: usize,
    level_counts: HashMap<Level, usize>,
    /// Message → count, only for ERROR / CRITICAL records.
    error_counts: HashMap<String, usize>,
    earliest: Option<DateTime<Utc>>,
    latest: Option<DateTime<Utc>>,
}

impl Analyzer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingest a raw line, parsing it internally.
    pub fn ingest_line(&mut self, line: &str) {
        self.total_lines += 1;
        match parse_str(line) {
            Ok(rec) => self.ingest_record(rec),
            Err(_) => self.skipped_lines += 1,
        }
    }

    /// Ingest an already-parsed record.
    pub fn ingest_record(&mut self, record: LogRecord) {
        self.parsed_lines += 1;
        *self.level_counts.entry(record.level).or_insert(0) += 1;

        if matches!(record.level, Level::Error | Level::Critical) {
            *self.error_counts.entry(record.message.clone()).or_insert(0) += 1;
        }

        self.earliest = Some(match self.earliest {
            Some(t) => t.min(record.timestamp),
            None => record.timestamp,
        });
        self.latest = Some(match self.latest {
            Some(t) => t.max(record.timestamp),
            None => record.timestamp,
        });
    }

    /// Consume self and produce the final report.
    pub fn finish(self) -> Report {
        let mut level_counts: Vec<_> = self.level_counts.into_iter().collect();
        level_counts.sort_by_key(|(level, _)| level.weight());

        let error_count = level_counts
            .iter()
            .filter(|(l, _)| matches!(l, Level::Error | Level::Critical))
            .map(|(_, c)| *c)
            .sum();

        let mut top_errors: Vec<_> = self.error_counts.into_iter().collect();
        top_errors.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        Report {
            total_lines: self.total_lines,
            parsed_lines: self.parsed_lines,
            skipped_lines: self.skipped_lines,
            level_counts,
            error_count,
            top_errors,
            earliest: self.earliest,
            latest: self.latest,
        }
    }

    /// Convenience: analyze an entire buffer.
    pub fn analyze_str(content: &str) -> Report {
        let mut a = Analyzer::new();
        for line in content.lines() {
            a.ingest_line(line);
        }
        a.finish()
    }

    /// Convenience: analyze with a level filter applied before counting.
    pub fn analyze_str_filtered(content: &str, filter: LevelFilter) -> Report {
        let mut a = Analyzer::new();
        for line in content.lines() {
            a.total_lines += 1;
            match parse_str(line) {
                Ok(rec) if filter.matches(rec.level) => a.ingest_record(rec),
                Ok(_) => a.skipped_lines += 1,
                Err(_) => a.skipped_lines += 1,
            }
        }
        a.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
[2024-01-15T08:00:00Z] [ERROR] boom
[2024-01-15T08:00:01Z] [INFO] ok
[2024-01-15T08:00:02Z] [ERROR] boom
[2024-01-15T08:00:03Z] [WARN] careful
garbage line
[2024-01-15T08:00:05Z] [ERROR] different
";

    #[test]
    fn counts_lines() {
        let r = Analyzer::analyze_str(SAMPLE);
        assert_eq!(r.total_lines, 6);
        assert_eq!(r.parsed_lines, 5);
        assert_eq!(r.skipped_lines, 1);
    }

    #[test]
    fn counts_levels() {
        let r = Analyzer::analyze_str(SAMPLE);
        let find = |lvl| r.level_counts.iter().find(|(l, _)| *l == lvl).map(|(_, c)| *c);
        assert_eq!(find(Level::Error), Some(3));
        assert_eq!(find(Level::Info), Some(1));
        assert_eq!(find(Level::Warn), Some(1));
    }

    #[test]
    fn groups_errors() {
        let r = Analyzer::analyze_str(SAMPLE);
        assert_eq!(r.error_count, 3);
        assert_eq!(r.top_errors[0], ("boom".to_string(), 2));
        assert_eq!(r.top_errors[1], ("different".to_string(), 1));
    }

    #[test]
    fn time_range() {
        let r = Analyzer::analyze_str(SAMPLE);
        assert_eq!(
            r.earliest.unwrap().to_rfc3339(),
            "2024-01-15T08:00:00+00:00"
        );
        assert_eq!(
            r.latest.unwrap().to_rfc3339(),
            "2024-01-15T08:00:05+00:00"
        );
    }

    #[test]
    fn filter_excludes_low_levels() {
        let r = Analyzer::analyze_str_filtered(SAMPLE, LevelFilter::AtLeast(Level::Error));
        // Only the 3 ERROR lines get counted as parsed; everything else skipped.
        assert_eq!(r.parsed_lines, 3);
        assert_eq!(r.skipped_lines, 3);
        assert_eq!(r.error_count, 3);
    }

    #[test]
    fn empty_input() {
        let r = Analyzer::analyze_str("");
        assert_eq!(r.total_lines, 0);
        assert_eq!(r.parsed_lines, 0);
        assert_eq!(r.earliest, None);
    }

    #[test]
    fn ingest_record_directly() {
        let rec = parse_str("[2024-01-15T08:00:00Z] [CRITICAL] kaboom").unwrap();
        let mut a = Analyzer::new();
        a.ingest_record(rec);
        let r = a.finish();
        assert_eq!(r.error_count, 1);
        assert_eq!(r.top_errors[0].0, "kaboom");
    }
}
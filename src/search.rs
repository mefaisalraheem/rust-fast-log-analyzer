//! Search engine with level and time filters.

use chrono::{DateTime, Utc};

use crate::parser::parse_str;
use crate::record::LevelFilter;

/// A single search hit. Borrows the raw line from the caller's buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit<'a> {
    pub raw: &'a str,
    pub line_number: usize,
}

/// Searches `content` for `needle` (case-insensitive substring).
///
/// - `filter` restricts by level.
/// - Only successfully parsed records participate in level filtering;
///   malformed lines are silently skipped.
///
/// Returns hits in file order. Zero-copy: each hit borrows from `content`.
pub fn search<'a>(content: &'a str, needle: &str, filter: LevelFilter) -> Vec<SearchHit<'a>> {
    if needle.is_empty() {
        return Vec::new();
    }
    let needle_lower = needle.to_ascii_lowercase();

    content
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            // Cheap substring check first (avoids parsing on miss).
            if !line.to_ascii_lowercase().contains(&needle_lower) {
                return None;
            }
            // Then verify level filter (requires a parse).
            match parse_str(line) {
                Ok(rec) if filter.matches(rec.level) => Some(SearchHit {
                    raw: line,
                    line_number: idx + 1,
                }),
                _ => None,
            }
        })
        .collect()
}

/// Searches within an optional time window (inclusive).
pub fn search_range<'a>(
    content: &'a str,
    needle: &str,
    filter: LevelFilter,
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
) -> Vec<SearchHit<'a>> {
    if needle.is_empty() {
        return Vec::new();
    }
    let needle_lower = needle.to_ascii_lowercase();

    content
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            if !line.to_ascii_lowercase().contains(&needle_lower) {
                return None;
            }
            let rec = parse_str(line).ok()?;
            if !filter.matches(rec.level) {
                return None;
            }
            if let Some(s) = since {
                if rec.timestamp < s {
                    return None;
                }
            }
            if let Some(u) = until {
                if rec.timestamp > u {
                    return None;
                }
            }
            Some(SearchHit {
                raw: line,
                line_number: idx + 1,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
[2024-01-15T08:00:00Z] [ERROR] Connection refused
[2024-01-15T08:00:01Z] [INFO] Connection established
[2024-01-15T08:00:02Z] [ERROR] Timeout
[2024-01-15T08:00:03Z] [WARN] Connection flaky
";

    #[test]
    fn search_case_insensitive() {
        let hits = search(SAMPLE, "connection", LevelFilter::All);
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn search_with_level_filter() {
        let hits = search(SAMPLE, "connection", LevelFilter::AtLeast(crate::Level::Error));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line_number, 1);
    }

    #[test]
    fn search_no_match() {
        let hits = search(SAMPLE, "zebra", LevelFilter::All);
        assert!(hits.is_empty());
    }

    #[test]
    fn search_empty_needle() {
        let hits = search(SAMPLE, "", LevelFilter::All);
        assert!(hits.is_empty());
    }

    #[test]
    fn search_hits_borrow_source() {
        let hits = search(SAMPLE, "timeout", LevelFilter::All);
        assert_eq!(hits.len(), 1);
        // Confirm the hit really points into SAMPLE (same pointer base).
        let base = SAMPLE.as_ptr() as usize;
        let hit_ptr = hits[0].raw.as_ptr() as usize;
        assert!(hit_ptr >= base && hit_ptr < base + SAMPLE.len());
    }

    #[test]
    fn search_range_filters_by_time() {
        let since = "2024-01-15T08:00:01Z".parse::<DateTime<Utc>>().unwrap();
        let until = "2024-01-15T08:00:02Z".parse::<DateTime<Utc>>().unwrap();
        let hits = search_range(
            SAMPLE,
            "connection",
            LevelFilter::All,
            Some(since),
            Some(until),
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line_number, 2);
    }

    #[test]
    fn search_range_since_only() {
        let since = "2024-01-15T08:00:02Z".parse::<DateTime<Utc>>().unwrap();
        let hits = search_range(SAMPLE, "connection", LevelFilter::All, Some(since), None);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line_number, 4);
    }
}
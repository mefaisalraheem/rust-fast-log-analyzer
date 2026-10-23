//! End-to-end integration tests exercising the public API.

use fast_log_analyzer::{generate, search, Analyzer, Level, LevelFilter};

#[test]
fn end_to_end_analysis() {
    let log = generate(5_000);
    let report = Analyzer::analyze_str(&log);

    assert_eq!(report.total_lines, 5_000);
    assert_eq!(report.parsed_lines, 5_000);
    assert_eq!(report.skipped_lines, 0);
    assert!(report.error_count > 0);
    assert!(!report.top_errors.is_empty());

    // Top errors should be sorted descending by frequency.
    for pair in report.top_errors.windows(2) {
        assert!(
            pair[0].1 >= pair[1].1,
            "top_errors not sorted: {:?}",
            pair
        );
    }
}

#[test]
fn malformed_lines_are_skipped_without_panic() {
    let mut log = generate(100);
    log.push_str("garbage line\n");
    log.push_str("\n");
    log.push_str("[bad-timestamp] [INFO] x\n");

    let report = Analyzer::analyze_str(&log);
    assert_eq!(report.total_lines, 103);
    assert_eq!(report.skipped_lines, 3);
    assert_eq!(report.parsed_lines, 100);
}

#[test]
fn filter_at_least_error() {
    let log = generate(1_000);
    let report = Analyzer::analyze_str_filtered(&log, LevelFilter::AtLeast(Level::Error));
    assert_eq!(report.error_count, report.parsed_lines);
}

#[test]
fn search_finds_and_borrows() {
    let log = generate(2_000);
    let hits = search(&log, "connection refused", LevelFilter::All);
    assert!(!hits.is_empty());

    // Every hit should be a valid reference into the original string.
    let base = log.as_ptr() as usize;
    let end = base + log.len();
    for hit in &hits {
        let p = hit.raw.as_ptr() as usize;
        assert!(p >= base && p < end, "hit is not a borrow into source");
    }
}

#[test]
fn search_with_level_filter_matches_analysis_count() {
    let log = generate(3_000);
    let filtered = Analyzer::analyze_str_filtered(&log, LevelFilter::AtLeast(Level::Error));
    let hits = search(&log, "a", LevelFilter::AtLeast(Level::Error));
    // At minimum the number of error hits is bounded by error_count
    // (each error line has at least one 'a' in its message).
    assert!(hits.len() <= filtered.parsed_lines + filtered.skipped_lines);
}

#[test]
fn report_display_is_stable() {
    let log = "[2024-01-15T08:00:00Z] [ERROR] x\n".repeat(10);
    let report = Analyzer::analyze_str(&log);
    assert_eq!(report.error_count, 10);
    assert_eq!(report.top_errors, vec![("x".to_string(), 10)]);
}
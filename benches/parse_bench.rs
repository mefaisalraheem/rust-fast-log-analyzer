//! Criterion benchmarks comparing naive vs streaming parsing
//! and measuring search / grouping throughput.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

use fast_log_analyzer::{generate, search, Analyzer, LevelFilter};

fn bench_parse_single(c: &mut Criterion) {
    let line = "[2024-01-15T08:00:00Z] [ERROR] Connection refused to upstream service";
    let mut group = c.benchmark_group("parse_line");
    group.throughput(Throughput::Bytes(line.len() as u64));
    group.bench_function("structured", |b| {
        b.iter(|| {
            let r = fast_log_analyzer::parse_str(black_box(line)).unwrap();
            black_box(r);
        })
    });
    group.finish();
}

fn bench_analyze(c: &mut Criterion) {
    let log = generate(10_000);
    let bytes = log.len() as u64;

    let mut group = c.benchmark_group("analyze");
    group.throughput(Throughput::Bytes(bytes));
    group.bench_function("10k_lines", |b| {
        b.iter(|| {
            let r = Analyzer::analyze_str(black_box(&log));
            black_box(r);
        })
    });
    group.finish();
}

fn bench_search(c: &mut Criterion) {
    let log = generate(10_000);
    let bytes = log.len() as u64;

    let mut group = c.benchmark_group("search");
    group.throughput(Throughput::Bytes(bytes));
    group.bench_function("10k_lines/connection", |b| {
        b.iter(|| {
            let hits = search(black_box(&log), "connection", LevelFilter::All);
            black_box(hits.len());
        })
    });
    group.finish();
}

criterion_group!(benches, bench_parse_single, bench_analyze, bench_search);
criterion_main!(benches);
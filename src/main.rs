//! CLI for `log-analyzer`.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use fast_log_analyzer::{
    generator, Analyzer, Level, LevelFilter, Report,
};

#[derive(Parser)]
#[command(
    name = "log-analyzer",
    version,
    about = "Fast log analyzer: parse, group, and search application logs"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Analyze a log file and print a summary report.
    Analyze {
        /// Path to the log file ("-" for stdin).
        file: PathBuf,
        /// Only consider records at or above this level.
        #[arg(long)]
        level: Option<String>,
    },
    /// Show top error messages by frequency.
    Top {
        /// Path to the log file.
        file: PathBuf,
        /// How many results to show.
        #[arg(short = 'n', long, default_value_t = 10)]
        n: usize,
        /// Only consider records at or above this level.
        #[arg(long)]
        level: Option<String>,
    },
    /// Search log lines (case-insensitive substring match).
    Search {
        /// Path to the log file.
        file: PathBuf,
        /// Substring to search for.
        needle: String,
        /// Only consider records at or above this level.
        #[arg(long)]
        level: Option<String>,
    },
    /// Generate a synthetic sample log to stdout.
    Generate {
        /// Number of lines to generate.
        #[arg(short = 'n', long, default_value_t = 1000)]
        lines: usize,
    },
}

fn read_input(path: &PathBuf) -> Result<String> {
    if path.as_os_str() == "-" {
        use std::io::Read;
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        Ok(s)
    } else {
        fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))
    }
}

fn parse_filter(level: Option<&String>) -> Result<LevelFilter> {
    match level {
        None => Ok(LevelFilter::All),
        Some(s) => {
            let lvl: Level = s.parse().map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(LevelFilter::AtLeast(lvl))
        }
    }
}

fn print_report(report: &Report) {
    println!("=== Log Analysis Report ===");
    println!("Total lines:   {}", report.total_lines);
    println!("Parsed:        {}", report.parsed_lines);
    println!("Skipped:       {}", report.skipped_lines);

    println!("\nLevel breakdown:");
    for (level, count) in &report.level_counts {
        println!("  {:>8}  {}", level.as_str(), count);
    }

    if let (Some(e), Some(l)) = (report.earliest, report.latest) {
        println!("\nTime range:    {} → {}", e.to_rfc3339(), l.to_rfc3339());
    }

    println!("\nTop errors:");
    if report.top_errors.is_empty() {
        println!("  (none)");
    } else {
        for (i, (msg, count)) in report.top_errors.iter().take(10).enumerate() {
            println!("  {:>2}. [{}x] {}", i + 1, count, msg);
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Analyze { file, level } => {
            let content = read_input(&file)?;
            let filter = parse_filter(level.as_ref())?;
            let report = match filter {
                LevelFilter::All => Analyzer::analyze_str(&content),
                _ => Analyzer::analyze_str_filtered(&content, filter),
            };
            print_report(&report);
        }
        Command::Top { file, n, level } => {
            let content = read_input(&file)?;
            let filter = parse_filter(level.as_ref())?;
            let report = match filter {
                LevelFilter::All => Analyzer::analyze_str(&content),
                _ => Analyzer::analyze_str_filtered(&content, filter),
            };
            if report.top_errors.is_empty() {
                println!("No errors found.");
            } else {
                for (i, (msg, count)) in report.top_errors.iter().take(n).enumerate() {
                    println!("{:>2}. [{}x] {}", i + 1, count, msg);
                }
            }
        }
        Command::Search { file, needle, level } => {
            let content = read_input(&file)?;
            let filter = parse_filter(level.as_ref())?;
            let hits = fast_log_analyzer::search(&content, &needle, filter);
            if hits.is_empty() {
                println!("No matches.");
            } else {
                for hit in &hits {
                    println!("{}: {}", hit.line_number, hit.raw);
                }
                println!("\n{} match(es).", hits.len());
            }
        }
        Command::Generate { lines } => {
            print!("{}", generator::generate(lines));
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
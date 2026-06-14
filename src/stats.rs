use crate::files::{discover_files, read_utf8_text, SkipReason};
use anyhow::Result;
use rayon::prelude::*;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct StatsOptions {
    pub path: PathBuf,
}

#[derive(Debug, Default, Serialize)]
pub struct StatsSummary {
    pub total_files: usize,
    pub scanned_files: usize,
    pub skipped_files: usize,
    pub binary_files: usize,
    pub non_utf8_files: usize,
    pub unreadable_files: usize,
    pub skipped_directories: usize,
    pub total_lines: usize,
    pub total_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct StatsReport {
    pub command: &'static str,
    pub root: String,
    pub stats: StatsSummary,
    pub errors: Vec<String>,
}

#[derive(Debug)]
struct FileStats {
    line_count: usize,
    byte_count: u64,
    skip_reason: Option<SkipReason>,
}

pub fn run(options: &StatsOptions) -> Result<StatsReport> {
    let discovery = discover_files(&options.path)?;
    let results: Vec<FileStats> = discovery
        .files
        .par_iter()
        .map(|path| stats_for_file(path))
        .collect();

    let mut stats = StatsSummary {
        total_files: discovery.files.len(),
        skipped_directories: discovery.skipped_directories,
        ..StatsSummary::default()
    };
    let mut errors = discovery.errors;

    for (path, result) in discovery.files.iter().zip(results) {
        match result.skip_reason {
            None => {
                stats.scanned_files += 1;
                stats.total_lines += result.line_count;
                stats.total_bytes += result.byte_count;
            }
            Some(SkipReason::Binary) => {
                stats.skipped_files += 1;
                stats.binary_files += 1;
            }
            Some(SkipReason::NonUtf8) => {
                stats.skipped_files += 1;
                stats.non_utf8_files += 1;
            }
            Some(SkipReason::Unreadable(message)) => {
                stats.skipped_files += 1;
                stats.unreadable_files += 1;
                errors.push(format!("Could not read {}: {message}", path.display()));
            }
        }
    }

    Ok(StatsReport {
        command: "stats",
        root: options.path.display().to_string(),
        stats,
        errors,
    })
}

fn stats_for_file(path: &Path) -> FileStats {
    match read_utf8_text(path) {
        Ok(content) => FileStats {
            line_count: content.lines().count(),
            byte_count: content.len() as u64,
            skip_reason: None,
        },
        Err(reason) => FileStats {
            line_count: 0,
            byte_count: 0,
            skip_reason: Some(reason),
        },
    }
}

use crate::files::{discover_files, read_utf8_text, SkipReason};
use anyhow::{Context, Result};
use rayon::prelude::*;
use regex::{Regex, RegexBuilder};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub pattern: String,
    pub path: PathBuf,
    pub ignore_case: bool,
    pub regex: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct SearchMatch {
    pub path: String,
    pub line_number: usize,
    pub line: String,
}

#[derive(Debug, Default, Serialize)]
pub struct SearchSummary {
    pub total_files: usize,
    pub scanned_files: usize,
    pub skipped_files: usize,
    pub binary_files: usize,
    pub non_utf8_files: usize,
    pub unreadable_files: usize,
    pub skipped_directories: usize,
    pub matches: usize,
}

#[derive(Debug, Serialize)]
pub struct SearchReport {
    pub command: &'static str,
    pub pattern: String,
    pub root: String,
    pub ignore_case: bool,
    pub regex: bool,
    pub matches: Vec<SearchMatch>,
    pub summary: SearchSummary,
    pub errors: Vec<String>,
}

#[derive(Debug)]
struct FileSearchResult {
    matches: Vec<SearchMatch>,
    skip_reason: Option<SkipReason>,
}

pub fn run(options: &SearchOptions) -> Result<SearchReport> {
    let matcher = build_matcher(&options.pattern, options.regex, options.ignore_case)?;
    let discovery = discover_files(&options.path)?;

    let results: Vec<FileSearchResult> = discovery
        .files
        .par_iter()
        .map(|path| search_file(path, &matcher))
        .collect();

    let mut summary = SearchSummary {
        total_files: discovery.files.len(),
        skipped_directories: discovery.skipped_directories,
        ..SearchSummary::default()
    };
    let mut matches = Vec::new();
    let mut errors = discovery.errors;

    for (path, result) in discovery.files.iter().zip(results) {
        match result.skip_reason {
            None => {
                summary.scanned_files += 1;
                matches.extend(result.matches);
            }
            Some(SkipReason::Binary) => {
                summary.skipped_files += 1;
                summary.binary_files += 1;
            }
            Some(SkipReason::NonUtf8) => {
                summary.skipped_files += 1;
                summary.non_utf8_files += 1;
            }
            Some(SkipReason::Unreadable(message)) => {
                summary.skipped_files += 1;
                summary.unreadable_files += 1;
                errors.push(format!("Could not read {}: {message}", path.display()));
            }
        }
    }

    summary.matches = matches.len();

    Ok(SearchReport {
        command: "search",
        pattern: options.pattern.clone(),
        root: options.path.display().to_string(),
        ignore_case: options.ignore_case,
        regex: options.regex,
        matches,
        summary,
        errors,
    })
}

fn build_matcher(pattern: &str, use_regex: bool, ignore_case: bool) -> Result<Regex> {
    let expression = if use_regex {
        pattern.to_owned()
    } else {
        regex::escape(pattern)
    };

    RegexBuilder::new(&expression)
        .case_insensitive(ignore_case)
        .build()
        .with_context(|| format!("Invalid regex pattern: {pattern}"))
}

fn search_file(path: &Path, matcher: &Regex) -> FileSearchResult {
    let content = match read_utf8_text(path) {
        Ok(content) => content,
        Err(reason) => {
            return FileSearchResult {
                matches: Vec::new(),
                skip_reason: Some(reason),
            };
        }
    };

    let display_path = path.display().to_string();
    let matches = content
        .lines()
        .enumerate()
        .filter(|(_, line)| matcher.is_match(line))
        .map(|(index, line)| SearchMatch {
            path: display_path.clone(),
            line_number: index + 1,
            line: line.to_owned(),
        })
        .collect();

    FileSearchResult {
        matches,
        skip_reason: None,
    }
}

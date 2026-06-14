use crate::files::{backup_path, discover_files, read_utf8_text, SkipReason};
use anyhow::{bail, Result};
use rayon::prelude::*;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ReplaceOptions {
    pub old_text: String,
    pub new_text: String,
    pub path: PathBuf,
    pub dry_run: bool,
    pub backup: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ReplacementChange {
    pub path: String,
    pub line_number: usize,
    pub old_line: String,
    pub new_line: String,
    pub replacements: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct ReplaceSummary {
    pub total_files: usize,
    pub scanned_files: usize,
    pub skipped_files: usize,
    pub binary_files: usize,
    pub non_utf8_files: usize,
    pub unreadable_files: usize,
    pub skipped_directories: usize,
    pub changed_files: usize,
    pub changed_lines: usize,
    pub replacements: usize,
    pub backups_created: usize,
    pub failed_files: usize,
}

#[derive(Debug, Serialize)]
pub struct ReplaceReport {
    pub command: &'static str,
    pub old_text: String,
    pub new_text: String,
    pub root: String,
    pub dry_run: bool,
    pub backup: bool,
    pub changes: Vec<ReplacementChange>,
    pub summary: ReplaceSummary,
    pub errors: Vec<String>,
}

#[derive(Debug)]
struct FileReplaceResult {
    scanned: bool,
    skip_reason: Option<SkipReason>,
    changes: Vec<ReplacementChange>,
    replacements: usize,
    backup_created: bool,
    failed: bool,
    error: Option<String>,
}

pub fn run(options: &ReplaceOptions) -> Result<ReplaceReport> {
    validate_options(options)?;
    let discovery = discover_files(&options.path)?;

    let results: Vec<FileReplaceResult> = discovery
        .files
        .par_iter()
        .map(|path| replace_file(path, options))
        .collect();

    let mut summary = ReplaceSummary {
        total_files: discovery.files.len(),
        skipped_directories: discovery.skipped_directories,
        ..ReplaceSummary::default()
    };
    let mut changes = Vec::new();
    let mut errors = discovery.errors;

    for (path, result) in discovery.files.iter().zip(results) {
        if result.scanned {
            summary.scanned_files += 1;
        }

        match result.skip_reason {
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
            None => {}
        }

        if result.failed {
            summary.failed_files += 1;
        }
        if result.backup_created {
            summary.backups_created += 1;
        }
        if !result.changes.is_empty() {
            summary.changed_files += 1;
            summary.changed_lines += result.changes.len();
            summary.replacements += result.replacements;
            changes.extend(result.changes);
        }
        if let Some(error) = result.error {
            errors.push(error);
        }
    }

    Ok(ReplaceReport {
        command: "replace",
        old_text: options.old_text.clone(),
        new_text: options.new_text.clone(),
        root: options.path.display().to_string(),
        dry_run: options.dry_run,
        backup: options.backup,
        changes,
        summary,
        errors,
    })
}

fn validate_options(options: &ReplaceOptions) -> Result<()> {
    if options.old_text.is_empty() {
        bail!("oldText cannot be empty");
    }
    if options.old_text.contains(['\r', '\n']) {
        bail!("oldText cannot contain line breaks because replacements are line-oriented");
    }
    if options.new_text.contains(['\r', '\n']) {
        bail!("newText cannot contain line breaks because replacements are line-oriented");
    }
    Ok(())
}

fn replace_file(path: &Path, options: &ReplaceOptions) -> FileReplaceResult {
    let content = match read_utf8_text(path) {
        Ok(content) => content,
        Err(reason) => return skipped_result(reason),
    };

    let (new_content, changes, replacements) =
        replace_lines(path, &content, &options.old_text, &options.new_text);

    if changes.is_empty() || options.dry_run {
        return FileReplaceResult {
            scanned: true,
            skip_reason: None,
            changes,
            replacements,
            backup_created: false,
            failed: false,
            error: None,
        };
    }

    let mut backup_created = false;
    if options.backup {
        let backup = backup_path(path);
        if let Err(error) = fs::copy(path, &backup) {
            return failed_result(
                format!(
                    "Could not create backup {} for {}: {error}",
                    backup.display(),
                    path.display()
                ),
                backup_created,
            );
        }
        backup_created = true;
    }

    if let Err(error) = fs::write(path, new_content) {
        return failed_result(
            format!("Could not write {}: {error}", path.display()),
            backup_created,
        );
    }

    FileReplaceResult {
        scanned: true,
        skip_reason: None,
        changes,
        replacements,
        backup_created,
        failed: false,
        error: None,
    }
}

fn replace_lines(
    path: &Path,
    content: &str,
    old_text: &str,
    new_text: &str,
) -> (String, Vec<ReplacementChange>, usize) {
    let mut new_content = String::with_capacity(content.len());
    let mut changes = Vec::new();
    let mut total_replacements = 0;
    let display_path = path.display().to_string();

    for (index, segment) in content.split_inclusive('\n').enumerate() {
        let (line, ending) = split_line_ending(segment);
        let replacement_count = line.matches(old_text).count();

        if replacement_count == 0 {
            new_content.push_str(segment);
            continue;
        }

        let new_line = line.replace(old_text, new_text);
        total_replacements += replacement_count;
        changes.push(ReplacementChange {
            path: display_path.clone(),
            line_number: index + 1,
            old_line: line.to_owned(),
            new_line: new_line.clone(),
            replacements: replacement_count,
        });
        new_content.push_str(&new_line);
        new_content.push_str(ending);
    }

    (new_content, changes, total_replacements)
}

fn split_line_ending(segment: &str) -> (&str, &str) {
    if let Some(line) = segment.strip_suffix("\r\n") {
        (line, "\r\n")
    } else if let Some(line) = segment.strip_suffix('\n') {
        (line, "\n")
    } else {
        (segment, "")
    }
}

fn skipped_result(reason: SkipReason) -> FileReplaceResult {
    FileReplaceResult {
        scanned: false,
        skip_reason: Some(reason),
        changes: Vec::new(),
        replacements: 0,
        backup_created: false,
        failed: false,
        error: None,
    }
}

fn failed_result(error: String, backup_created: bool) -> FileReplaceResult {
    FileReplaceResult {
        scanned: true,
        skip_reason: None,
        changes: Vec::new(),
        replacements: 0,
        backup_created,
        failed: true,
        error: Some(error),
    }
}

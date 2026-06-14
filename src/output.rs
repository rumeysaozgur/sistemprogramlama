use crate::replace::ReplaceReport;
use crate::search::SearchReport;
use crate::stats::StatsReport;
use anyhow::Result;
use serde::Serialize;

pub fn print_search(report: &SearchReport, json: bool) -> Result<()> {
    if json {
        return print_json(report);
    }

    for item in &report.matches {
        println!("{}:{}: {}", item.path, item.line_number, item.line);
    }
    println!(
        "\nMatches: {} | Scanned files: {} | Skipped files: {} | Skipped directories: {}",
        report.summary.matches,
        report.summary.scanned_files,
        report.summary.skipped_files,
        report.summary.skipped_directories
    );
    print_errors(&report.errors);
    Ok(())
}

pub fn print_replace(report: &ReplaceReport, json: bool) -> Result<()> {
    if json {
        return print_json(report);
    }

    let mode = if report.dry_run { "DRY-RUN" } else { "CHANGED" };
    for change in &report.changes {
        println!(
            "[{mode}] {}:{} ({} replacement(s))",
            change.path, change.line_number, change.replacements
        );
        println!("- {}", change.old_line);
        println!("+ {}", change.new_line);
    }
    println!(
        "\nChanged files: {} | Changed lines: {} | Replacements: {} | Backups: {} | Failed files: {}",
        report.summary.changed_files,
        report.summary.changed_lines,
        report.summary.replacements,
        report.summary.backups_created,
        report.summary.failed_files
    );
    print_errors(&report.errors);
    Ok(())
}

pub fn print_stats(report: &StatsReport, json: bool) -> Result<()> {
    if json {
        return print_json(report);
    }

    println!("Root: {}", report.root);
    println!("Total files: {}", report.stats.total_files);
    println!("Scanned files: {}", report.stats.scanned_files);
    println!("Skipped files: {}", report.stats.skipped_files);
    println!("Binary files: {}", report.stats.binary_files);
    println!("Non-UTF-8 files: {}", report.stats.non_utf8_files);
    println!("Unreadable files: {}", report.stats.unreadable_files);
    println!("Skipped directories: {}", report.stats.skipped_directories);
    println!("Total lines: {}", report.stats.total_lines);
    println!("Total bytes: {}", report.stats.total_bytes);
    print_errors(&report.errors);
    Ok(())
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn print_errors(errors: &[String]) {
    for error in errors {
        eprintln!("Warning: {error}");
    }
}

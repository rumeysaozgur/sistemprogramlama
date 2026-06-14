use aitoolgrep::output;
use aitoolgrep::replace::{self, ReplaceOptions};
use aitoolgrep::search::{self, SearchOptions};
use aitoolgrep::stats::{self, StatsOptions};
use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use serde_json::json;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(
    name = "aitoolgrep",
    version,
    about = "AI-friendly parallel search, replace, and codebase statistics tool"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Recursively search UTF-8 text files
    Search(SearchArgs),
    /// Replace literal text in UTF-8 text files
    Replace(ReplaceArgs),
    /// Show directory scan statistics
    Stats(StatsArgs),
}

#[derive(Debug, Args)]
struct SearchArgs {
    /// Literal text or regex pattern to find
    pattern: String,
    /// File or directory to search
    path: PathBuf,
    /// Perform a case-insensitive search
    #[arg(short = 'i', long, conflicts_with = "case_sensitive")]
    ignore_case: bool,
    /// Explicitly perform a case-sensitive search (the default)
    #[arg(short = 's', long, conflicts_with = "ignore_case")]
    case_sensitive: bool,
    /// Interpret pattern as a regular expression
    #[arg(short = 'e', long)]
    regex: bool,
    /// Print one valid JSON report
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ReplaceArgs {
    /// Literal text to replace
    old_text: String,
    /// Replacement text
    new_text: String,
    /// File or directory to process
    path: PathBuf,
    /// Report changes without writing files
    #[arg(long)]
    dry_run: bool,
    /// Create a .bak file before changing each file
    #[arg(long)]
    backup: bool,
    /// Print one valid JSON report
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct StatsArgs {
    /// File or directory to inspect
    path: PathBuf,
    /// Print one valid JSON report
    #[arg(long)]
    json: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json_output = cli.wants_json();

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if json_output {
                eprintln!("{}", json!({ "error": format!("{error:#}") }));
            } else {
                eprintln!("Error: {error:#}");
            }
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Search(args) => {
            let report = search::run(&SearchOptions {
                pattern: args.pattern,
                path: args.path,
                ignore_case: args.ignore_case,
                regex: args.regex,
            })?;
            output::print_search(&report, args.json)
        }
        Command::Replace(args) => {
            let report = replace::run(&ReplaceOptions {
                old_text: args.old_text,
                new_text: args.new_text,
                path: args.path,
                dry_run: args.dry_run,
                backup: args.backup,
            })?;
            output::print_replace(&report, args.json)
        }
        Command::Stats(args) => {
            let report = stats::run(&StatsOptions { path: args.path })?;
            output::print_stats(&report, args.json)
        }
    }
}

impl Cli {
    fn wants_json(&self) -> bool {
        match &self.command {
            Command::Search(args) => args.json,
            Command::Replace(args) => args.json,
            Command::Stats(args) => args.json,
        }
    }
}

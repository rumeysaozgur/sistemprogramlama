use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

const EXCLUDED_DIRECTORIES: &[&str] = &[".git", "bin", "obj", "node_modules", ".vs"];

#[derive(Debug)]
pub struct Discovery {
    pub files: Vec<PathBuf>,
    pub skipped_directories: usize,
    pub errors: Vec<String>,
}

#[derive(Debug)]
pub enum SkipReason {
    Binary,
    NonUtf8,
    Unreadable(String),
}

pub fn discover_files(root: &Path) -> Result<Discovery> {
    if !root.exists() {
        bail!("Path does not exist: {}", root.display());
    }

    if root.is_file() {
        return Ok(Discovery {
            files: vec![root.to_path_buf()],
            skipped_directories: 0,
            errors: Vec::new(),
        });
    }

    if !root.is_dir() {
        bail!("Path is neither a file nor a directory: {}", root.display());
    }

    let mut files = Vec::new();
    let mut skipped_directories = 0;
    let mut errors = Vec::new();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            let excluded = is_excluded_directory(entry);
            if excluded {
                skipped_directories += 1;
            }
            !excluded
        });

    for entry in walker {
        match entry {
            Ok(entry) if entry.file_type().is_file() => files.push(entry.into_path()),
            Ok(_) => {}
            Err(error) => errors.push(error.to_string()),
        }
    }

    files.sort();

    Ok(Discovery {
        files,
        skipped_directories,
        errors,
    })
}

pub fn read_utf8_text(path: &Path) -> Result<String, SkipReason> {
    let bytes = fs::read(path).map_err(|error| SkipReason::Unreadable(error.to_string()))?;

    if bytes.contains(&0) {
        return Err(SkipReason::Binary);
    }

    String::from_utf8(bytes).map_err(|_| SkipReason::NonUtf8)
}

pub fn backup_path(path: &Path) -> PathBuf {
    let mut backup_name = path.as_os_str().to_os_string();
    backup_name.push(".bak");
    PathBuf::from(backup_name)
}

fn is_excluded_directory(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }

    let name = entry.file_name().to_string_lossy();
    EXCLUDED_DIRECTORIES
        .iter()
        .any(|excluded| name.eq_ignore_ascii_case(excluded))
}

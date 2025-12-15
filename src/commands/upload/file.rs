//! File processing utilities for upload

use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

/// Validated file ready for upload
pub struct ValidatedFile {
    pub path: PathBuf,
}

/// File extension filter
pub struct FileFilter {
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
}

impl FileFilter {
    pub fn new(include: Option<Vec<String>>, exclude: Option<Vec<String>>) -> Self {
        // Normalize extensions (remove leading dots, lowercase)
        let include = include.map(|v| v.iter().map(|s| normalize_ext(s)).collect());
        let exclude = exclude.map(|v| v.iter().map(|s| normalize_ext(s)).collect());
        Self { include, exclude }
    }

    /// Check if a file passes the filter
    pub fn matches(&self, path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        // If include is set, file must match one of the extensions
        if let Some(ref includes) = self.include {
            if !includes.contains(&ext) {
                return false;
            }
        }

        // If exclude is set, file must not match any of the extensions
        if let Some(ref excludes) = self.exclude {
            if excludes.contains(&ext) {
                return false;
            }
        }

        true
    }
}

/// Normalize extension string (remove leading dot, lowercase)
fn normalize_ext(s: &str) -> String {
    s.trim_start_matches('.').to_lowercase()
}

/// Collect all files from paths (supports both files and directories)
pub fn collect_files(paths: &[String], filter: &FileFilter) -> (Vec<ValidatedFile>, usize) {
    let mut files = Vec::new();
    let mut failed = 0;

    for path_str in paths {
        let path = Path::new(path_str);

        if !path.exists() {
            println!("{} Path not found: {}", "✗".red(), path_str.red());
            failed += 1;
            continue;
        }

        if path.is_file() {
            if filter.matches(path) {
                files.push(ValidatedFile {
                    path: path.to_path_buf(),
                });
            }
        } else if path.is_dir() {
            let (dir_files, dir_failed) = collect_from_dir(path, filter);
            files.extend(dir_files);
            failed += dir_failed;
        }
    }

    (files, failed)
}

/// Recursively collect files from a directory
fn collect_from_dir(dir: &Path, filter: &FileFilter) -> (Vec<ValidatedFile>, usize) {
    let mut files = Vec::new();
    let mut failed = 0;

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            println!("{} Cannot read dir {}: {}", "✗".red(), dir.display(), e);
            return (files, 1);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if filter.matches(&path) {
                files.push(ValidatedFile {
                    path: path.to_path_buf(),
                });
            }
        } else if path.is_dir() {
            let (sub_files, sub_failed) = collect_from_dir(&path, filter);
            files.extend(sub_files);
            failed += sub_failed;
        }
    }

    (files, failed)
}

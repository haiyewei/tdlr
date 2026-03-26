//! File processing utilities for upload

use crate::i18n::pick;
use crate::utils::ExtFilter;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

/// Validated file ready for upload
pub struct ValidatedFile {
    pub path: PathBuf,
}

/// Collect all files from paths (supports both files and directories)
pub fn collect_files(paths: &[String], filter: &ExtFilter) -> (Vec<ValidatedFile>, usize) {
    let mut files = Vec::new();
    let mut failed = 0;

    for path_str in paths {
        let path = Path::new(path_str);

        if !path.exists() {
            println!(
                "{} {}: {}",
                "✗".red(),
                pick("路径不存在", "Path not found"),
                path_str.red()
            );
            failed += 1;
            continue;
        }

        if path.is_file() {
            if filter.matches_path(path) {
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
fn collect_from_dir(dir: &Path, filter: &ExtFilter) -> (Vec<ValidatedFile>, usize) {
    let mut files = Vec::new();
    let mut failed = 0;

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            println!(
                "{} {} {}: {}",
                "✗".red(),
                pick("无法读取目录", "Cannot read dir"),
                dir.display(),
                e
            );
            return (files, 1);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if filter.matches_path(&path) {
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

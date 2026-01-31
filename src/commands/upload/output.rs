//! Output formatting utilities for upload command

use colored::Colorize;
use std::path::Path;

pub use crate::utils::output::{
    print_account_header, print_account_not_authorized, print_failure,
    print_success_msg_id as print_success,
};

/// Print upload progress header
pub fn print_progress(index: usize, total: usize, path: &Path) {
    println!(
        "\n[{}/{}] {} {}",
        index + 1,
        total,
        "Uploading:".cyan(),
        path.display()
    );
}

/// Print upload summary
pub fn print_summary(success: usize, failed: usize) {
    crate::utils::output::print_summary("file(s) uploaded", success, failed);
}

/// Print media group progress
pub fn print_group_progress(batch_idx: usize, total_batches: usize, batch_size: usize) {
    println!(
        "{} Uploading media group [{}/{}] ({} files)",
        "→".cyan(),
        batch_idx + 1,
        total_batches,
        batch_size
    );
}

/// Print media group success
pub fn print_group_success(count: usize) {
    println!("{} Media group sent ({} files)", "✓".green(), count);
}

/// Print media group failure
pub fn print_group_failure(error: &str) {
    println!("{} Media group failed: {}", "✗".red(), error);
}

/// Print skipped files warning
pub fn print_skipped_files(count: usize, reason: &str) {
    println!("{} {} file(s) skipped ({})", "⚠".yellow(), count, reason);
}

/// Print no media files warning
pub fn print_no_media_files() {
    println!("{} No media files to upload as group", "⚠".yellow());
}

/// Print file removal result
pub fn print_removed_files(count: usize) {
    println!("{} {} file(s) removed", "🗑".dimmed(), count);
}

/// Print file removal failure
pub fn print_remove_failure(error: &str) {
    println!("  {} Failed to remove: {}", "⚠".yellow(), error);
}

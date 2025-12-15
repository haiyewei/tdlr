//! Output formatting utilities for upload command

use colored::Colorize;
use std::path::Path;

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

/// Print upload success
pub fn print_success(msg_id: i32) {
    println!("{} Uploaded (msg_id: {})", "✓".green(), msg_id);
}

/// Print upload failure
pub fn print_failure(error: &str) {
    println!("{} Failed: {}", "✗".red(), error.red());
}

/// Print upload summary
pub fn print_summary(success: usize, failed: usize) {
    println!();
    if failed == 0 {
        println!(
            "{} All {} file(s) uploaded successfully!",
            "✓".green(),
            success
        );
    } else {
        println!(
            "{}: {} success, {} failed",
            "Summary".cyan(),
            success.to_string().green(),
            failed.to_string().red()
        );
    }
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

/// Print account header
pub fn print_account_header(name: &str, user_id: i64) {
    println!("\n{} Account: {} ({})", "→".cyan(), name, user_id);
}

/// Print account not authorized warning
pub fn print_account_not_authorized(user_id: i64) {
    println!(
        "{} Account {} not authorized, skipping",
        "⚠".yellow(),
        user_id
    );
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

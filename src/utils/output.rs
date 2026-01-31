//! Common output formatting utilities

use colored::Colorize;
use std::io::Write;

/// Print account header
pub fn print_account_header(name: &str, user_id: i64) {
    println!(
        "\n{} {} ({})",
        "Account:".cyan().bold(),
        name,
        user_id.to_string().dimmed()
    );
}

/// Print account not authorized warning
pub fn print_account_not_authorized(user_id: i64) {
    println!(
        "{} Account {} not authorized, skipping",
        "⚠".yellow(),
        user_id
    );
}

/// Print success with message ID
pub fn print_success_msg_id(msg_id: i32) {
    println!(" {} msg_id={}", "✓".green(), msg_id);
}

/// Print success with filename and size
pub fn print_success_file(filename: &str, size: u64) {
    let size_str = super::format_size(size);
    println!("  {} Saved: {} ({})", "✓".green(), filename, size_str);
}

/// Print failure message
pub fn print_failure(error: &str) {
    println!(" {} {}", "✗".red(), error.red());
}

/// Print summary for operations
pub fn print_summary(operation: &str, success: usize, failed: usize) {
    println!();
    if failed == 0 {
        println!("{} {} {} successfully.", "✓".green(), success, operation);
    } else {
        println!(
            "{} {} succeeded, {} failed.",
            "Done:".cyan(),
            success.to_string().green(),
            failed.to_string().red()
        );
    }
}

/// Print progress inline (no newline, for updating same line)
pub fn print_progress_inline(operation: &str, current: usize, total: usize, detail: &str) {
    print!(
        "\r{} [{}/{}] {}",
        operation.cyan(),
        current + 1,
        total,
        detail.dimmed()
    );
    let _ = std::io::stdout().flush();
}

/// Print progress with newline
pub fn print_progress(operation: &str, current: usize, total: usize, detail: &str) {
    println!(
        "[{}/{}] {} {}",
        current + 1,
        total,
        operation.cyan(),
        detail
    );
}

/// Print warning message
pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow(), message);
}

/// Print skipped item
pub fn print_skipped(item: &str, reason: &str) {
    println!("  {} Skipped: {} ({})", "⊘".dimmed(), item, reason);
}

/// Print no media found
pub fn print_no_media() {
    println!("  {} No downloadable media in message", "⚠".yellow());
}

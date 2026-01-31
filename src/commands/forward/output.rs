//! Output formatting for forward command
//! Re-exports common output utilities

pub use crate::utils::output::{
    print_account_header, print_failure, print_success_msg_id as print_success,
};

use colored::Colorize;
use std::io::Write;

/// Print forward progress (inline, updates same line)
pub fn print_progress(current: usize, total: usize, source: &str) {
    print!(
        "\r{} [{}/{}] {}",
        "Forwarding".cyan(),
        current + 1,
        total,
        source.dimmed()
    );
    let _ = std::io::stdout().flush();
}

/// Print forward summary
pub fn print_summary(success: usize, failed: usize) {
    crate::utils::output::print_summary("message(s) forwarded", success, failed);
}

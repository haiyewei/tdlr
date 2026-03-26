//! Output formatting utilities for download command

use crate::i18n::pick;
use crate::utils::TelegramLink;

pub use crate::utils::output::{
    print_account_header, print_failure, print_no_media, print_skipped,
    print_success_file as print_success,
};

/// Print download progress
pub fn print_progress(index: usize, total: usize, link: &TelegramLink) {
    println!(
        "[{}/{}] {} {} msg:{}",
        index + 1,
        total,
        pick("下载自", "Downloading from"),
        link.chat.display(),
        link.effective_message_id()
    );
}

/// Print download summary
pub fn print_summary(success: usize, failed: usize) {
    crate::utils::output::print_summary("file(s) downloaded", success, failed);
}

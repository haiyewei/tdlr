//! Output formatting utilities for upload command

use crate::i18n::pick;
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
        pick("正在上传:", "Uploading:").cyan(),
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
        "{} {} [{}/{}] ({} {})",
        "→".cyan(),
        pick("正在上传媒体组", "Uploading media group"),
        batch_idx + 1,
        total_batches,
        batch_size,
        pick("个文件", "files")
    );
}

/// Print media group success
pub fn print_group_success(count: usize) {
    println!(
        "{} {} ({} {})",
        "✓".green(),
        pick("媒体组已发送", "Media group sent"),
        count,
        pick("个文件", "files")
    );
}

/// Print media group failure
pub fn print_group_failure(error: &str) {
    println!(
        "{} {}: {}",
        "✗".red(),
        pick("媒体组发送失败", "Media group failed"),
        error
    );
}

/// Print skipped files warning
pub fn print_skipped_files(count: usize, reason: &str) {
    println!(
        "{} {} {} ({})",
        "⚠".yellow(),
        count,
        pick("个文件已跳过", "file(s) skipped"),
        reason
    );
}

/// Print no media files warning
pub fn print_no_media_files() {
    println!(
        "{} {}",
        "⚠".yellow(),
        pick(
            "没有可作为媒体组上传的媒体文件",
            "No media files to upload as group"
        )
    );
}

/// Print unused thumbnail warning.
pub fn print_unused_thumbnails(count: usize) {
    println!(
        "{} {} {}",
        "⚠".yellow(),
        count,
        pick(
            "个封面文件未匹配到任何视频，已忽略",
            "thumbnail file(s) were not matched to any video and were ignored"
        )
    );
}

/// Print file removal result
pub fn print_removed_files(count: usize) {
    println!(
        "{} {} {}",
        "🗑".dimmed(),
        count,
        pick("个文件已删除", "file(s) removed")
    );
}

/// Print file removal failure
pub fn print_remove_failure(error: &str) {
    println!(
        "  {} {}: {}",
        "⚠".yellow(),
        pick("删除失败", "Failed to remove"),
        error
    );
}

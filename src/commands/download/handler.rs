//! Download handlers

use super::output;
use super::template::{render, TemplateContext};
use crate::telegram::download::{
    download_document, download_photo, fetch_message, DocumentInfo, MessageContent, PhotoInfo,
};
use crate::utils::{get_extension, ChatIdentifier, ExtFilter, TelegramLink};
use anyhow::Result;
use grammers_client::Client;
use std::path::Path;

/// Download result statistics
#[derive(Default)]
pub struct DownloadStats {
    pub success: usize,
    pub failed: usize,
    pub skipped: usize,
}

impl DownloadStats {
    pub fn add_success(&mut self) {
        self.success += 1;
    }

    pub fn add_failed(&mut self) {
        self.failed += 1;
    }

    pub fn add_skipped(&mut self) {
        self.skipped += 1;
    }
}

/// Download context
pub struct DownloadContext<'a> {
    pub client: &'a Client,
    pub output_dir: &'a Path,
    pub filter: &'a ExtFilter,
    pub template: &'a str,
}

/// Download a single link
pub async fn download_link(
    ctx: &DownloadContext<'_>,
    link: &TelegramLink,
    stats: &mut DownloadStats,
) -> Result<()> {
    // Build chat string and get dialog_id
    let (chat_str, dialog_id) = match &link.chat {
        ChatIdentifier::Username(u) => (u.clone(), 0i64), // Will be resolved later
        ChatIdentifier::ChannelId(id) => (format!("-100{}", id), -100 * 10_000_000_000 - id),
        ChatIdentifier::External => {
            output::print_failure("Plain message ID not supported for download - use full URL");
            stats.add_failed();
            return Ok(());
        }
    };

    // For comments, we need special handling
    if link.is_comment() {
        output::print_failure(
            "Comment download not yet implemented - please use direct message links",
        );
        stats.add_failed();
        return Ok(());
    }

    let message_id = link.effective_message_id();

    // Fetch message and extract content
    let (resolved, content_list) = match fetch_message(ctx.client, &chat_str, message_id).await {
        Ok(result) => result,
        Err(e) => {
            output::print_failure(&format!("Failed to fetch message: {}", e));
            stats.add_failed();
            return Ok(());
        }
    };

    // Get actual dialog_id from resolved chat
    let dialog_id = resolved
        .peer
        .as_ref()
        .map(|p| p.id().bare_id())
        .unwrap_or(dialog_id);

    if content_list.is_empty() {
        output::print_no_media();
        stats.add_failed();
        return Ok(());
    }

    // Download each content
    for content in content_list {
        match content {
            MessageContent::Photo(photo, _) => {
                // Photos are always jpg
                if !ctx.filter.should_include("jpg") {
                    output::print_skipped("photo", "jpg");
                    stats.add_skipped();
                    continue;
                }

                let info = PhotoInfo::from_photo(&photo);
                let template_ctx =
                    TemplateContext::new(dialog_id, message_id, info.default_filename());

                let filename = render(ctx.template, &template_ctx);
                let file_path = ctx.output_dir.join(&filename);

                match download_photo(ctx.client, &photo, &file_path).await {
                    Ok(result) => {
                        output::print_success(&result.filename, result.size);
                        stats.add_success();
                    }
                    Err(e) => {
                        output::print_failure(&e.to_string());
                        stats.add_failed();
                    }
                }
            }
            MessageContent::Document(doc, _) => {
                let info = DocumentInfo::from_document(&doc);
                let ext = get_extension(&info.filename);

                if !ctx.filter.should_include(&ext) {
                    output::print_skipped(&info.filename, &ext);
                    stats.add_skipped();
                    continue;
                }

                let template_ctx =
                    TemplateContext::new(dialog_id, message_id, info.filename.clone())
                        .with_size(info.size);

                let filename = render(ctx.template, &template_ctx);
                let file_path = ctx.output_dir.join(&filename);

                match download_document(ctx.client, &doc, &file_path).await {
                    Ok(result) => {
                        output::print_success(&result.filename, result.size);
                        stats.add_success();
                    }
                    Err(e) => {
                        output::print_failure(&e.to_string());
                        stats.add_failed();
                    }
                }
            }
            MessageContent::Text(text) => {
                // Text messages are saved as .txt
                if !ctx.filter.should_include("txt") {
                    output::print_skipped("text", "txt");
                    stats.add_skipped();
                    continue;
                }

                let default_filename = format!("{}.txt", message_id);
                let template_ctx = TemplateContext::new(dialog_id, message_id, default_filename)
                    .with_size(text.len() as u64);

                let filename = render(ctx.template, &template_ctx);
                let file_path = ctx.output_dir.join(&filename);

                match std::fs::write(&file_path, &text) {
                    Ok(_) => {
                        let saved_name = file_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("text.txt");
                        output::print_success(saved_name, text.len() as u64);
                        stats.add_success();
                    }
                    Err(e) => {
                        output::print_failure(&e.to_string());
                        stats.add_failed();
                    }
                }
            }
        }
    }

    Ok(())
}

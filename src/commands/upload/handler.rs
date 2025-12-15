//! Upload handlers for single files and media groups

use super::expr::{eval_routing, FileContext};
use super::file::ValidatedFile;
use super::output;
use crate::telegram::upload::{
    is_media_group_supported, resolve_chat, upload_file, upload_media_group, ResolvedChat,
    MAX_MEDIA_GROUP_SIZE,
};
use anyhow::Result;
use futures::stream::{self, StreamExt};
use grammers_client::Client;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Upload result statistics
#[derive(Default)]
pub struct UploadStats {
    pub success: usize,
    pub failed: usize,
}

impl UploadStats {
    pub fn add_success(&mut self, count: usize) {
        self.success += count;
    }

    pub fn add_failed(&mut self, count: usize) {
        self.failed += count;
    }
}

/// Upload context for a single upload operation
pub struct UploadContext<'a> {
    pub client: &'a Client,
    pub chat: &'a Option<String>,
    pub topic: Option<i32>,
    pub caption: &'a Option<String>,
    pub to: &'a Option<String>,
    pub concurrent: usize,
}

/// Handle single file uploads with concurrency
pub async fn upload_single_files(
    ctx: &UploadContext<'_>,
    files: &[ValidatedFile],
    stats: &mut UploadStats,
) -> Result<()> {
    let total = files.len();

    // Pre-resolve all unique destinations
    let mut chat_cache: std::collections::HashMap<String, ResolvedChat> =
        std::collections::HashMap::new();

    // Collect unique destinations
    let mut destinations: Vec<(usize, String)> = Vec::new();
    for (i, file) in files.iter().enumerate() {
        let file_ctx = FileContext::from_path_with_context(&file.path, i, total);
        let dest = if let Some(ref to_expr) = ctx.to {
            eval_routing(to_expr, &file_ctx)
        } else {
            ctx.chat.clone().unwrap_or_default()
        };
        destinations.push((i, dest));
    }

    // Pre-resolve unique chats
    let unique_dests: std::collections::HashSet<_> =
        destinations.iter().map(|(_, d)| d.clone()).collect();
    for dest in unique_dests {
        if !chat_cache.contains_key(&dest) {
            match resolve_chat(ctx.client, &dest).await {
                Ok(c) => {
                    chat_cache.insert(dest, c);
                }
                Err(e) => {
                    output::print_failure(&format!("Failed to resolve '{}': {}", dest, e));
                }
            }
        }
    }

    // Use Arc<Mutex> for thread-safe stats
    let stats_mutex = Arc::new(Mutex::new((0usize, 0usize))); // (success, failed)

    // Process files concurrently
    let caption_ref = ctx.caption.as_deref();
    let _: Vec<_> = stream::iter(files.iter().enumerate())
        .map(|(i, file)| {
            let file_ctx = FileContext::from_path_with_context(&file.path, i, total);
            let dest = if let Some(ref to_expr) = ctx.to {
                eval_routing(to_expr, &file_ctx)
            } else {
                ctx.chat.clone().unwrap_or_default()
            };
            let chat = chat_cache.get(&dest);
            let stats_mutex = Arc::clone(&stats_mutex);

            async move {
                output::print_progress(i, total, &file.path);

                let Some(chat) = chat else {
                    let mut s = stats_mutex.lock().await;
                    s.1 += 1;
                    return;
                };

                match upload_file(ctx.client, &file.path, chat, ctx.topic, caption_ref).await {
                    Ok(msg) => {
                        output::print_success(msg.id());
                        let mut s = stats_mutex.lock().await;
                        s.0 += 1;
                    }
                    Err(e) => {
                        output::print_failure(&e.to_string());
                        let mut s = stats_mutex.lock().await;
                        s.1 += 1;
                    }
                }
            }
        })
        .buffer_unordered(ctx.concurrent)
        .collect()
        .await;

    // Update stats
    let final_stats = stats_mutex.lock().await;
    stats.add_success(final_stats.0);
    stats.add_failed(final_stats.1);

    Ok(())
}

/// Handle media group uploads
pub async fn upload_media_groups(
    ctx: &UploadContext<'_>,
    files: &[ValidatedFile],
    stats: &mut UploadStats,
) -> Result<()> {
    // Filter files that support media group (photos/videos only)
    let media_files: Vec<_> = files
        .iter()
        .filter(|f| is_media_group_supported(&f.path))
        .collect();

    let non_media_count = files.len() - media_files.len();
    if non_media_count > 0 {
        output::print_skipped_files(non_media_count, "not photo/video");
        stats.add_failed(non_media_count);
    }

    if media_files.is_empty() {
        output::print_no_media_files();
        return Ok(());
    }

    // Determine destination
    let dest = if let Some(ref to_expr) = ctx.to {
        let file_ctx =
            FileContext::from_path_with_context(&media_files[0].path, 0, media_files.len());
        eval_routing(to_expr, &file_ctx)
    } else {
        ctx.chat.clone().unwrap_or_default()
    };

    // Resolve chat
    let chat = match resolve_chat(ctx.client, &dest).await {
        Ok(c) => c,
        Err(e) => {
            output::print_failure(&format!("Failed to resolve '{}': {}", dest, e));
            stats.add_failed(media_files.len());
            return Ok(());
        }
    };

    let total_batches = (media_files.len() + MAX_MEDIA_GROUP_SIZE - 1) / MAX_MEDIA_GROUP_SIZE;

    // Split into batches of MAX_MEDIA_GROUP_SIZE
    // Media groups are sent sequentially to maintain order
    for (batch_idx, batch) in media_files.chunks(MAX_MEDIA_GROUP_SIZE).enumerate() {
        let batch_paths: Vec<&std::path::Path> = batch.iter().map(|f| f.path.as_path()).collect();

        output::print_group_progress(batch_idx, total_batches, batch.len());

        match upload_media_group(
            ctx.client,
            &batch_paths,
            &chat,
            ctx.topic,
            ctx.caption.as_deref(),
        )
        .await
        {
            Ok(count) => {
                output::print_group_success(count);
                stats.add_success(count);
            }
            Err(e) => {
                output::print_group_failure(&e.to_string());
                stats.add_failed(batch.len());
            }
        }
    }

    Ok(())
}

/// Remove uploaded files
pub fn remove_files(files: &[ValidatedFile]) -> usize {
    let mut removed = 0;
    for file in files {
        if let Err(e) = std::fs::remove_file(&file.path) {
            output::print_remove_failure(&e.to_string());
        } else {
            removed += 1;
        }
    }
    removed
}

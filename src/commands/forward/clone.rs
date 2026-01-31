//! Clone forward: download then re-upload

use crate::telegram::download::{
    download_document_with_progress, download_photo_with_progress, fetch_message, DocumentInfo,
    MessageContent, PhotoInfo,
};
use crate::telegram::upload::{
    send_text, upload_file_with_progress, upload_media_group, ResolvedChat,
};
use crate::utils::{create_shared_progress_bar, ChatIdentifier, TelegramLink};
use anyhow::{bail, Result};
use grammers_client::Client;
use std::path::PathBuf;

/// Forward message by downloading and re-uploading
pub async fn forward_clone(
    client: &Client,
    src: &TelegramLink,
    from_chat: Option<&str>,
    dest: &ResolvedChat,
    topic: Option<i32>,
) -> Result<i32> {
    // Resolve source chat string
    let chat_str = match &src.chat {
        ChatIdentifier::Username(u) => u.clone(),
        ChatIdentifier::ChannelId(id) => format!("-100{}", id),
        ChatIdentifier::External => from_chat
            .ok_or_else(|| anyhow::anyhow!("--from-chat required"))?
            .to_string(),
    };

    let message_id = src.effective_message_id();

    // Fetch message and extract content
    let (_resolved, content_list) = fetch_message(client, &chat_str, message_id).await?;

    if content_list.is_empty() {
        bail!("Message is empty");
    }

    // Create temp directory for downloads
    let temp_dir = std::env::temp_dir().join(format!("tdlr_clone_{}", message_id));
    std::fs::create_dir_all(&temp_dir)?;

    // Check if this is a media group (multiple media items, no text)
    let media_count = content_list
        .iter()
        .filter(|c| !matches!(c, MessageContent::Text(_)))
        .count();
    let is_media_group = media_count > 1;

    let last_msg_id = if is_media_group {
        // Handle media group: download all, then upload as album
        clone_media_group(client, &content_list, &temp_dir, dest, topic).await?
    } else {
        // Handle single item
        clone_single(client, &content_list, &temp_dir, dest, topic).await?
    };

    // Cleanup temp directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    if last_msg_id == 0 {
        bail!("No content forwarded");
    }

    Ok(last_msg_id)
}

/// Clone a media group (album)
async fn clone_media_group(
    client: &Client,
    content_list: &[MessageContent],
    temp_dir: &PathBuf,
    dest: &ResolvedChat,
    topic: Option<i32>,
) -> Result<i32> {
    let mut downloaded_files: Vec<PathBuf> = Vec::new();

    // Calculate total size for progress bar
    let total_size: u64 = content_list
        .iter()
        .map(|c| match c {
            MessageContent::Photo(photo) => photo
                .sizes
                .iter()
                .filter_map(|s| match s {
                    grammers_tl_types::enums::PhotoSize::Size(ps) => Some(ps.size as u64),
                    grammers_tl_types::enums::PhotoSize::Progressive(ps) => {
                        ps.sizes.last().map(|&size| size as u64)
                    }
                    _ => None,
                })
                .max()
                .unwrap_or(0),
            MessageContent::Document(doc) => doc.size as u64,
            MessageContent::Text(_) => 0,
        })
        .sum();

    // Create progress bar for download phase only (upload has its own)
    let pb = create_shared_progress_bar(total_size)?;

    // Download all media files
    for (i, content) in content_list.iter().enumerate() {
        match content {
            MessageContent::Photo(photo) => {
                let info = PhotoInfo::from_photo(photo);
                // Use index to ensure unique filenames
                let file_path = temp_dir.join(format!("{}_{}", i, info.default_filename()));
                download_photo_with_progress(client, photo, &file_path, Some(pb.clone())).await?;
                downloaded_files.push(file_path);
            }
            MessageContent::Document(doc) => {
                let info = DocumentInfo::from_document(doc);
                let file_path = temp_dir.join(format!("{}_{}", i, info.filename));
                download_document_with_progress(client, doc, &file_path, Some(pb.clone())).await?;
                downloaded_files.push(file_path);
            }
            MessageContent::Text(_) => {
                // Skip text in media groups
            }
        }
    }

    pb.finish_and_clear();

    // Upload as media group
    let file_refs: Vec<&std::path::Path> = downloaded_files.iter().map(|p| p.as_path()).collect();
    let count = upload_media_group(client, &file_refs, dest, topic, None).await?;

    // Cleanup downloaded files
    for file in &downloaded_files {
        let _ = std::fs::remove_file(file);
    }

    // Return a pseudo message ID (we don't get individual IDs from send_album)
    Ok(count as i32)
}

/// Clone a single message (photo, document, or text)
async fn clone_single(
    client: &Client,
    content_list: &[MessageContent],
    temp_dir: &PathBuf,
    dest: &ResolvedChat,
    topic: Option<i32>,
) -> Result<i32> {
    let mut last_msg_id = 0i32;

    for content in content_list {
        match content {
            MessageContent::Photo(photo) => {
                let info = PhotoInfo::from_photo(photo);
                let file_path = temp_dir.join(info.default_filename());

                // Get file size for progress bar
                let file_size = photo
                    .sizes
                    .iter()
                    .filter_map(|s| match s {
                        grammers_tl_types::enums::PhotoSize::Size(ps) => Some(ps.size as u64),
                        grammers_tl_types::enums::PhotoSize::Progressive(ps) => {
                            ps.sizes.last().map(|&size| size as u64)
                        }
                        _ => None,
                    })
                    .max()
                    .unwrap_or(0);

                // Create shared progress bar for both download and upload
                let total_size = file_size * 2;
                let pb = create_shared_progress_bar(total_size)?;

                // Download
                download_photo_with_progress(client, photo, &file_path, Some(pb.clone())).await?;

                // Upload
                let msg = upload_file_with_progress(
                    client,
                    &file_path,
                    dest,
                    topic,
                    None,
                    Some(pb.clone()),
                )
                .await?;
                last_msg_id = msg.id();

                pb.finish_and_clear();
                let _ = std::fs::remove_file(&file_path);
            }
            MessageContent::Document(doc) => {
                let info = DocumentInfo::from_document(doc);
                let file_path = temp_dir.join(&info.filename);
                let file_size = info.size;

                // Create shared progress bar for both download and upload
                let total_size = file_size * 2;
                let pb = create_shared_progress_bar(total_size)?;

                // Download
                download_document_with_progress(client, doc, &file_path, Some(pb.clone())).await?;

                // Upload
                let msg = upload_file_with_progress(
                    client,
                    &file_path,
                    dest,
                    topic,
                    None,
                    Some(pb.clone()),
                )
                .await?;
                last_msg_id = msg.id();

                pb.finish_and_clear();
                let _ = std::fs::remove_file(&file_path);
            }
            MessageContent::Text(text) => {
                let msg = send_text(client, text, dest, topic).await?;
                last_msg_id = msg.id();
            }
        }
    }

    Ok(last_msg_id)
}

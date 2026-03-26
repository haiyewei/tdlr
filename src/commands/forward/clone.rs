//! Clone forward: download then re-upload

use crate::i18n::pick;
use crate::telegram::download::{
    download_document_with_progress, download_photo_with_progress, fetch_message, DocumentInfo,
    MessageContent, PhotoInfo,
};
use crate::telegram::upload::{send_text, upload_media_group, ResolvedChat};
use crate::utils::{create_shared_progress_bar, ChatIdentifier, TelegramLink};
use anyhow::{bail, Result};
use grammers_client::media::Attribute;
use grammers_client::{message::InputMessage, Client};
use grammers_tl_types as tl;
use indicatif::ProgressBar;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncRead, ReadBuf};

/// Progress-tracking wrapper for AsyncRead (same as upload/single.rs)
struct ProgressReader {
    inner: File,
    progress: Arc<ProgressBar>,
    start_offset: u64,
    bytes_read: u64,
}

impl AsyncRead for ProgressReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = &result {
            let after = buf.filled().len();
            let read = (after - before) as u64;
            self.bytes_read += read;
            self.progress
                .set_position(self.start_offset + self.bytes_read);
            self.progress.tick();
        }
        result
    }
}

/// Extract Video attribute from a TL Document if present
fn extract_video_attribute(doc: &tl::types::Document) -> Option<Attribute> {
    for attr in &doc.attributes {
        if let tl::enums::DocumentAttribute::Video(v) = attr {
            return Some(Attribute::Video {
                round_message: v.round_message,
                supports_streaming: v.supports_streaming,
                duration: Duration::from_secs_f64(v.duration),
                w: v.w,
                h: v.h,
            });
        }
    }
    None
}

/// Download thumbnail from a TL Document if available
/// Returns the path to the downloaded thumbnail file
async fn download_thumbnail(
    client: &Client,
    doc: &tl::types::Document,
    temp_dir: &std::path::Path,
) -> Option<std::path::PathBuf> {
    // Find the largest thumbnail
    let thumbs = doc.thumbs.as_ref()?;
    let (thumb_type, _thumb_size) = thumbs
        .iter()
        .filter_map(|t| match t {
            tl::enums::PhotoSize::Size(ps) => Some((ps.r#type.clone(), ps.size as u64)),
            tl::enums::PhotoSize::Progressive(ps) => ps
                .sizes
                .last()
                .map(|&size| (ps.r#type.clone(), size as u64)),
            _ => None,
        })
        .max_by_key(|(_, size)| *size)?;

    let thumb_path = temp_dir.join(format!("thumb_{}.jpg", doc.id));

    let location = tl::types::InputDocumentFileLocation {
        id: doc.id,
        access_hash: doc.access_hash,
        file_reference: doc.file_reference.clone(),
        thumb_size: thumb_type,
    };

    // Download thumbnail (small file, sequential is fine)
    let result = client
        .invoke(&tl::functions::upload::GetFile {
            precise: false,
            cdn_supported: false,
            location: location.into(),
            offset: 0,
            limit: 512 * 1024, // 512KB should be enough for a thumbnail
        })
        .await;

    match result {
        Ok(tl::enums::upload::File::File(f)) => {
            if f.bytes.is_empty() {
                return None;
            }
            if tokio::fs::write(&thumb_path, &f.bytes).await.is_ok() {
                Some(thumb_path)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Forward message by downloading and re-uploading
pub async fn forward_clone(
    tg: &crate::telegram::TelegramClient,
    src: &TelegramLink,
    from_chat: Option<&str>,
    dest: &ResolvedChat,
    topic: Option<i32>,
) -> Result<i32> {
    let client = tg.inner();
    // Resolve source chat string
    let chat_str = match &src.chat {
        ChatIdentifier::Username(u) => u.clone(),
        ChatIdentifier::ChannelId(id) => format!("-100{}", id),
        ChatIdentifier::External => from_chat
            .ok_or_else(|| anyhow::anyhow!(pick("需要 --from-chat", "--from-chat required")))?
            .to_string(),
    };

    let message_id = src.effective_message_id();

    // Fetch message and extract content
    let (_resolved, content_list) = fetch_message(tg, &chat_str, message_id).await?;

    if content_list.is_empty() {
        bail!("{}", pick("消息为空", "Message is empty"));
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
        bail!("{}", pick("没有转发任何内容", "No content forwarded"));
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
    let mut album_caption = String::new();

    // Calculate total size for progress bar
    let total_size: u64 = content_list
        .iter()
        .map(|c| match c {
            MessageContent::Photo(photo, _) => photo
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
            MessageContent::Document(doc, _) => doc.size as u64,
            MessageContent::Text(_) => 0,
        })
        .sum();

    // Create progress bar for download phase only (upload has its own)
    let pb = create_shared_progress_bar(total_size)?;

    // Download all media files
    for (i, content) in content_list.iter().enumerate() {
        match content {
            MessageContent::Photo(photo, caption) => {
                let info = PhotoInfo::from_photo(photo);
                // Use index to ensure unique filenames
                let file_path = temp_dir.join(format!("{}_{}", i, info.default_filename()));
                download_photo_with_progress(client, photo, &file_path, Some(pb.clone())).await?;
                downloaded_files.push(file_path);

                if album_caption.is_empty() && !caption.is_empty() {
                    album_caption = caption.clone();
                }
            }
            MessageContent::Document(doc, caption) => {
                let info = DocumentInfo::from_document(doc);
                let file_path = temp_dir.join(format!("{}_{}", i, info.filename));
                download_document_with_progress(client, doc, &file_path, Some(pb.clone())).await?;
                downloaded_files.push(file_path);

                if album_caption.is_empty() && !caption.is_empty() {
                    album_caption = caption.clone();
                }
            }
            MessageContent::Text(_) => {
                // Skip text in media groups
            }
        }
    }

    pb.finish_and_clear();

    // Upload as media group
    let file_refs: Vec<&std::path::Path> = downloaded_files.iter().map(|p| p.as_path()).collect();
    let caption_opt = if album_caption.is_empty() {
        None
    } else {
        Some(album_caption.as_str())
    };
    let count = upload_media_group(client, &file_refs, dest, topic, caption_opt).await?;

    // Cleanup downloaded files
    for file in &downloaded_files {
        let _ = std::fs::remove_file(file);
    }

    // Return a pseudo message ID (we don't get individual IDs from send_album)
    Ok(count as i32)
}

/// Clone a single message (photo, document, or text)
/// Uses original TL document attributes to preserve video/audio metadata
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
            MessageContent::Photo(photo, caption) => {
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

                // Upload as photo (same as upload command)
                let msg = upload_clone_file(
                    client,
                    &file_path,
                    dest,
                    topic,
                    if caption.is_empty() {
                        None
                    } else {
                        Some(caption)
                    },
                    Some(pb.clone()),
                    CloneMediaType::Photo,
                    None,
                )
                .await?;
                last_msg_id = msg.id();

                pb.finish_and_clear();
                let _ = std::fs::remove_file(&file_path);
            }
            MessageContent::Document(doc, caption) => {
                let info = DocumentInfo::from_document(doc);
                let file_path = temp_dir.join(&info.filename);
                let file_size = info.size;

                // Determine media type from original TL attributes
                let (media_type, thumb_path) =
                    if let Some(video_attr) = extract_video_attribute(doc) {
                        // Download thumbnail for video
                        let thumb = download_thumbnail(client, doc, temp_dir).await;
                        (CloneMediaType::Video(video_attr), thumb)
                    } else {
                        (CloneMediaType::Document, None)
                    };

                // Create shared progress bar for both download and upload
                let total_size = file_size * 2;
                let pb = create_shared_progress_bar(total_size)?;

                // Download
                download_document_with_progress(client, doc, &file_path, Some(pb.clone())).await?;

                // Upload with original attributes and thumbnail preserved
                let msg = upload_clone_file(
                    client,
                    &file_path,
                    dest,
                    topic,
                    if caption.is_empty() {
                        None
                    } else {
                        Some(caption)
                    },
                    Some(pb.clone()),
                    media_type,
                    thumb_path.as_deref(),
                )
                .await?;
                last_msg_id = msg.id();

                pb.finish_and_clear();
                let _ = std::fs::remove_file(&file_path);
                if let Some(tp) = &thumb_path {
                    let _ = std::fs::remove_file(tp);
                }
            }
            MessageContent::Text(text) => {
                let msg = send_text(client, text, dest, topic).await?;
                last_msg_id = msg.id();
            }
        }
    }

    Ok(last_msg_id)
}

/// Media type for clone upload, preserving original attributes
enum CloneMediaType {
    Photo,
    Video(Attribute),
    Document,
}

/// Upload a cloned file with correct media type, attributes, and optional thumbnail
/// This mirrors upload/single.rs but uses original TL attributes instead of guessing from extension
async fn upload_clone_file(
    client: &Client,
    file_path: &std::path::Path,
    chat: &ResolvedChat,
    topic_id: Option<i32>,
    caption: Option<&str>,
    external_progress: Option<Arc<ProgressBar>>,
    media_type: CloneMediaType,
    thumb_path: Option<&std::path::Path>,
) -> Result<grammers_client::message::Message> {
    let file = File::open(file_path).await?;
    let file_size = file.metadata().await?.len();
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    // Use external progress bar or create our own
    let (pb_arc, is_external, start_offset) = if let Some(ext_pb) = external_progress {
        let offset = ext_pb.position();
        (ext_pb, true, offset)
    } else {
        (create_shared_progress_bar(file_size)?, false, 0)
    };

    let mut reader = ProgressReader {
        inner: file,
        progress: Arc::clone(&pb_arc),
        start_offset,
        bytes_read: 0,
    };

    let uploaded = client
        .upload_stream(&mut reader, file_size as usize, file_name)
        .await?;

    if !is_external {
        pb_arc.finish();
    }

    // Upload thumbnail if available
    let thumb_uploaded = if let Some(tp) = thumb_path {
        match client.upload_file(tp).await {
            Ok(t) => Some(t),
            Err(_) => None,
        }
    } else {
        None
    };

    // Build InputMessage with caption
    let mut msg = if let Some(cap) = caption {
        InputMessage::new().html(cap)
    } else {
        InputMessage::default()
    };

    // Set media type using original TL attributes (not guessing from extension)
    match media_type {
        CloneMediaType::Photo => {
            msg = msg.photo(uploaded);
        }
        CloneMediaType::Video(attr) => {
            msg = msg.document(uploaded).attribute(attr);
            if let Some(thumb) = thumb_uploaded {
                msg = msg.thumbnail(thumb);
            }
        }
        CloneMediaType::Document => {
            msg = msg.document(uploaded);
        }
    }

    if let Some(tid) = topic_id {
        msg = msg.reply_to(Some(tid));
    }

    let message = client.send_message(chat.input_peer.clone(), msg).await?;
    Ok(message)
}

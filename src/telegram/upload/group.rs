//! Media group upload

use super::chat::ResolvedChat;
use super::mime::{is_photo_ext, is_video_ext};
use crate::i18n::{is_zh, pick};
use anyhow::{bail, Result};
use grammers_client::media::Attribute;
use grammers_client::{media::InputMedia, Client};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncRead, ReadBuf};

/// Maximum files per media group (Telegram limit)
pub const MAX_MEDIA_GROUP_SIZE: usize = 10;

#[derive(Clone, Copy)]
pub struct UploadMediaItem<'a> {
    pub file_path: &'a Path,
    pub thumbnail_path: Option<&'a Path>,
}

/// Progress-tracking wrapper for AsyncRead
struct ProgressReader {
    inner: File,
    progress: Arc<ProgressBar>,
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
            self.progress.set_position(self.bytes_read);
        }
        result
    }
}

/// Upload multiple files as a media group (album)
pub async fn upload_media_group(
    client: &Client,
    file_paths: &[&Path],
    chat: &ResolvedChat,
    topic_id: Option<i32>,
    caption: Option<&str>,
) -> Result<usize> {
    let items: Vec<_> = file_paths
        .iter()
        .map(|path| UploadMediaItem {
            file_path: path,
            thumbnail_path: None,
        })
        .collect();

    upload_media_group_with_thumbnails(client, &items, chat, topic_id, caption).await
}

pub async fn upload_media_group_with_thumbnails(
    client: &Client,
    items: &[UploadMediaItem<'_>],
    chat: &ResolvedChat,
    topic_id: Option<i32>,
    caption: Option<&str>,
) -> Result<usize> {
    if items.is_empty() {
        bail!("{}", pick("没有可上传的文件", "No files to upload"));
    }

    if items.len() > MAX_MEDIA_GROUP_SIZE {
        bail!(
            "{}",
            if is_zh() {
                format!("媒体组最多不能超过 {} 个文件", MAX_MEDIA_GROUP_SIZE)
            } else {
                format!("Media group cannot exceed {} files", MAX_MEDIA_GROUP_SIZE)
            }
        );
    }

    // send_album requires Peer, not InputPeer
    let target_peer = &chat.input_peer;

    let multi = MultiProgress::new();
    let mut media_items: Vec<InputMedia> = Vec::new();

    for (i, item) in items.iter().enumerate() {
        let file_path = item.file_path;
        let file = File::open(file_path).await?;
        let file_size = file.metadata().await?.len();
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();

        let pb =
            ProgressBar::with_draw_target(Some(file_size), ProgressDrawTarget::stderr_with_hz(10));
        pb.set_style(
            ProgressStyle::default_bar()
                .template(&format!(
                    "  {{spinner:.green}} [{}/{}] [{{bar:40.cyan/blue}}] {{bytes}}/{{total_bytes}} ({{bytes_per_sec}}, {{eta}})",
                    i + 1,
                    items.len()
                ))?
                .progress_chars("█▓░"),
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        let pb = multi.add(pb);

        let pb_arc = Arc::new(pb);
        let mut reader = ProgressReader {
            inner: file,
            progress: Arc::clone(&pb_arc),
            bytes_read: 0,
        };

        let uploaded = client
            .upload_stream(&mut reader, file_size as usize, file_name)
            .await?;
        pb_arc.finish();

        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Build InputMedia using high-level API
        // Caption only on first media (shows as album caption)
        // Use html() to parse HTML formatting
        let mut media = if i == 0 && caption.is_some() {
            InputMedia::new().html(caption.unwrap())
        } else {
            InputMedia::new()
        };

        // Set reply_to only on first media
        if i == 0 {
            media = media.reply_to(topic_id);
        }

        // Use photo() for images, document() with video attribute for videos
        media = if is_photo_ext(&ext) {
            media.photo(uploaded)
        } else if is_video_ext(&ext) {
            media.document(uploaded).attribute(Attribute::Video {
                round_message: false,
                supports_streaming: true,
                duration: Duration::from_secs(0),
                w: 0,
                h: 0,
            })
        } else {
            media.document(uploaded)
        };

        if let Some(path) = item.thumbnail_path {
            if is_video_ext(&ext) {
                let thumb_uploaded = client.upload_file(path).await?;
                media = media.thumbnail(thumb_uploaded);
            }
        }

        media_items.push(media);
    }

    let count = media_items.len();
    Box::pin(client.send_album(target_peer, media_items)).await?;

    Ok(count)
}

//! Media group upload

use super::chat::ResolvedChat;
use super::mime::{is_photo_ext, is_video_ext};
use anyhow::{bail, Result};
use grammers_client::types::Attribute;
use grammers_client::{Client, InputMedia};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncRead, ReadBuf};

/// Maximum files per media group (Telegram limit)
pub const MAX_MEDIA_GROUP_SIZE: usize = 10;

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
    if file_paths.is_empty() {
        bail!("No files to upload");
    }

    if file_paths.len() > MAX_MEDIA_GROUP_SIZE {
        bail!("Media group cannot exceed {} files", MAX_MEDIA_GROUP_SIZE);
    }

    // send_album requires Peer, not InputPeer
    let target_peer = chat
        .peer
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Cannot send album to 'me', use single file upload"))?;

    let multi = MultiProgress::new();
    let mut media_items: Vec<InputMedia> = Vec::new();

    for (i, file_path) in file_paths.iter().enumerate() {
        let file = File::open(file_path).await?;
        let file_size = file.metadata().await?.len();
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();

        let pb = multi.add(ProgressBar::new(file_size));
        pb.set_style(
            ProgressStyle::default_bar()
                .template(&format!(
                    "[{}/{}]   [{{bar:40.cyan/blue}}] {{bytes}}/{{total_bytes}}",
                    i + 1,
                    file_paths.len()
                ))?
                .progress_chars("█▓░"),
        );

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
        let mut media = InputMedia::new();
        if i == 0 {
            if let Some(cap) = caption {
                media = media.caption(cap);
            }
        }

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

        media_items.push(media);
    }

    let count = media_items.len();
    client.send_album(target_peer, media_items).await?;

    Ok(count)
}

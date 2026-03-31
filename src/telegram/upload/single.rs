//! Single file upload

use super::chat::ResolvedChat;
use super::mime::{is_photo_ext, is_video_ext};
use super::video_metadata::video_attribute_for_path;
use crate::utils::create_shared_progress_bar;
use anyhow::Result;
use grammers_client::{
    message::{InputMessage, Message},
    Client,
};
use indicatif::ProgressBar;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio::io::{AsyncRead, ReadBuf};

/// Progress-tracking wrapper for AsyncRead
struct ProgressReader {
    inner: File,
    progress: Arc<ProgressBar>,
    /// Starting offset for progress (used when sharing progress bar)
    start_offset: u64,
    /// Bytes read so far in this upload
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
            // Set position relative to start_offset
            self.progress
                .set_position(self.start_offset + self.bytes_read);
            // Force redraw for small files
            self.progress.tick();
        }
        result
    }
}

/// Upload a single file to Telegram
pub async fn upload_file(
    client: &Client,
    file_path: &Path,
    chat: &ResolvedChat,
    topic_id: Option<i32>,
    caption: Option<&str>,
) -> Result<Message> {
    upload_file_with_thumbnail(client, file_path, None, chat, topic_id, caption).await
}

/// Upload a single file to Telegram with an optional thumbnail/cover.
pub async fn upload_file_with_thumbnail(
    client: &Client,
    file_path: &Path,
    thumbnail_path: Option<&Path>,
    chat: &ResolvedChat,
    topic_id: Option<i32>,
    caption: Option<&str>,
) -> Result<Message> {
    upload_file_with_progress(
        client,
        file_path,
        thumbnail_path,
        chat,
        topic_id,
        caption,
        None,
    )
    .await
}

/// Upload a single file to Telegram with optional external progress bar
pub async fn upload_file_with_progress(
    client: &Client,
    file_path: &Path,
    thumbnail_path: Option<&Path>,
    chat: &ResolvedChat,
    topic_id: Option<i32>,
    caption: Option<&str>,
    external_progress: Option<Arc<ProgressBar>>,
) -> Result<Message> {
    let file = File::open(file_path).await?;
    let file_size = file.metadata().await?.len();
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    // Use external progress bar or create our own
    let (pb_arc, is_external, start_offset) = if let Some(ext_pb) = external_progress {
        // Get current position as start offset for upload phase
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
        .upload_stream(&mut reader, file_size as usize, file_name.clone())
        .await?;

    // Only finish if we created the progress bar
    if !is_external {
        pb_arc.finish();
    }

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Use html() if caption provided, otherwise default
    let mut msg = if let Some(cap) = caption {
        InputMessage::new().html(cap)
    } else {
        InputMessage::default()
    };

    // Use photo for images, video for videos, document for others
    if is_photo_ext(&ext) {
        msg = msg.photo(uploaded);
    } else if is_video_ext(&ext) {
        let video_attribute = video_attribute_for_path(file_path).await;
        let thumb_uploaded = if let Some(path) = thumbnail_path {
            Some(client.upload_file(path).await?)
        } else {
            None
        };
        msg = msg.document(uploaded).attribute(video_attribute);
        if let Some(thumb) = thumb_uploaded {
            msg = msg.thumbnail(thumb);
        }
    } else {
        msg = msg.document(uploaded);
    }

    if let Some(tid) = topic_id {
        msg = msg.reply_to(Some(tid));
    }

    let message = Box::pin(client.send_message(chat.input_peer.clone(), msg)).await?;

    Ok(message)
}

/// Send a text message to Telegram
pub async fn send_text(
    client: &Client,
    text: &str,
    chat: &ResolvedChat,
    topic_id: Option<i32>,
) -> Result<Message> {
    let mut msg = InputMessage::new().html(text);

    if let Some(tid) = topic_id {
        msg = msg.reply_to(Some(tid));
    }

    let message = Box::pin(client.send_message(chat.input_peer.clone(), msg)).await?;

    Ok(message)
}

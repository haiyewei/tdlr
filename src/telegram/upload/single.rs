//! Single file upload

use super::chat::ResolvedChat;
use super::mime::{is_photo_ext, is_video_ext};
use anyhow::Result;
use grammers_client::types::{Attribute, Message};
use grammers_client::{Client, InputMessage};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncRead, ReadBuf};

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

/// Upload a single file to Telegram
pub async fn upload_file(
    client: &Client,
    file_path: &Path,
    chat: &ResolvedChat,
    topic_id: Option<i32>,
    caption: Option<&str>,
) -> Result<Message> {
    let file = File::open(file_path).await?;
    let file_size = file.metadata().await?.len();
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    let pb = ProgressBar::new(file_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("█▓░"),
    );

    let pb_arc = Arc::new(pb);
    let mut reader = ProgressReader {
        inner: file,
        progress: Arc::clone(&pb_arc),
        bytes_read: 0,
    };

    let uploaded = client
        .upload_stream(&mut reader, file_size as usize, file_name.clone())
        .await?;
    pb_arc.finish();

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
        msg = msg.document(uploaded).attribute(Attribute::Video {
            round_message: false,
            supports_streaming: true,
            duration: Duration::from_secs(0),
            w: 0,
            h: 0,
        });
    } else {
        msg = msg.document(uploaded);
    }

    if let Some(tid) = topic_id {
        msg = msg.reply_to(Some(tid));
    }

    let message = client.send_message(chat.input_peer.clone(), msg).await?;

    Ok(message)
}

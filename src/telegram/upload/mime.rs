//! MIME type and media utilities

use std::path::Path;

/// Check if file extension is supported for media group
pub fn is_media_group_supported(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    matches!(
        ext.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | // photos
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "m4v" | "3gp" // videos
    )
}

/// Check if extension is a photo
pub fn is_photo_ext(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp"
    )
}

/// Check if extension is a video
pub fn is_video_ext(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "m4v" | "3gp"
    )
}

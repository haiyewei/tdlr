//! MIME type and media utilities

use std::path::Path;

fn ext_lower(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

/// Check if file extension is supported for media group
pub fn is_media_group_supported(path: &Path) -> bool {
    let ext = ext_lower(path);

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

/// Check if the file path looks like a photo.
pub fn is_photo_path(path: &Path) -> bool {
    is_photo_ext(&ext_lower(path))
}

/// Check if extension is a video
pub fn is_video_ext(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "m4v" | "3gp"
    )
}

/// Check if the file path looks like a video.
pub fn is_video_path(path: &Path) -> bool {
    is_video_ext(&ext_lower(path))
}

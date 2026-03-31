use crate::i18n::{is_zh, pick};
use anyhow::{anyhow, Context, Result};
use matroska::Attachment;
use mp4parse::unstable::{create_sample_table, CheckedInteger};
use mp4parse::{read_mp4, MediaContext, SampleEntry, Track, TrackType};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
struct EmbeddedImage {
    bytes: Vec<u8>,
    extension: &'static str,
}

#[derive(Debug)]
pub struct PreparedThumbnail {
    path: PathBuf,
}

impl PreparedThumbnail {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PreparedThumbnail {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub async fn prepare_embedded_thumbnail(file_path: &Path) -> Option<PreparedThumbnail> {
    let path = file_path.to_path_buf();
    match tokio::task::spawn_blocking(move || extract_embedded_thumbnail_to_temp(&path)).await {
        Ok(Ok(Some(thumbnail))) => Some(thumbnail),
        Ok(Ok(None)) => None,
        Ok(Err(error)) => {
            warn_prepare_error(file_path, &error);
            None
        }
        Err(error) => {
            warn_prepare_error(
                file_path,
                &anyhow!(error).context(pick(
                    "内嵌封面任务执行失败",
                    "embedded thumbnail task failed",
                )),
            );
            None
        }
    }
}

fn extract_embedded_thumbnail_to_temp(file_path: &Path) -> Result<Option<PreparedThumbnail>> {
    let image = match lower_extension(file_path).as_str() {
        "mp4" | "mov" | "m4v" | "3gp" => extract_mp4_family_thumbnail(file_path)?,
        "mkv" | "webm" => extract_matroska_thumbnail(file_path)?,
        _ => None,
    };

    let Some(image) = image else {
        return Ok(None);
    };

    let path = write_temp_thumbnail(&image)?;
    Ok(Some(PreparedThumbnail { path }))
}

fn extract_mp4_family_thumbnail(file_path: &Path) -> Result<Option<EmbeddedImage>> {
    let mut file = File::open(file_path).with_context(|| {
        if is_zh() {
            format!("无法打开视频文件 '{}'", file_path.display())
        } else {
            format!("failed to open video file '{}'", file_path.display())
        }
    })?;

    let context = read_mp4(&mut file).with_context(|| {
        if is_zh() {
            format!("无法解析 MP4 元数据 '{}'", file_path.display())
        } else {
            format!("failed to parse MP4 metadata '{}'", file_path.display())
        }
    })?;

    if let Some(image) = extract_mp4_covr(&context) {
        return Ok(Some(image));
    }

    extract_mp4_attached_picture(&mut file, &context)
}

fn extract_mp4_covr(context: &MediaContext) -> Option<EmbeddedImage> {
    let metadata = context
        .userdata
        .as_ref()
        .and_then(|userdata| userdata.as_ref().ok())
        .and_then(|userdata| userdata.meta.as_ref())?;

    let covers = metadata.cover_art.as_ref()?;
    for cover in covers.iter() {
        let bytes = cover.as_slice();
        if let Some(extension) = detect_supported_image_extension(bytes) {
            return Some(EmbeddedImage {
                bytes: bytes.to_vec(),
                extension,
            });
        }
    }

    None
}

fn extract_mp4_attached_picture(
    file: &mut File,
    context: &MediaContext,
) -> Result<Option<EmbeddedImage>> {
    for track in context
        .tracks
        .iter()
        .filter(|track| matches!(track.track_type, TrackType::Picture))
    {
        if let Some(image) = extract_mp4_track_image(file, track)? {
            return Ok(Some(image));
        }
    }

    for track in context
        .tracks
        .iter()
        .filter(|track| matches!(track.track_type, TrackType::Video))
    {
        if let Some(image) = extract_mp4_track_image(file, track)? {
            return Ok(Some(image));
        }
    }

    Ok(None)
}

fn extract_mp4_track_image(file: &mut File, track: &Track) -> Result<Option<EmbeddedImage>> {
    let Some(stsd) = track.stsd.as_ref() else {
        return Ok(None);
    };

    let is_video_description = stsd
        .descriptions
        .as_slice()
        .iter()
        .any(|entry| matches!(entry, SampleEntry::Video(_)));
    if !is_video_description {
        return Ok(None);
    }

    let Some(samples) = create_sample_table(track, CheckedInteger(0i64)) else {
        return Ok(None);
    };

    for sample in samples.iter().filter(|sample| sample.sync).take(4) {
        if let Some(image) =
            read_mp4_sample_as_image(file, sample.start_offset.0, sample.end_offset.0)?
        {
            return Ok(Some(image));
        }
    }

    for sample in samples.iter().take(4) {
        if let Some(image) =
            read_mp4_sample_as_image(file, sample.start_offset.0, sample.end_offset.0)?
        {
            return Ok(Some(image));
        }
    }

    Ok(None)
}

fn read_mp4_sample_as_image(
    file: &mut File,
    start_offset: u64,
    end_offset: u64,
) -> Result<Option<EmbeddedImage>> {
    if end_offset <= start_offset {
        return Ok(None);
    }

    let length = end_offset - start_offset;
    if length == 0 || length > MAX_IMAGE_BYTES {
        return Ok(None);
    }

    let length = usize::try_from(length)
        .map_err(|_| anyhow!(pick("封面样本过大", "thumbnail sample is too large")))?;

    file.seek(SeekFrom::Start(start_offset))?;
    let mut bytes = vec![0u8; length];
    file.read_exact(&mut bytes)?;

    let Some(extension) = detect_supported_image_extension(&bytes) else {
        return Ok(None);
    };

    Ok(Some(EmbeddedImage { bytes, extension }))
}

fn extract_matroska_thumbnail(file_path: &Path) -> Result<Option<EmbeddedImage>> {
    let matroska = matroska::open(file_path).with_context(|| {
        if is_zh() {
            format!("无法解析 Matroska 元数据 '{}'", file_path.display())
        } else {
            format!(
                "failed to parse Matroska metadata '{}'",
                file_path.display()
            )
        }
    })?;

    if let Some(image) = find_preferred_attachment(&matroska.attachments) {
        return Ok(Some(image));
    }

    Ok(find_fallback_attachment(&matroska.attachments))
}

fn find_preferred_attachment(attachments: &[Attachment]) -> Option<EmbeddedImage> {
    attachments
        .iter()
        .filter(|attachment| attachment_looks_like_cover(attachment))
        .find_map(extract_attachment_image)
}

fn find_fallback_attachment(attachments: &[Attachment]) -> Option<EmbeddedImage> {
    attachments.iter().find_map(extract_attachment_image)
}

fn extract_attachment_image(attachment: &Attachment) -> Option<EmbeddedImage> {
    if attachment.data.is_empty() {
        return None;
    }

    let extension = detect_supported_image_extension(&attachment.data)
        .or_else(|| supported_image_extension_for_mime(&attachment.mime_type))
        .or_else(|| supported_image_extension_for_name(&attachment.name))?;

    Some(EmbeddedImage {
        bytes: attachment.data.clone(),
        extension,
    })
}

fn attachment_looks_like_cover(attachment: &Attachment) -> bool {
    let name = attachment.name.to_ascii_lowercase();
    let description = attachment
        .description
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    ["cover", "poster", "thumb", "thumbnail", "front"]
        .iter()
        .any(|keyword| name.contains(keyword) || description.contains(keyword))
}

fn write_temp_thumbnail(image: &EmbeddedImage) -> Result<PathBuf> {
    let dir = std::env::temp_dir().join("tdlr");
    fs::create_dir_all(&dir).with_context(|| {
        if is_zh() {
            format!("无法创建临时目录 '{}'", dir.display())
        } else {
            format!("failed to create temp directory '{}'", dir.display())
        }
    })?;

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let path = dir.join(format!(
        "embedded-thumb-{}-{}-{}.{}",
        std::process::id(),
        millis,
        id,
        image.extension
    ));

    fs::write(&path, &image.bytes).with_context(|| {
        if is_zh() {
            format!("无法写入临时封面 '{}'", path.display())
        } else {
            format!("failed to write temp thumbnail '{}'", path.display())
        }
    })?;

    Ok(path)
}

fn lower_extension(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn supported_image_extension_for_mime(mime: &str) -> Option<&'static str> {
    match mime.trim().to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/bmp" => Some("bmp"),
        _ => None,
    }
}

fn supported_image_extension_for_name(name: &str) -> Option<&'static str> {
    Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .and_then(|value| match value.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => Some("jpg"),
            "png" => Some("png"),
            "gif" => Some("gif"),
            "webp" => Some("webp"),
            "bmp" => Some("bmp"),
            _ => None,
        })
}

fn detect_supported_image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some("jpg");
    }

    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("png");
    }

    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("gif");
    }

    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("webp");
    }

    if bytes.starts_with(b"BM") {
        return Some("bmp");
    }

    None
}

fn warn_prepare_error(file_path: &Path, error: &anyhow::Error) {
    eprintln!(
        "{}: {}",
        pick("警告", "Warning"),
        if is_zh() {
            format!("读取内嵌封面失败 '{}': {}", file_path.display(), error)
        } else {
            format!(
                "Failed to read embedded thumbnail for '{}': {}",
                file_path.display(),
                error
            )
        }
    );
}

#[cfg(test)]
mod tests {
    use super::{detect_supported_image_extension, extract_embedded_thumbnail_to_temp};
    use std::fs;
    use std::path::Path;

    #[test]
    fn detects_common_image_signatures() {
        assert_eq!(
            detect_supported_image_extension(&[0xff, 0xd8, 0xff, 0xe0]),
            Some("jpg")
        );
        assert_eq!(
            detect_supported_image_extension(b"\x89PNG\r\n\x1a\nrest"),
            Some("png")
        );
        assert_eq!(detect_supported_image_extension(b"GIF89a"), Some("gif"));
        assert_eq!(
            detect_supported_image_extension(b"RIFFabcdWEBPmore"),
            Some("webp")
        );
        assert_eq!(detect_supported_image_extension(b"BMrest"), Some("bmp"));
        assert_eq!(detect_supported_image_extension(b"not-an-image"), None);
    }

    #[test]
    fn extracts_real_sample_when_env_is_set() {
        let Ok(sample_path) = std::env::var("TDLR_EMBEDDED_THUMB_SAMPLE") else {
            return;
        };

        let thumbnail = extract_embedded_thumbnail_to_temp(Path::new(&sample_path))
            .expect("extract embedded thumbnail")
            .expect("expected embedded thumbnail");
        let bytes = fs::read(thumbnail.path()).expect("read extracted thumbnail");

        assert!(!bytes.is_empty());
        assert!(detect_supported_image_extension(&bytes).is_some());
    }

    #[test]
    fn returns_none_for_real_sample_without_thumbnail_when_env_is_set() {
        let Ok(sample_path) = std::env::var("TDLR_NO_EMBEDDED_THUMB_SAMPLE") else {
            return;
        };

        let thumbnail = extract_embedded_thumbnail_to_temp(Path::new(&sample_path))
            .expect("probe embedded thumbnail");

        assert!(thumbnail.is_none());
    }
}

use crate::i18n::{is_zh, pick};
use anyhow::{anyhow, Context, Result};
use grammers_client::media::Attribute;
use nom_exif::{MediaParser, MediaSource, TrackInfo, TrackInfoTag};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Default)]
struct VideoMetadata {
    duration: Option<Duration>,
    width: Option<i32>,
    height: Option<i32>,
}

impl VideoMetadata {
    fn into_attribute(self) -> Attribute {
        Attribute::Video {
            round_message: false,
            supports_streaming: true,
            duration: self.duration.unwrap_or_else(|| Duration::from_secs(0)),
            w: self.width.unwrap_or(0),
            h: self.height.unwrap_or(0),
        }
    }

    fn is_empty(&self) -> bool {
        self.duration.is_none() && self.width.is_none() && self.height.is_none()
    }
}

pub async fn video_attribute_for_path(file_path: &Path) -> Attribute {
    let path = file_path.to_path_buf();
    match tokio::task::spawn_blocking(move || probe_video_metadata(&path)).await {
        Ok(Ok(Some(metadata))) => metadata.into_attribute(),
        Ok(Ok(None)) => default_video_attribute(),
        Ok(Err(error)) => {
            warn_probe_error(file_path, &error);
            default_video_attribute()
        }
        Err(error) => {
            warn_probe_error(
                file_path,
                &anyhow!(error)
                    .context(pick("视频元数据任务执行失败", "video metadata task failed")),
            );
            default_video_attribute()
        }
    }
}

fn default_video_attribute() -> Attribute {
    VideoMetadata::default().into_attribute()
}

fn probe_video_metadata(file_path: &Path) -> Result<Option<VideoMetadata>> {
    let ext = file_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let metadata = if ext == "avi" {
        probe_avi_metadata(file_path)?
    } else {
        probe_with_nom_exif(file_path)?
    };

    if metadata.is_empty() {
        Ok(None)
    } else {
        Ok(Some(metadata))
    }
}

fn probe_with_nom_exif(file_path: &Path) -> Result<VideoMetadata> {
    let source = MediaSource::file_path(file_path).with_context(|| {
        if is_zh() {
            format!("无法打开视频文件 '{}'", file_path.display())
        } else {
            format!("failed to open video file '{}'", file_path.display())
        }
    })?;

    if !source.has_track() {
        return Ok(VideoMetadata::default());
    }

    let mut parser = MediaParser::new();
    let info: TrackInfo = parser.parse(source).with_context(|| {
        if is_zh() {
            format!("无法解析视频元数据 '{}'", file_path.display())
        } else {
            format!("failed to parse video metadata '{}'", file_path.display())
        }
    })?;

    Ok(VideoMetadata {
        duration: info
            .get(TrackInfoTag::DurationMs)
            .and_then(|value| value.as_u64())
            .map(Duration::from_millis),
        width: info
            .get(TrackInfoTag::ImageWidth)
            .and_then(|value| value.as_u32())
            .and_then(parse_positive_i32),
        height: info
            .get(TrackInfoTag::ImageHeight)
            .and_then(|value| value.as_u32())
            .and_then(parse_positive_i32),
    })
}

fn probe_avi_metadata(file_path: &Path) -> Result<VideoMetadata> {
    let mut file = File::open(file_path).with_context(|| {
        if is_zh() {
            format!("无法打开 AVI 文件 '{}'", file_path.display())
        } else {
            format!("failed to open AVI file '{}'", file_path.display())
        }
    })?;

    let mut header = [0u8; 12];
    file.read_exact(&mut header).with_context(|| {
        if is_zh() {
            format!("无法读取 AVI 头 '{}'", file_path.display())
        } else {
            format!("failed to read AVI header '{}'", file_path.display())
        }
    })?;

    if &header[0..4] != b"RIFF" || &header[8..12] != b"AVI " {
        return Err(anyhow!(pick("不是有效的 AVI 文件", "not a valid AVI file")));
    }

    let riff_size = u32::from_le_bytes(header[4..8].try_into().unwrap()) as u64;
    let file_len = file.metadata()?.len();
    let riff_end = (8 + riff_size).min(file_len);

    find_avi_header(&mut file, riff_end)?
        .ok_or_else(|| anyhow!(pick("未找到 AVI 主头", "AVI main header not found")))
}

fn find_avi_header(file: &mut File, end: u64) -> Result<Option<VideoMetadata>> {
    while file.stream_position()? + 8 <= end {
        let chunk_start = file.stream_position()?;

        let mut chunk_id = [0u8; 4];
        file.read_exact(&mut chunk_id)?;
        let chunk_size = read_u32_le(file)? as u64;
        let chunk_data_start = file.stream_position()?;
        let chunk_end = chunk_data_start
            .checked_add(chunk_size)
            .ok_or_else(|| anyhow!(pick("AVI 头大小溢出", "AVI chunk size overflow")))?;
        let aligned_chunk_end = align_to_even(chunk_end);

        if chunk_end > end {
            return Ok(None);
        }

        if &chunk_id == b"LIST" {
            if chunk_size < 4 {
                return Ok(None);
            }

            let mut list_type = [0u8; 4];
            file.read_exact(&mut list_type)?;
            if &list_type == b"hdrl" {
                if let Some(metadata) = find_avi_header(file, chunk_end)? {
                    return Ok(Some(metadata));
                }
            }
        } else if &chunk_id == b"avih" {
            if chunk_size < 40 {
                return Ok(None);
            }

            let mut avih = [0u8; 40];
            file.read_exact(&mut avih)?;

            let micro_sec_per_frame = u32::from_le_bytes(avih[0..4].try_into().unwrap());
            let total_frames = u32::from_le_bytes(avih[16..20].try_into().unwrap());
            let width = u32::from_le_bytes(avih[32..36].try_into().unwrap());
            let height = u32::from_le_bytes(avih[36..40].try_into().unwrap());

            return Ok(Some(VideoMetadata {
                duration: parse_avi_duration(micro_sec_per_frame, total_frames),
                width: parse_positive_i32(width),
                height: parse_positive_i32(height),
            }));
        }

        if aligned_chunk_end <= chunk_start {
            return Ok(None);
        }
        file.seek(SeekFrom::Start(aligned_chunk_end))?;
    }

    Ok(None)
}

fn parse_avi_duration(micro_sec_per_frame: u32, total_frames: u32) -> Option<Duration> {
    let total_micros = u64::from(micro_sec_per_frame).checked_mul(u64::from(total_frames))?;
    if total_micros == 0 {
        None
    } else {
        Some(Duration::from_micros(total_micros))
    }
}

fn parse_positive_i32(value: u32) -> Option<i32> {
    i32::try_from(value).ok().filter(|value| *value > 0)
}

fn read_u32_le(reader: &mut File) -> Result<u32> {
    let mut bytes = [0u8; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn align_to_even(value: u64) -> u64 {
    value + (value % 2)
}

fn warn_probe_error(file_path: &Path, error: &anyhow::Error) {
    eprintln!(
        "{}: {}",
        pick("警告", "Warning"),
        format_probe_error(file_path, error)
    );
}

fn format_probe_error(file_path: &Path, error: &anyhow::Error) -> String {
    if is_zh() {
        format!("读取视频元数据失败 '{}': {}", file_path.display(), error)
    } else {
        format!(
            "Failed to read video metadata for '{}': {}",
            file_path.display(),
            error
        )
    }
}

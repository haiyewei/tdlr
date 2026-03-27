//! Thumbnail assignment for upload.

use super::file::ValidatedFile;
use crate::i18n::{is_zh, pick};
use crate::telegram::upload::{is_photo_path, is_video_path};
use anyhow::{anyhow, bail, Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct ThumbnailAssignments {
    by_upload: HashMap<PathBuf, PathBuf>,
    pub unused_count: usize,
}

impl ThumbnailAssignments {
    pub fn get(&self, file_path: &Path) -> Option<&Path> {
        self.by_upload.get(file_path).map(PathBuf::as_path)
    }
}

#[derive(Debug)]
struct ThumbnailCandidate {
    path: PathBuf,
    normalized: PathBuf,
    stem_key: String,
}

pub fn resolve_thumbnail_assignments(
    files: &[ValidatedFile],
    thumb_inputs: Option<&[String]>,
    thumb_maps: Option<&[String]>,
) -> Result<ThumbnailAssignments> {
    if thumb_inputs.is_none() && thumb_maps.is_none() {
        return Ok(ThumbnailAssignments::default());
    }

    let video_files: Vec<_> = files.iter().filter(|file| is_video_path(&file.path)).collect();
    if video_files.is_empty() {
        bail!(
            "{}",
            pick(
                "当前上传列表里没有可附加封面的在线视频文件。",
                "There are no video files in this upload that can receive thumbnails."
            )
        );
    }

    let video_index = build_video_index(&video_files)?;
    let mut assignments = HashMap::new();
    let mut used_thumbnail_paths = HashSet::new();

    if let Some(maps) = thumb_maps {
        for raw in maps {
            let (video_selector, thumb_selector) = raw.split_once('=').ok_or_else(|| {
                anyhow!(
                    "{}",
                    pick(
                        "无效的 --thumb-map。格式必须是 <VIDEO>=<THUMB>。",
                        "Invalid --thumb-map. Expected <VIDEO>=<THUMB>."
                    )
                )
            })?;

            let video = resolve_video_selector(video_selector.trim(), &video_files, &video_index)?;
            if assignments.contains_key(&video.path) {
                bail!(
                    "{}",
                    if is_zh() {
                        format!("视频 '{}' 重复指定了封面映射。", video.path.display())
                    } else {
                        format!("Video '{}' has duplicate thumbnail mappings.", video.path.display())
                    }
                );
            }

            let thumbnail = validate_thumbnail_file(Path::new(thumb_selector.trim()))?;
            used_thumbnail_paths.insert(normalize_existing_path(&thumbnail)?);
            assignments.insert(video.path.clone(), thumbnail);
        }
    }

    let mut auto_candidates = collect_thumbnail_candidates(thumb_inputs.unwrap_or(&[]))?;
    auto_candidates.retain(|candidate| !used_thumbnail_paths.contains(&candidate.normalized));

    if thumb_inputs.is_some() && auto_candidates.is_empty() {
        bail!(
            "{}",
            pick(
                "没有从 --thumb 输入中找到可用的图片文件。",
                "No usable image files were found in --thumb inputs."
            )
        );
    }

    assign_unique_stem_matches(&video_files, &mut auto_candidates, &mut assignments);

    let remaining_videos: Vec<_> = video_files
        .iter()
        .copied()
        .filter(|file| !assignments.contains_key(&file.path))
        .collect();

    let fallback_count = remaining_videos.len().min(auto_candidates.len());
    for (video, thumbnail) in remaining_videos
        .into_iter()
        .zip(auto_candidates.iter())
        .take(fallback_count)
    {
        assignments.insert(video.path.clone(), thumbnail.path.clone());
    }

    let unused_count = auto_candidates.len().saturating_sub(fallback_count);

    Ok(ThumbnailAssignments {
        by_upload: assignments,
        unused_count,
    })
}

fn build_video_index<'a>(
    video_files: &[&'a ValidatedFile],
) -> Result<HashMap<PathBuf, &'a ValidatedFile>> {
    let mut index = HashMap::new();
    for file in video_files {
        index.insert(normalize_existing_path(&file.path)?, *file);
    }
    Ok(index)
}

fn resolve_video_selector<'a>(
    selector: &str,
    video_files: &[&'a ValidatedFile],
    video_index: &HashMap<PathBuf, &'a ValidatedFile>,
) -> Result<&'a ValidatedFile> {
    let selector = selector.trim();
    if selector.is_empty() {
        bail!(
            "{}",
            pick("空的视频映射目标。", "Empty video mapping target.")
        );
    }

    let selector_path = Path::new(selector);
    if selector_path.exists() {
        let normalized = normalize_existing_path(selector_path)?;
        if let Some(file) = video_index.get(&normalized) {
            return Ok(*file);
        }
    }

    let mut file_name_matches = Vec::new();
    let mut stem_matches = Vec::new();

    for file in video_files {
        if file
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case(selector))
        {
            file_name_matches.push(*file);
        }

        if file
            .path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem.eq_ignore_ascii_case(selector))
        {
            stem_matches.push(*file);
        }
    }

    match file_name_matches.len() {
        1 => return Ok(file_name_matches[0]),
        n if n > 1 => {
            bail!(
                "{}",
                if is_zh() {
                    format!("视频选择器 '{}' 匹配到了多个同名文件，请改用完整路径。", selector)
                } else {
                    format!(
                        "Video selector '{}' matched multiple files. Use the full path instead.",
                        selector
                    )
                }
            );
        }
        _ => {}
    }

    match stem_matches.len() {
        1 => Ok(stem_matches[0]),
        n if n > 1 => bail!(
            "{}",
            if is_zh() {
                format!("视频选择器 '{}' 匹配到了多个同名 stem，请改用完整路径。", selector)
            } else {
                format!(
                    "Video selector '{}' matched multiple file stems. Use the full path instead.",
                    selector
                )
            }
        ),
        _ => bail!(
            "{}",
            if is_zh() {
                format!("未找到与 '{}' 对应的视频文件。", selector)
            } else {
                format!("No uploaded video matches '{}'.", selector)
            }
        ),
    }
}

fn assign_unique_stem_matches(
    video_files: &[&ValidatedFile],
    auto_candidates: &mut Vec<ThumbnailCandidate>,
    assignments: &mut HashMap<PathBuf, PathBuf>,
) {
    loop {
        let remaining_videos: Vec<_> = video_files
            .iter()
            .copied()
            .filter(|file| !assignments.contains_key(&file.path))
            .collect();

        if remaining_videos.is_empty() || auto_candidates.is_empty() {
            break;
        }

        let mut match_attempts = Vec::new();
        for video in &remaining_videos {
            let stem_key = stem_key(&video.path);
            let candidate_indexes: Vec<_> = auto_candidates
                .iter()
                .enumerate()
                .filter_map(|(index, candidate)| {
                    if candidate.stem_key == stem_key {
                        Some(index)
                    } else {
                        None
                    }
                })
                .collect();

            if candidate_indexes.len() == 1 {
                match_attempts.push((video.path.clone(), candidate_indexes[0]));
            }
        }

        if match_attempts.is_empty() {
            break;
        }

        let mut candidate_usage = HashMap::<usize, usize>::new();
        for (_, index) in &match_attempts {
            *candidate_usage.entry(*index).or_insert(0) += 1;
        }

        let confirmed: Vec<_> = match_attempts
            .into_iter()
            .filter(|(_, index)| candidate_usage.get(index) == Some(&1))
            .collect();

        if confirmed.is_empty() {
            break;
        }

        let mut used_indexes = confirmed
            .iter()
            .map(|(_, index)| *index)
            .collect::<Vec<_>>();
        used_indexes.sort_unstable();
        used_indexes.dedup();

        for (video_path, index) in &confirmed {
            assignments.insert(video_path.clone(), auto_candidates[*index].path.clone());
        }

        for index in used_indexes.into_iter().rev() {
            auto_candidates.remove(index);
        }
    }
}

fn collect_thumbnail_candidates(inputs: &[String]) -> Result<Vec<ThumbnailCandidate>> {
    let mut candidates = Vec::new();

    for input in inputs {
        let path = Path::new(input);
        if !path.exists() {
            bail!(
                "{}",
                if is_zh() {
                    format!("封面路径不存在: {}", path.display())
                } else {
                    format!("Thumbnail path not found: {}", path.display())
                }
            );
        }

        if path.is_file() {
            let validated = validate_thumbnail_file(path)?;
            candidates.push(build_candidate(validated)?);
            continue;
        }

        collect_thumbnail_candidates_from_dir(path, &mut candidates)?;
    }

    candidates.sort_by(|left, right| {
        left.path
            .to_string_lossy()
            .cmp(&right.path.to_string_lossy())
    });

    let mut seen = HashSet::new();
    candidates.retain(|candidate| seen.insert(candidate.normalized.clone()));

    Ok(candidates)
}

fn collect_thumbnail_candidates_from_dir(
    dir: &Path,
    candidates: &mut Vec<ThumbnailCandidate>,
) -> Result<()> {
    let mut entries = fs::read_dir(dir)
        .with_context(|| {
            if is_zh() {
                format!("无法读取封面目录 '{}'", dir.display())
            } else {
                format!("Failed to read thumbnail directory '{}'", dir.display())
            }
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| {
            if is_zh() {
                format!("无法枚举封面目录 '{}'", dir.display())
            } else {
                format!("Failed to enumerate thumbnail directory '{}'", dir.display())
            }
        })?;

    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_thumbnail_candidates_from_dir(&path, candidates)?;
        } else if path.is_file() && is_photo_path(&path) {
            candidates.push(build_candidate(path)?);
        }
    }

    Ok(())
}

fn build_candidate(path: PathBuf) -> Result<ThumbnailCandidate> {
    Ok(ThumbnailCandidate {
        stem_key: stem_key(&path),
        normalized: normalize_existing_path(&path)?,
        path,
    })
}

fn validate_thumbnail_file(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        bail!(
            "{}",
            if is_zh() {
                format!("封面文件不存在: {}", path.display())
            } else {
                format!("Thumbnail file not found: {}", path.display())
            }
        );
    }

    if !path.is_file() {
        bail!(
            "{}",
            if is_zh() {
                format!("封面路径不是文件: {}", path.display())
            } else {
                format!("Thumbnail path is not a file: {}", path.display())
            }
        );
    }

    if !is_photo_path(path) {
        bail!(
            "{}",
            if is_zh() {
                format!("封面文件必须是图片: {}", path.display())
            } else {
                format!("Thumbnail file must be an image: {}", path.display())
            }
        );
    }

    Ok(path.to_path_buf())
}

fn normalize_existing_path(path: &Path) -> Result<PathBuf> {
    fs::canonicalize(path).with_context(|| {
        if is_zh() {
            format!("无法标准化路径 '{}'", path.display())
        } else {
            format!("Failed to normalize path '{}'", path.display())
        }
    })
}

fn stem_key(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

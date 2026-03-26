//! File download with progress and multi-threaded support

use crate::i18n::pick;
use crate::utils::create_progress_bar;
use anyhow::{bail, Result};
use grammers_client::Client;
use grammers_tl_types as tl;
use indicatif::ProgressBar;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::{Mutex, Semaphore};

/// Maximum number of concurrent download workers
const MAX_WORKERS: usize = 8;

/// Minimum number of concurrent download workers
const MIN_WORKERS: usize = 2;

/// Chunk size for each download request (512KB)
const CHUNK_SIZE: i32 = 512 * 1024;

/// Minimum file size to use multi-threaded download (2MB)
const MIN_SIZE_FOR_PARALLEL: u64 = 2 * 1024 * 1024;

/// Get optimal number of workers based on CPU cores
/// For I/O bound tasks, we can use more workers than CPU cores
fn get_workers() -> usize {
    let cpus = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(MIN_WORKERS);
    // For I/O tasks, use 2x CPU cores but within MIN/MAX bounds
    (cpus * 2).clamp(MIN_WORKERS, MAX_WORKERS)
}

/// Download result
pub struct DownloadResult {
    pub filename: String,
    pub size: u64,
}

/// Photo info for template context
pub struct PhotoInfo {
    pub id: i64,
    pub dc_id: i32,
}

impl PhotoInfo {
    pub fn from_photo(photo: &tl::types::Photo) -> Self {
        Self {
            id: photo.id,
            dc_id: photo.dc_id,
        }
    }

    /// Get default filename
    pub fn default_filename(&self) -> String {
        format!("{}.jpg", self.id)
    }
}

/// Document info for template context
pub struct DocumentInfo {
    pub id: i64,
    pub dc_id: i32,
    pub size: u64,
    pub filename: String,
}

impl DocumentInfo {
    pub fn from_document(doc: &tl::types::Document) -> Self {
        let filename = doc
            .attributes
            .iter()
            .find_map(|attr| {
                if let tl::enums::DocumentAttribute::Filename(f) = attr {
                    Some(f.file_name.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                let mut is_video = false;
                let mut is_audio = false;
                for attr in &doc.attributes {
                    match attr {
                        tl::enums::DocumentAttribute::Video(_) => is_video = true,
                        tl::enums::DocumentAttribute::Audio(_) => is_audio = true,
                        _ => {}
                    }
                }

                let ext = if is_video {
                    "mp4"
                } else if is_audio {
                    if doc.mime_type == "audio/ogg" {
                        "ogg"
                    } else {
                        "mp3"
                    }
                } else {
                    match doc.mime_type.as_str() {
                        "video/mp4" => "mp4",
                        "video/x-matroska" => "mkv",
                        "video/quicktime" => "mov",
                        "video/webm" => "webm",
                        "image/jpeg" => "jpg",
                        "image/png" => "png",
                        "image/gif" => "gif",
                        "image/webp" => "webp",
                        "audio/mpeg" => "mp3",
                        "audio/ogg" | "audio/opus" => "ogg",
                        "audio/m4a" | "audio/mp4" => "m4a",
                        "audio/x-wav" | "audio/wav" => "wav",
                        "application/pdf" => "pdf",
                        "application/zip" => "zip",
                        "application/x-rar-compressed" => "rar",
                        _ => {
                            let parts: Vec<&str> = doc.mime_type.split('/').collect();
                            if parts.len() == 2
                                && !parts[1].is_empty()
                                && parts[1].len() <= 5
                                && parts[1].chars().all(|c| c.is_ascii_alphanumeric())
                            {
                                parts[1]
                            } else {
                                "bin"
                            }
                        }
                    }
                };
                format!("{}.{}", doc.id, ext)
            });

        Self {
            id: doc.id,
            dc_id: doc.dc_id,
            size: doc.size as u64,
            filename,
        }
    }
}

/// Download a photo to the specified path
pub async fn download_photo(
    client: &Client,
    photo: &tl::types::Photo,
    file_path: &Path,
) -> Result<DownloadResult> {
    download_photo_with_progress(client, photo, file_path, None).await
}

/// Download a photo with optional external progress bar
pub async fn download_photo_with_progress(
    client: &Client,
    photo: &tl::types::Photo,
    file_path: &Path,
    progress: Option<Arc<ProgressBar>>,
) -> Result<DownloadResult> {
    // Find the largest photo size
    let (size_type, file_size) = photo
        .sizes
        .iter()
        .filter_map(|s| match s {
            tl::enums::PhotoSize::Size(ps) => Some((ps.r#type.clone(), ps.size as u64)),
            tl::enums::PhotoSize::Progressive(ps) => ps
                .sizes
                .last()
                .map(|&size| (ps.r#type.clone(), size as u64)),
            _ => None,
        })
        .max_by_key(|(_, size)| *size)
        .unwrap_or_default();

    let location = tl::types::InputPhotoFileLocation {
        id: photo.id,
        access_hash: photo.access_hash,
        file_reference: photo.file_reference.clone(),
        thumb_size: size_type,
    };

    let size = download_file_with_dc_progress(
        client,
        location.into(),
        file_path,
        file_size,
        photo.dc_id,
        progress,
    )
    .await?;

    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("photo.jpg")
        .to_string();

    Ok(DownloadResult { filename, size })
}

/// Download a document to the specified path
pub async fn download_document(
    client: &Client,
    doc: &tl::types::Document,
    file_path: &Path,
) -> Result<DownloadResult> {
    download_document_with_progress(client, doc, file_path, None).await
}

/// Download a document with optional external progress bar
pub async fn download_document_with_progress(
    client: &Client,
    doc: &tl::types::Document,
    file_path: &Path,
    progress: Option<Arc<ProgressBar>>,
) -> Result<DownloadResult> {
    let file_size = doc.size as u64;

    let location = tl::types::InputDocumentFileLocation {
        id: doc.id,
        access_hash: doc.access_hash,
        file_reference: doc.file_reference.clone(),
        thumb_size: String::new(),
    };

    let size = download_file_with_dc_progress(
        client,
        location.into(),
        file_path,
        file_size,
        doc.dc_id,
        progress,
    )
    .await?;

    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    Ok(DownloadResult { filename, size })
}

/// Download file with DC support and optional external progress bar
async fn download_file_with_dc_progress(
    client: &Client,
    location: tl::enums::InputFileLocation,
    file_path: &Path,
    file_size: u64,
    dc_id: i32,
    progress: Option<Arc<ProgressBar>>,
) -> Result<u64> {
    // Try to download from default DC first
    match download_file_parallel_progress(
        client,
        location.clone(),
        file_path,
        file_size,
        None,
        progress.clone(),
    )
    .await
    {
        Ok(size) => Ok(size),
        Err(e) => {
            let err_str = e.to_string();
            // Check for FILE_MIGRATE or AUTH_KEY errors - need to use file DC
            if err_str.contains("FILE_MIGRATE") || err_str.contains("AUTH_KEY") {
                // Export authorization to target DC and retry
                export_auth_to_dc(client, dc_id).await?;
                download_file_parallel_progress(
                    client,
                    location,
                    file_path,
                    file_size,
                    Some(dc_id),
                    progress,
                )
                .await
            } else {
                Err(e)
            }
        }
    }
}

/// Export authorization to another DC
async fn export_auth_to_dc(client: &Client, dc_id: i32) -> Result<()> {
    let export = client
        .invoke(&tl::functions::auth::ExportAuthorization { dc_id })
        .await?;

    let tl::enums::auth::ExportedAuthorization::Authorization(exported) = export;

    let _import = client
        .invoke_in_dc(
            dc_id,
            &tl::functions::auth::ImportAuthorization {
                id: exported.id,
                bytes: exported.bytes,
            },
        )
        .await?;

    Ok(())
}

/// Download file with parallel workers and optional external progress bar
async fn download_file_parallel_progress(
    client: &Client,
    location: tl::enums::InputFileLocation,
    file_path: &Path,
    file_size: u64,
    dc_id: Option<i32>,
    external_progress: Option<Arc<ProgressBar>>,
) -> Result<u64> {
    // For small files or unknown size, use sequential download
    if file_size == 0 || file_size < MIN_SIZE_FOR_PARALLEL {
        return download_file_sequential_progress(
            client,
            location,
            file_path,
            file_size,
            dc_id,
            external_progress,
        )
        .await;
    }

    // Create progress bar if not provided externally
    let (pb, is_external) = if let Some(ext_pb) = external_progress {
        (ext_pb, true)
    } else {
        (Arc::new(create_progress_bar(file_size)?), false)
    };

    // Pre-allocate file
    let file = File::create(file_path).await?;
    file.set_len(file_size).await?;
    let file = Arc::new(Mutex::new(file));

    // Calculate chunks
    let chunk_size = CHUNK_SIZE as u64;
    let num_chunks = (file_size + chunk_size - 1) / chunk_size;

    // Track progress
    let downloaded = Arc::new(AtomicU64::new(0));

    // Semaphore to limit concurrent requests based on CPU cores
    let workers = get_workers();
    let semaphore = Arc::new(Semaphore::new(workers));

    // Spawn tasks for each chunk
    let mut handles = Vec::new();

    for i in 0..num_chunks {
        let start = i * chunk_size;
        let end = std::cmp::min(start + chunk_size, file_size);

        let client = client.clone();
        let location = location.clone();
        let file = Arc::clone(&file);
        let downloaded = Arc::clone(&downloaded);
        let pb = pb.clone();
        let semaphore = Arc::clone(&semaphore);

        let handle = tokio::spawn(async move {
            // Acquire semaphore permit
            let _permit = semaphore.acquire().await.unwrap();

            let offset = start as i64;
            let _expected_size = (end - start) as i32;

            // For precise mode, limit must be a power of 2 between 4KB and 1MB
            // Use fixed chunk size instead of calculated limit
            let result = if let Some(dc) = dc_id {
                client
                    .invoke_in_dc(
                        dc,
                        &tl::functions::upload::GetFile {
                            precise: false,
                            cdn_supported: false,
                            location: location.clone(),
                            offset,
                            limit: CHUNK_SIZE,
                        },
                    )
                    .await
            } else {
                client
                    .invoke(&tl::functions::upload::GetFile {
                        precise: false,
                        cdn_supported: false,
                        location: location.clone(),
                        offset,
                        limit: CHUNK_SIZE,
                    })
                    .await
            };

            let bytes = match result {
                Ok(tl::enums::upload::File::File(f)) => f.bytes,
                Ok(tl::enums::upload::File::CdnRedirect(_)) => {
                    return Err(anyhow::anyhow!(pick(
                        "暂不支持 CDN 重定向",
                        "CDN redirect not supported"
                    )));
                }
                Err(e) => return Err(e.into()),
            };

            // Write to file at correct position
            {
                let mut file = file.lock().await;
                file.seek(std::io::SeekFrom::Start(start)).await?;
                file.write_all(&bytes).await?;
            }

            // Update progress
            let len = bytes.len() as u64;
            let total = downloaded.fetch_add(len, Ordering::Relaxed) + len;
            pb.set_position(total);

            Ok::<(), anyhow::Error>(())
        });

        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await??;
    }

    // Flush and close file
    {
        let mut file = file.lock().await;
        file.flush().await?;
    }

    // Only finish and clear if we created the progress bar
    if !is_external {
        pb.finish_and_clear();
    }

    Ok(downloaded.load(Ordering::Relaxed))
}

/// Sequential download with optional external progress bar
async fn download_file_sequential_progress(
    client: &Client,
    location: tl::enums::InputFileLocation,
    file_path: &Path,
    file_size: u64,
    dc_id: Option<i32>,
    external_progress: Option<Arc<ProgressBar>>,
) -> Result<u64> {
    // Use external progress bar or create our own
    let (pb, is_external) = if let Some(ext_pb) = external_progress {
        (Some(ext_pb), true)
    } else if file_size > 0 {
        (Some(Arc::new(create_progress_bar(file_size)?)), false)
    } else {
        (None, false)
    };

    let mut file = File::create(file_path).await?;
    let mut offset = 0i64;
    let limit = CHUNK_SIZE;
    let mut total_bytes = 0u64;

    loop {
        let result = if let Some(dc) = dc_id {
            client
                .invoke_in_dc(
                    dc,
                    &tl::functions::upload::GetFile {
                        precise: false,
                        cdn_supported: false,
                        location: location.clone(),
                        offset,
                        limit,
                    },
                )
                .await?
        } else {
            client
                .invoke(&tl::functions::upload::GetFile {
                    precise: false,
                    cdn_supported: false,
                    location: location.clone(),
                    offset,
                    limit,
                })
                .await?
        };

        let bytes = match result {
            tl::enums::upload::File::File(f) => f.bytes,
            tl::enums::upload::File::CdnRedirect(_) => {
                bail!(
                    "{}",
                    pick("暂不支持 CDN 重定向", "CDN redirect not supported")
                );
            }
        };

        if bytes.is_empty() {
            break;
        }

        let len = bytes.len();
        file.write_all(&bytes).await?;
        total_bytes += len as u64;
        offset += len as i64;

        if let Some(ref pb) = pb {
            pb.set_position(total_bytes);
            // Force redraw for small files that complete quickly
            pb.tick();
        }

        if len < limit as usize {
            break;
        }
    }

    file.flush().await?;

    // Only finish and clear if we created the progress bar
    if let Some(pb) = pb {
        if !is_external {
            pb.finish_and_clear();
        }
    }

    Ok(total_bytes)
}

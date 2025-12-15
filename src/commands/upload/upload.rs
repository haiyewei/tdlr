//! Upload command entry point

use super::file::{collect_files, FileFilter};
use super::handler::{
    remove_files, upload_media_groups, upload_single_files, UploadContext, UploadStats,
};
use super::output;
use crate::telegram::{pool, SessionManager};
use anyhow::{bail, Result};

/// Default concurrent upload count (max allowed by Telegram)
const DEFAULT_CONCURRENT: usize = 10;

pub async fn run(
    paths: Vec<String>,
    chat: Option<String>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    rm: bool,
    topic: Option<i32>,
    account: Option<Vec<i64>>,
    all_accounts: bool,
    caption: Option<String>,
    to: Option<String>,
    group: bool,
) -> Result<()> {
    if paths.is_empty() {
        bail!("No paths specified");
    }

    // Get clients based on account selection
    let clients = if all_accounts {
        pool().get_all().await?
    } else if let Some(ids) = account {
        pool().get_many(&ids).await?
    } else {
        vec![pool().get_active().await?]
    };

    if clients.is_empty() {
        bail!("No accounts available. Please login first with 'tdlr auth login add'");
    }

    // Build file filter and collect files
    let filter = FileFilter::new(include, exclude);
    let (files, initial_failed) = collect_files(&paths, &filter);

    if files.is_empty() {
        bail!("No valid files to upload");
    }

    let mut stats = UploadStats::default();
    stats.add_failed(initial_failed);

    // Upload to each client
    for client in &clients {
        if clients.len() > 1 {
            let account_info = SessionManager::get_account(client.user_id)?;
            let name = account_info
                .map(|a| a.display_name)
                .unwrap_or_else(|| client.user_id.to_string());
            output::print_account_header(&name, client.user_id);
        }

        if !client.is_authorized().await? {
            output::print_account_not_authorized(client.user_id);
            continue;
        }

        let ctx = UploadContext {
            client: client.inner(),
            chat: &chat,
            topic,
            caption: &caption,
            to: &to,
            concurrent: DEFAULT_CONCURRENT,
        };

        if group {
            upload_media_groups(&ctx, &files, &mut stats).await?;
        } else {
            upload_single_files(&ctx, &files, &mut stats).await?;
        }
    }

    // Remove files after all uploads complete
    if rm {
        let removed = remove_files(&files);
        if removed > 0 {
            output::print_removed_files(removed);
        }
    }

    output::print_summary(stats.success, stats.failed);

    Ok(())
}

//! Download command entry point

use super::handler::{download_link, DownloadContext, DownloadStats};
use super::output;
use super::template::DEFAULT_TEMPLATE;
use crate::telegram::{pool, SessionManager};
use crate::utils::{parse_link, ExtFilter};
use anyhow::{bail, Result};
use std::path::Path;

pub async fn run(
    urls: Vec<String>,
    output: String,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    template: Option<String>,
    account: Option<i64>,
) -> Result<()> {
    if urls.is_empty() {
        bail!("No URLs specified");
    }

    // Parse all URLs first
    let mut links = Vec::new();
    for url in &urls {
        match parse_link(url) {
            Ok(link) => links.push(link),
            Err(e) => {
                eprintln!("Warning: Invalid URL '{}': {}", url, e);
            }
        }
    }

    if links.is_empty() {
        bail!("No valid URLs to download");
    }

    // Ensure output directory exists
    let output_dir = Path::new(&output);
    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)?;
    }

    // Get client
    let client = if let Some(id) = account {
        pool().get(id).await?
    } else {
        pool().get_active().await?
    };

    if !client.is_authorized().await? {
        bail!(
            "Account {} not authorized. Please login first with 'tdlr auth login add'",
            client.user_id
        );
    }

    // Show account info
    let account_info = SessionManager::get_account(client.user_id)?;
    let name = account_info
        .map(|a| a.display_name)
        .unwrap_or_else(|| client.user_id.to_string());
    output::print_account_header(&name, client.user_id);

    // Build extension filter
    let filter = ExtFilter::new(include, exclude);

    // Use provided template or default
    let template = template.unwrap_or_else(|| DEFAULT_TEMPLATE.to_string());

    let mut stats = DownloadStats::default();
    let total = links.len();

    // Download each link
    for (i, link) in links.iter().enumerate() {
        output::print_progress(i, total, link);

        let ctx = DownloadContext {
            client: client.inner(),
            output_dir,
            filter: &filter,
            template: &template,
        };

        if let Err(e) = download_link(&ctx, link, &mut stats).await {
            output::print_failure(&e.to_string());
            stats.add_failed();
        }
    }

    output::print_summary(stats.success, stats.failed);

    Ok(())
}

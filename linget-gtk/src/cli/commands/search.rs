use crate::backend::PackageManager;
use crate::cli::OutputWriter;
use crate::models::PackageSource;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn run(
    pm: Arc<Mutex<PackageManager>>,
    query: &str,
    source: Option<PackageSource>,
    writer: &OutputWriter,
) -> Result<()> {
    let spinner = if !writer.is_quiet() && !writer.is_json() {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("Searching for '{}'...", query));
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    let manager = pm.lock().await;
    let packages = manager.search(query).await?;

    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    // Filter by source if specified
    let packages: Vec<_> = if let Some(src) = source {
        packages.into_iter().filter(|p| p.source == src).collect()
    } else {
        packages
    };

    let title = format!("Search Results for '{}' ({})", query, packages.len());
    writer.packages(&packages, Some(&title));

    if !writer.is_json() && !packages.is_empty() {
        writer.message("\nRun 'linget install <package>' to install a package");
    }

    Ok(())
}

use crate::backend::PackageManager;
use crate::cli::OutputWriter;
use crate::models::PackageSource;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn run(
    pm: Arc<Mutex<PackageManager>>,
    source: Option<PackageSource>,
    updates_only: bool,
    writer: &OutputWriter,
) -> Result<()> {
    let spinner = if !writer.is_quiet() && !writer.is_json() {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(if updates_only {
            "Checking for updates..."
        } else {
            "Loading packages..."
        });
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    let manager = pm.lock().await;
    writer.verbose("Querying package backends...");

    let packages = if updates_only {
        manager.check_all_updates().await?
    } else {
        manager.list_all_installed().await?
    };

    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }
    writer.verbose(&format!("Found {} packages from all sources", packages.len()));

    // Filter by source if specified
    let packages: Vec<_> = if let Some(src) = source {
        packages.into_iter().filter(|p| p.source == src).collect()
    } else {
        packages
    };

    let title = if updates_only {
        format!("Available Updates ({})", packages.len())
    } else {
        format!("Installed Packages ({})", packages.len())
    };

    writer.packages(&packages, Some(&title));

    if !writer.is_json() && updates_only && !packages.is_empty() {
        writer.message("\nRun 'linget update --all' to update all packages");
    }

    Ok(())
}

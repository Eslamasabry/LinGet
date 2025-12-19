use crate::backend::PackageManager;
use crate::cli::OutputWriter;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn run(pm: Arc<Mutex<PackageManager>>, writer: &OutputWriter) -> Result<()> {
    let spinner = if !writer.is_quiet() && !writer.is_json() {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message("Checking for updates across all sources...");
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    let manager = pm.lock().await;
    let updates = manager.check_all_updates().await?;

    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    if updates.is_empty() {
        writer.success("All packages are up to date!");
    } else {
        writer.packages(
            &updates,
            Some(&format!("Updates Available ({})", updates.len())),
        );
        if !writer.is_json() {
            writer.message("\nRun 'linget update --all' to update all packages");
            writer.message("Run 'linget update <package>' to update a specific package");
        }
    }

    Ok(())
}

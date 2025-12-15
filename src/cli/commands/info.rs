use crate::backend::PackageManager;
use crate::cli::OutputWriter;
use crate::models::PackageSource;
use anyhow::{bail, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn run(
    pm: Arc<Mutex<PackageManager>>,
    package_name: &str,
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
        pb.set_message(format!("Looking up {}...", package_name));
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    let manager = pm.lock().await;

    // First check installed packages
    let installed = manager.list_all_installed().await?;
    let mut candidates: Vec<_> = installed
        .into_iter()
        .filter(|p| p.name.eq_ignore_ascii_case(package_name) || p.name.contains(package_name))
        .collect();

    // If not found, search all sources
    if candidates.is_empty() {
        let search_results = manager.search(package_name).await?;
        candidates = search_results
            .into_iter()
            .filter(|p| {
                p.name.eq_ignore_ascii_case(package_name) || p.name.contains(package_name)
            })
            .collect();
    }

    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    // Filter by source if specified
    if let Some(src) = source {
        candidates.retain(|p| p.source == src);
    }

    if candidates.is_empty() {
        if let Some(src) = source {
            bail!("Package '{}' not found in {:?}", package_name, src);
        } else {
            bail!("Package '{}' not found", package_name);
        }
    }

    // Select the package
    let package = if candidates.len() == 1 {
        candidates.remove(0)
    } else if let Some(exact) = candidates
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(package_name))
    {
        exact.clone()
    } else {
        // Multiple candidates
        if !writer.is_json() {
            writer.message(&format!(
                "\nMultiple packages match '{}':",
                package_name
            ));
            for (i, pkg) in candidates.iter().enumerate() {
                println!(
                    "  {} {} ({:?})",
                    style(format!("[{}]", i + 1)).cyan(),
                    pkg.name,
                    pkg.source
                );
            }
            print!("\nSelect package [1-{}]: ", candidates.len());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let selection: usize = input
                .trim()
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid selection"))?;

            if selection < 1 || selection > candidates.len() {
                bail!("Invalid selection");
            }

            candidates.remove(selection - 1)
        } else {
            candidates.remove(0)
        }
    };

    writer.package_info(&package);
    Ok(())
}

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
    skip_confirm: bool,
    writer: &OutputWriter,
) -> Result<()> {
    let manager = pm.lock().await;

    // Find the installed package
    let installed = manager.list_all_installed().await?;

    let mut candidates: Vec<_> = installed
        .into_iter()
        .filter(|p| p.name.eq_ignore_ascii_case(package_name) || p.name.contains(package_name))
        .collect();

    // Filter by source if specified
    if let Some(src) = source {
        candidates.retain(|p| p.source == src);
    }

    if candidates.is_empty() {
        if let Some(src) = source {
            bail!(
                "Package '{}' not found in installed {:?} packages",
                package_name,
                src
            );
        } else {
            bail!("Package '{}' is not installed", package_name);
        }
    }

    // Select the package to remove
    let package = if candidates.len() == 1 {
        candidates.remove(0)
    } else if let Some(exact) = candidates
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(package_name))
    {
        exact.clone()
    } else {
        // Multiple candidates
        if !writer.is_json() && !skip_confirm {
            writer.message(&format!(
                "\nMultiple installed packages match '{}':",
                package_name
            ));
            for (i, pkg) in candidates.iter().enumerate() {
                println!(
                    "  {} {} ({:?}) v{}",
                    style(format!("[{}]", i + 1)).cyan(),
                    pkg.name,
                    pkg.source,
                    pkg.version
                );
            }
            print!("\nSelect package [1-{}] or 'q' to quit: ", candidates.len());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.eq_ignore_ascii_case("q") {
                writer.message("Removal cancelled");
                return Ok(());
            }

            let selection: usize = input.parse().map_err(|_| {
                anyhow::anyhow!("Invalid selection. Run again with --source to specify.")
            })?;

            if selection < 1 || selection > candidates.len() {
                bail!("Invalid selection");
            }

            candidates.remove(selection - 1)
        } else {
            candidates.remove(0)
        }
    };

    // Confirm removal
    if !skip_confirm && !writer.is_json() {
        print!(
            "\n{} {} ({:?})? [y/N] ",
            style("Remove").red().bold(),
            style(&package.name).cyan(),
            package.source
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            writer.message("Removal cancelled");
            return Ok(());
        }
    }

    // Show progress
    let spinner = if !writer.is_quiet() && !writer.is_json() {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("Removing {}...", package.name));
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    let result = manager.remove(&package).await;

    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    match result {
        Ok(_) => {
            writer.success(&format!(
                "Successfully removed {} from {:?}",
                package.name, package.source
            ));
            Ok(())
        }
        Err(e) => {
            writer.error(&format!("Failed to remove {}: {}", package.name, e));
            Err(e)
        }
    }
}

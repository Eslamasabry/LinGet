use crate::backend::PackageManager;
use crate::cli::OutputWriter;
use crate::models::{Package, PackageSource, PackageStatus};
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
    // First, search for the package to find which source(s) have it
    let manager = pm.lock().await;
    let search_results = manager.search(package_name).await?;

    // Filter to exact matches (or very close)
    let mut candidates: Vec<_> = search_results
        .into_iter()
        .filter(|p| p.name.eq_ignore_ascii_case(package_name) || p.name.contains(package_name))
        .collect();

    // Sort exact matches first
    candidates.sort_by(|a, b| {
        let a_exact = a.name.eq_ignore_ascii_case(package_name);
        let b_exact = b.name.eq_ignore_ascii_case(package_name);
        b_exact.cmp(&a_exact)
    });

    // Filter by source if specified
    if let Some(src) = source {
        candidates.retain(|p| p.source == src);
    }

    if candidates.is_empty() {
        if let Some(src) = source {
            bail!("Package '{}' not found in {:?}", package_name, src);
        } else {
            bail!("Package '{}' not found in any source", package_name);
        }
    }

    // Select the package to install
    let package = if candidates.len() == 1 {
        candidates.remove(0)
    } else if let Some(src) = source {
        // User specified source, find exact match
        candidates
            .into_iter()
            .find(|p| p.source == src && p.name.eq_ignore_ascii_case(package_name))
            .ok_or_else(|| anyhow::anyhow!("Package '{}' not found in {:?}", package_name, src))?
    } else {
        // Multiple candidates, prefer exact name match
        if let Some(exact) = candidates
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(package_name))
        {
            exact.clone()
        } else {
            // Show options and ask user (in non-JSON mode)
            if !writer.is_json() && !skip_confirm {
                writer.message(&format!(
                    "\nMultiple packages found matching '{}':",
                    package_name
                ));
                for (i, pkg) in candidates.iter().enumerate() {
                    println!(
                        "  {} {} ({:?}) - {}",
                        style(format!("[{}]", i + 1)).cyan(),
                        pkg.name,
                        pkg.source,
                        pkg.description
                    );
                }
                print!("\nSelect package [1-{}] or 'q' to quit: ", candidates.len());
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim();

                if input.eq_ignore_ascii_case("q") {
                    writer.message("Installation cancelled");
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
                // In JSON/quiet mode, pick first result
                candidates.remove(0)
            }
        }
    };

    // Confirm installation
    if !skip_confirm && !writer.is_json() {
        print!(
            "\nInstall {} ({:?})? [y/N] ",
            style(&package.name).cyan(),
            package.source
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            writer.message("Installation cancelled");
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
        pb.set_message(format!("Installing {}...", package.name));
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    // Create a Package struct for installation
    let install_pkg = Package {
        name: package.name.clone(),
        version: package.version.clone(),
        available_version: None,
        description: package.description.clone(),
        source: package.source,
        status: PackageStatus::NotInstalled,
        size: None,
        homepage: None,
        license: None,
        maintainer: None,
        dependencies: Vec::new(),
        install_date: None,
        enrichment: None,
    };

    let result = manager.install(&install_pkg).await;

    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    match result {
        Ok(_) => {
            writer.success(&format!(
                "Successfully installed {} from {:?}",
                package.name, package.source
            ));
            Ok(())
        }
        Err(e) => {
            writer.anyhow_error(&e);
            Err(e)
        }
    }
}

use crate::backend::PackageManager;
use crate::cli::OutputWriter;
use crate::models::PackageSource;
use anyhow::{bail, Result};
use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn run(
    pm: Arc<Mutex<PackageManager>>,
    package_name: Option<&str>,
    source: Option<PackageSource>,
    update_all: bool,
    skip_confirm: bool,
    writer: &OutputWriter,
) -> Result<()> {
    let manager = pm.lock().await;

    if update_all || package_name.is_none() {
        // Update all packages with available updates
        let spinner = if !writer.is_quiet() && !writer.is_json() {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap(),
            );
            pb.set_message("Checking for updates...");
            pb.enable_steady_tick(std::time::Duration::from_millis(80));
            Some(pb)
        } else {
            None
        };

        let mut updates = manager.check_all_updates().await?;

        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }

        // Filter by source if specified
        if let Some(src) = source {
            updates.retain(|p| p.source == src);
        }

        if updates.is_empty() {
            writer.success("All packages are up to date!");
            return Ok(());
        }

        // Show what will be updated
        if !writer.is_json() && !writer.is_quiet() {
            writer.header(&format!("Updates Available ({})", updates.len()));
            for pkg in &updates {
                let version_info = if let Some(ref avail) = pkg.available_version {
                    format!("{} → {}", pkg.version, avail)
                } else {
                    pkg.version.clone()
                };
                println!(
                    "  {} ({:?}) {}",
                    style(&pkg.name).cyan(),
                    pkg.source,
                    version_info
                );
            }
        }

        // Confirm
        if !skip_confirm && !writer.is_json() {
            print!("\nUpdate all {} packages? [y/N] ", updates.len());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                writer.message("Update cancelled");
                return Ok(());
            }
        }

        // Update each package
        let multi = if !writer.is_quiet() && !writer.is_json() {
            Some(MultiProgress::new())
        } else {
            None
        };

        let mut success_count = 0;
        let mut fail_count = 0;

        for pkg in &updates {
            let spinner = multi.as_ref().map(|m| {
                let pb = m.add(ProgressBar::new_spinner());
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.cyan} {msg}")
                        .unwrap(),
                );
                pb.set_message(format!("Updating {}...", pkg.name));
                pb.enable_steady_tick(std::time::Duration::from_millis(80));
                pb
            });

            match manager.update(pkg).await {
                Ok(_) => {
                    if let Some(pb) = spinner {
                        pb.finish_with_message(format!("{} {} updated", style("✓").green(), pkg.name));
                    }
                    success_count += 1;
                }
                Err(e) => {
                    if let Some(pb) = spinner {
                        pb.finish_with_message(format!(
                            "{} {} failed: {}",
                            style("✗").red(),
                            pkg.name,
                            e
                        ));
                    }
                    fail_count += 1;
                }
            }
        }

        if fail_count == 0 {
            writer.success(&format!("Successfully updated {} packages", success_count));
        } else {
            writer.warning(&format!(
                "Updated {} packages, {} failed",
                success_count, fail_count
            ));
        }
    } else if let Some(name) = package_name {
        // Update specific package
        let installed = manager.list_all_installed().await?;
        let mut candidates: Vec<_> = installed
            .into_iter()
            .filter(|p| p.name.eq_ignore_ascii_case(name))
            .collect();

        if let Some(src) = source {
            candidates.retain(|p| p.source == src);
        }

        if candidates.is_empty() {
            bail!("Package '{}' is not installed", name);
        }

        let package = if candidates.len() == 1 {
            candidates.remove(0)
        } else {
            // Multiple installed, pick first or exact match
            candidates
                .iter()
                .find(|p| p.name.eq_ignore_ascii_case(name))
                .cloned()
                .unwrap_or_else(|| candidates.remove(0))
        };

        // Confirm
        if !skip_confirm && !writer.is_json() {
            print!(
                "\nUpdate {} ({:?})? [y/N] ",
                style(&package.name).cyan(),
                package.source
            );
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                writer.message("Update cancelled");
                return Ok(());
            }
        }

        let spinner = if !writer.is_quiet() && !writer.is_json() {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap(),
            );
            pb.set_message(format!("Updating {}...", package.name));
            pb.enable_steady_tick(std::time::Duration::from_millis(80));
            Some(pb)
        } else {
            None
        };

        let result = manager.update(&package).await;

        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }

        match result {
            Ok(_) => {
                writer.success(&format!("Successfully updated {}", package.name));
            }
            Err(e) => {
                writer.error(&format!("Failed to update {}: {}", package.name, e));
                return Err(e);
            }
        }
    }

    Ok(())
}

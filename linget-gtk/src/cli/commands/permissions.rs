use crate::backend::PackageManager;
use crate::cli::OutputWriter;
use crate::models::{PrivacyLevel, SandboxRating};
use anyhow::Result;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Subcommand actions for permissions
pub enum PermissionsAction {
    /// Show permissions for an app
    Show,
    /// Show only override permissions
    Overrides,
    /// Reset all overrides for an app
    Reset,
}

pub async fn run(
    pm: Arc<Mutex<PackageManager>>,
    app_id: &str,
    action: PermissionsAction,
    writer: &OutputWriter,
) -> Result<()> {
    let spinner = if !writer.is_quiet() && !writer.is_json() {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("Fetching permissions for {}...", app_id));
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    let manager = pm.lock().await;

    match action {
        PermissionsAction::Show => {
            let metadata = manager.get_flatpak_metadata(app_id).await?;

            if let Some(pb) = spinner {
                pb.finish_and_clear();
            }

            if writer.is_json() {
                let json = serde_json::to_string_pretty(&metadata)?;
                println!("{}", json);
                return Ok(());
            }

            // Print header
            println!(
                "\n{} {}",
                style("Flatpak Permissions:").bold().cyan(),
                style(&metadata.app_id).bold()
            );
            println!("{}", style("─".repeat(60)).dim());

            // Print sandbox summary
            let summary = metadata.sandbox_summary();
            let rating_icon = match summary.rating {
                SandboxRating::Strong => style("●").green(),
                SandboxRating::Good => style("●").cyan(),
                SandboxRating::Moderate => style("●").yellow(),
                SandboxRating::Weak => style("●").red(),
            };
            println!(
                "\n{} {} {}",
                rating_icon,
                style("Sandbox Rating:").bold(),
                style(format!("{}", summary.rating)).bold()
            );
            println!("  {}", style(&summary.description).dim());
            println!(
                "  {} total permissions, {} high-risk",
                summary.total_permissions, summary.high_risk_count
            );

            // Print runtime info
            if let Some(ref runtime) = metadata.runtime {
                println!(
                    "\n{} {}",
                    style("Runtime:").bold(),
                    style(runtime.to_string()).dim()
                );
            }

            // Print installation info
            println!(
                "{} {}",
                style("Installation:").bold(),
                style(format!("{}", metadata.installation)).dim()
            );

            if let Some(ref remote) = metadata.remote {
                println!("{} {}", style("Remote:").bold(), style(remote).dim());
            }

            if metadata.is_eol {
                println!(
                    "\n{} {}",
                    style("WARNING:").bold().red(),
                    style("This application is end-of-life").red()
                );
                if let Some(ref reason) = metadata.eol_reason {
                    println!("  {}", style(reason).dim());
                }
            }

            // Print permissions by category
            let grouped = metadata.permissions_by_category();
            if grouped.is_empty() {
                println!("\n{}", style("No special permissions required").green());
            } else {
                println!("\n{}", style("Permissions:").bold());

                for (category, permissions) in grouped {
                    println!(
                        "\n  {} {}",
                        style("▸").cyan(),
                        style(category.description()).bold()
                    );

                    for perm in permissions {
                        let level_indicator = match perm.privacy_level {
                            PrivacyLevel::Low => style("○").dim(),
                            PrivacyLevel::Medium => style("◐").yellow(),
                            PrivacyLevel::High => style("●").red(),
                        };

                        let value_style = if perm.negated {
                            style(&perm.value).strikethrough().dim()
                        } else {
                            style(&perm.value).white()
                        };

                        println!(
                            "    {} {} - {}",
                            level_indicator,
                            value_style,
                            style(&perm.description).dim()
                        );
                    }
                }
            }

            // Print legend
            println!("\n{}", style("─".repeat(60)).dim());
            println!(
                "{} {} Low  {} Medium  {} High risk",
                style("Legend:").dim(),
                style("○").dim(),
                style("◐").yellow(),
                style("●").red()
            );
        }

        PermissionsAction::Overrides => {
            let overrides = manager.get_flatpak_overrides(app_id).await?;

            if let Some(pb) = spinner {
                pb.finish_and_clear();
            }

            if writer.is_json() {
                let json = serde_json::to_string_pretty(&overrides)?;
                println!("{}", json);
                return Ok(());
            }

            println!(
                "\n{} {}",
                style("Permission Overrides:").bold().cyan(),
                style(app_id).bold()
            );
            println!("{}", style("─".repeat(60)).dim());

            if overrides.is_empty() {
                println!("\n{}", style("No permission overrides set").dim());
            } else {
                for perm in &overrides {
                    let prefix = if perm.negated {
                        style("- DENIED").red()
                    } else {
                        style("+ ALLOWED").green()
                    };
                    println!(
                        "  {} {} ({})",
                        prefix,
                        style(&perm.value).bold(),
                        perm.category.description()
                    );
                }
            }

            println!(
                "\n{}",
                style("Tip: Use 'flatpak override' to modify permissions").dim()
            );
        }

        PermissionsAction::Reset => {
            manager.reset_flatpak_overrides(app_id).await?;

            if let Some(pb) = spinner {
                pb.finish_and_clear();
            }

            if !writer.is_quiet() {
                writer.success(&format!("Reset all permission overrides for {}", app_id));
            }
        }
    }

    Ok(())
}

/// Show a quick sandbox summary for a Flatpak app
pub async fn show_sandbox_summary(
    pm: Arc<Mutex<PackageManager>>,
    app_id: &str,
    writer: &OutputWriter,
) -> Result<()> {
    let manager = pm.lock().await;
    let metadata = manager.get_flatpak_metadata(app_id).await?;
    let summary = metadata.sandbox_summary();

    if writer.is_json() {
        let json = serde_json::to_string_pretty(&summary)?;
        println!("{}", json);
        return Ok(());
    }

    let rating_color = match summary.rating {
        SandboxRating::Strong => style(format!("{}", summary.rating)).green(),
        SandboxRating::Good => style(format!("{}", summary.rating)).cyan(),
        SandboxRating::Moderate => style(format!("{}", summary.rating)).yellow(),
        SandboxRating::Weak => style(format!("{}", summary.rating)).red(),
    };

    println!(
        "{}: {} - {}",
        style("Sandbox").bold(),
        rating_color,
        style(&summary.description).dim()
    );

    Ok(())
}

/// List all Flatpak runtimes
pub async fn list_runtimes(pm: Arc<Mutex<PackageManager>>, writer: &OutputWriter) -> Result<()> {
    let spinner = if !writer.is_quiet() && !writer.is_json() {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message("Listing Flatpak runtimes...");
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    let manager = pm.lock().await;
    let runtimes = manager.list_flatpak_runtimes().await?;

    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    if writer.is_json() {
        let json = serde_json::to_string_pretty(&runtimes)?;
        println!("{}", json);
        return Ok(());
    }

    println!(
        "\n{} ({} runtimes)",
        style("Flatpak Runtimes").bold().cyan(),
        runtimes.len()
    );
    println!("{}", style("─".repeat(60)).dim());

    for runtime in &runtimes {
        println!(
            "  {} {} {}",
            style("•").dim(),
            style(&runtime.name).bold(),
            style(format!("v{}", runtime.version)).dim()
        );
        if !runtime.description.is_empty() {
            println!("    {}", style(&runtime.description).dim());
        }
    }

    Ok(())
}

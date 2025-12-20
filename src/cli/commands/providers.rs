use crate::backend::{detect_providers, ProviderStatus};
use crate::cli::OutputWriter;
use anyhow::Result;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use tabled::{
    settings::{object::Columns, Alignment, Modify, Style as TableStyle},
    Table, Tabled,
};

/// Run the providers detection command.
///
/// This command detects all package managers installed on the system
/// and displays their status, version information, and executable paths.
pub async fn run(writer: &OutputWriter, show_all: bool) -> Result<()> {
    let spinner = if !writer.is_quiet() && !writer.is_json() {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message("Detecting package managers...");
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    // Run detection in a blocking task since it involves synchronous I/O
    let providers = tokio::task::spawn_blocking(detect_providers)
        .await
        .unwrap_or_default();

    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    // Filter to only available providers unless --all is specified
    let providers: Vec<ProviderStatus> = if show_all {
        providers
    } else {
        providers.into_iter().filter(|p| p.available).collect()
    };

    match writer.format() {
        crate::cli::OutputFormat::Human => print_providers_human(&providers, writer, show_all),
        crate::cli::OutputFormat::Json => print_providers_json(&providers),
    }

    Ok(())
}

fn print_providers_human(providers: &[ProviderStatus], writer: &OutputWriter, show_all: bool) {
    if writer.is_quiet() {
        // Quiet mode: just print provider names
        for p in providers {
            if p.available {
                println!("{}", p.display_name.to_lowercase());
            }
        }
        return;
    }

    let available_count = providers.iter().filter(|p| p.available).count();
    let title = if show_all {
        format!("Package Managers ({} available)", available_count)
    } else {
        format!("Available Package Managers ({})", available_count)
    };

    println!();
    println!("{}", style(&title).bold().underlined());
    println!();

    if providers.is_empty() {
        println!(
            "{}",
            style("No package managers detected on this system").dim()
        );
        return;
    }

    let rows: Vec<ProviderRow> = providers.iter().map(ProviderRow::from).collect();
    let mut table = Table::new(rows);
    table
        .with(TableStyle::rounded())
        .with(Modify::new(Columns::single(0)).with(Alignment::left()))
        .with(Modify::new(Columns::single(1)).with(Alignment::center()))
        .with(Modify::new(Columns::single(2)).with(Alignment::left()))
        .with(Modify::new(Columns::single(3)).with(Alignment::left()));

    println!("{}", table);
    println!();

    if !show_all {
        let total = crate::models::PackageSource::ALL.len();
        let unavailable = total - available_count;
        if unavailable > 0 {
            println!(
                "{}",
                style(format!(
                    "Tip: Use --all to show {} unavailable providers",
                    unavailable
                ))
                .dim()
            );
        }
    }
}

fn print_providers_json(providers: &[ProviderStatus]) {
    #[derive(Serialize)]
    struct ProvidersOutput {
        total: usize,
        available: usize,
        providers: Vec<ProviderStatus>,
    }

    let available = providers.iter().filter(|p| p.available).count();
    let output = ProvidersOutput {
        total: providers.len(),
        available,
        providers: providers.to_vec(),
    };

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

#[derive(Tabled)]
struct ProviderRow {
    #[tabled(rename = "Provider")]
    name: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Commands")]
    commands: String,
}

impl From<&ProviderStatus> for ProviderRow {
    fn from(p: &ProviderStatus) -> Self {
        let status = if p.available {
            style("Available").green().to_string()
        } else {
            style("Unavailable").dim().to_string()
        };

        let version = p
            .version
            .as_ref()
            .map(|v| {
                // Truncate long version strings
                if v.len() > 30 {
                    format!("{}...", &v[..27])
                } else {
                    v.clone()
                }
            })
            .unwrap_or_else(|| style("-").dim().to_string());

        let commands = if p.list_cmds.is_empty() {
            style("-").dim().to_string()
        } else {
            p.list_cmds.join(", ")
        };

        Self {
            name: p.display_name.clone(),
            status,
            version,
            commands,
        }
    }
}

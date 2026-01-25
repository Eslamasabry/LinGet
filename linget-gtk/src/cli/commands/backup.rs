use crate::backend::PackageManager;
use crate::cli::{BackupAction, OutputWriter};
use crate::models::{Config, Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

const DEFAULT_BACKUP_FILE: &str = "linget-backup.json";

#[derive(Serialize, Deserialize)]
struct BackupData {
    version: u32,
    created_at: String,
    config: BackupConfig,
    packages: HashMap<String, Vec<PackageEntry>>,
}

#[derive(Serialize, Deserialize)]
struct BackupConfig {
    enabled_sources: Vec<String>,
    ignored_packages: Vec<String>,
    favorite_packages: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct PackageEntry {
    name: String,
    version: String,
}

pub async fn run(
    pm: Arc<Mutex<PackageManager>>,
    action: BackupAction,
    writer: &OutputWriter,
) -> Result<()> {
    match action {
        BackupAction::Create { output } => create_backup(pm, output, writer).await,
        BackupAction::Restore { file, yes } => restore_backup(pm, &file, yes, writer).await,
    }
}

async fn create_backup(
    pm: Arc<Mutex<PackageManager>>,
    output: Option<String>,
    writer: &OutputWriter,
) -> Result<()> {
    let output_path = output.unwrap_or_else(|| DEFAULT_BACKUP_FILE.to_string());
    let config = Config::load();

    writer.message("Collecting installed packages...");

    let manager = pm.lock().await;
    let all_packages = manager.list_all_installed().await?;

    let mut packages_by_source: HashMap<String, Vec<PackageEntry>> = HashMap::new();
    for pkg in all_packages {
        let source_key = format!("{:?}", pkg.source).to_lowercase();
        packages_by_source
            .entry(source_key)
            .or_default()
            .push(PackageEntry {
                name: pkg.name,
                version: pkg.version,
            });
    }

    let enabled_sources: Vec<String> = config
        .enabled_sources
        .to_sources()
        .iter()
        .map(|s| format!("{:?}", s).to_lowercase())
        .collect();

    let backup = BackupData {
        version: 1,
        created_at: chrono::Utc::now().to_rfc3339(),
        config: BackupConfig {
            enabled_sources,
            ignored_packages: config.ignored_packages.clone(),
            favorite_packages: config.favorite_packages.clone(),
        },
        packages: packages_by_source,
    };

    let json = serde_json::to_string_pretty(&backup).context("Failed to serialize backup")?;
    std::fs::write(&output_path, json).context("Failed to write backup file")?;

    let total_packages: usize = backup.packages.values().map(|v| v.len()).sum();
    writer.success(&format!(
        "Backup created: {} ({} packages from {} sources)",
        output_path,
        total_packages,
        backup.packages.len()
    ));

    Ok(())
}

async fn restore_backup(
    pm: Arc<Mutex<PackageManager>>,
    file: &str,
    skip_confirm: bool,
    writer: &OutputWriter,
) -> Result<()> {
    let path = PathBuf::from(file);
    if !path.exists() {
        anyhow::bail!("Backup file not found: {}", file);
    }

    let content = std::fs::read_to_string(&path).context("Failed to read backup file")?;
    let backup: BackupData =
        serde_json::from_str(&content).context("Failed to parse backup file")?;

    writer.message(&format!("Backup from: {}", backup.created_at));

    let total_packages: usize = backup.packages.values().map(|v| v.len()).sum();
    writer.message(&format!(
        "Contains {} packages from {} sources",
        total_packages,
        backup.packages.len()
    ));

    if !skip_confirm {
        writer.warning("This will attempt to install all packages from the backup.");
        writer.message("Run with --yes to skip this prompt.");
        return Ok(());
    }

    let manager = pm.lock().await;
    let mut installed = 0;
    let mut failed = 0;

    for (source_str, packages) in &backup.packages {
        let source = PackageSource::from_str(source_str);
        if source.is_none() {
            writer.warning(&format!("Unknown source: {}, skipping", source_str));
            continue;
        }
        let source = source.unwrap();

        writer.message(&format!(
            "Installing {} packages from {:?}...",
            packages.len(),
            source
        ));

        for pkg in packages {
            let stub_pkg = Package {
                name: pkg.name.clone(),
                version: String::new(),
                available_version: None,
                description: String::new(),
                source,
                status: PackageStatus::NotInstalled,
                size: None,
                homepage: None,
                license: None,
                maintainer: None,
                dependencies: Vec::new(),
                install_date: None,
                update_category: None,
                enrichment: None,
            };

            match manager.install(&stub_pkg).await {
                Ok(_) => {
                    writer.verbose(&format!("Installed: {}", pkg.name));
                    installed += 1;
                }
                Err(e) => {
                    writer.warning(&format!("Failed to install {}: {}", pkg.name, e));
                    failed += 1;
                }
            }
        }
    }

    let mut config = Config::load();
    config.ignored_packages = backup.config.ignored_packages;
    config.favorite_packages = backup.config.favorite_packages;
    config.save()?;

    writer.success(&format!(
        "Restore complete: {} installed, {} failed",
        installed, failed
    ));

    Ok(())
}

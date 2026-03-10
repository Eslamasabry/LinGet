use crate::backend::PackageManager;
use crate::cli::{BackupAction, OutputWriter};
use crate::models::{Config, PackageListExport};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

const DEFAULT_BACKUP_FILE: &str = "linget-backup.json";

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
    let backup = PackageListExport::from_installed_with_config(&all_packages, Some(&config));

    let json = backup
        .to_json_pretty()
        .context("Failed to serialize backup")?;
    std::fs::write(&output_path, json).context("Failed to write backup file")?;

    writer.success(&format!(
        "Backup created: {} ({} packages from {} sources)",
        output_path,
        backup.package_count(),
        backup.source_count()
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
    let parsed =
        PackageListExport::from_json_str(&content).context("Failed to parse backup file")?;
    let backup = parsed.export;

    for warning in parsed.warnings {
        writer.warning(&warning);
    }

    writer.message(&format!("Backup from: {}", backup.export_date_label()));

    writer.message(&format!(
        "Contains {} packages from {} sources",
        backup.package_count(),
        backup.source_count()
    ));

    if !skip_confirm {
        writer.warning("This will attempt to install all packages from the backup.");
        writer.message("Run with --yes to skip this prompt.");
        return Ok(());
    }

    let manager = pm.lock().await;
    let mut installed = 0;
    let mut failed = 0;
    let mut packages_by_source = BTreeMap::new();
    for package in &backup.packages {
        packages_by_source
            .entry(package.source)
            .or_insert_with(Vec::new)
            .push(package);
    }

    for (source, packages) in packages_by_source {
        writer.message(&format!(
            "Installing {} packages from {:?}...",
            packages.len(),
            source
        ));

        for pkg in packages {
            let stub_pkg = pkg.to_install_stub();

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

    if let Some(saved_config) = backup.config.as_ref() {
        let mut config = Config::load();
        saved_config.apply_preferences(&mut config);
        config.save()?;
    }

    writer.success(&format!(
        "Restore complete: {} installed, {} failed",
        installed, failed
    ));

    Ok(())
}

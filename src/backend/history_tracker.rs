#![allow(dead_code)]

use crate::models::{
    HistoryEntry, HistoryOperation, OperationHistory, Package, PackageSnapshot, PackageSource,
};
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info, warn};

const HISTORY_FILE: &str = "history.json";
const SNAPSHOT_FILE: &str = "snapshot.json";

fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("linget")
}

fn history_path() -> PathBuf {
    data_dir().join(HISTORY_FILE)
}

fn snapshot_path() -> PathBuf {
    data_dir().join(SNAPSHOT_FILE)
}

pub struct HistoryTracker {
    history: OperationHistory,
    snapshot: Option<PackageSnapshot>,
}

impl HistoryTracker {
    pub async fn load() -> Result<Self> {
        let dir = data_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir)
                .await
                .context("Failed to create data directory")?;
        }

        let history = load_history().await.unwrap_or_default();
        let snapshot = load_snapshot().await.ok();

        debug!(
            history_entries = history.entries.len(),
            has_snapshot = snapshot.is_some(),
            "Loaded history tracker"
        );

        Ok(Self { history, snapshot })
    }

    pub fn history(&self) -> &OperationHistory {
        &self.history
    }

    pub fn history_mut(&mut self) -> &mut OperationHistory {
        &mut self.history
    }

    pub async fn record_install(&mut self, package: &Package) {
        let entry = HistoryEntry::new(
            HistoryOperation::Install,
            package.name.clone(),
            package.source,
        )
        .with_versions(None, Some(package.version.clone()))
        .with_size_change(package.size.map(|s| s as i64).unwrap_or(0));

        self.history.add(entry);
        if let Err(e) = self.save().await {
            warn!(error = %e, "Failed to save history after install");
        }
    }

    pub async fn record_remove(&mut self, package: &Package) {
        let entry = HistoryEntry::new(
            HistoryOperation::Remove,
            package.name.clone(),
            package.source,
        )
        .with_versions(Some(package.version.clone()), None)
        .with_size_change(package.size.map(|s| -(s as i64)).unwrap_or(0));

        self.history.add(entry);
        if let Err(e) = self.save().await {
            warn!(error = %e, "Failed to save history after remove");
        }
    }

    pub async fn record_update(&mut self, package: &Package, old_version: Option<String>) {
        let entry = HistoryEntry::new(
            HistoryOperation::Update,
            package.name.clone(),
            package.source,
        )
        .with_versions(
            old_version,
            package
                .available_version
                .clone()
                .or(Some(package.version.clone())),
        );

        self.history.add(entry);
        if let Err(e) = self.save().await {
            warn!(error = %e, "Failed to save history after update");
        }
    }

    pub async fn record_cleanup(&mut self, source: Option<PackageSource>, freed_bytes: u64) {
        let source_name = source
            .map(|s| s.to_string())
            .unwrap_or_else(|| "all".to_string());

        let mut entry = HistoryEntry::new(
            HistoryOperation::Cleanup,
            format!("{} cache", source_name),
            source.unwrap_or(PackageSource::Apt),
        );
        entry.size_change = Some(-(freed_bytes as i64));

        self.history.add(entry);
        if let Err(e) = self.save().await {
            warn!(error = %e, "Failed to save history after cleanup");
        }
    }

    pub fn detect_external_changes(&self, current_packages: &[Package]) -> Vec<HistoryEntry> {
        let Some(old_snapshot) = &self.snapshot else {
            debug!("No previous snapshot, skipping external change detection");
            return Vec::new();
        };

        let mut current_snapshot = PackageSnapshot::new();
        for pkg in current_packages {
            current_snapshot.add(pkg.name.clone(), pkg.version.clone(), pkg.source);
        }

        let entries = old_snapshot.to_history_entries(&current_snapshot);
        if !entries.is_empty() {
            info!(
                external_changes = entries.len(),
                "Detected external package changes"
            );
        }

        entries
    }

    pub async fn apply_external_changes(&mut self, entries: Vec<HistoryEntry>) {
        for entry in entries {
            self.history.add(entry);
        }
        if let Err(e) = self.save().await {
            warn!(error = %e, "Failed to save history after external changes");
        }
    }

    pub async fn take_snapshot(&mut self, packages: &[Package]) {
        let mut snapshot = PackageSnapshot::new();
        for pkg in packages {
            snapshot.add(pkg.name.clone(), pkg.version.clone(), pkg.source);
        }

        self.snapshot = Some(snapshot.clone());

        if let Err(e) = save_snapshot(&snapshot).await {
            warn!(error = %e, "Failed to save snapshot");
        } else {
            debug!(package_count = packages.len(), "Saved package snapshot");
        }
    }

    pub async fn save(&self) -> Result<()> {
        save_history(&self.history).await
    }

    pub fn mark_undone(&mut self, entry_id: &str) {
        self.history.mark_undone(entry_id);
    }

    pub async fn export_json(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.history).context("Failed to serialize history")
    }

    pub async fn export_csv(&self) -> Result<String> {
        let mut csv = String::from(
            "timestamp,operation,package,source,version_before,version_after,size_change,undone\n",
        );

        for entry in &self.history.entries {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                entry.operation.label(),
                entry.package_name,
                entry.package_source,
                entry.version_before.as_deref().unwrap_or(""),
                entry.version_after.as_deref().unwrap_or(""),
                entry.size_change.unwrap_or(0),
                entry.undone
            ));
        }

        Ok(csv)
    }
}

async fn load_history() -> Result<OperationHistory> {
    let path = history_path();
    if !path.exists() {
        return Ok(OperationHistory::new());
    }

    let content = fs::read_to_string(&path)
        .await
        .context("Failed to read history file")?;

    serde_json::from_str(&content).context("Failed to parse history file")
}

async fn save_history(history: &OperationHistory) -> Result<()> {
    let path = history_path();

    if let Some(dir) = path.parent() {
        if !dir.exists() {
            fs::create_dir_all(dir)
                .await
                .context("Failed to create data directory")?;
        }
    }

    let content = serde_json::to_string_pretty(history).context("Failed to serialize history")?;

    fs::write(&path, content)
        .await
        .context("Failed to write history file")
}

async fn load_snapshot() -> Result<PackageSnapshot> {
    let path = snapshot_path();
    if !path.exists() {
        anyhow::bail!("No snapshot file found");
    }

    let content = fs::read_to_string(&path)
        .await
        .context("Failed to read snapshot file")?;

    serde_json::from_str(&content).context("Failed to parse snapshot file")
}

async fn save_snapshot(snapshot: &PackageSnapshot) -> Result<()> {
    let path = snapshot_path();

    if let Some(dir) = path.parent() {
        if !dir.exists() {
            fs::create_dir_all(dir)
                .await
                .context("Failed to create data directory")?;
        }
    }

    let content = serde_json::to_string_pretty(snapshot).context("Failed to serialize snapshot")?;

    fs::write(&path, content)
        .await
        .context("Failed to write snapshot file")
}

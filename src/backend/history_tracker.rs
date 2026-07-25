#![allow(dead_code)]

use crate::models::history::{TaskQueueEntry, TaskQueueStatus};
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

        let mut history = load_history().await.unwrap_or_default();
        let snapshot = load_snapshot().await.ok();

        let interrupted = Self::reclaim_interrupted_tasks(&mut history);

        debug!(
            history_entries = history.entries.len(),
            has_snapshot = snapshot.is_some(),
            interrupted_tasks = interrupted,
            "Loaded history tracker"
        );

        let tracker = Self { history, snapshot };
        if interrupted > 0 {
            warn!(
                count = interrupted,
                "Reclaimed tasks left running by a previous session"
            );
            tracker
                .save()
                .await
                .context("Failed to persist reclaimed task state")?;
        }

        Ok(tracker)
    }

    /// Fail any task still marked running from a previous process.
    ///
    /// Only one executor runs per process, and it has not started yet at load
    /// time, so a `Running` entry on disk cannot have anything behind it: the
    /// app was closed or killed mid-task. Left alone these sit in the queue
    /// forever, showing "started 54m ago" with no process to show for it and no
    /// way to retry, because a terminal status is what makes a task actionable
    /// again.
    fn reclaim_interrupted_tasks(history: &mut OperationHistory) -> usize {
        let mut reclaimed = 0;
        for entry in history
            .task_queue
            .entries
            .iter_mut()
            .filter(|entry| entry.status == TaskQueueStatus::Running)
        {
            entry.mark_failed(
                "Interrupted — LinGet exited while this task was running. It may or may not have \
                 completed; check the package before retrying."
                    .to_string(),
            );
            reclaimed += 1;
        }
        reclaimed
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

    pub async fn record_downgrade(&mut self, package: &Package, target_version: &str) {
        let entry = HistoryEntry::new(
            HistoryOperation::Downgrade,
            package.name.clone(),
            package.source,
        )
        .with_versions(
            Some(package.version.clone()),
            Some(target_version.to_string()),
        );

        self.history.add(entry);
        if let Err(e) = self.save().await {
            warn!(error = %e, "Failed to save history after downgrade");
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

    pub async fn enqueue_task(&mut self, entry: TaskQueueEntry) -> Result<()> {
        self.history.task_queue.enqueue(entry);
        self.save()
            .await
            .context("Failed to save history after enqueueing task")
    }

    pub async fn claim_next_task(&mut self) -> Result<Option<TaskQueueEntry>> {
        let entry = self
            .history
            .task_queue
            .entries
            .iter_mut()
            .find(|entry| entry.status == TaskQueueStatus::Queued);

        let Some(entry) = entry else {
            return Ok(None);
        };

        entry.mark_running();
        let entry_clone = entry.clone();
        self.save()
            .await
            .context("Failed to save history after starting task")?;
        Ok(Some(entry_clone))
    }

    pub async fn mark_task_completed(&mut self, entry_id: &str) -> Result<Option<TaskQueueEntry>> {
        let entry = self.history.task_queue.get_mut(entry_id);
        let Some(entry) = entry else {
            return Ok(None);
        };

        entry.mark_completed();
        let entry_clone = entry.clone();
        self.save()
            .await
            .context("Failed to save history after completing task")?;
        Ok(Some(entry_clone))
    }

    pub async fn attach_task_verification_receipt(
        &mut self,
        entry_id: &str,
        receipt_json: String,
    ) -> Result<Option<TaskQueueEntry>> {
        let entry = self.history.task_queue.get_mut(entry_id);
        let Some(entry) = entry else {
            return Ok(None);
        };
        entry.verification_receipt_json = Some(receipt_json);
        let entry_clone = entry.clone();
        self.save()
            .await
            .context("Failed to save task verification receipt")?;
        Ok(Some(entry_clone))
    }

    pub async fn mark_task_failed(
        &mut self,
        entry_id: &str,
        error: String,
    ) -> Result<Option<TaskQueueEntry>> {
        let entry = self.history.task_queue.get_mut(entry_id);
        let Some(entry) = entry else {
            return Ok(None);
        };

        entry.mark_failed(error);
        let entry_clone = entry.clone();
        self.save()
            .await
            .context("Failed to save history after task failure")?;
        Ok(Some(entry_clone))
    }

    pub async fn mark_task_cancelled(&mut self, entry_id: &str) -> Result<Option<TaskQueueEntry>> {
        let entry = self.history.task_queue.get_mut(entry_id);
        let Some(entry) = entry else {
            return Ok(None);
        };

        entry.mark_cancelled();
        let entry_clone = entry.clone();
        self.save()
            .await
            .context("Failed to save history after task cancellation")?;
        Ok(Some(entry_clone))
    }

    pub async fn replace_task_queue(&mut self, entries: Vec<TaskQueueEntry>) -> Result<()> {
        self.history.task_queue.entries = entries;
        self.history.task_queue.prune();
        self.save()
            .await
            .context("Failed to save history after replacing task queue")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::history::TaskQueueAction;
    use crate::models::PackageSource;

    fn running_entry(name: &str) -> TaskQueueEntry {
        let mut entry = TaskQueueEntry::new(
            TaskQueueAction::Update,
            format!("apt:{name}"),
            name.to_string(),
            PackageSource::Apt,
        );
        entry.mark_running();
        entry
    }

    /// Reproduces a task left as Running by a previous session: the queue
    /// showed "libk5crypto3 · APT update · started 54m ago" with no process
    /// anywhere on the machine, and no way to retry it.
    #[test]
    fn tasks_left_running_by_a_dead_session_are_reclaimed() {
        let mut history = OperationHistory::default();
        history.task_queue.enqueue(running_entry("libk5crypto3"));

        let reclaimed = HistoryTracker::reclaim_interrupted_tasks(&mut history);

        assert_eq!(reclaimed, 1);
        let entry = &history.task_queue.entries[0];
        assert_eq!(entry.status, TaskQueueStatus::Failed);
        assert!(
            entry
                .error
                .as_deref()
                .is_some_and(|e| e.contains("Interrupted")),
            "the user needs to know why it failed: {:?}",
            entry.error
        );
        assert!(
            entry.completed_at.is_some(),
            "a terminal task needs an end time"
        );
    }

    #[test]
    fn queued_and_finished_tasks_are_left_alone() {
        let mut history = OperationHistory::default();
        let queued = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "apt:vim".to_string(),
            "vim".to_string(),
            PackageSource::Apt,
        );
        let mut done = running_entry("curl");
        done.mark_completed();
        history.task_queue.enqueue(queued);
        history.task_queue.enqueue(done);

        assert_eq!(HistoryTracker::reclaim_interrupted_tasks(&mut history), 0);
        assert_eq!(
            history.task_queue.entries[0].status,
            TaskQueueStatus::Queued
        );
        assert_eq!(
            history.task_queue.entries[1].status,
            TaskQueueStatus::Completed
        );
    }

    /// End-to-end through the real load path: a Running entry on disk must come
    /// back terminal and be persisted that way. Serialised via the shared env
    /// lock because it repoints XDG_DATA_HOME. Run with `--ignored`.
    #[tokio::test]
    #[ignore = "mutates XDG_DATA_HOME"]
    async fn load_persists_reclaimed_tasks_to_disk() {
        let _guard = crate::backend::TEST_PATH_ENV_LOCK.lock().await;
        let previous = std::env::var_os("XDG_DATA_HOME");

        let root = std::env::temp_dir().join(format!("linget-reclaim-{}", std::process::id()));
        let dir = root.join("linget");
        std::fs::create_dir_all(&dir).expect("create data dir");
        std::env::set_var("XDG_DATA_HOME", &root);

        let mut history = OperationHistory::default();
        history.task_queue.enqueue(running_entry("libk5crypto3"));
        std::fs::write(
            dir.join(HISTORY_FILE),
            serde_json::to_string(&history).expect("serialise history"),
        )
        .expect("write history");

        HistoryTracker::load().await.expect("load tracker");

        let reloaded: OperationHistory =
            serde_json::from_str(&std::fs::read_to_string(dir.join(HISTORY_FILE)).unwrap())
                .expect("reparse history");
        assert_eq!(
            reloaded.task_queue.entries[0].status,
            TaskQueueStatus::Failed,
            "the reclaimed status must survive on disk, not just in memory"
        );

        std::fs::remove_dir_all(&root).ok();
        match previous {
            Some(value) => std::env::set_var("XDG_DATA_HOME", value),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
    }
}

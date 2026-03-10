#![allow(dead_code)]

use crate::models::PackageSource;
use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const HISTORY_FILE: &str = "history.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryOperation {
    Install,
    Remove,
    Update,
    Downgrade,
    Cleanup,
    ExternalInstall,
    ExternalRemove,
    ExternalUpdate,
}

impl HistoryOperation {
    pub fn label(&self) -> &'static str {
        match self {
            HistoryOperation::Install => "Installed",
            HistoryOperation::Remove => "Removed",
            HistoryOperation::Update => "Updated",
            HistoryOperation::Downgrade => "Downgraded",
            HistoryOperation::Cleanup => "Cleaned up",
            HistoryOperation::ExternalInstall => "Installed (external)",
            HistoryOperation::ExternalRemove => "Removed (external)",
            HistoryOperation::ExternalUpdate => "Updated (external)",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            HistoryOperation::Install => "list-add-symbolic",
            HistoryOperation::Remove => "user-trash-symbolic",
            HistoryOperation::Update => "software-update-available-symbolic",
            HistoryOperation::Downgrade => "go-down-symbolic",
            HistoryOperation::Cleanup => "edit-clear-all-symbolic",
            HistoryOperation::ExternalInstall => "terminal-symbolic",
            HistoryOperation::ExternalRemove => "terminal-symbolic",
            HistoryOperation::ExternalUpdate => "terminal-symbolic",
        }
    }

    pub fn undo_label(&self) -> &'static str {
        match self {
            HistoryOperation::Install | HistoryOperation::ExternalInstall => "Uninstall",
            HistoryOperation::Remove | HistoryOperation::ExternalRemove => "Reinstall",
            HistoryOperation::Update | HistoryOperation::ExternalUpdate => "Downgrade",
            HistoryOperation::Downgrade => "Upgrade",
            HistoryOperation::Cleanup => "N/A",
        }
    }

    pub fn is_reversible(&self) -> bool {
        !matches!(self, HistoryOperation::Cleanup)
    }

    pub fn is_external(&self) -> bool {
        matches!(
            self,
            HistoryOperation::ExternalInstall
                | HistoryOperation::ExternalRemove
                | HistoryOperation::ExternalUpdate
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub operation: HistoryOperation,
    pub package_name: String,
    pub package_source: PackageSource,
    pub version_before: Option<String>,
    pub version_after: Option<String>,
    pub timestamp: DateTime<Local>,
    pub size_change: Option<i64>,
    pub undone: bool,
}

impl HistoryEntry {
    pub fn new(
        operation: HistoryOperation,
        package_name: String,
        package_source: PackageSource,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            operation,
            package_name,
            package_source,
            version_before: None,
            version_after: None,
            timestamp: Local::now(),
            size_change: None,
            undone: false,
        }
    }

    pub fn with_versions(mut self, before: Option<String>, after: Option<String>) -> Self {
        self.version_before = before;
        self.version_after = after;
        self
    }

    pub fn with_size_change(mut self, bytes: i64) -> Self {
        self.size_change = Some(bytes);
        self
    }

    pub fn is_reversible(&self) -> bool {
        self.operation.is_reversible() && !self.undone
    }

    pub fn version_display(&self) -> Option<String> {
        match self.operation {
            HistoryOperation::Update
            | HistoryOperation::Downgrade
            | HistoryOperation::ExternalUpdate => {
                if let (Some(before), Some(after)) = (&self.version_before, &self.version_after) {
                    Some(format!("{} → {}", before, after))
                } else {
                    None
                }
            }
            HistoryOperation::Install | HistoryOperation::ExternalInstall => {
                self.version_after.clone()
            }
            HistoryOperation::Remove | HistoryOperation::ExternalRemove => {
                self.version_before.clone()
            }
            HistoryOperation::Cleanup => None,
        }
    }

    pub fn relative_time(&self) -> String {
        let now = Local::now();
        let duration = now.signed_duration_since(self.timestamp);

        if duration.num_minutes() < 1 {
            "Just now".to_string()
        } else if duration.num_minutes() < 60 {
            format!("{} min ago", duration.num_minutes())
        } else if duration.num_hours() < 24 {
            format!("{} hours ago", duration.num_hours())
        } else if duration.num_days() == 1 {
            "Yesterday".to_string()
        } else if duration.num_days() < 7 {
            format!("{} days ago", duration.num_days())
        } else {
            self.timestamp.format("%b %d, %Y").to_string()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationHistory {
    pub entries: Vec<HistoryEntry>,
    #[serde(default)]
    pub max_entries: usize,
    #[serde(default)]
    pub task_queue: TaskQueueState,
}

impl OperationHistory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 500,
            task_queue: TaskQueueState::new(),
        }
    }

    pub fn add(&mut self, entry: HistoryEntry) {
        self.entries.insert(0, entry);
        self.prune();
    }

    pub fn prune(&mut self) {
        if self.entries.len() > self.max_entries {
            self.entries.truncate(self.max_entries);
        }
    }

    pub fn mark_undone(&mut self, entry_id: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == entry_id) {
            entry.undone = true;
        }
    }

    pub fn filter_by_operation(&self, op: HistoryOperation) -> Vec<&HistoryEntry> {
        self.entries.iter().filter(|e| e.operation == op).collect()
    }

    pub fn filter_by_source(&self, source: PackageSource) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.package_source == source)
            .collect()
    }

    pub fn filter_by_date_range(
        &self,
        start: DateTime<Local>,
        end: DateTime<Local>,
    ) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        let query_lower = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.package_name.to_lowercase().contains(&query_lower))
            .collect()
    }

    pub fn group_by_date(&self) -> HashMap<NaiveDate, Vec<&HistoryEntry>> {
        let mut groups: HashMap<NaiveDate, Vec<&HistoryEntry>> = HashMap::new();

        for entry in &self.entries {
            let date = entry.timestamp.date_naive();
            groups.entry(date).or_default().push(entry);
        }

        groups
    }

    pub fn recent(&self, count: usize) -> Vec<&HistoryEntry> {
        self.entries.iter().take(count).collect()
    }

    pub fn reversible_entries(&self) -> Vec<&HistoryEntry> {
        self.entries.iter().filter(|e| e.is_reversible()).collect()
    }

    pub fn today_entries(&self) -> Vec<&HistoryEntry> {
        let today = Local::now().date_naive();
        self.entries
            .iter()
            .filter(|e| e.timestamp.date_naive() == today)
            .collect()
    }

    pub fn stats(&self) -> HistoryStats {
        let mut stats = HistoryStats::default();

        for entry in &self.entries {
            match entry.operation {
                HistoryOperation::Install | HistoryOperation::ExternalInstall => {
                    stats.installs += 1
                }
                HistoryOperation::Remove | HistoryOperation::ExternalRemove => stats.removes += 1,
                HistoryOperation::Update | HistoryOperation::ExternalUpdate => stats.updates += 1,
                HistoryOperation::Downgrade => stats.downgrades += 1,
                HistoryOperation::Cleanup => stats.cleanups += 1,
            }
        }

        stats.total = self.entries.len();
        stats
    }
}

impl Default for OperationHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct HistoryStats {
    pub total: usize,
    pub installs: usize,
    pub removes: usize,
    pub updates: usize,
    pub downgrades: usize,
    pub cleanups: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskQueueAction {
    Install,
    Remove,
    Update,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailureCategory {
    Permissions,
    Network,
    NotFound,
    Conflict,
    Unknown,
}

impl FailureCategory {
    pub fn classify(error_text: &str) -> Self {
        let normalized = error_text.to_ascii_lowercase();
        if normalized.contains("lock")
            || normalized.contains("conflict")
            || normalized.contains("held broken")
            || normalized.contains("another process")
            || normalized.contains("already running")
            || normalized.contains("dependency problem")
            || normalized.contains("dependency problems")
            || normalized.contains("dpkg was interrupted")
        {
            Self::Conflict
        } else if normalized.contains("permission denied")
            || normalized.contains("not permitted")
            || normalized.contains("operation not permitted")
            || normalized.contains("must be root")
            || normalized.contains("not authorized")
            || normalized.contains("authentication")
            || normalized.contains("authorization")
            || normalized.contains("pkexec")
            || normalized.contains("access denied")
            || normalized.contains("eacces")
            || normalized.contains("sudo")
        {
            Self::Permissions
        } else if normalized.contains("timed out")
            || normalized.contains("timeout")
            || normalized.contains("temporary failure")
            || normalized.contains("could not resolve")
            || normalized.contains("name resolution")
            || normalized.contains("network")
            || normalized.contains("connection refused")
            || normalized.contains("connection reset")
            || normalized.contains("unreachable")
            || normalized.contains("failed to fetch")
            || normalized.contains("resolve")
            || normalized.contains("dns")
            || normalized.contains("offline")
        {
            Self::Network
        } else if normalized.contains("not found")
            || normalized.contains("unable to locate")
            || normalized.contains("no package")
            || normalized.contains("no match")
            || normalized.contains("could not find")
            || normalized.contains("no matching")
            || normalized.contains("404")
        {
            Self::NotFound
        } else {
            Self::Unknown
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Permissions => "Permissions",
            Self::Network => "Network",
            Self::NotFound => "Not Found",
            Self::Conflict => "Conflict",
            Self::Unknown => "Unknown",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Permissions => "E_PERMISSION",
            Self::Network => "E_NETWORK",
            Self::NotFound => "E_NOT_FOUND",
            Self::Conflict => "E_CONFLICT",
            Self::Unknown => "E_UNKNOWN",
        }
    }

    pub fn remediation_copy(self) -> &'static str {
        match self {
            Self::Permissions => "Sign in again or approve privilege escalation before retrying.",
            Self::Network => "Reconnect or refresh package metadata before retrying.",
            Self::NotFound => "Verify the package name, version, and source before retrying.",
            Self::Conflict => "Resolve package locks or dependency conflicts before retrying.",
            Self::Unknown => "Review the provider error output, then retry or apply guided fixes.",
        }
    }

    pub fn action_hint(self) -> &'static str {
        match self {
            Self::Permissions => "Use M to re-authenticate, then R to retry. [M] re-authenticate",
            Self::Network => "Use M to refresh metadata, then R to retry. [M] refresh metadata",
            Self::NotFound => {
                "Use M to refresh sources and verify the package, then retry. [M] verify package/source"
            }
            Self::Conflict => {
                "Use M to clear locks or conflicts, then R to retry. [M] resolve lock/conflict"
            }
            Self::Unknown => "Retry with R, then use M if you need guided recovery. [R] retry",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskQueueStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl TaskQueueStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskQueueStatus::Completed | TaskQueueStatus::Failed | TaskQueueStatus::Cancelled
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskQueueEntry {
    pub id: String,
    pub action: TaskQueueAction,
    pub package_id: String,
    pub package_name: String,
    pub package_source: PackageSource,
    pub status: TaskQueueStatus,
    pub queued_at: DateTime<Local>,
    pub started_at: Option<DateTime<Local>>,
    pub completed_at: Option<DateTime<Local>>,
    pub error: Option<String>,
}

impl TaskQueueEntry {
    pub fn new(
        action: TaskQueueAction,
        package_id: String,
        package_name: String,
        package_source: PackageSource,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            action,
            package_id,
            package_name,
            package_source,
            status: TaskQueueStatus::Queued,
            queued_at: Local::now(),
            started_at: None,
            completed_at: None,
            error: None,
        }
    }

    pub fn mark_running(&mut self) {
        self.status = TaskQueueStatus::Running;
        if self.started_at.is_none() {
            self.started_at = Some(Local::now());
        }
    }

    pub fn mark_completed(&mut self) {
        self.status = TaskQueueStatus::Completed;
        self.completed_at = Some(Local::now());
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = TaskQueueStatus::Failed;
        self.completed_at = Some(Local::now());
        self.error = Some(error);
    }

    pub fn mark_cancelled(&mut self) {
        self.status = TaskQueueStatus::Cancelled;
        self.completed_at = Some(Local::now());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskQueueState {
    pub entries: Vec<TaskQueueEntry>,
    #[serde(default)]
    pub max_entries: usize,
}

impl TaskQueueState {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 200,
        }
    }

    pub fn enqueue(&mut self, entry: TaskQueueEntry) {
        self.entries.push(entry);
        self.prune();
    }

    pub fn get_mut(&mut self, entry_id: &str) -> Option<&mut TaskQueueEntry> {
        self.entries.iter_mut().find(|e| e.id == entry_id)
    }

    pub fn prune(&mut self) {
        if self.max_entries == 0 || self.entries.len() <= self.max_entries {
            return;
        }

        let overflow = self.entries.len() - self.max_entries;
        self.entries.drain(0..overflow);
    }

    pub fn retain_active(&mut self) {
        self.entries.retain(|entry| !entry.status.is_terminal());
    }
}

impl Default for TaskQueueState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryFilter {
    All,
    Installs,
    Removes,
    Updates,
    Today,
    ThisWeek,
}

impl HistoryFilter {
    pub fn label(&self) -> &'static str {
        match self {
            HistoryFilter::All => "All",
            HistoryFilter::Installs => "Installs",
            HistoryFilter::Removes => "Removes",
            HistoryFilter::Updates => "Updates",
            HistoryFilter::Today => "Today",
            HistoryFilter::ThisWeek => "This Week",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSnapshot {
    pub packages: HashMap<String, SnapshotEntry>,
    pub timestamp: DateTime<Local>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub name: String,
    pub version: String,
    pub source: PackageSource,
}

impl PackageSnapshot {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
            timestamp: Local::now(),
        }
    }

    pub fn add(&mut self, name: String, version: String, source: PackageSource) {
        let key = format!("{}:{}", source, name);
        self.packages.insert(
            key,
            SnapshotEntry {
                name,
                version,
                source,
            },
        );
    }

    pub fn diff(&self, current: &PackageSnapshot) -> SnapshotDiff {
        let mut diff = SnapshotDiff::default();

        for (key, current_entry) in &current.packages {
            match self.packages.get(key) {
                None => {
                    diff.added.push(current_entry.clone());
                }
                Some(old_entry) => {
                    if old_entry.version != current_entry.version {
                        diff.updated.push(UpdatedEntry {
                            name: current_entry.name.clone(),
                            source: current_entry.source,
                            old_version: old_entry.version.clone(),
                            new_version: current_entry.version.clone(),
                        });
                    }
                }
            }
        }

        for (key, old_entry) in &self.packages {
            if !current.packages.contains_key(key) {
                diff.removed.push(old_entry.clone());
            }
        }

        diff
    }

    pub fn to_history_entries(&self, current: &PackageSnapshot) -> Vec<HistoryEntry> {
        let diff = self.diff(current);
        let mut entries = Vec::new();

        for added in diff.added {
            let mut entry =
                HistoryEntry::new(HistoryOperation::ExternalInstall, added.name, added.source);
            entry.version_after = Some(added.version);
            entries.push(entry);
        }

        for removed in diff.removed {
            let mut entry = HistoryEntry::new(
                HistoryOperation::ExternalRemove,
                removed.name,
                removed.source,
            );
            entry.version_before = Some(removed.version);
            entries.push(entry);
        }

        for updated in diff.updated {
            let entry = HistoryEntry::new(
                HistoryOperation::ExternalUpdate,
                updated.name,
                updated.source,
            )
            .with_versions(Some(updated.old_version), Some(updated.new_version));
            entries.push(entry);
        }

        entries
    }
}

impl Default for PackageSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct SnapshotDiff {
    pub added: Vec<SnapshotEntry>,
    pub removed: Vec<SnapshotEntry>,
    pub updated: Vec<UpdatedEntry>,
}

impl SnapshotDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.updated.is_empty()
    }

    pub fn total_changes(&self) -> usize {
        self.added.len() + self.removed.len() + self.updated.len()
    }
}

#[derive(Debug, Clone)]
pub struct UpdatedEntry {
    pub name: String,
    pub source: PackageSource,
    pub old_version: String,
    pub new_version: String,
}

fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("linget")
}

fn history_path() -> PathBuf {
    data_dir().join(HISTORY_FILE)
}

pub fn load_operation_history() -> Result<OperationHistory> {
    let path = history_path();
    if !path.exists() {
        return Ok(OperationHistory::new());
    }

    let content = fs::read_to_string(&path).context("Failed to read history file")?;
    serde_json::from_str(&content).context("Failed to parse history file")
}

pub fn save_operation_history(history: &OperationHistory) -> Result<()> {
    let path = history_path();

    if let Some(dir) = path.parent() {
        if !dir.exists() {
            fs::create_dir_all(dir).context("Failed to create data directory")?;
        }
    }

    let content = serde_json::to_string_pretty(history).context("Failed to serialize history")?;
    fs::write(&path, content).context("Failed to write history file")
}

#[cfg(test)]
mod tests {
    use super::FailureCategory;

    #[test]
    fn classifies_permission_failures() {
        assert_eq!(
            FailureCategory::classify("permission denied while invoking pkexec"),
            FailureCategory::Permissions
        );
    }

    #[test]
    fn classifies_network_failures() {
        assert_eq!(
            FailureCategory::classify("failed to fetch metadata: connection refused"),
            FailureCategory::Network
        );
    }

    #[test]
    fn classifies_missing_package_failures() {
        assert_eq!(
            FailureCategory::classify("unable to locate package missing-pkg"),
            FailureCategory::NotFound
        );
    }

    #[test]
    fn classifies_conflict_failures() {
        assert_eq!(
            FailureCategory::classify("dpkg was interrupted and another process holds the lock"),
            FailureCategory::Conflict
        );
    }
}

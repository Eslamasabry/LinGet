use crate::models::{PackageSource, PackageStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

/// Type of package operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    Install,
    Remove,
    Update,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::Install => write!(f, "Install"),
            OperationType::Remove => write!(f, "Remove"),
            OperationType::Update => write!(f, "Update"),
        }
    }
}

/// Record of a single package operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRecord {
    /// When the operation was performed
    pub timestamp: DateTime<Utc>,
    /// Type of operation
    pub operation: OperationType,
    /// Package name
    pub package_name: String,
    /// Package version before operation (for update/remove)
    pub old_version: Option<String>,
    /// Package version after operation (for install/update)
    pub new_version: Option<String>,
    /// Package source
    pub source: PackageSource,
    /// Package status before operation
    pub old_status: PackageStatus,
    /// Whether the operation succeeded
    pub success: bool,
}

impl OperationRecord {
    /// Create a new operation record
    pub fn new(
        operation: OperationType,
        package_name: String,
        source: PackageSource,
        old_status: PackageStatus,
        old_version: Option<String>,
        new_version: Option<String>,
        success: bool,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            operation,
            package_name,
            old_version,
            new_version,
            source,
            old_status,
            success,
        }
    }

    /// Get the reverse operation type for undo
    pub fn reverse_operation(&self) -> Option<OperationType> {
        match self.operation {
            OperationType::Install => Some(OperationType::Remove),
            OperationType::Remove => Some(OperationType::Install),
            OperationType::Update => None, // Can't easily undo an update
        }
    }

    /// Check if this operation can be undone
    pub fn can_undo(&self) -> bool {
        self.success
            && matches!(
                self.operation,
                OperationType::Install | OperationType::Remove
            )
    }

    /// Human-readable description
    pub fn description(&self) -> String {
        match self.operation {
            OperationType::Install => format!("Installed {}", self.package_name),
            OperationType::Remove => format!("Removed {}", self.package_name),
            OperationType::Update => {
                if let (Some(old), Some(new)) = (&self.old_version, &self.new_version) {
                    format!("Updated {} from {} to {}", self.package_name, old, new)
                } else {
                    format!("Updated {}", self.package_name)
                }
            }
        }
    }
}

/// Operation history with a maximum size
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OperationHistory {
    /// Recent operations (newest first)
    pub records: VecDeque<OperationRecord>,
    /// Maximum number of records to keep
    #[serde(default = "default_max_records")]
    pub max_records: usize,
}

fn default_max_records() -> usize {
    50
}

impl OperationHistory {
    /// Create a new empty history
    pub fn new() -> Self {
        Self {
            records: VecDeque::new(),
            max_records: default_max_records(),
        }
    }

    /// Add an operation record
    pub fn push(&mut self, record: OperationRecord) {
        self.records.push_front(record);
        while self.records.len() > self.max_records {
            self.records.pop_back();
        }
    }

    /// Get the most recent undoable operation
    pub fn last_undoable(&self) -> Option<&OperationRecord> {
        self.records.iter().find(|r| r.can_undo())
    }

    /// Remove and return the most recent undoable operation
    pub fn pop_undoable(&mut self) -> Option<OperationRecord> {
        let idx = self.records.iter().position(|r| r.can_undo())?;
        self.records.remove(idx)
    }

    /// Get history file path
    fn history_path() -> Option<PathBuf> {
        let config_dir = dirs::config_dir()?;
        Some(config_dir.join("linget").join("history.json"))
    }

    /// Load history from disk
    pub fn load() -> Self {
        let Some(path) = Self::history_path() else {
            return Self::new();
        };

        if !path.exists() {
            return Self::new();
        }

        match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::new(),
        }
    }

    /// Save history to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let Some(path) = Self::history_path() else {
            anyhow::bail!("Could not determine config directory");
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }
}

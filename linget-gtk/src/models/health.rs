#![allow(dead_code)]

use crate::models::PackageSource;
use std::collections::HashMap;

const ONE_GB: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct SystemHealth {
    pub score: u8,
    pub issues: Vec<HealthIssue>,
    pub stats: HealthStats,
}

#[derive(Debug, Clone, Default)]
pub struct HealthStats {
    pub pending_updates: usize,
    pub security_updates: usize,
    pub orphaned_packages: usize,
    pub recoverable_space: u64,
    pub broken_deps: usize,
}

#[derive(Debug, Clone)]
pub enum HealthIssue {
    SecurityUpdates {
        count: usize,
    },
    PendingUpdates {
        count: usize,
    },
    RecoverableSpace {
        bytes: u64,
    },
    OrphanedPackages {
        count: usize,
        source: PackageSource,
    },
    BrokenDependencies {
        count: usize,
    },
    UnreachableRepo {
        name: String,
    },
    PackageManagerLocked {
        source: PackageSource,
        holder: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    Critical,
    Warning,
    Info,
}

impl HealthIssue {
    pub fn severity(&self) -> IssueSeverity {
        match self {
            HealthIssue::SecurityUpdates { .. } => IssueSeverity::Critical,
            HealthIssue::BrokenDependencies { .. } => IssueSeverity::Critical,
            HealthIssue::UnreachableRepo { .. } => IssueSeverity::Critical,
            HealthIssue::PackageManagerLocked { .. } => IssueSeverity::Critical,
            HealthIssue::PendingUpdates { .. } => IssueSeverity::Warning,
            HealthIssue::OrphanedPackages { .. } => IssueSeverity::Warning,
            HealthIssue::RecoverableSpace { .. } => IssueSeverity::Info,
        }
    }

    pub fn title(&self) -> String {
        match self {
            HealthIssue::SecurityUpdates { count } => {
                format!(
                    "{} security update{} available",
                    count,
                    if *count == 1 { "" } else { "s" }
                )
            }
            HealthIssue::PendingUpdates { count } => {
                format!(
                    "{} pending update{}",
                    count,
                    if *count == 1 { "" } else { "s" }
                )
            }
            HealthIssue::RecoverableSpace { bytes } => {
                let size = humansize::format_size(*bytes, humansize::BINARY);
                format!("{} of recoverable space", size)
            }
            HealthIssue::OrphanedPackages { count, source } => {
                format!(
                    "{} orphaned package{} in {}",
                    count,
                    if *count == 1 { "" } else { "s" },
                    source
                )
            }
            HealthIssue::BrokenDependencies { count } => {
                format!(
                    "{} broken dependenc{}",
                    count,
                    if *count == 1 { "y" } else { "ies" }
                )
            }
            HealthIssue::UnreachableRepo { name } => {
                format!("Repository '{}' is unreachable", name)
            }
            HealthIssue::PackageManagerLocked { source, holder } => match holder {
                Some(process) => format!("{} is locked by '{}'", source, process),
                None => format!("{} is locked by another process", source),
            },
        }
    }

    pub fn action_label(&self) -> &'static str {
        match self {
            HealthIssue::SecurityUpdates { .. } => "Update Now",
            HealthIssue::PendingUpdates { .. } => "View Updates",
            HealthIssue::RecoverableSpace { .. } => "Clean Up",
            HealthIssue::OrphanedPackages { .. } => "Remove",
            HealthIssue::BrokenDependencies { .. } => "Repair",
            HealthIssue::UnreachableRepo { .. } => "Check Settings",
            HealthIssue::PackageManagerLocked { .. } => "View Process",
        }
    }
}

impl SystemHealth {
    pub fn compute(
        pending_updates: usize,
        security_updates: usize,
        orphaned_packages: HashMap<PackageSource, usize>,
        recoverable_space: u64,
    ) -> Self {
        let mut score: i32 = 100;
        let mut issues = Vec::new();

        if security_updates > 0 {
            score -= 20;
            issues.push(HealthIssue::SecurityUpdates {
                count: security_updates,
            });
        }

        if pending_updates > 0 {
            let penalty = ((pending_updates / 10) * 5).min(20) as i32;
            score -= penalty;
            issues.push(HealthIssue::PendingUpdates {
                count: pending_updates,
            });
        }

        if recoverable_space > ONE_GB {
            score -= 10;
            issues.push(HealthIssue::RecoverableSpace {
                bytes: recoverable_space,
            });
        }

        let mut total_orphaned = 0usize;
        for (source, count) in &orphaned_packages {
            if *count > 0 {
                score -= 5;
                total_orphaned += count;
                issues.push(HealthIssue::OrphanedPackages {
                    count: *count,
                    source: *source,
                });
            }
        }

        issues.sort_by_key(|i| match i.severity() {
            IssueSeverity::Critical => 0,
            IssueSeverity::Warning => 1,
            IssueSeverity::Info => 2,
        });

        let final_score = score.max(0) as u8;

        let stats = HealthStats {
            pending_updates,
            security_updates,
            orphaned_packages: total_orphaned,
            recoverable_space,
            broken_deps: 0,
        };

        Self {
            score: final_score,
            issues,
            stats,
        }
    }
}

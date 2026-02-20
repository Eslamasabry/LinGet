use crate::models::history::{TaskQueueEntry, TaskQueueStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureCategory {
    Permissions,
    Network,
    NotFound,
    Conflict,
    Unknown,
}

impl FailureCategory {
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
            Self::Permissions => "Authentication or privilege escalation was rejected.",
            Self::Network => "Network issue detected while contacting package source.",
            Self::NotFound => "Package or version was not found in enabled repositories.",
            Self::Conflict => "Package manager reported lock/dependency conflicts.",
            Self::Unknown => "No specific category matched this failure.",
        }
    }

    pub fn action_hint(self) -> &'static str {
        match self {
            Self::Permissions => "[M] re-authenticate, then [R] retry",
            Self::Network => "[M] refresh metadata, then [R] retry",
            Self::NotFound => "[M] refresh and verify package/source before retry",
            Self::Conflict => "[M] resolve lock/conflict, then [R] retry",
            Self::Unknown => "[R] retry, then [M] inspect remediation guidance",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RecoveryState {
    pub attempts: usize,
    pub last_outcome: Option<TaskQueueStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueJourneyLane {
    Now,
    Next,
    NeedsAttention,
    Done,
}

impl QueueJourneyLane {
    pub fn label(self) -> &'static str {
        match self {
            Self::Now => "now",
            Self::Next => "next",
            Self::NeedsAttention => "needs attention",
            Self::Done => "done",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueFailureFilter {
    All,
    Permissions,
    Network,
    Conflict,
    Other,
}

impl QueueFailureFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Permissions => "permissions",
            Self::Network => "network",
            Self::Conflict => "conflict",
            Self::Other => "other",
        }
    }

    pub fn matches(self, category: FailureCategory) -> bool {
        match self {
            Self::All => true,
            Self::Permissions => category == FailureCategory::Permissions,
            Self::Network => category == FailureCategory::Network,
            Self::Conflict => category == FailureCategory::Conflict,
            Self::Other => matches!(
                category,
                FailureCategory::NotFound | FailureCategory::Unknown
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueClinicActionability {
    pub failed_in_scope: usize,
    pub safe_retry_count: usize,
    pub remediation_retry_count: usize,
    pub remediation_guidance_count: usize,
    pub remediation_skipped_count: usize,
}

impl QueueClinicActionability {
    pub fn remediation_actionable_count(self) -> usize {
        self.remediation_retry_count + self.remediation_guidance_count
    }
}

#[derive(Debug, Clone)]
pub struct ClinicRemediationPlan {
    pub retries: Vec<TaskQueueEntry>,
    pub guidance_only: usize,
    pub skipped: usize,
    pub preview_count: usize,
}

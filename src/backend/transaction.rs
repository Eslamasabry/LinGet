use super::PackageManager;
use crate::models::{Package, PackageSource, PackageStatus};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::Mutex;
use uuid::Uuid;

pub const TRANSACTION_SCHEMA_VERSION: u16 = 1;
pub const PLAN_TTL_SECONDS: i64 = 120;
pub const STABLE_PROVIDERS: [PackageSource; 3] = [
    PackageSource::Apt,
    PackageSource::Flatpak,
    PackageSource::Npm,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationAction {
    Install,
    Remove,
    Update,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageRef {
    pub name: String,
    pub source: PackageSource,
    pub installed_version: Option<String>,
    pub available_version: Option<String>,
}

impl PackageRef {
    pub fn from_package(package: &Package) -> Self {
        let installed_version = matches!(
            package.status,
            PackageStatus::Installed | PackageStatus::UpdateAvailable | PackageStatus::Updating
        )
        .then(|| package.version.clone());
        Self {
            name: package.name.clone(),
            source: package.source,
            installed_version,
            available_version: package.available_version.clone(),
        }
    }

    fn as_package(&self, action: OperationAction) -> Package {
        Package {
            name: self.name.clone(),
            version: self.installed_version.clone().unwrap_or_default(),
            available_version: self.available_version.clone(),
            description: String::new(),
            source: self.source,
            status: match action {
                OperationAction::Install => PackageStatus::NotInstalled,
                OperationAction::Remove => PackageStatus::Installed,
                OperationAction::Update => PackageStatus::UpdateAvailable,
            },
            size: None,
            homepage: None,
            license: None,
            maintainer: None,
            dependencies: Vec::new(),
            install_date: None,
            update_category: None,
            enrichment: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestedBy {
    Tui,
    Cli,
    Scheduler,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationRequest {
    pub id: String,
    pub action: OperationAction,
    pub targets: Vec<PackageRef>,
    pub requested_by: RequestedBy,
}

impl OperationRequest {
    pub fn new(
        action: OperationAction,
        targets: Vec<PackageRef>,
        requested_by: RequestedBy,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            action,
            targets,
            requested_by,
        }
    }

    pub fn source(&self) -> Result<PackageSource, ProviderError> {
        let source = self
            .targets
            .first()
            .map(|target| target.source)
            .ok_or_else(|| ProviderError::protocol(None, "No packages were selected"))?;
        if self.targets.iter().any(|target| target.source != source) {
            return Err(ProviderError::protocol(
                None,
                "A provider plan cannot mix package sources",
            ));
        }
        Ok(source)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub privileged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageChange {
    pub name: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderTier {
    Stable,
    Beta,
    DetectionOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanFidelity {
    Exact,
    BestEffort,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivilegeRequirement {
    None,
    MayRequire,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CancellationSupport {
    Cooperative,
    BetweenStepsOnly,
    NotSafe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RollbackCapability {
    Unsupported,
    ProviderManaged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderDescriptor {
    pub source: PackageSource,
    pub tier: ProviderTier,
    pub fidelity: PlanFidelity,
    pub privilege: PrivilegeRequirement,
    pub cancellation: CancellationSupport,
    pub rollback: RollbackCapability,
}

impl ProviderDescriptor {
    pub fn for_source(source: PackageSource) -> Self {
        match source {
            PackageSource::Apt => Self {
                source,
                tier: ProviderTier::Stable,
                fidelity: PlanFidelity::Exact,
                privilege: PrivilegeRequirement::Required,
                cancellation: CancellationSupport::BetweenStepsOnly,
                rollback: RollbackCapability::Unsupported,
            },
            PackageSource::Flatpak | PackageSource::Npm => Self {
                source,
                tier: ProviderTier::Stable,
                fidelity: PlanFidelity::BestEffort,
                privilege: PrivilegeRequirement::MayRequire,
                cancellation: CancellationSupport::BetweenStepsOnly,
                rollback: RollbackCapability::Unsupported,
            },
            PackageSource::Aur | PackageSource::AppImage => Self {
                source,
                tier: ProviderTier::DetectionOnly,
                fidelity: PlanFidelity::Unsupported,
                privilege: PrivilegeRequirement::MayRequire,
                cancellation: CancellationSupport::NotSafe,
                rollback: RollbackCapability::Unsupported,
            },
            _ => Self {
                source,
                tier: ProviderTier::Beta,
                fidelity: PlanFidelity::BestEffort,
                privilege: PrivilegeRequirement::MayRequire,
                cancellation: CancellationSupport::BetweenStepsOnly,
                rollback: RollbackCapability::Unsupported,
            },
        }
    }

    pub fn stable() -> Vec<Self> {
        STABLE_PROVIDERS.into_iter().map(Self::for_source).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderPlan {
    pub id: String,
    pub operation_id: String,
    pub provider: ProviderDescriptor,
    pub action: OperationAction,
    pub targets: Vec<PackageRef>,
    pub exact_commands: Vec<CommandSpec>,
    pub expected_changes: Vec<PackageChange>,
    pub inventory_fingerprint: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl ProviderPlan {
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Caution,
    High,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskReason {
    RemovesPackages,
    RequiresPrivilege,
    BestEffortPlan,
    ProviderNotStable,
    UnsupportedProvider,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub level: RiskLevel,
    pub reasons: Vec<RiskReason>,
    pub requires_explicit_confirmation: bool,
}

impl RiskAssessment {
    pub fn for_plan(plan: &ProviderPlan) -> Self {
        let mut reasons = Vec::new();
        if plan.action == OperationAction::Remove {
            reasons.push(RiskReason::RemovesPackages);
        }
        if plan.provider.privilege == PrivilegeRequirement::Required {
            reasons.push(RiskReason::RequiresPrivilege);
        }
        if plan.provider.fidelity == PlanFidelity::BestEffort {
            reasons.push(RiskReason::BestEffortPlan);
        }
        if plan.provider.tier == ProviderTier::Beta {
            reasons.push(RiskReason::ProviderNotStable);
        }
        if plan.provider.fidelity == PlanFidelity::Unsupported {
            reasons.push(RiskReason::UnsupportedProvider);
        }

        let level = if reasons.contains(&RiskReason::UnsupportedProvider) {
            RiskLevel::Blocked
        } else if reasons.contains(&RiskReason::RemovesPackages) {
            RiskLevel::High
        } else if reasons.is_empty() {
            RiskLevel::Low
        } else {
            RiskLevel::Caution
        };
        Self {
            level,
            reasons,
            requires_explicit_confirmation: matches!(level, RiskLevel::High | RiskLevel::Blocked),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationOutcome {
    Verified,
    Mismatch,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReceipt {
    pub operation_id: String,
    pub plan_id: String,
    pub provider: PackageSource,
    pub expected: Vec<PackageChange>,
    pub observed: Vec<PackageChange>,
    pub outcome: VerificationOutcome,
    pub warnings: Vec<String>,
    pub verified_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderErrorCode {
    AuthorizationCancelled,
    AuthorizationDenied,
    NoPrivilegeAgent,
    LockBusy,
    DependencyConflict,
    Network,
    NotFound,
    RepositoryUnavailable,
    SignatureInvalid,
    DiskFull,
    Timeout,
    Interrupted,
    PlanExpired,
    PlanChanged,
    VerificationMismatch,
    Protocol,
    Persistence,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderError {
    pub code: ProviderErrorCode,
    pub provider: Option<PackageSource>,
    pub safe_message: String,
    pub diagnostic: String,
    pub retryable: bool,
    pub recovery_actions: Vec<String>,
}

impl ProviderError {
    pub fn protocol(provider: Option<PackageSource>, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            code: ProviderErrorCode::Protocol,
            provider,
            safe_message: message.clone(),
            diagnostic: message,
            retryable: false,
            recovery_actions: vec!["Review the selected provider and packages".to_string()],
        }
    }

    pub fn classify(provider: PackageSource, diagnostic: impl Into<String>) -> Self {
        let diagnostic = diagnostic.into();
        let lower = diagnostic.to_ascii_lowercase();
        let (code, safe_message, retryable, recovery) =
            if lower.contains("cancel") && (lower.contains("auth") || lower.contains("pkexec")) {
                (
                    ProviderErrorCode::AuthorizationCancelled,
                    "Authorization was cancelled",
                    true,
                    "Approve the authorization prompt and retry",
                )
            } else if lower.contains("no authentication agent")
                || lower.contains("no polkit agent")
                || lower.contains("cannot open display")
            {
                (
                    ProviderErrorCode::NoPrivilegeAgent,
                    "No authorization agent is available",
                    true,
                    "Start an authorization agent or use an interactive session",
                )
            } else if lower.contains("permission")
                || lower.contains("not authorized")
                || lower.contains("eacces")
            {
                (
                    ProviderErrorCode::AuthorizationDenied,
                    "The package manager denied authorization",
                    true,
                    "Check your privileges and retry",
                )
            } else if lower.contains("lock") || lower.contains("another process") {
                (
                    ProviderErrorCode::LockBusy,
                    "The package manager is busy",
                    true,
                    "Wait for the other package operation to finish",
                )
            } else if lower.contains("dependency") || lower.contains("conflict") {
                (
                    ProviderErrorCode::DependencyConflict,
                    "Package dependencies conflict",
                    false,
                    "Inspect the provider output before changing dependencies",
                )
            } else if lower.contains("signature")
                && (lower.contains("invalid") || lower.contains("not verified"))
            {
                (
                    ProviderErrorCode::SignatureInvalid,
                    "The repository signature could not be verified",
                    false,
                    "Refresh trusted repository keys before retrying",
                )
            } else if lower.contains("repository")
                && (lower.contains("unavailable") || lower.contains("no release file"))
            {
                (
                    ProviderErrorCode::RepositoryUnavailable,
                    "The configured repository is unavailable",
                    true,
                    "Check the repository configuration and refresh metadata",
                )
            } else if lower.contains("timeout") || lower.contains("timed out") {
                (
                    ProviderErrorCode::Timeout,
                    "The provider operation timed out",
                    true,
                    "Check the network and retry",
                )
            } else if lower.contains("interrupted") || lower.contains("terminated by signal") {
                (
                    ProviderErrorCode::Interrupted,
                    "The provider operation was interrupted",
                    true,
                    "Refresh package state before retrying",
                )
            } else if lower.contains("network") || lower.contains("resolve") {
                (
                    ProviderErrorCode::Network,
                    "The provider could not reach its repository",
                    true,
                    "Check the network and refresh provider metadata",
                )
            } else if lower.contains("not found") || lower.contains("404") {
                (
                    ProviderErrorCode::NotFound,
                    "The package was not found",
                    false,
                    "Verify the package name and provider",
                )
            } else if lower.contains("no space") || lower.contains("disk full") {
                (
                    ProviderErrorCode::DiskFull,
                    "There is not enough disk space",
                    true,
                    "Free disk space and retry",
                )
            } else {
                (
                    ProviderErrorCode::Unknown,
                    "The provider operation failed",
                    true,
                    "Review the provider output and retry",
                )
            };
        Self {
            code,
            provider: Some(provider),
            safe_message: safe_message.to_string(),
            diagnostic,
            retryable,
            recovery_actions: vec![recovery.to_string()],
        }
    }

    fn persistence(diagnostic: impl Into<String>) -> Self {
        let diagnostic = diagnostic.into();
        Self {
            code: ProviderErrorCode::Persistence,
            provider: None,
            safe_message: "LinGet could not save the transaction record".to_string(),
            diagnostic,
            retryable: true,
            recovery_actions: vec!["Check the LinGet data directory permissions".to_string()],
        }
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.safe_message, self.diagnostic)
    }
}

impl std::error::Error for ProviderError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationState {
    Planning,
    Ready,
    NeedsReview,
    Running,
    Verifying,
    Succeeded,
    Partial,
    Failed,
    Cancelled,
    Interrupted,
}

impl OperationState {
    pub fn can_transition_to(self, next: Self) -> bool {
        use OperationState::*;
        matches!(
            (self, next),
            (Planning, Ready | Failed | Cancelled)
                | (Ready, NeedsReview | Running | Cancelled)
                | (NeedsReview, Ready | Cancelled)
                | (
                    Running,
                    Verifying | Partial | Failed | Cancelled | Interrupted
                )
                | (Verifying, Succeeded | Partial | Failed)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRecord {
    pub operation_id: String,
    pub state: OperationState,
    pub plan: ProviderPlan,
    pub risk: RiskAssessment,
    pub receipt: Option<VerificationReceipt>,
    pub error: Option<ProviderError>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStore {
    pub schema_version: u16,
    pub operations: Vec<OperationRecord>,
}

impl Default for TransactionStore {
    fn default() -> Self {
        Self {
            schema_version: TRANSACTION_SCHEMA_VERSION,
            operations: Vec::new(),
        }
    }
}

impl TransactionStore {
    pub async fn load(path: &Path) -> Result<Self, ProviderError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = fs::read(path)
            .await
            .map_err(|error| ProviderError::persistence(error.to_string()))?;
        if let Ok(store) = serde_json::from_slice::<Self>(&bytes) {
            if store.schema_version == TRANSACTION_SCHEMA_VERSION {
                return Ok(store);
            }
            return Err(ProviderError::persistence(format!(
                "Unsupported transaction schema version {}",
                store.schema_version
            )));
        }

        let operations = serde_json::from_slice::<Vec<OperationRecord>>(&bytes)
            .map_err(|error| ProviderError::persistence(error.to_string()))?;
        let backup = path.with_extension("legacy.bak");
        fs::copy(path, &backup)
            .await
            .map_err(|error| ProviderError::persistence(error.to_string()))?;
        Ok(Self {
            schema_version: TRANSACTION_SCHEMA_VERSION,
            operations,
        })
    }

    pub async fn save_atomic(&self, path: &Path) -> Result<(), ProviderError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|error| ProviderError::persistence(error.to_string()))?;
        }
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| ProviderError::persistence(error.to_string()))?;
        let temp_path = path.with_extension(format!("tmp-{}", Uuid::new_v4()));
        fs::write(&temp_path, bytes)
            .await
            .map_err(|error| ProviderError::persistence(error.to_string()))?;
        fs::rename(&temp_path, path)
            .await
            .map_err(|error| ProviderError::persistence(error.to_string()))
    }
}

#[derive(Clone, Default)]
pub struct CancellationFlag(Arc<AtomicBool>);

impl CancellationFlag {
    pub fn request(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_requested(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub struct TransactionEngine {
    package_manager: Arc<Mutex<PackageManager>>,
    store: Arc<Mutex<TransactionStore>>,
    store_path: PathBuf,
    execution_lock: Mutex<()>,
}

impl TransactionEngine {
    pub async fn load(
        package_manager: Arc<Mutex<PackageManager>>,
        store_path: PathBuf,
    ) -> Result<Self, ProviderError> {
        let mut store = TransactionStore::load(&store_path).await?;
        for record in &mut store.operations {
            if record.state == OperationState::Running || record.state == OperationState::Verifying
            {
                record.state = OperationState::Interrupted;
                record.updated_at = Utc::now();
            } else if record.state == OperationState::Ready {
                record.state = OperationState::NeedsReview;
                record.updated_at = Utc::now();
            }
        }
        store.save_atomic(&store_path).await?;
        Ok(Self {
            package_manager,
            store: Arc::new(Mutex::new(store)),
            store_path,
            execution_lock: Mutex::new(()),
        })
    }

    pub async fn plan(
        &self,
        request: OperationRequest,
    ) -> Result<(ProviderPlan, RiskAssessment), ProviderError> {
        let source = request.source()?;
        validate_targets(&request.targets, source)?;
        let inventory = self.inventory(source).await?;
        let apt_changes = if source == PackageSource::Apt {
            Some(probe_apt_changes(request.action, &request.targets).await?)
        } else {
            None
        };
        let plan = build_plan(&request, &inventory, apt_changes);
        let risk = RiskAssessment::for_plan(&plan);
        let record = OperationRecord {
            operation_id: request.id,
            state: if risk.level == RiskLevel::Blocked {
                OperationState::Failed
            } else {
                OperationState::Ready
            },
            plan: plan.clone(),
            risk: risk.clone(),
            receipt: None,
            error: None,
            updated_at: Utc::now(),
        };
        self.upsert_record(record).await?;
        Ok((plan, risk))
    }

    pub async fn execute(
        &self,
        plan: ProviderPlan,
        cancellation: CancellationFlag,
    ) -> Result<VerificationReceipt, ProviderError> {
        let _execution_guard = self.execution_lock.lock().await;
        if plan.is_expired(Utc::now()) {
            self.mark_error(
                &plan.operation_id,
                OperationState::NeedsReview,
                ProviderError {
                    code: ProviderErrorCode::PlanExpired,
                    provider: Some(plan.provider.source),
                    safe_message: "The reviewed plan expired".to_string(),
                    diagnostic: "Provider plans are valid for two minutes".to_string(),
                    retryable: true,
                    recovery_actions: vec!["Review a fresh plan".to_string()],
                },
            )
            .await?;
            return Err(ProviderError {
                code: ProviderErrorCode::PlanExpired,
                provider: Some(plan.provider.source),
                safe_message: "The reviewed plan expired".to_string(),
                diagnostic: "Provider plans are valid for two minutes".to_string(),
                retryable: true,
                recovery_actions: vec!["Review a fresh plan".to_string()],
            });
        }

        let current_inventory = self.inventory(plan.provider.source).await?;
        let current_fingerprint = inventory_fingerprint(&current_inventory);
        if current_fingerprint != plan.inventory_fingerprint {
            let error = ProviderError {
                code: ProviderErrorCode::PlanChanged,
                provider: Some(plan.provider.source),
                safe_message: "Package state changed after review".to_string(),
                diagnostic: "The provider inventory fingerprint no longer matches".to_string(),
                retryable: true,
                recovery_actions: vec!["Review a fresh plan".to_string()],
            };
            self.mark_error(
                &plan.operation_id,
                OperationState::NeedsReview,
                error.clone(),
            )
            .await?;
            return Err(error);
        }
        if cancellation.is_requested() {
            let error = ProviderError {
                code: ProviderErrorCode::Interrupted,
                provider: Some(plan.provider.source),
                safe_message: "The operation was cancelled before execution".to_string(),
                diagnostic: "Cancellation was requested while the plan was queued".to_string(),
                retryable: true,
                recovery_actions: vec!["Review and queue the plan again".to_string()],
            };
            self.mark_error(&plan.operation_id, OperationState::Cancelled, error.clone())
                .await?;
            return Err(error);
        }

        self.mark_state(&plan.operation_id, OperationState::Running)
            .await?;
        for (completed, target) in plan.targets.iter().enumerate() {
            if cancellation.is_requested() && completed > 0 {
                let error = ProviderError {
                    code: ProviderErrorCode::Interrupted,
                    provider: Some(plan.provider.source),
                    safe_message: "The operation stopped between package steps".to_string(),
                    diagnostic: format!("{} package step(s) completed", completed),
                    retryable: true,
                    recovery_actions: vec!["Refresh and review the remaining work".to_string()],
                };
                self.mark_error(&plan.operation_id, OperationState::Partial, error.clone())
                    .await?;
                return Err(error);
            }
            let result = {
                let manager = self.package_manager.lock().await;
                let package = target.as_package(plan.action);
                match plan.action {
                    OperationAction::Install => manager.install(&package).await,
                    OperationAction::Remove => manager.remove(&package).await,
                    OperationAction::Update => manager.update(&package).await,
                }
            };
            if let Err(error) = result {
                let provider_error =
                    ProviderError::classify(plan.provider.source, error.to_string());
                let state = if completed == 0 {
                    OperationState::Failed
                } else {
                    OperationState::Partial
                };
                self.mark_error(&plan.operation_id, state, provider_error.clone())
                    .await?;
                return Err(provider_error);
            }
        }

        self.mark_state(&plan.operation_id, OperationState::Verifying)
            .await?;
        let receipt = self.verify(&plan).await?;
        let final_state = match receipt.outcome {
            VerificationOutcome::Verified => OperationState::Succeeded,
            VerificationOutcome::Mismatch => OperationState::Failed,
            VerificationOutcome::Inconclusive => OperationState::Partial,
        };
        self.mark_receipt(&plan.operation_id, final_state, receipt.clone())
            .await?;
        Ok(receipt)
    }

    pub async fn records(&self) -> Vec<OperationRecord> {
        self.store.lock().await.operations.clone()
    }

    async fn inventory(&self, source: PackageSource) -> Result<Vec<Package>, ProviderError> {
        let manager = self.package_manager.lock().await;
        manager
            .list_installed_for_source(source)
            .await
            .map_err(|error| ProviderError::classify(source, error.to_string()))
    }

    async fn verify(&self, plan: &ProviderPlan) -> Result<VerificationReceipt, ProviderError> {
        let inventory = self.inventory(plan.provider.source).await?;
        Ok(verify_inventory(plan, &inventory, Utc::now()))
    }

    async fn upsert_record(&self, record: OperationRecord) -> Result<(), ProviderError> {
        let mut store = self.store.lock().await;
        if let Some(existing) = store
            .operations
            .iter_mut()
            .find(|entry| entry.operation_id == record.operation_id)
        {
            *existing = record;
        } else {
            store.operations.push(record);
        }
        store.save_atomic(&self.store_path).await
    }

    async fn mark_state(
        &self,
        operation_id: &str,
        next: OperationState,
    ) -> Result<(), ProviderError> {
        let mut store = self.store.lock().await;
        let record = store
            .operations
            .iter_mut()
            .find(|record| record.operation_id == operation_id)
            .ok_or_else(|| ProviderError::protocol(None, "Transaction record is missing"))?;
        if !record.state.can_transition_to(next) {
            return Err(ProviderError::protocol(
                Some(record.plan.provider.source),
                format!(
                    "Invalid transaction transition {:?} -> {:?}",
                    record.state, next
                ),
            ));
        }
        record.state = next;
        record.updated_at = Utc::now();
        store.save_atomic(&self.store_path).await
    }

    async fn mark_error(
        &self,
        operation_id: &str,
        next: OperationState,
        error: ProviderError,
    ) -> Result<(), ProviderError> {
        let mut store = self.store.lock().await;
        let record = store
            .operations
            .iter_mut()
            .find(|record| record.operation_id == operation_id)
            .ok_or_else(|| ProviderError::protocol(None, "Transaction record is missing"))?;
        record.state = next;
        record.error = Some(error);
        record.updated_at = Utc::now();
        store.save_atomic(&self.store_path).await
    }

    async fn mark_receipt(
        &self,
        operation_id: &str,
        next: OperationState,
        receipt: VerificationReceipt,
    ) -> Result<(), ProviderError> {
        let mut store = self.store.lock().await;
        let record = store
            .operations
            .iter_mut()
            .find(|record| record.operation_id == operation_id)
            .ok_or_else(|| ProviderError::protocol(None, "Transaction record is missing"))?;
        record.state = next;
        record.receipt = Some(receipt);
        record.updated_at = Utc::now();
        store.save_atomic(&self.store_path).await
    }
}

fn verify_inventory(
    plan: &ProviderPlan,
    inventory: &[Package],
    verified_at: DateTime<Utc>,
) -> VerificationReceipt {
    let mut observed = Vec::new();
    let mut matches = 0usize;
    let mut warnings = Vec::new();
    for expected in &plan.expected_changes {
        let installed = inventory
            .iter()
            .find(|package| package.name == expected.name);
        let observed_after = installed.map(|package| package.version.clone());
        let target_matches = match plan.action {
            OperationAction::Install => match (&expected.after, installed) {
                (Some(version), Some(package)) => &package.version == version,
                (None, Some(_)) => true,
                _ => false,
            },
            OperationAction::Remove => installed.is_none(),
            OperationAction::Update => match (&expected.after, installed) {
                (Some(version), Some(package)) => &package.version == version,
                (None, Some(_)) => false,
                _ => false,
            },
        };
        if target_matches {
            matches += 1;
        }
        observed.push(PackageChange {
            name: expected.name.clone(),
            before: expected.before.clone(),
            after: observed_after,
        });
    }
    let outcome = if matches == plan.expected_changes.len() {
        VerificationOutcome::Verified
    } else if plan.action == OperationAction::Update
        && plan
            .expected_changes
            .iter()
            .any(|change| change.after.is_none())
    {
        warnings.push(
            "The provider did not expose a target version, so the update cannot be proven"
                .to_string(),
        );
        VerificationOutcome::Inconclusive
    } else {
        VerificationOutcome::Mismatch
    };
    VerificationReceipt {
        operation_id: plan.operation_id.clone(),
        plan_id: plan.id.clone(),
        provider: plan.provider.source,
        expected: plan.expected_changes.clone(),
        observed,
        outcome,
        warnings,
        verified_at,
    }
}

fn validate_targets(targets: &[PackageRef], source: PackageSource) -> Result<(), ProviderError> {
    for target in targets {
        if target.name.is_empty()
            || target.name.len() > 256
            || target.name.starts_with('-')
            || target
                .name
                .chars()
                .any(|character| character == '\0' || character.is_control())
        {
            return Err(ProviderError::protocol(
                Some(source),
                format!("Unsafe package name: {:?}", target.name),
            ));
        }
    }
    Ok(())
}

fn build_plan(
    request: &OperationRequest,
    inventory: &[Package],
    apt_changes: Option<Vec<PackageChange>>,
) -> ProviderPlan {
    let source = request.targets[0].source;
    let expected_changes = apt_changes.unwrap_or_else(|| {
        request
            .targets
            .iter()
            .map(|target| {
                let installed = inventory.iter().find(|package| package.name == target.name);
                PackageChange {
                    name: target.name.clone(),
                    before: installed.map(|package| package.version.clone()),
                    after: match request.action {
                        OperationAction::Remove => None,
                        OperationAction::Install | OperationAction::Update => {
                            target.available_version.clone()
                        }
                    },
                }
            })
            .collect()
    });
    let exact_commands = request
        .targets
        .iter()
        .map(|target| command_for(source, request.action, &target.name))
        .collect();
    let created_at = Utc::now();
    ProviderPlan {
        id: Uuid::new_v4().to_string(),
        operation_id: request.id.clone(),
        provider: ProviderDescriptor::for_source(source),
        action: request.action,
        targets: request.targets.clone(),
        exact_commands,
        expected_changes,
        inventory_fingerprint: inventory_fingerprint(inventory),
        created_at,
        expires_at: created_at + Duration::seconds(PLAN_TTL_SECONDS),
    }
}

fn command_for(source: PackageSource, action: OperationAction, name: &str) -> CommandSpec {
    match (source, action) {
        (PackageSource::Apt, OperationAction::Install) => CommandSpec {
            program: "pkexec".to_string(),
            args: vec!["apt", "install", "-y", "--", name]
                .into_iter()
                .map(ToString::to_string)
                .collect(),
            privileged: true,
        },
        (PackageSource::Apt, OperationAction::Remove) => CommandSpec {
            program: "pkexec".to_string(),
            args: vec!["apt", "remove", "-y", "--", name]
                .into_iter()
                .map(ToString::to_string)
                .collect(),
            privileged: true,
        },
        (PackageSource::Apt, OperationAction::Update) => CommandSpec {
            program: "pkexec".to_string(),
            args: vec!["apt", "install", "--only-upgrade", "-y", "--", name]
                .into_iter()
                .map(ToString::to_string)
                .collect(),
            privileged: true,
        },
        (PackageSource::Flatpak, OperationAction::Install) => {
            command("flatpak", &["install", "-y", name])
        }
        (PackageSource::Flatpak, OperationAction::Remove) => {
            command("flatpak", &["uninstall", "-y", name])
        }
        (PackageSource::Flatpak, OperationAction::Update) => {
            command("flatpak", &["update", "-y", name])
        }
        (PackageSource::Npm, OperationAction::Install) => command("npm", &["install", "-g", name]),
        (PackageSource::Npm, OperationAction::Remove) => command("npm", &["uninstall", "-g", name]),
        (PackageSource::Npm, OperationAction::Update) => {
            let spec = format!("{}@latest", name);
            command("npm", &["install", "-g", &spec])
        }
        (_, _) => CommandSpec {
            program: source.to_string().to_ascii_lowercase(),
            args: vec![
                format!("{:?}", action).to_ascii_lowercase(),
                name.to_string(),
            ],
            privileged: false,
        },
    }
}

fn command(program: &str, args: &[&str]) -> CommandSpec {
    CommandSpec {
        program: program.to_string(),
        args: args
            .iter()
            .map(|argument| (*argument).to_string())
            .collect(),
        privileged: false,
    }
}

fn inventory_fingerprint(inventory: &[Package]) -> String {
    let mut entries: Vec<_> = inventory
        .iter()
        .map(|package| {
            format!(
                "{}\0{}\0{:?}",
                package.name, package.version, package.status
            )
        })
        .collect();
    entries.sort();
    let mut hash = 0xcbf29ce484222325u64;
    for byte in entries.join("\n").bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{:016x}", hash)
}

async fn probe_apt_changes(
    action: OperationAction,
    targets: &[PackageRef],
) -> Result<Vec<PackageChange>, ProviderError> {
    let mut args = vec!["-s".to_string()];
    match action {
        OperationAction::Install => args.push("install".to_string()),
        OperationAction::Remove => args.push("remove".to_string()),
        OperationAction::Update => {
            args.push("install".to_string());
            args.push("--only-upgrade".to_string());
        }
    }
    args.push("--".to_string());
    args.extend(targets.iter().map(|target| target.name.clone()));
    let output = Command::new("apt-get")
        .args(&args)
        .output()
        .await
        .map_err(|error| ProviderError::classify(PackageSource::Apt, error.to_string()))?;
    if !output.status.success() {
        return Err(ProviderError::classify(
            PackageSource::Apt,
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let changes = parse_apt_simulation(&text);
    if changes.is_empty() {
        return Err(ProviderError::protocol(
            Some(PackageSource::Apt),
            "APT simulation returned no concrete package changes",
        ));
    }
    Ok(changes)
}

pub fn parse_apt_simulation(output: &str) -> Vec<PackageChange> {
    let mut changes = Vec::new();
    for line in output.lines().map(str::trim) {
        let mut parts = line.split_whitespace();
        let Some(kind) = parts.next() else {
            continue;
        };
        let Some(name) = parts.next() else {
            continue;
        };
        match kind {
            "Inst" => {
                let remainder = line.split_once(name).map(|(_, rest)| rest).unwrap_or("");
                let before = remainder
                    .split('[')
                    .nth(1)
                    .and_then(|part| part.split(']').next())
                    .map(str::to_string);
                let after = remainder
                    .split('(')
                    .nth(1)
                    .and_then(|part| part.split_whitespace().next())
                    .map(str::to_string);
                changes.push(PackageChange {
                    name: name.to_string(),
                    before,
                    after,
                });
            }
            "Remv" => {
                let before = line
                    .split('[')
                    .nth(1)
                    .and_then(|part| part.split(']').next())
                    .map(str::to_string);
                changes.push(PackageChange {
                    name: name.to_string(),
                    before,
                    after: None,
                });
            }
            _ => {}
        }
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::PackageBackend;
    use anyhow::Result;
    use async_trait::async_trait;
    use serde_json::Value;
    use std::collections::{HashMap, HashSet};

    struct ContractBackend {
        source: PackageSource,
        inventory: Arc<Mutex<Vec<Package>>>,
    }

    #[async_trait]
    impl PackageBackend for ContractBackend {
        fn is_available() -> bool {
            true
        }

        async fn list_installed(&self) -> Result<Vec<Package>> {
            Ok(self.inventory.lock().await.clone())
        }

        async fn check_updates(&self) -> Result<Vec<Package>> {
            Ok(Vec::new())
        }

        async fn install(&self, name: &str) -> Result<()> {
            let mut inventory = self.inventory.lock().await;
            inventory.retain(|package| package.name != name);
            inventory.push(package(name, self.source, "2.0"));
            Ok(())
        }

        async fn remove(&self, name: &str) -> Result<()> {
            self.inventory
                .lock()
                .await
                .retain(|package| package.name != name);
            Ok(())
        }

        async fn update(&self, name: &str) -> Result<()> {
            self.install(name).await
        }

        async fn search(&self, _query: &str) -> Result<Vec<Package>> {
            Ok(Vec::new())
        }

        fn source(&self) -> PackageSource {
            self.source
        }
    }

    fn package_ref(name: &str, source: PackageSource) -> PackageRef {
        PackageRef {
            name: name.to_string(),
            source,
            installed_version: Some("1.0".to_string()),
            available_version: Some("2.0".to_string()),
        }
    }

    fn package(name: &str, source: PackageSource, version: &str) -> Package {
        Package {
            name: name.to_string(),
            version: version.to_string(),
            available_version: None,
            description: String::new(),
            source,
            status: PackageStatus::Installed,
            size: None,
            homepage: None,
            license: None,
            maintainer: None,
            dependencies: Vec::new(),
            install_date: None,
            update_category: None,
            enrichment: None,
        }
    }

    fn fixture_source(value: &str) -> PackageSource {
        match value {
            "Apt" => PackageSource::Apt,
            "Flatpak" => PackageSource::Flatpak,
            "Npm" => PackageSource::Npm,
            other => panic!("unknown fixture provider {other}"),
        }
    }

    fn fixture_action(value: &str) -> OperationAction {
        match value {
            "Install" => OperationAction::Install,
            "Remove" => OperationAction::Remove,
            "Update" => OperationAction::Update,
            other => panic!("unknown fixture action {other}"),
        }
    }

    #[test]
    fn stable_provider_descriptors_are_truthful() {
        assert_eq!(
            ProviderDescriptor::for_source(PackageSource::Apt).fidelity,
            PlanFidelity::Exact
        );
        for source in [PackageSource::Flatpak, PackageSource::Npm] {
            let descriptor = ProviderDescriptor::for_source(source);
            assert_eq!(descriptor.tier, ProviderTier::Stable);
            assert_eq!(descriptor.fidelity, PlanFidelity::BestEffort);
        }
        assert_eq!(
            ProviderDescriptor::for_source(PackageSource::Aur).tier,
            ProviderTier::DetectionOnly
        );
        assert_eq!(
            ProviderDescriptor::stable()
                .into_iter()
                .map(|descriptor| descriptor.source)
                .collect::<Vec<_>>(),
            STABLE_PROVIDERS
        );
    }

    #[test]
    fn stable_provider_contracts_match_tracked_fixtures() {
        let fixture: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/stable-provider-contracts.json"
        )))
        .expect("valid provider fixture");
        let providers = fixture["providers"].as_array().expect("provider array");
        assert_eq!(providers.len(), STABLE_PROVIDERS.len());

        for provider in providers {
            let source = fixture_source(provider["source"].as_str().expect("source"));
            let descriptor = ProviderDescriptor::for_source(source);
            assert_eq!(descriptor.tier, ProviderTier::Stable);
            assert_eq!(format!("{:?}", descriptor.tier), provider["tier"]);
            assert_eq!(format!("{:?}", descriptor.fidelity), provider["fidelity"]);

            let operations = provider["operations"].as_array().expect("operations");
            assert_eq!(operations.len(), 3);
            for operation in operations {
                let action = fixture_action(operation["action"].as_str().expect("action"));
                let actual = command_for(source, action, "demo");
                let expected_args = operation["args"]
                    .as_array()
                    .expect("args")
                    .iter()
                    .map(|value| value.as_str().expect("arg").to_string())
                    .collect::<Vec<_>>();
                assert_eq!(actual.program, operation["program"]);
                assert_eq!(actual.args, expected_args);
                assert_eq!(actual.privileged, source == PackageSource::Apt);
            }
        }
    }

    #[test]
    fn stable_provider_error_contracts_match_tracked_fixtures() {
        let fixture: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/stable-provider-contracts.json"
        )))
        .expect("valid provider fixture");
        for error in fixture["errors"].as_array().expect("error array") {
            let classified = ProviderError::classify(
                PackageSource::Apt,
                error["diagnostic"].as_str().expect("diagnostic"),
            );
            assert_eq!(format!("{:?}", classified.code), error["code"]);
            assert!(!classified.safe_message.is_empty());
            assert!(!classified.recovery_actions.is_empty());
        }
    }

    #[test]
    fn stable_provider_plan_and_verification_matrix_is_complete() {
        for source in STABLE_PROVIDERS {
            for action in [
                OperationAction::Install,
                OperationAction::Remove,
                OperationAction::Update,
            ] {
                let request = OperationRequest::new(
                    action,
                    vec![package_ref("demo", source)],
                    RequestedBy::Tui,
                );
                let before = [package("demo", source, "1.0")];
                let apt_changes = (source == PackageSource::Apt).then(|| {
                    vec![PackageChange {
                        name: "demo".to_string(),
                        before: Some("1.0".to_string()),
                        after: (action != OperationAction::Remove).then(|| "2.0".to_string()),
                    }]
                });
                let plan = build_plan(&request, &before, apt_changes);
                assert_eq!(plan.provider.tier, ProviderTier::Stable);
                assert_eq!(plan.exact_commands.len(), 1);
                assert_eq!(plan.expected_changes.len(), 1);
                assert!(!plan.inventory_fingerprint.is_empty());

                let after = if action == OperationAction::Remove {
                    Vec::new()
                } else {
                    vec![package("demo", source, "2.0")]
                };
                let receipt = verify_inventory(&plan, &after, Utc::now());
                assert_eq!(receipt.outcome, VerificationOutcome::Verified);
                assert_eq!(receipt.provider, source);
                assert_eq!(receipt.expected.len(), 1);
                assert_eq!(receipt.observed.len(), 1);
            }
        }
    }

    #[test]
    fn verification_rejects_wrong_installed_version() {
        let request = OperationRequest::new(
            OperationAction::Install,
            vec![package_ref("demo", PackageSource::Npm)],
            RequestedBy::Tui,
        );
        let plan = build_plan(&request, &[], None);
        let receipt = verify_inventory(
            &plan,
            &[package("demo", PackageSource::Npm, "1.5")],
            Utc::now(),
        );
        assert_eq!(receipt.outcome, VerificationOutcome::Mismatch);
    }

    #[tokio::test]
    async fn stable_provider_execution_and_receipt_matrix_is_complete() {
        for source in STABLE_PROVIDERS {
            for action in [
                OperationAction::Install,
                OperationAction::Remove,
                OperationAction::Update,
            ] {
                let initial_inventory = if action == OperationAction::Install {
                    Vec::new()
                } else {
                    vec![package("demo", source, "1.0")]
                };
                let inventory = Arc::new(Mutex::new(initial_inventory.clone()));
                let backend = ContractBackend {
                    source,
                    inventory: Arc::clone(&inventory),
                };
                let mut backends: HashMap<PackageSource, Box<dyn PackageBackend>> = HashMap::new();
                backends.insert(source, Box::new(backend));
                let manager = PackageManager {
                    backends,
                    enabled_sources: HashSet::from([source]),
                    provider_statuses: HashMap::new(),
                };
                let root = std::env::temp_dir().join(format!(
                    "linget-provider-contract-{}-{}",
                    source,
                    Uuid::new_v4()
                ));
                let store_path = root.join("transactions.json");
                let engine = TransactionEngine::load(Arc::new(Mutex::new(manager)), store_path)
                    .await
                    .expect("load contract engine");
                let request = OperationRequest::new(
                    action,
                    vec![package_ref("demo", source)],
                    RequestedBy::Tui,
                );
                let apt_changes = (source == PackageSource::Apt).then(|| {
                    vec![PackageChange {
                        name: "demo".to_string(),
                        before: (action != OperationAction::Install).then(|| "1.0".to_string()),
                        after: (action != OperationAction::Remove).then(|| "2.0".to_string()),
                    }]
                });
                let plan = build_plan(&request, &initial_inventory, apt_changes);
                engine
                    .upsert_record(OperationRecord {
                        operation_id: request.id,
                        state: OperationState::Ready,
                        risk: RiskAssessment::for_plan(&plan),
                        plan: plan.clone(),
                        receipt: None,
                        error: None,
                        updated_at: Utc::now(),
                    })
                    .await
                    .expect("persist reviewed plan");
                let receipt = engine
                    .execute(plan, CancellationFlag::default())
                    .await
                    .expect("execute contract operation");
                assert_eq!(receipt.outcome, VerificationOutcome::Verified);
                assert_eq!(engine.records().await[0].state, OperationState::Succeeded);
                fs::remove_dir_all(root)
                    .await
                    .expect("remove provider contract directory");
            }
        }
    }

    #[test]
    fn command_specs_keep_package_names_as_single_argv_values() {
        let command = command_for(
            PackageSource::Npm,
            OperationAction::Install,
            "@scope/example",
        );
        assert_eq!(command.program, "npm");
        assert_eq!(command.args, ["install", "-g", "@scope/example"]);
        assert!(!command
            .args
            .iter()
            .any(|argument| argument.contains("sh -c")));
    }

    #[test]
    fn unsafe_targets_are_rejected_before_planning() {
        for name in ["-rf", "bad\nname", ""] {
            let error = validate_targets(
                &[PackageRef {
                    name: name.to_string(),
                    source: PackageSource::Npm,
                    installed_version: None,
                    available_version: None,
                }],
                PackageSource::Npm,
            )
            .expect_err("unsafe name must fail");
            assert_eq!(error.code, ProviderErrorCode::Protocol);
        }
    }

    #[test]
    fn apt_simulation_parser_captures_dependency_changes() {
        let changes = parse_apt_simulation(
            "Inst demo [1.0] (2.0 Ubuntu:stable [amd64])\nInst dependency (3.0 Ubuntu:stable [amd64])\nRemv old-lib [0.9]",
        );
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0].before.as_deref(), Some("1.0"));
        assert_eq!(changes[0].after.as_deref(), Some("2.0"));
        assert_eq!(changes[2].after, None);
    }

    #[test]
    fn risk_assessment_requires_explicit_remove_confirmation() {
        let request = OperationRequest::new(
            OperationAction::Remove,
            vec![package_ref("demo", PackageSource::Apt)],
            RequestedBy::Tui,
        );
        let plan = build_plan(
            &request,
            &[package("demo", PackageSource::Apt, "1.0")],
            Some(vec![PackageChange {
                name: "demo".to_string(),
                before: Some("1.0".to_string()),
                after: None,
            }]),
        );
        let risk = RiskAssessment::for_plan(&plan);
        assert_eq!(risk.level, RiskLevel::High);
        assert!(risk.requires_explicit_confirmation);
    }

    #[test]
    fn operation_state_machine_rejects_skipping_verification() {
        assert!(OperationState::Ready.can_transition_to(OperationState::Running));
        assert!(OperationState::Running.can_transition_to(OperationState::Verifying));
        assert!(!OperationState::Running.can_transition_to(OperationState::Succeeded));
    }

    #[tokio::test]
    async fn store_round_trips_atomically_and_migrates_legacy_vectors() {
        let root = std::env::temp_dir().join(format!("linget-transaction-{}", Uuid::new_v4()));
        let path = root.join("transactions.json");
        let request = OperationRequest::new(
            OperationAction::Update,
            vec![package_ref("demo", PackageSource::Npm)],
            RequestedBy::Tui,
        );
        let plan = build_plan(
            &request,
            &[package("demo", PackageSource::Npm, "1.0")],
            None,
        );
        let record = OperationRecord {
            operation_id: request.id,
            state: OperationState::Ready,
            plan: plan.clone(),
            risk: RiskAssessment::for_plan(&plan),
            receipt: None,
            error: None,
            updated_at: Utc::now(),
        };
        let store = TransactionStore {
            schema_version: TRANSACTION_SCHEMA_VERSION,
            operations: vec![record.clone()],
        };
        store.save_atomic(&path).await.expect("atomic save");
        let loaded = TransactionStore::load(&path).await.expect("load store");
        assert_eq!(loaded.operations.len(), 1);

        fs::write(
            &path,
            serde_json::to_vec(&vec![record]).expect("legacy json"),
        )
        .await
        .expect("write legacy store");
        let migrated = TransactionStore::load(&path).await.expect("migrate legacy");
        assert_eq!(migrated.schema_version, TRANSACTION_SCHEMA_VERSION);
        assert!(path.with_extension("legacy.bak").exists());
        fs::remove_dir_all(root)
            .await
            .expect("remove temp directory");
    }

    #[test]
    fn error_classifier_maps_common_recovery_categories() {
        assert_eq!(
            ProviderError::classify(PackageSource::Apt, "dpkg lock held").code,
            ProviderErrorCode::LockBusy
        );
        assert_eq!(
            ProviderError::classify(PackageSource::Npm, "EACCES permission denied").code,
            ProviderErrorCode::AuthorizationDenied
        );
        assert_eq!(
            ProviderError::classify(PackageSource::Flatpak, "network timed out").code,
            ProviderErrorCode::Timeout
        );
    }
}

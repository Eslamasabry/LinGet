use crate::backend::transaction::{
    PlanFidelity, ProviderDescriptor, ProviderTier, VerificationOutcome, VerificationReceipt,
};
use crate::backend::ProviderStatus;
use crate::models::history::{OperationHistory, TaskQueueStatus};
use crate::models::PackageSource;
use crate::product::APP_VERSION;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

const REPORT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy)]
pub enum SavedHistory<'a> {
    Readable(&'a OperationHistory),
    Unreadable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CohortReport {
    schema_version: u16,
    linget_version: String,
    data_handling: DataHandling,
    providers: Vec<ProviderReadiness>,
    saved_history: HistoryReadiness,
    task_outcomes: TaskOutcomeSummary,
    verification_outcomes: VerificationOutcomeSummary,
    next_steps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DataHandling {
    local_only: bool,
    transmitted_by_linget: bool,
    package_inventory_included: bool,
    personal_identifiers_included: bool,
    arbitrary_command_output_included: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ProviderReadiness {
    provider: String,
    detected: bool,
    tier: &'static str,
    plan_fidelity: &'static str,
    ready_for_verified_operation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum HistoryReadiness {
    Readable,
    Unreadable,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
struct TaskOutcomeSummary {
    total: usize,
    queued: usize,
    running: usize,
    completed: usize,
    failed: usize,
    cancelled: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
struct VerificationOutcomeSummary {
    receipts: usize,
    verified: usize,
    mismatch: usize,
    inconclusive: usize,
    unreadable_receipts: usize,
}

pub fn build(providers: &[ProviderStatus], history: SavedHistory<'_>) -> CohortReport {
    let providers: Vec<ProviderReadiness> = providers
        .iter()
        .map(|status| {
            let descriptor = ProviderDescriptor::for_source(status.source);
            ProviderReadiness {
                provider: safe_provider_name(status.source),
                detected: status.available,
                tier: tier_name(descriptor.tier),
                plan_fidelity: fidelity_name(descriptor.fidelity),
                ready_for_verified_operation: status.available
                    && descriptor.tier == ProviderTier::Stable,
            }
        })
        .collect();

    let (saved_history, task_outcomes, verification_outcomes) = match history {
        SavedHistory::Readable(history) => (
            HistoryReadiness::Readable,
            summarize_tasks(history),
            summarize_receipts(history),
        ),
        SavedHistory::Unreadable => (
            HistoryReadiness::Unreadable,
            TaskOutcomeSummary::default(),
            VerificationOutcomeSummary::default(),
        ),
    };

    let next_steps = next_steps(&providers, saved_history, &verification_outcomes);

    CohortReport {
        schema_version: REPORT_SCHEMA_VERSION,
        linget_version: APP_VERSION.to_string(),
        data_handling: DataHandling {
            local_only: true,
            transmitted_by_linget: false,
            package_inventory_included: false,
            personal_identifiers_included: false,
            arbitrary_command_output_included: false,
        },
        providers,
        saved_history,
        task_outcomes,
        verification_outcomes,
        next_steps,
    }
}

pub fn render_json(report: &CohortReport) -> Result<String> {
    serde_json::to_string_pretty(report).context("Failed to serialize cohort report")
}

pub fn render_human(report: &CohortReport) -> String {
    let ready: Vec<&str> = report
        .providers
        .iter()
        .filter(|provider| provider.ready_for_verified_operation)
        .map(|provider| provider.provider.as_str())
        .collect();
    let ready = if ready.is_empty() {
        "none".to_string()
    } else {
        ready.join(", ")
    };

    let mut lines = vec![
        "LinGet privacy-safe cohort report".to_string(),
        format!("LinGet version: {}", report.linget_version),
        "Data handling: local only; nothing is transmitted".to_string(),
        format!("Ready Stable providers: {ready}"),
        format!(
            "Task outcomes: {} total, {} completed, {} failed, {} cancelled, {} active",
            report.task_outcomes.total,
            report.task_outcomes.completed,
            report.task_outcomes.failed,
            report.task_outcomes.cancelled,
            report.task_outcomes.queued + report.task_outcomes.running,
        ),
        format!(
            "Verification receipts: {} verified, {} mismatch, {} inconclusive, {} unreadable",
            report.verification_outcomes.verified,
            report.verification_outcomes.mismatch,
            report.verification_outcomes.inconclusive,
            report.verification_outcomes.unreadable_receipts,
        ),
        String::new(),
        "Next steps:".to_string(),
    ];
    lines.extend(report.next_steps.iter().map(|step| format!("- {step}")));
    lines.join("\n")
}

pub fn write_json(path: &Path, report: &CohortReport) -> Result<()> {
    let content = render_json(report)?;
    std::fs::write(path, content).context("Failed to write cohort report")
}

fn summarize_tasks(history: &OperationHistory) -> TaskOutcomeSummary {
    let mut summary = TaskOutcomeSummary::default();
    for entry in &history.task_queue.entries {
        summary.total += 1;
        match entry.status {
            TaskQueueStatus::Queued => summary.queued += 1,
            TaskQueueStatus::Running => summary.running += 1,
            TaskQueueStatus::Completed => summary.completed += 1,
            TaskQueueStatus::Failed => summary.failed += 1,
            TaskQueueStatus::Cancelled => summary.cancelled += 1,
        }
    }
    summary
}

fn summarize_receipts(history: &OperationHistory) -> VerificationOutcomeSummary {
    let mut summary = VerificationOutcomeSummary::default();
    for receipt_json in history
        .task_queue
        .entries
        .iter()
        .filter_map(|entry| entry.verification_receipt_json.as_deref())
    {
        match serde_json::from_str::<VerificationReceipt>(receipt_json) {
            Ok(receipt) => {
                summary.receipts += 1;
                match receipt.outcome {
                    VerificationOutcome::Verified => summary.verified += 1,
                    VerificationOutcome::Mismatch => summary.mismatch += 1,
                    VerificationOutcome::Inconclusive => summary.inconclusive += 1,
                }
            }
            Err(_) => summary.unreadable_receipts += 1,
        }
    }
    summary
}

fn next_steps(
    providers: &[ProviderReadiness],
    history: HistoryReadiness,
    receipts: &VerificationOutcomeSummary,
) -> Vec<String> {
    let mut steps = Vec::new();
    if !providers
        .iter()
        .any(|provider| provider.ready_for_verified_operation)
    {
        steps.push(
            "Enable APT, Flatpak, or npm before attempting the cohort's verified operation."
                .to_string(),
        );
    }
    if history == HistoryReadiness::Unreadable {
        steps.push(
            "Saved outcome history could not be read; do not count this report as receipt evidence."
                .to_string(),
        );
    } else if receipts.receipts == 0 {
        steps.push(
            "Complete one intended, reviewed operation with a Stable provider, then run this report again."
                .to_string(),
        );
    }
    if receipts.mismatch > 0 || receipts.inconclusive > 0 || receipts.unreadable_receipts > 0 {
        steps.push(
            "Review the affected receipt locally in Queue; do not paste package or command details into the cohort scorecard."
                .to_string(),
        );
    }
    steps.push(
        "Share only this generated summary and your anonymous participant ID with the cohort facilitator."
            .to_string(),
    );
    steps
}

fn safe_provider_name(source: PackageSource) -> String {
    source.to_string().to_lowercase()
}

fn tier_name(tier: ProviderTier) -> &'static str {
    match tier {
        ProviderTier::Stable => "stable",
        ProviderTier::Beta => "beta",
        ProviderTier::DetectionOnly => "detection_only",
    }
}

fn fidelity_name(fidelity: PlanFidelity) -> &'static str {
    match fidelity {
        PlanFidelity::Exact => "exact",
        PlanFidelity::BestEffort => "best_effort",
        PlanFidelity::Unsupported => "unsupported",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::transaction::{PackageChange, VerificationReceipt};
    use crate::models::history::{TaskQueueAction, TaskQueueEntry};
    use chrono::Utc;
    use std::path::PathBuf;

    fn provider(source: PackageSource, available: bool) -> ProviderStatus {
        ProviderStatus {
            source,
            display_name: "arbitrary display output".to_string(),
            available,
            list_cmds: vec!["secret-command".to_string()],
            privileged_cmds: vec!["secret-privileged-command".to_string()],
            found_paths: vec![PathBuf::from("/home/alice/private/bin")],
            version: Some("provider output with alice-host".to_string()),
            reason: Some("private diagnostic /home/alice".to_string()),
        }
    }

    fn task_with_receipt(outcome: VerificationOutcome) -> TaskQueueEntry {
        let mut task = TaskQueueEntry::new(
            TaskQueueAction::Update,
            "secret.package.id".to_string(),
            "secret-package-name".to_string(),
            PackageSource::Apt,
        );
        task.id = "secret-task-id".to_string();
        task.status = TaskQueueStatus::Completed;
        task.error = Some("private command output from alice-host".to_string());
        task.reviewed_operation_id = Some("secret-reviewed-operation".to_string());
        task.reviewed_plan_json = Some("/home/alice/secret-plan".to_string());
        task.verification_receipt_json = Some(
            serde_json::to_string(&VerificationReceipt {
                operation_id: "secret-operation-id".to_string(),
                plan_id: "secret-plan-id".to_string(),
                provider: PackageSource::Apt,
                expected: vec![PackageChange {
                    name: "secret-package-name".to_string(),
                    before: Some("private-old-version".to_string()),
                    after: Some("private-new-version".to_string()),
                }],
                observed: Vec::new(),
                outcome,
                warnings: vec!["private arbitrary warning".to_string()],
                verified_at: Utc::now(),
            })
            .expect("serialize fixture receipt"),
        );
        task
    }

    #[test]
    fn report_is_an_aggregate_without_sensitive_source_fields() {
        let mut history = OperationHistory::new();
        history
            .task_queue
            .entries
            .push(task_with_receipt(VerificationOutcome::Verified));

        let report = build(
            &[
                provider(PackageSource::Apt, true),
                provider(PackageSource::Flatpak, false),
            ],
            SavedHistory::Readable(&history),
        );
        let json = render_json(&report).expect("render report");

        assert!(json.contains("\"linget_version\""));
        assert!(json.contains("\"verified\": 1"));
        assert!(json.contains("\"provider\": \"apt\""));
        for secret in [
            "alice",
            "/home/",
            "secret-package",
            "secret-task",
            "secret-operation",
            "secret-plan",
            "private",
            "secret-command",
            "arbitrary display",
        ] {
            assert!(!json.contains(secret), "report leaked {secret:?}: {json}");
        }
    }

    #[test]
    fn report_counts_task_and_receipt_outcomes_without_details() {
        let mut history = OperationHistory::new();
        history
            .task_queue
            .entries
            .push(task_with_receipt(VerificationOutcome::Mismatch));
        let mut failed = TaskQueueEntry::new(
            TaskQueueAction::Install,
            "private-id".to_string(),
            "private-name".to_string(),
            PackageSource::Npm,
        );
        failed.mark_failed("arbitrary command output".to_string());
        failed.verification_receipt_json = Some("not valid JSON /home/alice".to_string());
        history.task_queue.entries.push(failed);

        let report = build(
            &[provider(PackageSource::Apt, true)],
            SavedHistory::Readable(&history),
        );

        assert_eq!(report.task_outcomes.total, 2);
        assert_eq!(report.task_outcomes.completed, 1);
        assert_eq!(report.task_outcomes.failed, 1);
        assert_eq!(report.verification_outcomes.receipts, 1);
        assert_eq!(report.verification_outcomes.mismatch, 1);
        assert_eq!(report.verification_outcomes.unreadable_receipts, 1);
        assert!(report
            .next_steps
            .iter()
            .any(|step| step.contains("Review the affected receipt locally")));
    }

    #[test]
    fn unreadable_history_is_reported_without_an_error_detail() {
        let report = build(
            &[provider(PackageSource::Apt, false)],
            SavedHistory::Unreadable,
        );
        let json = render_json(&report).expect("render report");

        assert!(json.contains("\"saved_history\": \"unreadable\""));
        assert!(json.contains("Enable APT, Flatpak, or npm"));
        assert!(json.contains("could not be read"));
    }
}

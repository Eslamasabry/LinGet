## BRD-005 — Update Policy Engine, Automation, and Observability

### 0) TUI-First Delivery Scope
- Current implementation target: **TUI first**.
- Policy definition, scheduling, staged execution, and reporting are implemented in TUI workflows first.
- CLI commands are used as machine-facing entry points within TUI sessions where needed; GUI remains deferred.

### 1) Strategic Intent
- Make updates predictable, configurable, and transparent.
- Reduce manual decision overhead while maintaining user control and trust.

### 2) Current Problem
- Update behavior is often reactive and inconsistent across users and environments.
- Hard to explain why packages were skipped or blocked.

### 3) Scope
- Policy definition and scheduling layer.
- Update execution pipeline with staged validation and reporting.
- Health/telemetry output for operation quality and noise control.

### 4) Success Definition
- A user can define policies and receive deterministic, explainable update behavior.
- Scheduled updates run safely with clear outcomes and recovery options.

### 5) Requirements

#### 5.1 Functional Requirements
- FR1: Add policy profiles (`security-first`, `stable`, `latest`, `conservative`, `manual`).
- FR2: Add scheduling controls (cron-like, timezone-aware, quiet windows, maintenance windows).
- FR3: Add update stages: check → stage → validate → apply → verify → report.
- FR4: Add per-source allow/deny filters, risk thresholds, and skip policies.
- FR5: Add pre-update backup snapshot and post-update diff summary.
- FR6: Add notification controls per channel (none/terminal/popup/log only).
- FR7: Emit structured run reports and health metrics for each scheduled cycle.

#### 5.2 Non-Functional Requirements
- NFR1: Scheduled tasks must not block UI startup.
- NFR2: Deterministic policy evaluation (same input/state => same output).
- NFR3: Failed runs must always emit a report with reason and next-best action.

### 6) Frontend Requirements
- GUI
  - Add Policy Center UI with profiles, calendars, and per-source override controls.
  - Show last run summary cards: attempted/skipped/block reasons and blockers.
  - Add dry-run preview for policy execution.
- TUI
  - Add policy status and next run summary in dashboard view.
  - Add quick commands to switch profile and toggle update window modes.
  - Show per-cycle outcome counters and remediation hints.
- CLI
  - Add commands: `policy list`, `policy show`, `policy apply`, `policy run`, `policy test`.
  - Add JSON reports and exit codes for automation pipelines.

### 7) Data Model (Initial)
- `UpdatePolicy`
  - `id`, `name`, `rules`, `sources`, `risk_threshold`, `allowed_windows`, `source_prefs`, `created_at`, `updated_at`
- `ScheduledRun`
  - `run_id`, `policy_id`, `status`, `started_at`, `finished_at`, `packages_detected`, `packages_applied`, `packages_skipped`, `errors`, `report_path`

### 8) Risks
- Over-aggressive auto-update may conflict with user preference for manual flow.
- Timezone and locale edge cases can misfire update windows.
- Notification fatigue if policy events are too frequent.

### 9) Acceptance Criteria
- P0: User can create and run a policy end-to-end with deterministic output.
- P0: All update outcomes (success, skip, failure) are reported with reason codes.
- P1: Scheduled policy can run without UI interaction and produce a machine-readable report.
- P2: Quiet windows and skip reasons are visibly applied before update execution.

### 10) Additional Cross-Cutting Coverage
- RQ-21: Security-sensitive prompts for operations requiring privilege and policy bypass actions.
- RQ-22: Telemetry/observability data is opt-in and privacy redacted.
- RQ-23: CLI and GUI policy messages prepared for future localization.
- RQ-24: User onboarding for policy profiles and recovery behavior at first run.
- RQ-25: Versioned policy schema and migration path for persisted schedules.

### 11) KPIs
- Update completion reliability (successful cycles / total cycles).
- Drop in manual update operations for users with active policies.
- Reduced skipped-update surprises (measured by support tickets/reports).

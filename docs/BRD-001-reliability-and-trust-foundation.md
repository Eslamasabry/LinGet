## BRD-001 — Reliability & Trust Foundation

### 0) TUI-First Delivery Scope
- Current implementation target: **TUI first**.
- All FRs and acceptance criteria are expected to be delivered in TUI workflows before GUI/CLI rollouts.
- GUI and CLI sections remain as contract references for future reuse.

### 1) Strategic Intent
- Increase user confidence by making every package action explainable, traceable, and recoverable.
- Eliminate ambiguity during failures by surfacing clear causes, recovery options, and next actions.

### 2) Current Problem
- Failures often surface late and are difficult to recover from.
- Users can execute destructive actions without a guaranteed visible safety path.
- Cross-interface behavior is partially consistent but lacks a shared recovery workflow.

### 3) Scope
- All mutating operations: install, remove, update, backup restore, and batch queue tasks.
- CLI, TUI, and GUI operation lifecycle and failure reporting.
- Persisted operation history, error taxonomy, and rollback metadata.

### 4) Success Definition
- A user can see what an action will do before it runs.
- A failed or partial run produces a complete recovery context.
- All interfaces expose the same reliability guarantees.

### 5) Requirements

#### 5.1 Functional Requirements
- FR1: Add `--dry-run` (and equivalent UI action) for all mutating commands and queue actions.
- FR2: Show a stepwise execution plan before run (`prepare`, `validate`, `execute`, `post-check`, `finalize`).
- FR3: Persist `operation_id` for every mutation, reused across logs, queue entries, and status views.
- FR4: Provide structured failure codes with a remediation path and one-click retry/rollback actions where available.
- FR5: Classify failures by code (at minimum): authentication, permission, lock conflict, dependency conflict, network timeout, checksum/signature verification, repo unavailable, disk full, unknown.
- FR6: Maintain an operation history of the last configurable N records (default 20) and expose it from every interface.
- FR7: Add rollback metadata with `rollback_supported` flag, provider capability details, and fallback guidance.
- FR8: Introduce best-effort post-failure state snapshot for partial failure recovery.

#### 5.2 Non-Functional Requirements
- NFR1: Operation records must be immutable and append-only.
- NFR2: Persistence failures must not block action UI completion; show recovery warning and fallback path.
- NFR3: Failure-to-message latency must be low enough to keep operators in flow (`< 1s` for cached failure reason mapping).

### 6) Frontend Requirements
- GUI
  - Add a persistent **Operation Timeline** area with plan/phase/result states.
  - Add a recoverability view with `Rollback`, `Retry`, `Mark-Resolved`, and `Copy Incident Bundle` actions.
  - Show rollback availability badge before executing high-risk actions.
- TUI
  - Add status column/status bar fields for lifecycle and `dry-run` mode.
  - Add modal/overlay for quick failure summary and recovery path.
  - Keyboard actions: `d` dry-run view, `r` retry, `x` cancel, `i` inspect incident details.
- CLI
  - Ensure `--json` exposes `operation_id`, dry-run plan, phase status, remediation steps, and rollback metadata.
  - Add `linget ops history`, `linget ops show <id>`, and machine-friendly JSON output for CI usage.

### 7) Data Model (Initial)
- `OperationLog`
  - `id`, `operation_id`, `action`, `provider`, `targets`, `arguments`, `plan`, `pre_snapshot`, `post_snapshot`, `status`, `error_code`, `remediation`, `rollback_plan`, `created_at`, `finished_at`, `started_by`
- `FailureProfile`
  - `code`, `provider`, `root_causes`, `remediation_steps`, `retry_policy`, `supports_retry`, `supports_rollback`
- `QueueDecision`
  - `operation_id`, `risk_level`, `confirmation_level`, `required_permissions`, `blocking_reason`, `policy_id`

### 8) Dependencies
- Shared action and queue state engine for all interfaces.
- Persistent config storage and history cache migration utility.
- Error classifier module with source/provider-specific adapters.

### 9) Risks
- Provider limitations may block full rollback and force manual steps.
- Snapshot/cached state overhead could increase package action latency.
- Persisted logs need secure local storage and redaction for sensitive data.

### 10) Acceptance Criteria
- P0: All mutating operations can be dry-run reviewed before execution.
- P0: Every failure has a structured code and one of `retry`, `rollback`, or `manual` recovery.
- P1: Operation history is queryable from all interfaces and always attached to queue tasks.
- P2: No unhandled “unknown failure” class for known provider error buckets in tested scenarios.

### 11) Additional Cross-Cutting Coverage
- RQ-17: Accessibility baseline for failure and recovery interactions (keyboard-first recovery actions and readable status labels).
- RQ-25: Configuration and migration safety for operation/log retention settings.
- RQ-29: Clearly mark deterministic versus best-effort rollback/recovery behavior by provider and action.
- RQ-30: Explicit cancellation/retry state machine for long-running and partial operations.

### 12) KPIs
- 95% reduction in user-reported “I don’t know why it failed” incidents.
- 100% of failed actions include a remediation hint.
- Recovery time target: no operation recovery takes longer than 60s for known rollback-capable actions.

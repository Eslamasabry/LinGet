## BRD-002 — Unified Cross-Interface Experience

### 0) TUI-First Delivery Scope
- Current implementation target: **TUI first**.
- All policy, queue, and action-schema requirements are implemented in TUI command and rendering logic first.
- GUI and CLI requirements should mirror this behavior when those surfaces are expanded later.

### 1) Strategic Intent
- Eliminate interface drift by making CLI, TUI, and GUI obey one shared action contract.
- Guarantee users can transfer workflows between interfaces without relearning rules.

### 2) Current Problem
- Some workflows can be completed in one interface but behave differently in others.
- Risk and confirmation policies are not uniformly surfaced across all frontends.

### 3) Scope
- Shared command/action model and policy engine across CLI, TUI, and GUI.
- Queue state machine and confirmation logic as a single source of truth.
- Shared status language and error/risk vocabulary.

### 4) Success Definition
- Given the same task input and package state, all interfaces produce equivalent states and outcomes.
- No divergence in disable/restriction reasons for the same action.

### 5) Requirements

#### 5.1 Functional Requirements
- FR1: Introduce a shared action schema used by all UIs (`action`, `target`, `provider`, `mode`, `risk`, `flags`, `scope`).
- FR2: Move risk detection + confirmation rules to a backend policy service.
- FR3: Introduce a shared queue state model: `queued`, `running`, `blocked`, `waiting_input`, `succeeded`, `failed_partial`, `failed`, `retrying`, `rolled_back`.
- FR4: Standardize disabled action reasons and return codes (`requires_refresh`, `high_risk`, `provider_unavailable`, `invalid_state`, `dependency_blocked`).
- FR5: Publish shared filter DSL and sorting keys so all interfaces show consistent list behavior.
- FR6: Add parity fixtures for scenarios (remove with dependency blocker, queued batch with one failure, forced confirmation path).

#### 5.2 Non-Functional Requirements
- NFR1: No interface-specific business logic outside adaptation/ rendering layers.
- NFR2: Regression tests must compare parity across interfaces for the same synthetic state.
- NFR3: Keep adapter translation overhead minimal (`< 5ms` for action conversion paths).

### 6) Frontend Requirements
- GUI
  - Shared command palette entries must match CLI commands and TUI command list.
  - Disable/revert/confirm buttons must follow centralized `ActionPolicy` results.
  - Confirmation text and risk levels must be generated from shared policy metadata.
- TUI
  - Key bindings should be documented from shared action matrix.
  - Display action hints for disabled reasons and blocked state from policy engine.
  - Queue transitions should mirror GUI labels exactly.
- CLI
  - Keep command semantics aligned; `--dry-run`, `--json`, and confirmation prompts should use same policy decisions.
  - Return machine-readable `disabled_reason` and `required_confirmation` in JSON.

### 7) Contract & Data
- `ActionDefinition`
  - `id`, `label`, `allowed_modes`, `requires_confirmation`, `requires_source`, `scope`, `risk_level`, `provider_filters`, `disabled_reasons[]`, `ui_hints`
- `ActionRequest`
  - `operation_id`, `definition_id`, `target_selector`, `provider_selector`, `options`, `context`
- `ActionResult`
  - `status`, `message_code`, `details`, `next_steps`, `rollback_id`, `policy_version`

### 8) Risks
- Drift can reappear if local hacks bypass adapter and call backend directly.
- Feature matrix differences can grow unless parity tests fail-fast.
- Translation bugs could hide policy intent in specific interfaces.

### 9) Acceptance Criteria
- P0: Same test scenario produces identical action outcome and queue state across CLI/TUI/GUI.
- P0: All high-risk actions require confirmation in all interfaces.
- P1: Status vocabulary and disabled reasons are identical across interface outputs.
- P2: Action schema versioning exists with backward-compatible migration for existing consumers.

### 10) Additional Cross-Cutting Coverage
- RQ-17: Accessibility baseline for all interfaces (keyboard, focus order, readable statuses, destructive actions discoverable via non-mouse controls).
- RQ-18: Stable machine-readable schema for CLI and frontend integrations (`--json`, status codes, and machine event fields).
- RQ-23: Message keys and user-facing copy prepared for localization and future i18n extraction.
- RQ-25: State migration strategy for persisted action metadata and queued tasks.

### 11) KPIs
- Parity test pass rate: 100% in CI for action scenarios.
- Reduction in interface-specific bug reports quarter-over-quarter.
- New feature rollout time improves due to shared contract adoption.

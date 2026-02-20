# BRD-TUI-EPICS - TUI-First Implementation Breakdown

## Goal

Turn the TUI scope in `BRD_TUI-ALL-REQUIREMENTS.md` into an executable sequence of epics and stories so work can be started immediately.

## Acceptance Framing

- No epic is complete until every story has explicit pass/fail criteria and a trace to at least one `RQ-##` row.
- P0 stories are blocked behind explicit preconditions and can only be promoted to done after CLI/TUI test validation.
- P1/P2 stories can start while P0 flows are in progress only if they do not increase operator risk.

## Epic TUI-E01: TUI Execution Core and Contracts (P0)

- Owner BRDs: `BRD-001`, `BRD-002`

- `TUI-E01-S01` - Define and publish TUI operation schema v1
  - Deliverable: canonical action model for plan, validation result, execution result, and transition events.
  - Acceptance: documented structs/enums are consumed by all TUI action entrypoints; unknown payloads fail with a recoverable validation error.
  - `RQ` coverage: `RQ-15`, `RQ-16`, `RQ-17`, `RQ-26`

- `TUI-E01-S02` - Implement TUI action pipeline and operation IDs
  - Deliverable: each requested mutation receives deterministic ID and status transitions (`Queued` -> `Running` -> `Done/Failed/Canceled`).
  - Acceptance: repeated action submission with same intent returns idempotent queue behavior when safe.
  - `RQ` coverage: `RQ-26`, `RQ-30`

- `TUI-E01-S03` - Preflight + dry-run enforcement for all mutating flows
  - Deliverable: every mutating action enters preflight before execution.
  - Acceptance: no mutating path can execute without dry-run summary + confirmation in UI.
  - `RQ` coverage: `RQ-01`, `RQ-21`, `RQ-29`

## Epic TUI-E02: Trust, Recovery, and Observability (P0)

- Owner BRDs: `BRD-001`

- `TUI-E02-S01` - Structured error taxonomies and recovery branches
  - Deliverable: error classes mapped to user actions (`retry`, `rollback`, `abort`, `manual-fallback`).
  - Acceptance: each failure path exposes at least one explicit recovery action in TUI.
  - `RQ` coverage: `RQ-03`, `RQ-29`

- `TUI-E02-S02` - Operation history, timeline, and rollback controls
  - Deliverable: TUI screen/panel for persisted operation ledger and last N operations.
  - Acceptance: completed and failed operations remain queryable after restart.
  - `RQ` coverage: `RQ-04`, `RQ-30`

- `TUI-E02-S03` - Privilege and auth state surfacing
  - Deliverable: explicit blocked/approved/required states before and during execution.
  - Acceptance: all privilege failures classify as blocked vs denied vs retriable and show next steps.
  - `RQ` coverage: `RQ-21`

## Epic TUI-E03: Discovery and Recommendation UX in TUI (P0/P1)

- Owner BRDs: `BRD-003`, `BRD-004`

- `TUI-E03-S01` - Ranked list rendering and explanation pane
  - Deliverable: ranked list plus per-item reason text for each score component.
  - Acceptance: user can expand an item and see at least 3 explanation factors.
  - `RQ` coverage: `RQ-08`, `RQ-10`

- `TUI-E03-S02` - Collections and navigation surfaces
  - Deliverable: quick-switchable curated groups and saved searches.
  - Acceptance: switching groups preserves query and filter state.
  - `RQ` coverage: `RQ-09`, `RQ-11`

- `TUI-E03-S03` - Discovery safety and dependency awareness
  - Deliverable: impact tree shown for install/update/remove with confidence and unknown-data warnings.
  - Acceptance: actions with high-impact impact require additional explicit confirmation path.
  - `RQ` coverage: `RQ-06`, `RQ-07`, `RQ-05`

## Epic TUI-E04: Policy and Automation Surfaces (P0)

- Owner BRDs: `BRD-005`

- `TUI-E04-S01` - Policy builder and profile list in TUI
  - Deliverable: create/edit/activate/deactivate/update policies with clear state and summary.
  - Acceptance: policy CRUD can be completed without leaving TUI and is persisted.
  - `RQ` coverage: `RQ-11`, `RQ-13`

- `TUI-E04-S02` - Scheduled operations and forecast preview
  - Deliverable: next-run plan view with estimated count, estimated affected packages, and conflicts.
  - Acceptance: schedule edit reflects in preview and applies at run-time.
  - `RQ` coverage: `RQ-11`, `RQ-13`

- `TUI-E04-S03` - TUI reporting outputs and blocked-reason visibility
  - Deliverable: JSON + human summaries for policy runs, including skip/blocked reasons.
  - Acceptance: report includes machine-readable `result schema_version` and per-item reason codes.
  - `RQ` coverage: `RQ-12`, `RQ-18`

## Epic TUI-E05: Cross-cutting hardening and completeness (P1/P2)

- Owner BRDs: `BRD-002`, `BRD-006`

- `TUI-E05-S01` - Localization and wording consistency
  - Deliverable: action labels/status strings moved behind centralized key-value maps.
  - Acceptance: changing one key updates all affected TUI views without component edits.
  - `RQ` coverage: `RQ-23`

- `TUI-E05-S02` - Onboarding, help, and trust signaling
  - Deliverable: first-run tour, context help, and explicit best-effort vs guaranteed markers.
  - Acceptance: onboarding appears once for new profile and can be reopened from Help.
  - `RQ` coverage: `RQ-24`, `RQ-22`

- `TUI-E05-S03` - Configuration migration and startup validation
  - Deliverable: version checks and migration flow with recoverable fallback.
  - Acceptance: invalid/missing config versions fail safely with a migration notice and continue in minimal mode.
  - `RQ` coverage: `RQ-25`

- `TUI-E05-S04` - Test plan per epic and matrix coverage
  - Deliverable: acceptance matrix aligned to each story and a required test script list.
  - Acceptance: every P0 story has at least one manual and one automated test path.
  - `RQ` coverage: `RQ-27`

## Suggested Execution Order

1. `TUI-E01` Foundation and Contracts
2. `TUI-E02` Trust and Recovery
3. `TUI-E03` Discovery/Safety
4. `TUI-E04` Policy/Automation
5. `TUI-E05` Hardening and completeness

## Readiness Gate

- Continue to the next epic only when all P0 stories in current epic are complete and verified.
- Use this document to create implementation issues/branches with explicit story IDs and references to `RQ-##` owners.

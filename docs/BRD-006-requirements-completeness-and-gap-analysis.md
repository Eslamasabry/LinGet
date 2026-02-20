# BRD-006 — Requirements Completeness and Gap Analysis

## Purpose

Ensure all requirements for the “ultimate package/apps manager” roadmap are explicitly captured and assigned to implementation BRDs.

### TUI Implementation Scope
- This file tracks requirement coverage for the **TUI-first** implementation path.
- All requirement owners (`BRD-001` to `BRD-005`) are expected to be executed through TUI first, unless explicitly marked for later GUI/CLI-only handling.

## Required Coverage Rule

Every requirement in this document must be covered by at least one BRD and mapped to a priority (P0/P1/P2).

- P0: Must ship for launch
- P1: Must ship for v1.0 parity
- P2: Enhancements or refinements

## Coverage Matrix

| Req ID | Requirement | Priority | Owner BRD | Notes |
|---|---|---|---|---|
| RQ-01 | Deterministic safety preview (dry-run) before mutating actions | P0 | BRD-001 | Includes operation timeline and action plan |
| RQ-02 | Cross-interface parity for action, queue, and confirmation flow | P0 | BRD-002 | Core reliability for CLI/TUI/GUI consistency |
| RQ-03 | Structured error taxonomy + recovery suggestions | P0 | BRD-001 | Includes retry/rollback/manual fallback |
| RQ-04 | Operation history and traceability for audit/debug | P0 | BRD-001 | Persist across sessions |
| RQ-05 | Role-aware confidence and risk labeling | P1 | BRD-001, BRD-004 | Risk scores per operation |
| RQ-06 | Dependency graph visualization and impact preview | P0 | BRD-004 | Before install/remove/update when available |
| RQ-07 | High-impact operation guardrails and override flow | P0 | BRD-004 | Explicit confirmation path in all UIs |
| RQ-08 | Provider-aware discovery and search ranking | P0 | BRD-003 | Heuristics then policy-based scoring |
| RQ-09 | Collections and curated pathways (developer, gaming, media, etc.) | P1 | BRD-003 | Optional starter set for v1.0 |
| RQ-10 | Explainable recommendation reason strings | P1 | BRD-003 | “Why this result” and relevance factors |
| RQ-11 | Update policy profiles and scheduling engine | P0 | BRD-005 | Manual/automatic policy modes |
| RQ-12 | Reportable update outcomes and skipped/blocked reasons | P0 | BRD-005 | Includes JSON + human view |
| RQ-13 | Quiet windows and user-defined automation boundaries | P1 | BRD-005 | Timezone-aware execution policy |
| RQ-14 | Backup/restore integration before large changes | P1 | BRD-005 | Should include manifest/summary |
| RQ-15 | Shared action schema for all interfaces | P0 | BRD-002 | Includes request/result/decision contracts |
| RQ-16 | Shared vocabulary for status, errors, and policy reasons | P0 | BRD-002 | Use same tokens in all interfaces |
| RQ-17 | Accessibility baseline (keyboard-first operations + navigation clarity) | P1 | BRD-002, BRD-003 | Explicitly documented in all frontends |
| RQ-18 | CLI machine-readable mode for automation (`--json` consistency) | P0 | BRD-002, BRD-005 | Structured schema versioning |
| RQ-19 | Responsive behavior on large package datasets | P1 | BRD-001, BRD-002 | Streaming + bounded UI update strategy |
| RQ-20 | Offline/low-connectivity discoverability cache | P1 | BRD-003 | Cached index + freshness marker |
| RQ-21 | Security-sensitive operation prompt and privilege handling clarity | P0 | BRD-001 | pkexec/auth failure classification |
| RQ-22 | Privacy-aware telemetry and local-only defaults | P2 | BRD-005 | Opt-in reporting and redaction policy |
| RQ-23 | Localization and message externalization baseline | P2 | BRD-002 | Stable message keys and translation path |
| RQ-24 | Install/restore profile presets and onboarding | P2 | BRD-005 / BRD-003 | Guided first-run and first-operation onboarding |
| RQ-25 | Config schema migration and backward compatibility | P1 | BRD-002 | Add config-version and migration checks |
| RQ-26 | Regression safety for queue behavior and retries | P0 | BRD-002, BRD-001 | Shared queue state machine |
| RQ-27 | Test and validation coverage for all BRDs | P0 | BRD-002 | Unit/integration/fixture matrix |
| RQ-28 | Provider metadata freshness + health reporting | P1 | BRD-005 | Track provider check failures and availability |
| RQ-29 | Clear separation of best-effort vs guaranteed behaviors | P1 | BRD-001, BRD-004 | Required for trust signaling |
| RQ-30 | User-facing cancellation/retry for long-running operations | P1 | BRD-001 | Keep operations stoppable with state update |

## Gap Analysis

 - Current coverage for each BRD is strong on strategic areas, and all `RQ-##` rows are now mapped.
 - Remaining work is implementation-level decomposition into epics and stories.

## Gap Closure Plan

- RQ-17 (accessibility), RQ-23 (localization), RQ-22 (telemetry privacy), RQ-25 (migration), and RQ-24 (onboarding) are now documented as cross-cutting requirements in the BRD set.

## Completion Criteria

- A reviewer can trace each `RQ-##` to one or more BRD owners.
- No item remains un-mapped in the matrix.
- Future BRD edits must add/adjust rows instead of creating undocumented requirements.

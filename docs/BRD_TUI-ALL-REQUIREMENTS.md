# BRD-TUI-ALL — TUI-First Delivery of All BRDs

## Intent

Implement the roadmap entirely in **TUI first**, so every feature and quality bar in BRD-001 to BRD-006 is delivered in the terminal UI before broadening to GUI/CLI.

## Scope

- Full TUI feature parity for all roadmap areas.
- All interface mentions in existing BRDs are now normalized to TUI as the current implementation target.
- The same backend/domain contracts are still expected to be reusable for later GUI/CLI rollout.

## TUI Coverage Matrix

| BRD | TUI Delivery Scope | Key TUI Commitments |
| --- | --- | --- |
| BRD-001 Reliability & Trust | Full | Preflight timeline, dry-run view, failure details, rollback/retry actions, operation history, structured error output |
| BRD-002 Unified Experience | Full | Shared action contract adapters, deterministic key-based confirmations, queue state rendering, parity semantics enforced in TUI command and queue layers |
| BRD-003 Discovery Intelligence | Full | Ranked list + explainability pane, collections navigation, recommendation mode, offline index indicators, score visibility |
| BRD-004 Dependency Safety | Full | Preflight impact tree, uncertain-data warnings, override path for high-impact removals, dependency-aware batching previews |
| BRD-005 Policy/Automation | Full | Policy list/show/run/test commands from TUI, scheduled update forecast, machine-readable report output, skip-block reason visibility |
| BRD-006 Gap Analysis | Full | Requirement traceability maintained as authoritative source for TUI decomposition |

## TUI-Normalized Cross-BRD Requirements

- RQ-17 (Accessibility): keyboard-first interactions and explicit focus-safe status/decision labels in all TUI workflows.
- RQ-18 (`--json` parity): every TUI command path that mutates or inspects queue/state exposes machine-readable output variants.
- RQ-21 (privilege-aware operations): explicit blocked/needs-auth state in TUI preflight and execution stages.
- RQ-22 (privacy): no external telemetry by default; optional reporting only.
- RQ-23 (localization): action labels and status text from central string maps.
- RQ-24 (onboarding): first-run tour + contextual hints embedded in TUI help panels.
- RQ-25 (migration): persisted action/queue/state versions are validated during startup.
- RQ-27 (validation): all BRD scenarios have TUI test coverage in acceptance harness.

## TUI Execution Order (No GUI/CLI Required)

1. **Execution Core for BRD-001**
   - Action plan state machine, operation IDs, error classification, recovery actions.
2. **Unified TUI Contract (BRD-002)**
   - Shared action schema adapters and queue state renderer.
3. **TUI Discovery and Safety Layer (BRD-003 + BRD-004)**
   - Discovery ranking and dependency/impact preflight combined in list + detail overlays.
4. **Policy and Automation in TUI (BRD-005)**
   - Policy lifecycle and scheduled-run surfaces with output summaries.
5. **Coverage closeout (BRD-006)**
   - Maintain traceability matrix updates for each implemented requirement.

## Acceptance Baseline (TUI)

- All mutating TUI actions require explicit preflight and confirmation where risk applies.
- Every failure path in TUI includes remediation and retry/abort decisions.
- All ranking/safety/policy outputs are readable and explainable in TUI context.
- Discovery and updates are usable without a mouse and with deterministic command bindings.

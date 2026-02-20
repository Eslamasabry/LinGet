## BRD-004 — Dependency-Aware Safe Operations

### 0) TUI-First Delivery Scope
- Current implementation target: **TUI first**.
- Dependency graph extraction and impact previews are delivered in TUI preflight views and queue summaries first.
- GUI/CLI receive equivalent behavior later from shared domain services.

### 1) Strategic Intent
- Make destructive operations safe by surfacing dependency impact before execution and minimizing accidental breakage.
- Standardize dependency visibility and warnings across providers.

### 2) Current Problem
- Dependency and reverse-dependency effects are currently provider-specific and not always visible before execution.
- Users often discover consequences only after action completion.

### 3) Scope
- Preflight dependency analysis for install/remove/update operations where metadata is available.
- Impact preview UI and CLI reporting.
- Queue-level optimization to reduce conflict and duplicate work.

### 4) Success Definition
- Users can see and act on dependency impact for risky operations.
- Prevent preventable breakage by blocking or escalating critical removals by default.

### 5) Requirements

#### 5.1 Functional Requirements
- FR1: Add dependency graph extraction pipeline per provider (requires, optional, conflicts, reverse, optional-recommendations where possible).
- FR2: Add `what-if` mode that preview package outcomes before mutating actions.
- FR3: Add critical dependency blocking rules and severity labels (low/med/high).
- FR4: Show impact summary in queue preflight: direct removal/install effects and collateral changes.
- FR5: Implement provider capability matrix for dependency depth/accuracy.
- FR6: Add queue optimizer pass to minimize conflicting updates/removals order.
- FR7: Mark non-deterministic provider dependency data as best-effort and show uncertainty to user.

#### 5.2 Non-Functional Requirements
- NFR1: Dependency evaluation must be cancellable and non-blocking.
- NFR2: No UI freeze; background worker and progressive rendering of graph slices.
- NFR3: Timeout behavior: if dependency query exceeds threshold, present partial result + risk warning.

### 6) Frontend Requirements
- GUI
  - Add dependency impact drawer with requires/conflicts/reverse dependency trees.
  - Add explicit high-risk override flow for critical dependency removals.
  - Add package-impact score chips and “likely breakage” markers.
- TUI
  - Show compact impact tree with foldable sections and severity indicators.
  - Add key to open dependency reason for each queued action.
  - Warn on high-impact operations and require extra confirmation token.
- CLI
  - Add `--what-if` output for install/remove/update with dependency summary.
  - Include `impact_depth`, `critical_reverse_dependencies`, and `confidence` fields in JSON.

### 7) Data Model (Initial)
- `DependencyGraph`
  - `operation_id`, `target`, `provider`, `requires[]`, `conflicts[]`, `reverse[]`, `impact_estimate`, `confidence`, `generated_at`
- `DependencyPolicy`
  - `action`, `dependency_rule`, `min_impact`, `requires_override`, `provider_support_level`, `notes`

### 8) Risks
- Incomplete dependency metadata may produce optimistic previews.
- Queue-level optimization can change action ordering and surprise users if not clearly explained.
- Large dependency graphs can be expensive in very large source snapshots.

### 9) Acceptance Criteria
- P0: Remove operations show dependency impact when graph data is available.
- P0: High-impact removals require explicit acknowledgment in all interfaces.
- P1: Batch queue preview includes aggregate dependency impact and conflict count.
- P2: Best-effort mode is explicitly labeled and test-covered per provider class.

### 10) Additional Cross-Cutting Coverage
- RQ-29: Explicitly show certainty level for dependency data and fallback messaging.
- RQ-30: Cancellation behavior for dependency analysis jobs and long-running graph expansion.
- RQ-28: Track metadata freshness and stale dependency data windows by provider.

### 11) KPIs
- Fewer post-action rollbacks due to dependency fallout.
- Fewer “why did package X break” incidents.
- Increase in successful bulk operations without intervention.

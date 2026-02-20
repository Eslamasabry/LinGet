# LinGet TUI UX Review (Current State)

## Executive Verdict
The TUI is powerful but still confusing for normal users during queue recovery.
The main issue is not capability; it is decision clarity under failure.

## What Is Working
- Queue lanes (`now`, `next`, `needs attention`, `done`) are strong and trustworthy.
- Preflight + risk messaging improves confidence before execution.
- Failure filtering (`All`, `Permissions`, `Network`, `Conflict`, `Other`) is now visible and keyboard-accessible.

## Primary UX Problems
1. Action labels are still too abstract.
Current queue actions (`R`, `M`, `A`) require prior mental model.
User question remains: "What should I press right now?"

2. Too many simultaneous signals in expanded queue.
Task summary, filter tabs, footer commands, and logs compete for attention.

3. Failure-to-action path is not explicit enough.
Users see failures, but the best next button is not visually dominant.

4. Inconsistent wording across surfaces.
Some copy is user-friendly; some still sounds operator/internal.

## Must-Have UX Direction
### Goal
Reduce decision time in queue from "interpret" to "act".

### NBA (Next Best Action)
In expanded queue, always show one dominant recommendation line:
- `Recommended: Retry Safe (A)` when safe retries exist.
- `Recommended: Fix Filtered (M)` when guidance/recovery is needed.
- `Recommended: Review Selected Failure` when neither bulk path is actionable.

### CTA (Clear Prompt)
Show one direct prompt under recommendation:
- `Press A to retry safe failures now`
- `Press M to apply filtered fixes now`
- `Select a failed task and press R to retry`

## Priority Fixes
### P0
- Add a dedicated `Recommended` strip above logs in expanded queue.
- Keep only high-value actions in footer (`R`, `M`, `A`, filter keys).
- Replace ambiguous action text with outcome text:
  - `A retry safe failures`
  - `M fix filtered failures`

### P1
- Collapse secondary metrics by default in queue summary.
- Keep advanced performance hints behind compact wording.

### P2
- Add "novice hint mode" toggle in config for first-run guidance.

## Acceptance Criteria
- A first-time user can identify the best queue action in under 5 seconds.
- No line in expanded queue uses internal/jargon terms.
- For every failure filter state, exactly one recommendation is visible.
- `cargo test cli::tui::` and `cargo clippy -- -D warnings` stay green.

## Suggested Implementation Order
1. Add `recommended_queue_action()` rendering block in expanded queue.
2. Trim/footer simplify command labels.
3. Harmonize copy in help, decision card, and status messages.
4. Re-run TUI tests and snapshot expectations.

## Final Rating (User-Centric)
- Capability: 9/10
- Learnability: 6/10
- Action Clarity Under Failure: 5/10
- Overall UX (non-expert): 6/10

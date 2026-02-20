# BRD Index

This folder contains the Product Requirements Documents (BRDs) for the “ultimate package/apps manager” roadmap.

- `BRD-001-reliability-and-trust-foundation.md`
  - Core safety, dry-run, rollback, and trust guarantees.
- `BRD-002-unified-cross-interface-experience.md`
  - Shared CLI/TUI/GUI behavior, policies, and queue state contract.
- `BRD-003-discovery-and-recommendation-intelligence.md`
  - Search ranking, collections, recommendations, and explainability.
- `BRD-004-dependency-aware-safe-operations.md`
  - Dependency impact preview, what-if mode, and safety blocking.
- `BRD-005-update-policy-engine-automation-observability.md`
  - Policy profiles, scheduling, staged updates, and reporting.
- `BRD-006-requirements-completeness-and-gap-analysis.md`
  - Full requirement matrix and gap tracking for roadmap completeness.
- `BRD_TUI-ALL-REQUIREMENTS.md`
  - TUI-only execution scope and full-coverage mapping for all BRDs.
- `BRD_TUI_EPICS_AND_STORIES.md`
  - TUI-first epics and story-level decomposition from all BRD requirements.

## Common Notes

- All BRDs include frontend requirements for GUI/TUI/CLI.
- The frontends should consume shared backend action/queue policy engines where possible.
- Use the BRDs as implementation inputs for issue creation and roadmap sequencing.

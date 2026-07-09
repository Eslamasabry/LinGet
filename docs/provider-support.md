# Provider support tiers

LinGet reports a provider tier and plan fidelity before it mutates packages. A provider being detected does not mean every operation has the same safety guarantees.

| Tier | Providers in v0.2 | Guarantee |
| --- | --- | --- |
| Stable | APT, Flatpak, npm | Contract-tested command construction, structured errors, post-operation verification, and an explicit plan-fidelity label. |
| Beta | Other implemented providers | Useful for evaluation, but command semantics and verification coverage are not yet held to the Stable contract. Review the exact plan and provider output. |
| Detection-only | Providers shown as unavailable or unsupported at runtime | Discovery only. LinGet must not claim or attempt an unsupported mutation. |

Stable does not mean identical fidelity. APT can provide an exact simulated change set on supported Debian/Ubuntu systems. Flatpak and npm currently provide best-effort plans, because their available preview data is less complete. LinGet labels this difference instead of presenting an estimate as exact.

Provider failures should include the provider, operation, LinGet version, provider version, and a redacted error transcript. Use the provider issue form.

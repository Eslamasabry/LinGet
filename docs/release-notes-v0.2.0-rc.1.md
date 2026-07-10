# LinGet v0.2.0-rc.1

This is a terminal-first prerelease for a ten-person Linux evaluation cohort. Install it on a non-critical machine or disposable VM and review every proposed package change.

## What changed

- `linget` now opens the GTK-free terminal interface by default; the GUI is a separate optional artifact.
- The TUI has three real screens: Today, Browse, and Queue.
- APT, Flatpak, and npm operations use persisted reviewed plans and durable post-operation verification receipts.
- APT plans are exact when simulation succeeds. Flatpak and npm plans are explicitly labeled best effort.
- The layout remains usable from a degraded 60x15 terminal through wide layouts, with hidden-focus repair, `NO_COLOR`, high contrast, and `LINGET_TUI_ASCII=1` support.
- Release archives are deterministic for identical inputs and ship with checksums, SPDX and CycloneDX SBOMs, and GitHub build-provenance attestations.

## Install this prerelease

The generic `latest` installer intentionally follows stable releases. Install this cohort build explicitly:

```bash
curl -fsSLO https://github.com/Eslamasabry/LinGet/releases/download/v0.2.0-rc.1/linget-v0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz
curl -fsSLO https://github.com/Eslamasabry/LinGet/releases/download/v0.2.0-rc.1/linget-v0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz.sha256
sha256sum --check linget-v0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz.sha256
tar -xzf linget-v0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz
./linget-v0.2.0-rc.1-x86_64-unknown-linux-gnu/install.sh \
  --archive "$PWD/linget-v0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz"
```

Use the matching `aarch64-unknown-linux-gnu` archive on 64-bit ARM. The optional GUI archive is explicitly prefixed `linget-gui-`.

## Evaluation gate

Publishing this prerelease does not make v0.2 stable. Promotion remains blocked until the activation, plan-comprehension, verified-operation, safety, task-success, week-one retention, and usefulness thresholds in [the cohort scorecard](v0.2-cohort-scorecard.md) are evaluated.

Report provider failures with the provider issue form. Do not attach full package inventories, hostnames, usernames, or other identifying system output.

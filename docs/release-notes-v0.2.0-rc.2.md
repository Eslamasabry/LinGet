# LinGet v0.2.0-rc.2

This terminal-first prerelease replaces `v0.2.0-rc.1` for the ten-person Linux
evaluation cohort. Use it only on a non-critical machine or disposable VM, and
review every proposed package change before confirming.

## What changed since rc.1

- `linget cohort-report` creates an explicitly requested, local-only summary of
  provider readiness plus aggregate task and verification outcomes. It
  transmits nothing and excludes package details, identities, paths, commands,
  logs, raw receipts, and errors.
- TUI help now explains the Today → Browse → Queue safety workflow, retry
  boundaries, failure category codes, and the meaning of VERIFIED, MISMATCH,
  and INCONCLUSIVE receipts.
- The repository now includes a consent-safe participant/facilitator workflow,
  structured first-session and day-seven check-ins, a strict aggregate schema,
  and a deterministic stable-promotion decision tool.
- The README installer now follows the maintained `master` branch instead of
  the stale `main` branch, and package metadata points to the current project.
- The privacy-safe sharing loop is offered only after demonstrated usefulness
  and no safety concern; it uses no tracking links, rewards, contact access, or
  automatic messages.

## Install this prerelease

The generic `latest` installer intentionally follows stable releases. Install
this cohort build explicitly on x86_64 Linux:

```bash
curl -fsSLO https://github.com/Eslamasabry/LinGet/releases/download/v0.2.0-rc.2/linget-v0.2.0-rc.2-x86_64-unknown-linux-gnu.tar.gz
curl -fsSLO https://github.com/Eslamasabry/LinGet/releases/download/v0.2.0-rc.2/linget-v0.2.0-rc.2-x86_64-unknown-linux-gnu.tar.gz.sha256
sha256sum --check linget-v0.2.0-rc.2-x86_64-unknown-linux-gnu.tar.gz.sha256
tar -xzf linget-v0.2.0-rc.2-x86_64-unknown-linux-gnu.tar.gz
./linget-v0.2.0-rc.2-x86_64-unknown-linux-gnu/install.sh \
  --archive "$PWD/linget-v0.2.0-rc.2-x86_64-unknown-linux-gnu.tar.gz"
```

Use the matching `aarch64-unknown-linux-gnu` files on 64-bit ARM. The optional
GUI archive is explicitly prefixed `linget-gui-`; the cohort evaluates the
terminal artifact.

## Evaluation gate

Start with the [participant guide](cohort-v0.2/participant-guide.md) and run the
[ten-user operations kit](cohort-v0.2/README.md). Publishing this prerelease is
not evidence of usefulness or retention. Stable promotion remains blocked until
all gates in the [cohort scorecard](v0.2-cohort-scorecard.md) pass, including
zero unreviewed mutations and zero unexplained package changes.

# Cohort report privacy contract

`linget cohort-report` creates an explicitly requested summary for the v0.2
prerelease cohort. LinGet does not create it in the background and does not
transmit it. The participant decides whether to save or share it.

## Included

- LinGet version and report schema version.
- Supported provider names, whether each provider was detected, its published
  support tier and plan fidelity, and whether it is ready for a verified cohort
  operation.
- Aggregate queued, running, completed, failed, and cancelled task counts.
- Aggregate verified, mismatch, inconclusive, and unreadable receipt counts.
- Generic next steps derived only from those aggregate states.

## Excluded

- Package names, IDs, versions, inventories, and expected or observed changes.
- Usernames, hostnames, email addresses, and other participant identifiers.
- Filesystem and executable paths.
- Commands, provider version output, errors, logs, warnings, and other arbitrary
  command output.
- Operation, plan, task, or receipt identifiers and timestamps.

Provider detection uses local executable checks but the report retains only a
boolean readiness result. Receipt parsing reads LinGet's saved task history but
retains only outcome counts. An unreadable history or receipt is reported as a
count/status; its content and parse error are never copied into the report.

## Usage

Print a readable local summary:

```bash
linget cohort-report
```

Print the complete machine-readable schema:

```bash
linget cohort-report --format json
```

Write that JSON to a participant-selected file:

```bash
linget cohort-report --output cohort-report.json
```

Always review the file before sharing it. Use an anonymous participant ID in
the separate cohort scorecard; the report intentionally does not create or
store one.

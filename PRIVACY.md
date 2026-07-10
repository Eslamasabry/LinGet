# Privacy

LinGet does not send product analytics, package inventories, command history, crash reports, or other telemetry to the LinGet project.

`linget cohort-report` is an explicitly invoked, local-only summary. It reads
provider readiness and saved LinGet outcomes, retains only aggregate counts,
and transmits nothing. Review any generated file before choosing to share it;
the exact field contract is documented in [docs/cohort-report.md](docs/cohort-report.md).

LinGet runs package-manager commands on your machine and may contact the repositories configured for those package managers. Those network requests and their logs are governed by the provider and repository operator, not LinGet. GitHub receives ordinary request metadata when you download LinGet releases or use GitHub issue forms.

Diagnostic output can contain usernames, paths, repositories, and package names. Review and redact it before sharing. Never attach credentials, tokens, private registry URLs, or an unreviewed full inventory to an issue.

If opt-in telemetry is added in the future, it will be documented here before release and will remain disabled by default.

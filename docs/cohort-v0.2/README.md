# LinGet v0.2 ten-user cohort operations kit

This directory contains the complete, consent-safe operating kit for deciding whether `v0.2.0-rc.2` is ready for stable promotion. The process measures use of the terminal-first workflow; it is not a request for more repository downloads.

## Operating sequence

1. Recruit exactly ten Linux participants with the copy in [outreach.md](outreach.md).
2. Give every participant [participant-guide.md](participant-guide.md) before consent.
3. Run the first session with [facilitator-guide.md](facilitator-guide.md) and the structured [first-session check-in](first-session-check-in.md).
4. Run the structured [day-seven check-in](day-seven-check-in.md) seven days later.
5. Add counts, never row-level responses, to a local copy of the [aggregate schema](cohort-scorecard.schema.json).
6. Generate the deterministic decision:

   ```bash
   python3 scripts/cohort_report.py private-aggregate-counts.json --output cohort-decision.md
   ```

7. Promote only when the report says `PROMOTE — ALL STABLE-PROMOTION GATES PASSED`. A safety report always blocks promotion.
8. Offer the [privacy-safe sharing loop](sharing-loop.md) only to eligible participants. Sharing is never a condition of participation.

## Data boundary

The public repository may contain only the blank structured forms, synthetic fixtures, and aggregate decision report. Never commit raw check-ins or participant-level rows.

Do not request or retain names, package inventories, commands or receipts, hostnames, usernames, email addresses, contact lists, IP addresses, or identifying free text. Recruitment happens through an existing communication channel; the evaluation dataset does not copy or link that account identity. Participants keep their own random session code for the day-seven check-in.

Delete temporary structured responses after their counts have been checked by two people and aggregated. Keep only the aggregate JSON and generated decision report. If a participant withdraws before aggregation, discard that response and recruit a replacement; never infer missing answers.

## Dry run

The fixtures exercise promotion, safety hold, and incomplete-evidence hold outcomes:

```bash
python3 -m unittest discover -s tests -p 'test_cohort_report.py'
python3 scripts/cohort_report.py tests/fixtures/cohort-scorecard-pass.json
python3 scripts/cohort_report.py tests/fixtures/cohort-scorecard-hold-safety.json
python3 scripts/cohort_report.py tests/fixtures/cohort-scorecard-hold-incomplete.json
```

Fixtures are synthetic and must never be replaced with participant-level data.

Do not create public GitHub issues for individual check-ins: GitHub account identity makes that route unsuitable for anonymous participation. A facilitator may file an aggregate gate failure after removing cohort codes and all participant- or machine-specific details.

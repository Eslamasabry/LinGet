# Facilitator guide

This script standardizes ten first sessions and ten day-seven check-ins. Facilitate the process; do not coach participants toward a passing result.

## Before recruitment

1. Use the published `v0.2.0-rc.2` terminal artifact and confirm its checksum instructions still work.
2. Configure the local form to generate a random six-character cohort code after consent. The participant keeps the code for day seven. Do not keep a code-to-person mapping.
3. Use an access-controlled temporary form that accepts only the choices in the two check-in templates. Disable telemetry, account collection, and free-text fields.
4. Schedule the same day-seven message to the recruitment channel without exporting contact details.

## Opening script

> LinGet is experimental and package operations can damage a system. Please use a non-critical machine or VM. You can stop at any time, skip any operation, or withdraw without explanation. We record only structured outcomes under the random code you keep; we do not record your screen, commands, packages, receipt, machine details, contact details, or spoken wording. Do you understand the risks and consent to continue?

If the answer is not an unambiguous yes, stop and store nothing.

## First-session script

1. Share the participant guide and remain silent during installation unless a safety stop is needed.
2. Ask the participant to launch `linget` and say when provider detection and Today are visible.
3. Ask: “What does the reviewed plan say will happen if you continue?” Do not transcribe the answer.
4. Mark plan comprehension `pass` only if the participant identifies the intended operation, target scope, provider or source, and whether the plan is exact or best effort. Otherwise mark `not yet`.
5. Ask them to choose an operation they already intended. A read-only review or cancellation is valid participation but does not count as a verified operation.
6. Intervene only to prevent likely harm, answer a consent question, or recover from a test-environment failure. Any product-navigation hint means task success is `with intervention`.
7. Stop immediately on an unexpected privilege request, change before explicit confirmation, unexplained package change, or participant concern.
8. After the participant closes LinGet, administer the structured first-session check-in. Do not ask for diagnostic output.

## Classification rules

- **Activated:** Today is visible after provider detection without facilitator navigation.
- **Plan understood:** the spoken answer meets all four rubric elements above.
- **Verified operation:** one intended install, update, or removal ends with a verification receipt. Do not store the receipt.
- **Task success without intervention:** the participant reaches the intended maintenance outcome without a product-navigation hint. A safe cancellation is valuable safety evidence but does not count as task success. Installation-environment help does not count as navigation help but must be classified separately in the temporary form.
- **Safety report:** count `yes` or `unsure` for a mutation before confirmation or an unexplained change. One report blocks promotion; never investigate by requesting an inventory.
- **Useful:** the participant selects 4 or 5 at day seven.
- **Returned:** the participant independently used LinGet again before the day-seven check-in.

## Day seven and aggregation

1. Send the same neutral day-seven prompt to everyone. Do not send extra reminders based on prior answers.
2. Offer sharing only when the eligibility rule in [sharing-loop.md](sharing-loop.md) passes.
3. Two people independently total each structured choice. Reconcile counts without copying rows into the repository.
4. Treat `unsure` safety responses as reports. Enter aggregate totals in the strict JSON format, run `scripts/cohort_report.py`, and compare the output from both reviewers.
5. Delete temporary participant-level responses after reconciliation. Commit only aggregate counts and the deterministic report.

If the report says hold, create privacy-scrubbed technical issues for failed product gates. Do not put cohort codes, spoken feedback, or participant metadata in issues.

Never ask participants to submit a cohort check-in through a public GitHub issue. The account attached to an issue is identifying. Public issues are only for facilitator-authored aggregate findings or independently reported technical bugs after privacy scrubbing.

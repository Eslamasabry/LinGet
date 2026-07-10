# Participant guide: LinGet v0.2 prerelease

Thank you for considering a LinGet evaluation session. LinGet is experimental package-management software. Participation is voluntary, you may stop at any time, and choosing not to participate has no consequence.

## What the session involves

- About 20 minutes for installation, a terminal walkthrough, and one structured check-in.
- An optional structured check-in seven days later, about 3 minutes.
- Use only a non-critical Linux machine or disposable virtual machine.
- Choose a package operation you already intended to do. You never need to change a system just to complete the evaluation.
- You control every confirmation. Stop if a plan is surprising, unclear, asks for unexpected privilege, or includes an unintended change.

The facilitator records only structured yes/no or multiple-choice outcomes under a random code. They do not collect package names, package inventories, terminal output, receipts, hostnames, usernames, email addresses, or free-text feedback. You may describe your experience aloud; it is classified by the rubric and not transcribed.

## Consent

Proceed only if all are true:

- I understand this is experimental software and I will use a non-critical system or VM.
- I understand that package operations can change or damage a system.
- I know I can stop before or during any operation without explanation.
- I consent to anonymous structured results being combined into aggregate counts.
- I know that raw structured responses will be deleted after aggregation.

If any statement is not true, do not continue.

## Install the explicit prerelease

The generic installer follows stable releases, so this cohort must use the explicit `v0.2.0-rc.1` archive. For x86_64 Linux:

```bash
curl -fsSLO https://github.com/Eslamasabry/LinGet/releases/download/v0.2.0-rc.1/linget-v0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz
curl -fsSLO https://github.com/Eslamasabry/LinGet/releases/download/v0.2.0-rc.1/linget-v0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz.sha256
sha256sum --check linget-v0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz.sha256
tar -xzf linget-v0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz
./linget-v0.2.0-rc.1-x86_64-unknown-linux-gnu/install.sh \
  --archive "$PWD/linget-v0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz"
```

Use the matching `aarch64-unknown-linux-gnu` files on 64-bit ARM. Do not use the `linget-gui-*` artifact for this terminal-interface cohort.

## Session task

1. Run `linget` and wait for provider detection.
2. Find Today and explain aloud what it recommends.
3. Navigate to a package action you already intended to perform.
4. Review the provider, operation, plan fidelity, expected scope, and privilege requirement.
5. Confirm only if the plan matches your intent. Otherwise cancel; a safe cancellation is useful evidence.
6. If you proceed, inspect the final verification receipt.
7. Complete the structured check-in without pasting any command, package name, output, or receipt.

Support and public issue forms are linked from the repository. Before reporting a technical defect publicly, remove participant codes and all machine- or account-specific data.

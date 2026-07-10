#!/usr/bin/env python3
"""Validate aggregate LinGet cohort results and render a promotion decision."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from decimal import Decimal, ROUND_HALF_UP
from pathlib import Path
from typing import Any


SCHEMA_VERSION = 1
EXPECTED_KEYS = {
    "schema_version",
    "cohort_size",
    "first_session_responses",
    "day_seven_responses",
    "activated",
    "plan_understood",
    "verified_operation",
    "unreviewed_mutation_reports",
    "unexplained_change_reports",
    "task_success_without_intervention",
    "returned_by_day_7",
    "usefulness_4_or_5",
    "share_prompt_eligible",
    "share_prompt_offered",
    "participants_chose_to_share",
    "referred_first_sessions",
    "referred_activations",
}
COHORT_BOUNDED_KEYS = EXPECTED_KEYS - {
    "schema_version",
    "cohort_size",
    "unreviewed_mutation_reports",
    "unexplained_change_reports",
    "referred_first_sessions",
    "referred_activations",
}


class ValidationError(ValueError):
    """Raised when a cohort aggregate violates the public data contract."""


@dataclass(frozen=True)
class Gate:
    label: str
    value: int
    threshold: int
    safety: bool = False

    @property
    def passed(self) -> bool:
        return self.value == 0 if self.safety else self.value >= self.threshold

    @property
    def result(self) -> str:
        return "PASS" if self.passed else "FAIL"

    @property
    def comparison(self) -> str:
        if self.safety:
            return f"{self.value} reports (required: 0)"
        return f"{self.value}/10 (required: at least {self.threshold})"


def _require_integer(data: dict[str, Any], key: str) -> int:
    value = data[key]
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValidationError(f"{key} must be an integer")
    if value < 0:
        raise ValidationError(f"{key} must be non-negative")
    return value


def validate(data: Any) -> dict[str, int]:
    if not isinstance(data, dict):
        raise ValidationError("input must be a JSON object")

    keys = set(data)
    missing = sorted(EXPECTED_KEYS - keys)
    unexpected = sorted(keys - EXPECTED_KEYS)
    if missing:
        raise ValidationError(f"missing fields: {', '.join(missing)}")
    if unexpected:
        raise ValidationError(
            "unexpected fields are forbidden to keep the input aggregate-only: "
            + ", ".join(unexpected)
        )

    clean = {key: _require_integer(data, key) for key in EXPECTED_KEYS}
    if clean["schema_version"] != SCHEMA_VERSION:
        raise ValidationError(
            f"schema_version must be {SCHEMA_VERSION}, got {clean['schema_version']}"
        )
    if clean["cohort_size"] != 10:
        raise ValidationError("cohort_size must be exactly 10")

    for key in COHORT_BOUNDED_KEYS:
        if clean[key] > clean["cohort_size"]:
            raise ValidationError(f"{key} cannot exceed cohort_size")

    if clean["share_prompt_offered"] > clean["share_prompt_eligible"]:
        raise ValidationError("share_prompt_offered cannot exceed share_prompt_eligible")
    if clean["participants_chose_to_share"] > clean["share_prompt_offered"]:
        raise ValidationError(
            "participants_chose_to_share cannot exceed share_prompt_offered"
        )
    if clean["referred_activations"] > clean["referred_first_sessions"]:
        raise ValidationError(
            "referred_activations cannot exceed referred_first_sessions"
        )
    return clean


def gates(data: dict[str, int]) -> list[Gate]:
    return [
        Gate("First-session activation", data["activated"], 8),
        Gate("Plan comprehension", data["plan_understood"], 8),
        Gate("Verified operation", data["verified_operation"], 7),
        Gate("No unreviewed mutations", data["unreviewed_mutation_reports"], 0, True),
        Gate("No unexplained package changes", data["unexplained_change_reports"], 0, True),
        Gate("Task success without intervention", data["task_success_without_intervention"], 8),
        Gate("Week-one retention", data["returned_by_day_7"], 4),
        Gate("Usefulness rated 4 or 5", data["usefulness_4_or_5"], 7),
    ]


def decision(data: dict[str, int], evaluated_gates: list[Gate]) -> tuple[str, str]:
    safety_failed = any(gate.safety and not gate.passed for gate in evaluated_gates)
    evidence_complete = (
        data["first_session_responses"] == data["cohort_size"]
        and data["day_seven_responses"] == data["cohort_size"]
    )
    non_safety_failed = any(not gate.safety and not gate.passed for gate in evaluated_gates)

    if safety_failed:
        return "HOLD", "SAFETY GATE FAILED"
    if not evidence_complete:
        return "HOLD", "EVIDENCE INCOMPLETE"
    if non_safety_failed:
        return "HOLD", "ONE OR MORE PROMOTION GATES FAILED"
    return "PROMOTE", "ALL STABLE-PROMOTION GATES PASSED"


def _percent(numerator: int, denominator: int) -> str:
    if denominator == 0:
        return "n/a"
    value = (Decimal(numerator) * 100 / Decimal(denominator)).quantize(
        Decimal("0.1"), rounding=ROUND_HALF_UP
    )
    return f"{value}%"


def _ratio(numerator: int, denominator: int) -> str:
    if denominator == 0:
        return "n/a"
    value = (Decimal(numerator) / Decimal(denominator)).quantize(
        Decimal("0.01"), rounding=ROUND_HALF_UP
    )
    return str(value)


def render_markdown(data: dict[str, int]) -> str:
    evaluated_gates = gates(data)
    action, reason = decision(data, evaluated_gates)
    lines = [
        "# LinGet v0.2 cohort decision",
        "",
        f"Decision: **{action} — {reason}**",
        "",
        "This report contains aggregate counts only. It must not be used to store participant identities, package inventories, hostnames, usernames, email addresses, or free-text feedback.",
        "",
        "## Evidence completeness",
        "",
        f"- First-session check-ins: {data['first_session_responses']}/{data['cohort_size']}",
        f"- Day-seven check-ins: {data['day_seven_responses']}/{data['cohort_size']}",
        "",
        "## Promotion gates",
        "",
        "| Gate | Result | Evidence |",
        "| --- | --- | ---: |",
    ]
    for gate in evaluated_gates:
        lines.append(f"| {gate.label} | {gate.result} | {gate.comparison} |")

    lines.extend(
        [
            "",
            "## Privacy-safe sharing loop",
            "",
            f"- Eligible for optional sharing prompt: {data['share_prompt_eligible']}",
            f"- Offered the prompt: {data['share_prompt_offered']}",
            f"- Chose to share: {data['participants_chose_to_share']} ({_percent(data['participants_chose_to_share'], data['share_prompt_offered'])} of offers)",
            f"- Referred first sessions: {data['referred_first_sessions']}",
            f"- Referred activations: {data['referred_activations']} ({_percent(data['referred_activations'], data['referred_first_sessions'])} of referred first sessions)",
            f"- Referral activation coefficient: {_ratio(data['referred_activations'], data['cohort_size'])} per cohort participant",
            "",
            "## Required next action",
            "",
        ]
    )
    if action == "PROMOTE":
        lines.append(
            "Create the stable-release promotion change, attach this aggregate report, and preserve the prerelease safety language until the stable artifacts pass release verification."
        )
    elif reason == "SAFETY GATE FAILED":
        lines.append(
            "Do not promote. File a private, redacted safety investigation without participant or machine identifiers, fix the cause, and run a new cohort."
        )
    elif reason == "EVIDENCE INCOMPLETE":
        lines.append(
            "Do not promote. Complete the missing structured check-ins or begin a fresh ten-person cohort; do not infer missing responses."
        )
    else:
        lines.append(
            "Do not promote. Create one issue per failed gate using only aggregate evidence and reproducible, privacy-scrubbed technical details, then run a new cohort after fixes."
        )
    return "\n".join(lines) + "\n"


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate a deterministic stable-promotion decision from aggregate cohort counts."
    )
    parser.add_argument("input", type=Path, help="aggregate cohort JSON file")
    parser.add_argument(
        "--output", type=Path, help="write Markdown report here instead of stdout"
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        raw = json.loads(args.input.read_text(encoding="utf-8"))
        data = validate(raw)
        report = render_markdown(data)
    except (OSError, json.JSONDecodeError, ValidationError) as error:
        print(f"cohort-report: {error}", file=sys.stderr)
        return 2

    if args.output:
        args.output.write_text(report, encoding="utf-8")
    else:
        sys.stdout.write(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

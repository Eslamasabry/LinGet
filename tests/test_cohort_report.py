import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "cohort_report.py"
FIXTURES = ROOT / "tests" / "fixtures"
SCHEMA = ROOT / "docs" / "cohort-v0.2" / "cohort-scorecard.schema.json"

SPEC = importlib.util.spec_from_file_location("cohort_report", SCRIPT)
assert SPEC and SPEC.loader
cohort_report = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = cohort_report
SPEC.loader.exec_module(cohort_report)


class CohortReportTests(unittest.TestCase):
    def load(self, name: str) -> dict[str, int]:
        return json.loads((FIXTURES / name).read_text(encoding="utf-8"))

    def test_pass_fixture_promotes(self) -> None:
        report = cohort_report.render_markdown(
            cohort_report.validate(self.load("cohort-scorecard-pass.json"))
        )
        self.assertIn("**PROMOTE — ALL STABLE-PROMOTION GATES PASSED**", report)
        self.assertEqual(report.count("| PASS |"), 8)
        self.assertIn("Referral activation coefficient: 0.10", report)

    def test_safety_failure_overrides_perfect_product_metrics(self) -> None:
        report = cohort_report.render_markdown(
            cohort_report.validate(self.load("cohort-scorecard-hold-safety.json"))
        )
        self.assertIn("**HOLD — SAFETY GATE FAILED**", report)
        self.assertIn("Do not promote", report)

    def test_incomplete_day_seven_evidence_holds(self) -> None:
        report = cohort_report.render_markdown(
            cohort_report.validate(self.load("cohort-scorecard-hold-incomplete.json"))
        )
        self.assertIn("**HOLD — EVIDENCE INCOMPLETE**", report)

    def test_unknown_fields_are_rejected(self) -> None:
        data = self.load("cohort-scorecard-pass.json")
        data["participant_email"] = "not-allowed"
        with self.assertRaisesRegex(
            cohort_report.ValidationError, "aggregate-only"
        ):
            cohort_report.validate(data)

    def test_inconsistent_sharing_counts_are_rejected(self) -> None:
        data = self.load("cohort-scorecard-pass.json")
        data["participants_chose_to_share"] = 8
        with self.assertRaisesRegex(
            cohort_report.ValidationError, "cannot exceed share_prompt_offered"
        ):
            cohort_report.validate(data)

    def test_schema_and_script_accept_the_same_exact_fields(self) -> None:
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        self.assertEqual(set(schema["required"]), cohort_report.EXPECTED_KEYS)
        self.assertEqual(set(schema["properties"]), cohort_report.EXPECTED_KEYS)
        self.assertFalse(schema["additionalProperties"])

    def test_cli_output_is_deterministic(self) -> None:
        fixture = FIXTURES / "cohort-scorecard-pass.json"
        first = subprocess.run(
            [sys.executable, str(SCRIPT), str(fixture)],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
        second = subprocess.run(
            [sys.executable, str(SCRIPT), str(fixture)],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
        self.assertEqual(first, second)

    def test_cli_can_write_report(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "decision.md"
            subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    str(FIXTURES / "cohort-scorecard-pass.json"),
                    "--output",
                    str(output),
                ],
                check=True,
            )
            self.assertTrue(output.read_text(encoding="utf-8").endswith("\n"))


if __name__ == "__main__":
    unittest.main()

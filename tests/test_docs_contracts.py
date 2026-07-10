import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class DocumentationContractsTests(unittest.TestCase):
    def text(self, relative_path: str) -> str:
        return (ROOT / relative_path).read_text(encoding="utf-8")

    def test_quick_installer_uses_current_default_branch(self) -> None:
        readme = self.text("README.md")
        expected = (
            "https://raw.githubusercontent.com/Eslamasabry/LinGet/master/install.sh"
        )
        self.assertIn(expected, readme)
        self.assertNotIn(
            "https://raw.githubusercontent.com/Eslamasabry/LinGet/main/install.sh",
            readme,
        )

    def test_contributor_links_match_repository_contract(self) -> None:
        contributing = self.text("CONTRIBUTING.md")
        self.assertIn("https://github.com/Eslamasabry/LinGet.git", contributing)
        self.assertIn("branch from `master`", contributing)
        self.assertNotIn("https://github.com/linget/linget", contributing)
        self.assertNotIn("branch from `main`", contributing)

    def test_cohort_install_is_pinned_to_the_prerelease(self) -> None:
        guide = self.text("docs/cohort-v0.2/participant-guide.md")
        release_prefix = (
            "https://github.com/Eslamasabry/LinGet/releases/download/v0.2.0-rc.1/"
        )
        self.assertGreaterEqual(guide.count(release_prefix), 2)
        self.assertNotIn("raw.githubusercontent.com", guide)

    def test_internal_markdown_links_resolve(self) -> None:
        files = list((ROOT / "docs" / "cohort-v0.2").glob("*.md"))
        files.extend(
            [
                ROOT / "docs" / "v0.2-cohort-scorecard.md",
                ROOT / "docs" / "release-notes-v0.2.0-rc.1.md",
            ]
        )
        for document in files:
            for target in re.findall(r"\[[^]]+\]\(([^)]+)\)", document.read_text()):
                if target.startswith(("https://", "http://", "#")):
                    continue
                resolved = (document.parent / target.split("#", 1)[0]).resolve()
                self.assertTrue(
                    resolved.exists(),
                    f"{document.relative_to(ROOT)} has broken link {target}",
                )


if __name__ == "__main__":
    unittest.main()

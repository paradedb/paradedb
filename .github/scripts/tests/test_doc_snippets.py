"""Regressions for missing, overwritten and silently untested examples."""

import importlib.util
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "extract_code_snippets.py"
spec = importlib.util.spec_from_file_location("extractor", SCRIPT)
extractor = importlib.util.module_from_spec(spec)
spec.loader.exec_module(extractor)


class DocumentationCoverageTests(unittest.TestCase):
    """Guard against silent gaps and stale coverage decisions."""

    def group(self):
        """Build one application CodeGroup with all supported variants."""
        labels = [
            "sql SQL",
            "python Django",
            "python SQLAlchemy",
            "ruby Rails",
            "ts Drizzle",
            "cs EF Core",
        ]
        return (
            "<CodeGroup>\n"
            + "\n".join(f"```{label}\nexample\n```" for label in labels)
            + "\n</CodeGroup>"
        )

    def test_all_variants_are_required(self):
        """Reject a missing ORM tab."""
        with self.assertRaisesRegex(ValueError, "rails"):
            extractor.extract_snippets(
                self.group().replace("```ruby Rails\nexample\n```", "")
            )

    def test_duplicate_variants_cannot_overwrite(self):
        """Reject a second example for the same target."""
        with self.assertRaisesRegex(ValueError, "Duplicate sql"):
            extractor.extract_snippets(self.group() + "\n```sql SQL\nsecond\n```")

    def test_unknown_language_does_not_silently_disappear(self):
        """Reject an unrecognized CodeGroup language."""
        with self.assertRaisesRegex(ValueError, "Unrecognized"):
            extractor.extract_snippets(
                self.group().replace("ts Drizzle", "javascript Drizzle")
            )

    def test_inventory_covers_tabs_and_standalone_but_excludes_history(self):
        """Inventory setup and standalone fences, excluding release history."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "start.mdx").write_text(
                (
                    '<Tabs><Tab title="Django">\n```python models.py\nclass Model: pass'
                    "\n```\n</Tab></Tabs>\n```sql\nSELECT 1;\n```\n"
                )
                + self.group()
            )
            history = root / "project/changelog"
            history.mkdir(parents=True)
            (history / "0.1.mdx").write_text("```sql\nobsolete API\n```")
            groups, outside = extractor.inventory(root)
            self.assertEqual(len(groups), 1)
            self.assertEqual(len(outside), 2)
            self.assertEqual(
                {entry["info"] for entry in outside.values()},
                {"python models.py", "sql"},
            )

    def test_new_snippet_requires_coverage_decision(self):
        """Reject newly added fences without a reviewed decision."""
        with self.assertRaisesRegex(ValueError, "Unclassified"):
            extractor.validate_coverage({"new": {"sha256": "hash"}}, {})

    def test_changed_excluded_snippet_must_be_reviewed(self):
        """Require review when an excluded example changes."""
        with self.assertRaisesRegex(ValueError, "Snippet changed"):
            extractor.validate_coverage(
                {"sample": {"sha256": "new"}},
                {
                    "sample": {
                        "sha256": "old",
                        "mode": "skip",
                        "reason": "Requires another database",
                    }
                },
            )

    def test_unknown_fixture_cannot_pass(self):
        """Reject scenario dependencies that do not exist."""
        with self.assertRaisesRegex(ValueError, "Unknown scenario fixture"):
            extractor.validate_coverage(
                {"sample": {"sha256": "digest"}},
                {
                    "sample": {
                        "sha256": "digest",
                        "mode": "sql",
                        "reason": "Executable",
                        "setup": ["missing"],
                    }
                },
            )

    def test_setup_mode_requires_registered_scenario(self):
        """Reject setup classifications outside the documented tutorial runner."""
        with self.assertRaisesRegex(ValueError, "No setup scenario"):
            extractor.validate_coverage(
                {"reference/example.mdx::fence-001": {"sha256": "digest"}},
                {
                    "reference/example.mdx::fence-001": {
                        "sha256": "digest",
                        "mode": "setup",
                        "reason": "Tutorial",
                    }
                },
            )

    def test_language_changes_invalidate_inventory_digest(self):
        """Require coverage review when only the fence language changes."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            page = root / "example.mdx"
            page.write_text("```python\nexample\n```\n")
            _, before = extractor.inventory(root)
            page.write_text("```javascript\nexample\n```\n")
            _, after = extractor.inventory(root)
            key = "example.mdx::fence-001"
            self.assertNotEqual(before[key]["sha256"], after[key]["sha256"])


if __name__ == "__main__":
    unittest.main()

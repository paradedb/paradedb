"""Regression coverage for retrying partially completed benchmark publications."""

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from benchmark_publication import PREFIX, prepare


class BenchmarkPublicationTests(unittest.TestCase):
    """A pushed metric must survive failures in the rest of its publication job."""

    def setUp(self):
        # unittest closes this context after each test.
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))  # pylint: disable=consider-using-with
        self.history = self.root / "data.js"
        self.comparison = self.root / "comparison.js"
        self.baseline = self.entry("baseline", 100)
        self.candidate = self.entry("candidate", 150)

    @staticmethod
    def entry(commit, value):
        """Build a metric usable by the real regression comparison script."""
        return {
            "commit": {"id": commit},
            "benches": [{"name": "query latency", "unit": "ms", "value": value}],
        }

    def write(self, entries):
        """Write a history in github-action-benchmark's format."""
        self.history.write_text(PREFIX + json.dumps({"entries": {"suite": entries}}))

    def entries(self, path):
        """Read the suite from a history or comparison snapshot."""
        return json.loads(path.read_text()[len(PREFIX) :])["entries"]["suite"]

    def test_retry_after_push_preserves_regression_with_later_publications(self):
        """A missing workflow marker must not turn the candidate into its baseline."""
        self.write([self.baseline, self.candidate])
        prepare(self.history, "suite", "123-queries", self.comparison, record=True)
        published = self.entries(self.history)
        # Another run publishes before this job is retried.
        self.write(published + [self.entry("newer", 120)])
        before = self.history.read_bytes()
        self.assertTrue(prepare(self.history, "suite", "123-queries", self.comparison))
        self.assertEqual(before, self.history.read_bytes())
        self.assertEqual(self.entries(self.comparison), published)

        results = self.root / "results.json"
        results.write_text(json.dumps(self.candidate["benches"]))
        script = (
            Path(__file__).resolve().parents[1]
            / "actions/publish-queries-results/compare_baseline.py"
        )
        subprocess.run(
            [
                "python3",
                str(script),
                "--results",
                str(results),
                "--data-js",
                str(self.comparison),
                "--suite",
                "suite",
                "--out",
                str(self.root / "alert.md"),
            ],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn("1.50", (self.root / "alert.md").read_text())

    def test_metrics_are_independent_and_same_commit_runs_are_distinct(self):
        """A successful TPS push must not skip a failed Other Metrics push."""
        self.write([self.baseline, self.candidate])
        prepare(self.history, "suite", "123-tps", self.comparison, record=True)
        self.assertFalse(prepare(self.history, "suite", "123-other", self.comparison))
        self.assertFalse(prepare(self.history, "suite", "124-tps", self.comparison))
        # A new artifact may benchmark the same SHA; it still gets a separate result.
        self.write(self.entries(self.history) + [self.candidate])
        prepare(self.history, "suite", "124-tps", self.comparison, record=True)
        self.assertEqual(len(self.entries(self.history)), 3)
        self.assertEqual(self.entries(self.comparison)[0]["publication_id"], "123-tps")

    def test_first_publication_has_no_baseline(self):
        """Retrying the first entry must not invent a comparison against itself."""
        self.write([self.candidate])
        prepare(self.history, "suite", "123-queries", self.comparison, record=True)
        self.assertTrue(prepare(self.history, "suite", "123-queries", self.comparison))
        self.assertEqual(len(self.entries(self.comparison)), 1)

    def test_missing_history_and_invalid_history(self):
        """Missing history is a miss, while corrupt history fails before a push."""
        self.assertFalse(prepare(self.history, "suite", "123-queries", self.comparison))
        with self.assertRaises(ValueError):
            prepare(self.history, "suite", "123-queries", self.comparison, record=True)
        self.history.write_text("invalid")
        with self.assertRaises(ValueError):
            prepare(self.history, "suite", "123-queries", self.comparison)

    def test_id_and_result_are_pushed_in_the_same_commit(self):
        """Recover after a real Git push without any workflow completion marker."""
        remote = self.root / "remote.git"
        checkout = self.root / "checkout"
        subprocess.run(
            ["git", "init", "--bare", str(remote)], check=True, capture_output=True
        )
        subprocess.run(["git", "init", str(checkout)], check=True, capture_output=True)

        def git(*args):
            return subprocess.run(
                [
                    "git",
                    "-c",
                    "user.name=Test",
                    "-c",
                    "user.email=test@example.com",
                    "-c",
                    "commit.gpgsign=false",
                    *args,
                ],
                cwd=checkout,
                check=True,
                capture_output=True,
                text=True,
            ).stdout.strip()

        history = checkout / "data.js"
        history.write_text(PREFIX + json.dumps({"entries": {"suite": [self.baseline]}}))
        git("add", ".")
        git("commit", "-m", "baseline")
        git("push", str(remote), "HEAD:gh-pages")
        history.write_text(
            PREFIX + json.dumps({"entries": {"suite": [self.baseline, self.candidate]}})
        )
        git("add", ".")
        git("commit", "-m", "upstream metric commit")
        prepare(history, "suite", "123-queries", self.comparison, record=True)
        git("add", "data.js")
        git("commit", "--amend", "--no-edit")
        git("push", str(remote), "HEAD:gh-pages")
        # Lose the runner after the push, before the workflow marker can be uploaded.
        restored = self.root / "restored"
        subprocess.run(
            ["git", "clone", "--branch", "gh-pages", str(remote), str(restored)],
            check=True,
            capture_output=True,
        )
        self.assertTrue(
            prepare(restored / "data.js", "suite", "123-queries", self.comparison)
        )
        self.assertEqual(len(self.entries(restored / "data.js")), 2)
        self.assertEqual(self.entries(self.comparison)[0], self.baseline)


if __name__ == "__main__":
    unittest.main()

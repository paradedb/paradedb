"""Protect query pairing and config compatibility in mainline comparisons."""

import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from compare_search import compare, search_reports
from prepare_k6 import QUERIES, SUITES


class CompareSearchTest(unittest.TestCase):
    """Exercise matched-recall comparisons for both vector suites."""

    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.root)
        self.baseline = self.root / "baseline"
        self.current = self.root / "current"
        self.output = self.root / "comparison"
        for directory, queries in (
            (self.baseline, QUERIES),
            (self.current, reversed(QUERIES)),
        ):
            directory.mkdir()
            # Timestamp order differs across runs; pair on query, not filename.
            for index, query in enumerate(queries):
                report = {
                    "kind": "search",
                    "profile": "1m",
                    "fingerprint": query,
                    "config": {"search": {"query": query}},
                    "data": {"variants": {query: {}}},
                }
                (directory / f"{index}_search.json").write_text(json.dumps(report))

    def compare(self):
        """Compare fixtures with fixed commit provenance."""
        compare(
            self.baseline,
            self.current,
            self.output,
            {
                "baseline_commit": "a" * 40,
                "baseline_url": "https://github.com/paradedb/paradedb/actions/runs/123",
                "current_commit": "b" * 40,
            },
        )

    @patch("compare_search.subprocess.run")
    def test_pairs_by_query_and_labels_commits_without_modifying_sources(self, run):
        """Pair on query identity and retain original reports."""
        before = search_reports(self.baseline)
        self.compare()
        self.assertEqual(run.call_count, 3)
        for call in run.call_args_list:
            command = call.args[0]
            self.assertEqual(command[:2], ["pdbench", "compare"])
            base = json.loads(Path(command[2]).read_text(encoding="utf-8"))
            candidate = json.loads(Path(command[3]).read_text(encoding="utf-8"))
            self.assertEqual(base["config"], candidate["config"])
            self.assertEqual(base["fingerprint"], candidate["fingerprint"])
            self.assertEqual(base["profile"], "main@aaaaaaaaaaaa")
            self.assertEqual(candidate["profile"], "current@bbbbbbbbbbbb")
            self.assertEqual(command[4], "--html")
            self.assertTrue(call.kwargs["check"])
        self.assertEqual(search_reports(self.baseline), before)
        metadata = json.loads((self.output / "metadata.json").read_text())
        self.assertEqual(metadata["baseline_commit"], "a" * 40)
        self.assertEqual(metadata["current_commit"], "b" * 40)
        self.assertEqual(set(metadata["queries"]), set(QUERIES))

    @patch("compare_search.subprocess.run")
    def test_missing_and_incompatible_baselines_skip_only_affected_queries(self, run):
        """Skip only queries without a comparable baseline."""
        (self.baseline / "0_search.json").unlink()
        path = self.baseline / "1_search.json"
        report = json.loads(path.read_text())
        report["fingerprint"] = "different benchmark config"
        path.write_text(json.dumps(report))
        self.compare()
        self.assertEqual(run.call_count, 1)
        metadata = json.loads((self.output / "metadata.json").read_text())
        self.assertIn("skipped", metadata["queries"][QUERIES[0]])
        self.assertIn("fingerprints differ", metadata["queries"][QUERIES[1]]["skipped"])
        self.assertIn("html", metadata["queries"][QUERIES[2]])
        self.assertIn("skipped", (self.output / "summary.md").read_text())

    def test_missing_current_sweep_or_duplicate_query_is_an_error(self):
        """Reject incomplete or ambiguous current results."""
        (self.current / "0_search.json").unlink()
        with self.assertRaisesRegex(ValueError, "missing recall sweeps"):
            self.compare()
        (self.baseline / "duplicate_search.json").write_text(
            (self.baseline / "0_search.json").read_text()
        )
        with self.assertRaisesRegex(ValueError, "duplicate query"):
            search_reports(self.baseline)

    @patch("compare_search.subprocess.run")
    def test_pdbench_failure_is_reported(self, run):
        """Propagate comparison failures."""
        run.side_effect = subprocess.CalledProcessError(1, "pdbench")
        with self.assertRaises(subprocess.CalledProcessError):
            self.compare()

    @patch("compare_search.subprocess.run")
    def test_msmarco_pairs_only_unfiltered_top10_and_top100(self, run):
        """Compare both k values without requiring Cohere's filtered sweeps."""
        for directory in (self.baseline, self.current):
            for path in directory.glob("*.json"):
                path.unlink()
            for query in SUITES["msmarco"]:
                (directory / f"{query}_search.json").write_text(
                    json.dumps(
                        {
                            "kind": "search",
                            "fingerprint": query,
                            "config": {},
                            "data": {"variants": {query: {}}},
                        }
                    )
                )
        compare(
            self.baseline,
            self.current,
            self.output,
            {
                "baseline_commit": "a",
                "baseline_url": "https://example.com/run",
                "current_commit": "b",
            },
            dataset="msmarco",
        )
        self.assertEqual(run.call_count, 2)
        metadata = json.loads((self.output / "metadata.json").read_text())
        self.assertEqual(set(metadata["queries"]), set(SUITES["msmarco"]))


if __name__ == "__main__":
    unittest.main()

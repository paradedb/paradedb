"""Keep the pinned comparison faithful to publisher measurement definitions."""

import hashlib
import json
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import compare_selected


class SelectedComparisonTests(unittest.TestCase):
    """Check measurement definitions and comparison provenance."""

    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.root)

    def test_floor_quantiles_and_observed_window(self):
        """Quantiles use lower sample indexes and QPS includes the first latency."""
        path = self.root / "point.json"
        path.write_text(
            json.dumps(
                {
                    "runs": {
                        "paradedb": {
                            "queries": {
                                "q": {
                                    "latencies": [10, 40, 20, 30],
                                    "timestamps": [110, 240, 320, 430],
                                }
                            }
                        }
                    }
                }
            )
        )
        self.assertEqual(
            compare_selected.k6_metrics(path), ["20.000", "30.000", "30.000", "12.121"]
        )

    def test_reject_missing_timestamps(self):
        """Incomplete raw samples must not yield plausible throughput."""
        path = self.root / "point.json"
        path.write_text(
            json.dumps(
                {
                    "runs": {
                        "paradedb": {
                            "queries": {
                                "q": {"latencies": [10, 40], "timestamps": [110]}
                            }
                        }
                    }
                }
            )
        )
        with self.assertRaisesRegex(ValueError, "timestamps"):
            compare_selected.k6_metrics(path)

    def test_unreached_and_missing_measurements_stay_explicit(self):
        """Keep absent results visible and reject mismatched configurations or queries."""
        query = b"[]\n"
        for name, replicas in (("baseline", 2), ("current", 4)):
            root = self.root / name / "benchmark-artifacts"
            root.mkdir(parents=True)
            (root / "query_vectors.json").write_bytes(query)
            (root / "config.toml").write_text("[index]\nspill = true\n")
            (root / "vector-info.json").write_text(
                json.dumps(
                    [{"vector_num_vectors": 10, "vector_total_rows": replicas * 10}]
                )
            )
            (root / "extension-commit.txt").write_text(compare_selected.BASELINE_SHA)
            points = [
                {
                    "name": f"point{i}",
                    "recall": 0.9,
                    "reached": i < 4,
                    "probe": "100" if i < 4 else None,
                    "buffers": None,
                }
                for i in range(6)
            ]
            (root / "operating_points.json").write_text(json.dumps(points))
        with patch.object(
            compare_selected, "QUERY_SHA", hashlib.sha256(query).hexdigest()
        ):
            result = compare_selected.comparison(
                self.root / "baseline", self.root / "current"
            )
            self.assertEqual(result.count("unreached"), 4)
            self.assertIn("| missing | missing | missing | missing |", result)
            info = self.root / "current/benchmark-artifacts/vector-info.json"
            info.write_text('[{"vector_num_vectors": 10, "vector_total_rows": 20}]')
            with self.assertRaisesRegex(ValueError, "4 postings"):
                compare_selected.comparison(
                    self.root / "baseline", self.root / "current"
                )
            info.write_text('[{"vector_num_vectors": 10, "vector_total_rows": 40}]')
            (self.root / "current/benchmark-artifacts/config.toml").write_text(
                "[index]\nspill = true\ntarget_segment_count = 1\n"
            )
            with self.assertRaisesRegex(ValueError, "configuration"):
                compare_selected.comparison(
                    self.root / "baseline", self.root / "current"
                )
        with self.assertRaisesRegex(ValueError, "hash"):
            (self.root / "current/benchmark-artifacts/config.toml").write_text(
                "[index]\nspill = true\n"
            )
            compare_selected.comparison(self.root / "baseline", self.root / "current")


if __name__ == "__main__":
    unittest.main()

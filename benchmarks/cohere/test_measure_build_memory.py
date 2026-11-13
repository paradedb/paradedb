"""Check container accounting and artifact retention when builds fail."""

import json
import sys
import tempfile
import unittest
from pathlib import Path

from measure_build_memory import measure, memory_values


def stats(usage=1000, **details):
    """Construct a Docker memory-stats response."""
    return {"memory_stats": {"usage": usage, "stats": details}}


class BuildMemoryTests(unittest.TestCase):
    """Cover cache accounting and subprocess failure paths."""

    def test_cgroup_v2_keeps_usage_and_working_set_separate(self):
        """Retain full usage separately from inactive-file-adjusted usage."""
        values = memory_values(stats(inactive_file=700))
        self.assertEqual(values["container_memory"], 300)
        self.assertEqual(values["container_memory_usage"], 1000)

    def test_cgroup_v1_cache_accounting(self):
        """Use the v1 inactive-file and cache keys independently."""
        values = memory_values(stats(total_inactive_file=600, cache=800))
        self.assertEqual(values["container_memory"], 400)
        self.assertEqual(values["container_memory_usage"], 200)

    def test_invalid_stats_cannot_be_reported_as_zero(self):
        """Reject missing or empty container statistics."""
        for value in [stats(inactive_file=0, usage=0), stats(cache=100)]:
            with self.assertRaises(ValueError):
                memory_values(value)

    def test_failed_build_retains_samples_and_exit_status(self):
        """Flush diagnostics even when the build command fails."""
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            status = measure(
                [sys.executable, "-c", "raise SystemExit(7)"],
                "paradedb",
                output,
                interval=0.01,
                reader=lambda _: stats(inactive_file=250),
            )
            self.assertEqual(status, 7)
            summary = json.loads((output / "summary.json").read_text())
            self.assertEqual(summary["exit_code"], 7)
            self.assertEqual(summary["peak_container_memory_bytes"], 750)
            self.assertGreaterEqual(summary["sample_count"], 2)
            points = [
                json.loads(line)
                for line in (output / "points.jsonl").read_text().splitlines()
            ]
            self.assertTrue(any(p["type"] == "Point" for p in points))

    def test_sampling_failure_is_not_a_successful_measurement(self):
        """Flag an incomplete trace even when the build succeeds."""
        calls = 0

        def reader(_):
            nonlocal calls
            calls += 1
            if calls > 1:
                raise RuntimeError("container unavailable")
            return stats(inactive_file=100)

        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            status = measure(
                [sys.executable, "-c", "pass"],
                "paradedb",
                output,
                interval=1,
                reader=reader,
            )
            self.assertEqual(status, 1)
            summary = json.loads((output / "summary.json").read_text())
            self.assertEqual(summary["exit_code"], 0)
            self.assertEqual(summary["sampling_errors"], ["container unavailable"])


if __name__ == "__main__":
    unittest.main()

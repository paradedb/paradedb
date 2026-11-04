"""Protect the fixed-budget controls used by the binary comparison."""

import json
import struct
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import tomllib
from probe_overhead import measure_k6, probe_config


class ProbeOverheadTest(unittest.TestCase):
    """Verify equivalent effective settings across the scale boundary."""

    def test_controls_preserve_effective_f32_budget(self):
        """Avoid sweep rounding while matching the fraction passed to Tantivy."""
        template = Path(__file__).with_name("1m.toml").read_text(encoding="utf-8")
        for raw in [0.00000123456789, 0.00274193, 0.02, 0.124999, 0.125, 0.21, 1.0]:
            for arm in ["baseline", "disabled", "scaled"]:
                config = tomllib.loads(probe_config(template, arm, 0, raw, 0.11))
                session = config["search"]["session"]
                probe = float(session["paradedb.vector_cluster_max_probe"])
                if arm == "scaled":
                    probe *= min(probe / 0.125, 1) ** 3
                self.assertEqual(struct.pack("f", probe), struct.pack("f", raw))
                self.assertEqual(session["max_parallel_workers_per_gather"], "0")
                self.assertEqual(session["cohere.titles_1pct"], "battle")
                self.assertEqual(
                    config["search"]["sweep"]["param"], "cohere.diagnostic_point"
                )
                if arm == "baseline":
                    self.assertNotIn("paradedb.vector_probe_scale_power", session)

    def test_normalized_arm_retains_selected_probe(self):
        """Keep the candidate's measured point as a separate comparison arm."""
        template = Path(__file__).with_name("1m.toml").read_text(encoding="utf-8")
        config = tomllib.loads(probe_config(template, "normalized", 8, 0.02, 0.109))
        session = config["search"]["session"]
        self.assertEqual(session["paradedb.vector_cluster_max_probe"], "0.109")
        self.assertEqual(session["paradedb.vector_probe_selectivity_slope"], "-0.73")
        self.assertEqual(config["search"]["runs"], 3)
        self.assertEqual(config["search"]["warmup_runs"], 1)

    def test_k6_uses_control_session_and_preserves_dashboard(self):
        """The timing harness must receive the equivalent probe, not the sweep value."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            destination = root / "case"
            point = {
                "name": "knn_top10_unfiltered@r95",
                "probe": "0.103",
                "session": {"paradedb.vector_probe_scale_power": "4"},
            }
            session = {"paradedb.vector_cluster_max_probe": "0.0576"}

            def fake_run(command, **kwargs):
                self.assertEqual(kwargs["cwd"], destination)
                self.assertEqual(
                    command[-1], str(root / "benchmarks/cohere/k6/search.js")
                )
                self.assertIn(
                    f"QUERY_VECTORS={root / 'benchmark-artifacts/query_vectors.json'}",
                    command,
                )
                report = {
                    "startTime": "2026-11-04T00:00:00Z",
                    "runs": {
                        "paradedb": {
                            "queries": {"search": {"latencies": [1.0, 2.0, 3.0]}}
                        }
                    },
                }
                (destination / "dashboard_test.json").write_text(json.dumps(report))
                (destination / "dashboard_test.html").write_text("dashboard")
                return subprocess.CompletedProcess(command, 0)

            with (
                patch.dict(
                    "os.environ", {"PARADEDB_URL": "postgresql://localhost/test"}
                ),
                patch("probe_overhead.subprocess.run", side_effect=fake_run),
            ):
                result = measure_k6(root, destination, point, session)
            exported = json.loads((destination / "operating_points.json").read_text())[
                0
            ]
            self.assertEqual(exported["probe"], "0.0576")
            self.assertEqual(exported["session"], session)
            self.assertEqual(result["p50_ms"], 2.0)
            self.assertTrue((destination / "k6.json").exists())
            self.assertTrue((destination / "k6.html").exists())


if __name__ == "__main__":
    unittest.main()

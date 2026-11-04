"""Protect the fixed-budget controls used by the binary comparison."""

import struct
import unittest
from pathlib import Path

import tomllib
from probe_overhead import probe_config


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


if __name__ == "__main__":
    unittest.main()

"""Keep shared calibration distinct from independent per-case tuning."""

import copy
import unittest

from admission_experiment import SHARED_EXPONENT, choose_calibration, fixed_point
from prepare_k6 import EXPONENT, PROBE, QUERIES, RECALL_TARGET
from test_prepare_k6 import reports


class AdmissionExperimentTest(unittest.TestCase):
    """Protect calibration scope and recall labels."""

    def test_one_calibration_must_cover_every_filter(self):
        """Individually successful settings cannot substitute for a shared setting."""
        points = [
            {
                "query": query,
                "probe": knob,
                "exponent": SHARED_EXPONENT,
                "recall": recall,
                "session": {PROBE: "0"},
            }
            for knob, recall in ((0.8, 0.91), (0.9, 0.96), (0.99, 0.995))
            for query in QUERIES
        ]
        points[0]["recall"] = 0.89
        selected = choose_calibration(points)
        self.assertEqual(len(selected), 9)
        self.assertEqual({p["probe"] for p in selected if p["target"] == 0.9}, {0.9})
        self.assertEqual({p["probe"] for p in selected if p["target"] == 0.95}, {0.9})
        self.assertEqual({p["probe"] for p in selected if p["target"] == 0.99}, {0.99})
        self.assertTrue(all(p["reached"] for p in selected))
        with self.assertRaises(ValueError):
            choose_calibration(points[:-1])
        with self.assertRaises(ValueError):
            choose_calibration(points + points[:1])
        with self.assertRaises(ValueError):
            choose_calibration([])
        points[0]["exponent"] = 2
        with self.assertRaises(ValueError):
            choose_calibration(points)
        points[0]["exponent"] = SHARED_EXPONENT
        points[0]["session"][PROBE] = "0.2"
        with self.assertRaises(ValueError):
            choose_calibration(points)

    def test_unreached_goal_keeps_measured_knob_and_recall(self):
        """A failed shared calibration must not invent a successful operating point."""
        points = [
            {
                "query": query,
                "probe": 0.9,
                "exponent": SHARED_EXPONENT,
                "recall": 0.96,
                "session": {PROBE: "0"},
            }
            for query in QUERIES
        ]
        selected = [p for p in choose_calibration(points) if p["target"] == 0.99]
        self.assertTrue(all(not p["reached"] for p in selected))
        self.assertEqual({p["probe"] for p in selected}, {0.9})
        self.assertEqual({p["recall"] for p in selected}, {0.96})

    def test_a_missed_target_is_exported_as_missed(self):
        """Preserve the requested setting and its observed recall shortfall."""
        report = copy.deepcopy(reports()[0])
        query = QUERIES[0]
        report["config"]["search"]["session"].update({EXPONENT: "1", PROBE: "0"})
        variant = report["data"]["variants"][query]
        variant["points"][0]["value"] = "0.99"
        variant["points"] = variant["points"][:1]
        point = fixed_point(
            report,
            {"query": query, "name": "test", "target": 0.99},
            1.0,
            "automatic",
        )
        self.assertFalse(point["reached"])
        self.assertEqual(point["recall"], 0.96)
        self.assertEqual(point["param"], RECALL_TARGET)
        self.assertEqual(point["probe"], 0.99)
        self.assertEqual(point["session"][EXPONENT], "1")
        self.assertEqual(point["session"][PROBE], "0")


if __name__ == "__main__":
    unittest.main()

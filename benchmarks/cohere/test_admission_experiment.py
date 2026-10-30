"""Keep shared calibration distinct from independent per-case tuning."""

import copy
import unittest

from admission_experiment import choose_calibration, fixed_point
from prepare_k6 import EXPONENT, PROBE, QUERIES, RECALL_TARGET, TARGETS
from test_prepare_k6 import reports


class AdmissionExperimentTest(unittest.TestCase):
    """Protect calibration scope and recall labels."""

    def test_one_calibration_must_cover_every_filter_and_target(self):
        """Individually successful settings cannot substitute for a shared setting."""
        points = [
            {"query": query, "target": target, "exponent": exponent, "recall": target}
            for exponent in (0.5, 1.0, 2.0)
            for query in QUERIES
            for target in TARGETS
        ]
        points[0]["recall"] -= 0.01
        points[10]["recall"] -= 0.02
        self.assertEqual(choose_calibration(points), 2.0)
        with self.assertRaises(ValueError):
            choose_calibration(points[:-1])

    def test_a_missed_target_is_exported_as_missed(self):
        """Preserve the requested setting and its observed recall shortfall."""
        report = copy.deepcopy(reports()[0])
        query = QUERIES[0]
        report["config"]["search"]["session"].update({EXPONENT: "1", PROBE: "0.125"})
        variant = report["data"]["variants"][query]
        variant["points"][0]["value"] = "0.99"
        variant["points"] = variant["points"][:1]
        point = fixed_point(
            report,
            {"query": query, "name": "test", "target": 0.99},
            1.0,
            "shared-budget",
        )
        self.assertFalse(point["reached"])
        self.assertEqual(point["recall"], 0.96)
        self.assertEqual(point["param"], RECALL_TARGET)
        self.assertEqual(point["probe"], 0.99)
        self.assertEqual(point["session"][EXPONENT], "1")
        self.assertEqual(point["session"][PROBE], "0.125")


if __name__ == "__main__":
    unittest.main()

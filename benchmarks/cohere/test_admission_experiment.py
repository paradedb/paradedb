"""Check that CI calibrates exponents without changing nominal recall targets."""

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from admission_experiment import main
from prepare_k6 import EXPONENT, PROBE, QUERIES, RECALL_TARGET, TARGETS
from test_prepare_k6 import reports


class AdmissionExperimentTest(unittest.TestCase):
    """Exercise report selection through the benchmark command handoff."""

    def run_experiment(self, miss=False):
        """Replace pdbench with measured fixtures while running the orchestration."""
        calls = []
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)

            def measure(command, check):
                self.assertTrue(check)
                self.assertNotIn("--probe", command)
                settings = dict(
                    command[i + 1].split("=", 1)
                    for i, arg in enumerate(command)
                    if arg == "--set"
                )
                query = command[command.index("--query") + 1]
                report = reports()[QUERIES.index(query)]
                search = report["config"]["search"]
                search["session"].update(
                    (key.removeprefix("search.session."), value)
                    for key, value in settings.items()
                    if key.startswith("search.session.")
                )
                param = settings["search.sweep.param"]
                report["data"]["sweep_param"] = param
                variant = report["data"]["variants"][query]
                if param == EXPONENT:
                    target = float(search["session"][RECALL_TARGET])
                    self.assertEqual(search["session"][PROBE], "0")
                    self.assertEqual(
                        json.loads(settings["search.sweep.targets"]), [target]
                    )
                    search["sweep"]["targets"] = [target]
                    value = str(1 + TARGETS.index(target) + QUERIES.index(query) / 10)
                    reached = not (miss and target == 0.99 and query == QUERIES[0])
                    recall = target if reached else 0.98
                    variant["targets"] = (
                        {str(target): {"value": value, "recall": recall}}
                        if reached
                        else {}
                    )
                    calls.append((query, target))
                else:
                    self.assertEqual(param, PROBE)
                    self.assertEqual(
                        search["session"]["paradedb.vector_adaptive_probing"], "off"
                    )
                    value, recall = "0.001", 1.0
                    variant["targets"] = {
                        str(target): {"value": value, "recall": recall}
                        for target in TARGETS
                    }
                variant["points"][0]["value"] = value
                variant["points"][0]["metrics"]["recall"]["recall"] = recall
                output = Path(command[command.index("--runs-dir") + 1])
                output.mkdir(parents=True, exist_ok=True)
                (output / f"{query}_search.json").write_text(json.dumps(report))

            with (
                patch(
                    "admission_experiment.Path", side_effect=lambda value: root / value
                ),
                patch("admission_experiment.subprocess.run", side_effect=measure),
                patch.dict(
                    "os.environ", {"PARADEDB_URL": "test-only", "COHERE_SIZE": "10m"}
                ),
            ):
                main()
            output = root / "benchmark-artifacts"
            selected = json.loads((output / "operating_points.json").read_text())
            controls = json.loads(
                (output / "fixed-budget/operating_points.json").read_text()
            )
            self.assertEqual(set(calls), {(q, t) for q in QUERIES for t in TARGETS})
            self.assertEqual(len(calls), 9)
            self.assertEqual(len(controls), 9)
            self.assertTrue(all(p["param"] == PROBE and p["reached"] for p in controls))
            self.assertIn("max_probe=0", (output / "operating_points.md").read_text())
            return selected

    def test_preserves_each_calibrated_exponent_and_nominal_target(self):
        """k6 receives the selected exponent, fixed target, and automatic ceiling."""
        selected = self.run_experiment()
        self.assertEqual(len(selected), 9)
        for point in selected:
            self.assertTrue(point["reached"])
            self.assertEqual(point["param"], EXPONENT)
            self.assertEqual(float(point["session"][RECALL_TARGET]), point["target"])
            self.assertEqual(point["session"][PROBE], "0")
            self.assertEqual(
                float(point["probe"]),
                1 + TARGETS.index(point["target"]) + QUERIES.index(point["query"]) / 10,
            )

    def test_unreached_target_is_not_exported_as_a_latency_operating_point(self):
        """A calibration miss remains visible and cannot be measured by k6."""
        selected = self.run_experiment(miss=True)
        missed = [point for point in selected if not point["reached"]]
        self.assertEqual(len(missed), 1)
        self.assertEqual(missed[0]["target"], 0.99)
        self.assertEqual(missed[0]["recall"], 0.98)
        self.assertIsNone(missed[0]["probe"])
        self.assertIsNone(missed[0]["buffers"])


if __name__ == "__main__":
    unittest.main()

"""Regression checks for the pdbench-to-k6 handoff."""

import copy
import unittest

from prepare_k6 import PROBE, QUERIES, SUITES, operating_points, summary
from verify_plans import verify


def reports(dataset="cohere"):
    """Build reports with shared probes and an unreached r99 target."""
    return [
        {
            "kind": "search",
            "config": {
                "dataset": {"name": dataset, "size": "1m"},
                "search": {
                    "query": query,
                    "k": neighbors,
                    "sweep": {"tolerance": 0.000001},
                    "session": {"cohere.titles_10pct": "year", "jit": "off"},
                },
            },
            "data": {
                "sweep_param": PROBE,
                "query_vectors": 100,
                "variants": {
                    query: {
                        "query": f"SELECT _id FROM {dataset} ORDER BY emb <=> "
                        f"current_setting('{dataset}.qvec')::vector(1024) LIMIT {neighbors}",
                        "targets": {
                            "0.9": {"value": "0.001", "recall": 0.96},
                            "0.95": {"value": "0.001", "recall": 0.96},
                        },
                        "points": [
                            {
                                "value": "0.001",
                                "metrics": {
                                    "recall": {"recall": 0.96, "k": neighbors},
                                    "buffers": {
                                        "queries_measured": 100,
                                        "shared_hit_mean": 1234.5,
                                        "shared_read_mean": 0,
                                    },
                                },
                            }
                        ],
                    }
                },
            },
        }
        for query, neighbors in SUITES[dataset].items()
    ]


class OperatingPointsTest(unittest.TestCase):
    """Protect recall labels, session settings, and filter pushdown."""

    def test_exports_exact_selected_settings_and_buffers_without_changing_inputs(self):
        """Preserve reported settings and identify unreached targets."""
        inputs = reports()
        original = copy.deepcopy(inputs)
        points = operating_points(inputs)
        self.assertEqual(len(points), 9)
        self.assertEqual(sum(point["reached"] for point in points), 6)
        for point in points:
            self.assertIn("$1::vector(1024)", point["sql"])
            self.assertEqual(point["session"], inputs[0]["config"]["search"]["session"])
            if point["reached"]:
                self.assertEqual(point["probe"], "0.001")
                self.assertEqual(point["buffers"]["shared_hit_mean"], 1234.5)
            else:
                self.assertIsNone(point["probe"])
                self.assertEqual(point["recall"], 0.96)
        self.assertIn("Not reached", summary(points))
        self.assertEqual(inputs, original)

    def test_refuses_below_target_point_even_when_pdbench_labels_it_selected(self):
        """Reject a below-target selection rather than relabeling it."""
        inputs = reports()
        inputs[0]["data"]["variants"][QUERIES[0]]["targets"]["0.99"] = {
            "value": "0.001",
            "recall": 0.96,
        }
        with self.assertRaisesRegex(ValueError, "invalid selected point"):
            operating_points(inputs)

    def test_msmarco_exports_both_neighbor_counts_and_preserves_reports(self):
        """Keep top-10 and top-100 selections distinct in the shared handoff."""
        inputs = reports("msmarco")
        original = copy.deepcopy(inputs)
        points = operating_points(inputs, dataset="msmarco")
        self.assertEqual(len(points), 6)
        for point in points:
            self.assertEqual(point["neighbors"], SUITES["msmarco"][point["query"]])
            self.assertIn(f"LIMIT {point['neighbors']}", point["sql"])
            self.assertIn("$1::vector(1024)", point["sql"])
        self.assertIn("| 100 |", summary(points))
        self.assertEqual(inputs, original)
        with self.assertRaisesRegex(ValueError, "Missing recall sweeps"):
            operating_points(inputs[:1], dataset="msmarco")

    def test_msmarco_rejects_wrong_recall_k_and_loose_top100_tolerance(self):
        """Reject recall measurements that use the wrong denominator."""
        inputs = reports("msmarco")
        result = inputs[1]["data"]["variants"]["knn_top100_unfiltered"]
        result["points"][0]["metrics"]["recall"]["k"] = 10
        with self.assertRaisesRegex(ValueError, "expected recall@100"):
            operating_points(inputs, dataset="msmarco")
        inputs = reports("msmarco")
        inputs[1]["config"]["search"]["k"] = 10
        with self.assertRaisesRegex(ValueError, "expected recall@100"):
            operating_points(inputs, dataset="msmarco")
        inputs = reports("msmarco")
        for report in inputs:
            report["config"]["search"]["sweep"]["tolerance"] = 0.0001
        with self.assertRaisesRegex(ValueError, "strict recall targets"):
            operating_points(inputs, dataset="msmarco")

    def test_top100_plan_requires_cosine_topk_inside_paradedb(self):
        """Reject plans that do not push the requested TopK into the index."""
        scan = {
            "Custom Plan Provider": "ParadeDB Base Scan",
            "   TopK Limit": 100,
            "   TopK Order By": "emb <=> vector asc",
        }
        plan = [{"Plan": scan}]
        verify(plan, filtered=False, neighbors=100)
        scan["   TopK Limit"] = 10
        with self.assertRaisesRegex(ValueError, "TopK with limit 100"):
            verify(plan, filtered=False, neighbors=100)

    def test_refuses_mixed_sessions_and_incomplete_query_sets(self):
        """Reject mismatched or missing sweep reports."""
        inputs = reports()
        inputs[0]["config"]["search"]["session"]["jit"] = "on"
        with self.assertRaisesRegex(ValueError, "same dataset, index, and session"):
            operating_points(inputs)
        with self.assertRaisesRegex(ValueError, "Missing recall sweeps"):
            operating_points(reports()[:2])
        with self.assertRaisesRegex(ValueError, "duplicate query"):
            operating_points(reports() + reports()[:1])

    def test_refuses_partial_query_set_or_loose_recall_tolerance(self):
        """Require the full held-out set and precise recall selection."""
        for section, key, value in [
            ("data", "query_vectors", 99),
            ("sweep", "tolerance", 0.005),
        ]:
            inputs = reports()
            if section == "sweep":
                inputs[0]["config"]["search"]["sweep"][key] = value
            else:
                inputs[0][section][key] = value
            with self.assertRaisesRegex(ValueError, "strict recall targets"):
                operating_points(inputs)

    def test_plan_guard_requires_index_scan_and_filter_pushdown(self):
        """Reject fallback scans and filters outside the index."""
        scan = {
            "Custom Plan Provider": "ParadeDB Base Scan",
            "Tantivy Query": "text:year",
        }
        plan = [{"Plan": {"Node Type": "Gather", "Plans": [scan]}}]
        verify(plan, filtered=True)
        scan["Tantivy Query"] = "all"
        with self.assertRaisesRegex(ValueError, "predicate pushed down"):
            verify(plan, filtered=True)
        scan["Filter"] = "text = 'year'"
        with self.assertRaisesRegex(ValueError, "outside"):
            verify(plan, filtered=False)
        with self.assertRaisesRegex(ValueError, "Expected a ParadeDB scan"):
            verify([{"Plan": {"Node Type": "Seq Scan"}}], filtered=False)


if __name__ == "__main__":
    unittest.main()

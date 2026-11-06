"""Regression checks for the pdbench-to-k6 handoff."""

import copy
import unittest

from prepare_k6 import EFFORT, NPROBE, PROBE, QUERIES, SUITES, operating_points, summary
from verify_plans import verify


def reports(dataset="cohere"):
    """Build suite reports with shared probes and an unreached r99 target."""
    return [
        {
            "kind": "search",
            "config": {
                "dataset": {"name": dataset, "size": "1m", "dims": 1024},
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

    def test_msmarco_exports_top10_and_top100_as_unfiltered_workloads(self):
        """Keep k6 result counts, vector binding, and recall labels consistent."""
        points = operating_points(reports("msmarco"), dataset="msmarco")
        self.assertEqual(len(points), 6)
        for point in points:
            self.assertFalse(point["filtered"])
            self.assertEqual(point["query_vectors"], 100)
            self.assertEqual(point["dimensions"], 1024)
            self.assertEqual(point["neighbors"], SUITES["msmarco"][point["query"]])
            self.assertIn(f"LIMIT {point['neighbors']}", point["sql"])
            self.assertIn("$1::vector(1024)", point["sql"])
        self.assertIn("recall@k", summary(points))
        with self.assertRaisesRegex(ValueError, "Expected cohere"):
            operating_points(reports("msmarco"))

    def test_top100_recall_tolerance_is_smaller_than_one_neighbor(self):
        """A tolerance valid for top-10 can hide a missed top-100 target."""
        inputs = reports("msmarco")
        for report in inputs:
            report["config"]["search"]["sweep"]["tolerance"] = 0.0005
        with self.assertRaisesRegex(ValueError, "strict recall targets"):
            operating_points(inputs, dataset="msmarco")

    def test_msmarco_preserves_adaptive_effort_above_one(self):
        """Carry the selected effort and automatic probe budget into k6."""
        inputs = reports("msmarco")
        for report in inputs:
            report["data"]["sweep_param"] = EFFORT
            report["config"]["search"]["session"][PROBE] = "0"
            for result in report["data"]["variants"].values():
                result["points"][0]["value"] = "5.5"
                for target in result["targets"].values():
                    target["value"] = "5.5"
        points = operating_points(inputs, dataset="msmarco")
        for point in points:
            self.assertEqual(point["param"], EFFORT)
            self.assertEqual(point["session"][PROBE], "0")
            if point["reached"]:
                self.assertEqual(point["probe"], "5.5")
        self.assertIn("search effort", summary(points))
        caps = {"knn_top10_unfiltered": "0.106", "knn_top100_unfiltered": "0.079"}
        for report in inputs:
            query = report["config"]["search"]["query"]
            report["config"]["search"]["session"].update(
                {PROBE: caps[query], "paradedb.vector_router_rerank_factor": "2"}
            )
        original = copy.deepcopy(inputs)
        for point in operating_points(inputs, dataset="msmarco"):
            self.assertEqual(point["session"][PROBE], caps[point["query"]])
            self.assertEqual(
                point["session"]["paradedb.vector_router_rerank_factor"], "2"
            )
        self.assertEqual(inputs, original)
        for cap in ("-0.1", "1.1", "nan"):
            inputs[0]["config"]["search"]["session"][PROBE] = cap
            with self.assertRaisesRegex(ValueError, "work ceilings"):
                operating_points(inputs, dataset="msmarco")

    def test_msmarco_preserves_pg_vectorsearch_integer_probes(self):
        """Use PRISM sessions and backend for both unfiltered values of k."""
        inputs = reports("msmarco")
        for report in inputs:
            report["config"]["engine"] = "pg_vectorsearch"
            report["config"]["dataset"]["table"] = "msmarco"
            report["config"]["search"]["sweep"].update(min=1, max=4096, integer=True)
            report["data"]["sweep_param"] = NPROBE
            for result in report["data"]["variants"].values():
                result["points"][0]["value"] = "64"
                for target in result["targets"].values():
                    target["value"] = "64"
        points = operating_points(inputs, dataset="msmarco")
        for point in points:
            self.assertEqual(point["param"], NPROBE)
            self.assertEqual(point["backend"], "pg_vectorsearch")
            self.assertEqual(point["index_name"], "msmarco_prism_idx")
        self.assertIn("nprobe", summary(points))
        invalid = copy.deepcopy(inputs)
        invalid[0]["data"]["sweep_param"] = PROBE
        with self.assertRaisesRegex(ValueError, "Expected msmarco"):
            operating_points(invalid, dataset="msmarco")
        for result in inputs[0]["data"]["variants"].values():
            result["points"][0]["value"] = "64.5"
            for target in result["targets"].values():
                target["value"] = "64.5"
        with self.assertRaisesRegex(ValueError, "invalid selected point"):
            operating_points(inputs, dataset="msmarco")

    def test_pg_vectorsearch_rejects_fallback_plans(self):
        """Require the expected PRISM index before measuring latency."""
        scan = {"Node Type": "Index Scan", "Index Name": "msmarco_prism_idx"}
        verify([{"Plan": scan}], False, 100, "pg_vectorsearch", "msmarco_prism_idx")
        for fallback in (
            {"Node Type": "Seq Scan"},
            {"Node Type": "Index Scan", "Index Name": "other_idx"},
            {"Custom Plan Provider": "ParadeDB Scan"},
        ):
            with self.assertRaisesRegex(ValueError, "pg_vectorsearch index scan"):
                verify(
                    [{"Plan": fallback}],
                    False,
                    100,
                    "pg_vectorsearch",
                    "msmarco_prism_idx",
                )

    def test_refuses_incomplete_recall_targets(self):
        """Do not omit r99 when exporting a partial sweep configuration."""
        inputs = reports("msmarco")
        inputs[0]["config"]["search"]["sweep"]["targets"] = [0.9, 0.95]
        with self.assertRaisesRegex(ValueError, "all three recall targets"):
            operating_points(inputs, dataset="msmarco")

    def test_refuses_top100_without_explicit_recall_k(self):
        """Default top-ten scoring must not pass as a top-100 measurement."""
        inputs = reports("msmarco")
        del inputs[1]["config"]["search"]["k"]
        with self.assertRaisesRegex(ValueError, "expected recall@100"):
            operating_points(inputs, dataset="msmarco")

    def test_refuses_top10_metrics_labeled_as_top100(self):
        """Do not publish recall measured at the wrong k."""
        inputs = reports("msmarco")
        inputs[1]["data"]["variants"]["knn_top100_unfiltered"]["points"][0]["metrics"][
            "recall"
        ]["k"] = 10
        with self.assertRaisesRegex(ValueError, "expected recall@100"):
            operating_points(inputs, dataset="msmarco")

    def test_plan_guard_requires_vector_order_and_matching_topk(self):
        """Reject a custom scan that leaves vector ranking to PostgreSQL."""
        scan = {
            "Custom Plan Provider": "ParadeDB Base Scan",
            "   TopK Order By": "emb <=> vector asc",
            "   TopK Limit": 100,
        }
        plan = [{"Plan": {"Node Type": "Gather Merge", "Plans": [scan]}}]
        verify(plan, filtered=False, neighbors=100)
        with self.assertRaisesRegex(ValueError, "cosine vector TopK"):
            verify(plan, filtered=False, neighbors=10)
        del scan["   TopK Order By"]
        with self.assertRaisesRegex(ValueError, "cosine vector TopK"):
            verify(plan, filtered=False, neighbors=100)

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

"""Regression checks for the pdbench-to-k6 handoff."""

import copy
import json
import unittest
from pathlib import Path

import tomllib
from prepare_k6 import (
    DIMENSIONS,
    NPROBE,
    PROBE,
    QUERIES,
    SUITES,
    operating_points,
    summary,
)
from verify_plans import verify


def reports(dataset="cohere", engine="pg_search"):
    """Build reports with shared probes and an unreached r99 target."""
    return [
        {
            "kind": "search",
            "config": {
                "engine": engine,
                "dataset": {
                    "name": dataset,
                    "table": "hn_items" if dataset == "hn" else dataset,
                    "size": "1m",
                    "dims": DIMENSIONS[dataset],
                },
                "search": {
                    "query": query,
                    "k": neighbors,
                    "sweep": {"min": 1, "max": 4096, "tolerance": 0.000001},
                    "session": {"cohere.titles_10pct": "year", "jit": "off"},
                },
            },
            "data": {
                "sweep_param": NPROBE if engine == "pg_vectorsearch" else PROBE,
                "query_vectors": 100,
                "variants": {
                    query: {
                        "query": f"SELECT _id FROM {dataset} ORDER BY emb <=> "
                        f"current_setting('{dataset}.qvec')::vector({DIMENSIONS[dataset]}) "
                        f"LIMIT {neighbors}",
                        "targets": {
                            "0.9": {"value": "100", "recall": 0.96},
                            "0.95": {"value": "100", "recall": 0.96},
                        },
                        "points": [
                            {
                                "value": "100",
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
                self.assertEqual(point["probe"], "100")
                self.assertEqual(point["buffers"]["shared_hit_mean"], 1234.5)
            else:
                self.assertIsNone(point["probe"])
                self.assertEqual(point["recall"], 0.96)
        self.assertIn("Not reached", summary(points))
        self.assertEqual(inputs, original)

    def test_fast_search_rejects_pilots_as_published_measurements(self):
        """Require 100 measured vectors even if the report claims a full population."""
        for count, samples in [(25, 25), (100, 25), (25, 100)]:
            inputs = reports("hn", "pg_vectorsearch")
            for report in inputs:
                report["data"]["fast_search"] = True
                for result in report["data"]["variants"].values():
                    result["points"][0]["query_vectors"] = count
                    result["points"][0]["metrics"]["recall"]["samples"] = samples
            with self.assertRaisesRegex(ValueError, "full 100-vector validation"):
                operating_points(inputs, "hn")

    def test_fast_search_keeps_budget_exhaustion_distinct_and_skips_pilots(self):
        """Preserve budget outcomes without publishing incomplete recall measurements."""
        inputs = reports("hn", "pg_vectorsearch")
        for report in inputs:
            report["data"]["fast_search"] = True
            for result in report["data"]["variants"].values():
                result["pilot_points"] = result.pop("points")
                result["points"] = []
                result["targets"] = {}
                result["target_status"] = {
                    "0.9": {"status": "budget_exceeded", "reason": "statement timeout"},
                    "0.95": {"status": "probe_budget_exceeded"},
                    "0.99": {"status": "unreachable"},
                }
        points = operating_points(inputs, "hn")
        self.assertTrue(all(not point["reached"] for point in points))
        self.assertTrue(all(point["recall"] is None for point in points))
        rendered = summary(points)
        self.assertIn("Time budget exceeded", rendered)
        self.assertIn("Probe budget exceeded", rendered)
        self.assertIn("Not reached", rendered)
        self.assertNotIn("96.0%", rendered)

    def test_fast_search_publishes_full_validation_with_sampled_diagnostics(self):
        """Allow fewer diagnostics while retaining the full recall population."""
        inputs = reports("hn", "pg_vectorsearch")
        for report in inputs:
            report["data"]["fast_search"] = True
            for result in report["data"]["variants"].values():
                point = result["points"][0]
                point["query_vectors"] = 100
                point["metrics"]["recall"]["samples"] = 100
                point["metrics"]["buffers"]["queries_measured"] = 10
                result["pilot_points"] = [
                    {"value": "1", "metrics": {"recall": {"recall": 1}}}
                ]
        points = operating_points(inputs, "hn")
        self.assertTrue(
            all(point["probe"] == "100" for point in points if point["reached"])
        )
        self.assertIn("| 1234.5 | 0.0 | 10 |", summary(points))

    def test_prism_preserves_backend_and_integer_probe_for_both_datasets(self):
        """Carry the chosen engine through the recall-to-k6 handoff."""
        for dataset in ("hn", "msmarco"):
            inputs = reports(dataset, "pg_vectorsearch")
            original = copy.deepcopy(inputs)
            points = operating_points(inputs, dataset)
            for point in points:
                self.assertEqual(point["engine"], "pg_vectorsearch")
                self.assertEqual(point["backend"], "pg_vectorsearch")
                self.assertEqual(point["param"], NPROBE)
                self.assertEqual(point["dimensions"], DIMENSIONS[dataset])
                self.assertEqual(
                    point["index_name"],
                    f"{inputs[0]['config']['dataset']['table']}_prism_idx",
                )
            self.assertIn("nprobe", summary(points))
            self.assertEqual(inputs, original)

    def test_prism_rejects_wrong_parameter_and_out_of_range_probes(self):
        """Reject backend mismatches and invalid PRISM operating points."""
        inputs = reports("msmarco", "pg_vectorsearch")
        inputs[0]["data"]["sweep_param"] = PROBE
        with self.assertRaisesRegex(ValueError, "probe search report"):
            operating_points(inputs, "msmarco")
        for value in ("0", "0.5", "100.5", "4097", "nan"):
            inputs = reports("msmarco", "pg_vectorsearch")
            result = inputs[0]["data"]["variants"]["knn_top10_unfiltered"]
            for selected in result["targets"].values():
                selected["value"] = value
            result["points"][0]["value"] = value
            with self.assertRaisesRegex(ValueError, "invalid selected point"):
                operating_points(inputs, "msmarco")

    def test_only_hn_prism_allows_short_results_after_recall_validation(self):
        """Keep the result-count exception scoped to HN's post-filtered PRISM scan."""
        for dataset in SUITES:
            for engine in ("pg_search", "pg_vectorsearch"):
                with self.subTest(dataset=dataset, engine=engine):
                    points = operating_points(reports(dataset, engine), dataset)
                    relaxed = dataset == "hn" and engine == "pg_vectorsearch"
                    self.assertTrue(
                        all(
                            p["require_full_neighbors"] == (not relaxed) for p in points
                        )
                    )
                    self.assertEqual(
                        "reports short result counts" in summary(points), relaxed
                    )

    def test_hn_paradedb_sweep_keeps_configured_bounds(self):
        """Use valid integer work budgets through the HN recall-to-k6 handoff."""
        profile = Path(__file__).resolve().parents[1] / "hn" / "28m.toml"
        sweep = tomllib.loads(profile.read_text())["search"]["sweep"]
        self.assertTrue(sweep["integer"])
        self.assertLessEqual(1, sweep["min"])
        self.assertLessEqual(sweep["min"], sweep["start"])
        self.assertLessEqual(sweep["start"], sweep["max"])
        self.assertLessEqual(sweep["max"], 100_000)
        for value in (sweep["min"], sweep["start"], sweep["max"]):
            self.assertTrue(float(value).is_integer())
            inputs = reports("hn")
            for report in inputs:
                report["config"]["search"]["sweep"] = sweep
                for result in report["data"]["variants"].values():
                    for selected in result["targets"].values():
                        selected["value"] = str(value)
                    result["points"][0]["value"] = str(value)
            points = operating_points(inputs, "hn")
            self.assertEqual(points[0]["probe"], str(value))
            self.assertEqual(points[0]["filter_fields"], ["time"])

    def test_refuses_below_target_point_even_when_pdbench_labels_it_selected(self):
        """Reject a below-target selection rather than relabeling it."""
        inputs = reports()
        inputs[0]["data"]["variants"][QUERIES[0]]["targets"]["0.99"] = {
            "value": "100",
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
            "Tantivy Query": {"term": {"field": "text", "value": "year"}},
        }
        plan = [{"Plan": {"Node Type": "Gather", "Plans": [scan]}}]
        verify(plan, filtered=True)
        scan["Tantivy Query"] = '"all"'
        with self.assertRaisesRegex(ValueError, "predicate pushed down"):
            verify(plan, filtered=True)
        scan["Filter"] = "text = 'year'"
        with self.assertRaisesRegex(ValueError, "outside"):
            verify(plan, filtered=False)
        with self.assertRaisesRegex(ValueError, "Expected a ParadeDB scan"):
            verify([{"Plan": {"Node Type": "Seq Scan"}}], filtered=False)

    def test_plan_guard_accepts_a_setting_driven_filter_on_a_required_field(self):
        """Accept the Cohere filtered plan, whose query string comes from a session setting."""
        fixture = Path(__file__).parent / "fixtures" / "knn_top10_10pct_plan.json"
        plan = json.loads(fixture.read_text())
        verify(plan, filtered=True, neighbors=10, fields=["text"])
        with self.assertRaisesRegex(ValueError, "unresolved expression"):
            verify(plan, filtered=True, neighbors=10, fields=["title"])
        scan = plan[0]["Plan"]["Plans"][0]["Plans"][0]
        expression = json.loads(scan["Tantivy Query"])["postgres_expression"]["expr"]
        setting_driven = expression["expr_desc"]
        for desc in (
            setting_driven.replace(
                "current_setting('cohere.titles_10pct'::text)", "'year'::text"
            ),
            setting_driven.replace("parse_with_field", "parse"),
            "upper(" + setting_driven + ")",
        ):
            expression["expr_desc"] = desc
            scan["Tantivy Query"] = json.dumps(
                {"postgres_expression": {"expr": expression}}
            )
            with self.assertRaisesRegex(ValueError, "unresolved expression"):
                verify(plan, filtered=True, neighbors=10, fields=["text"])
        expression["expr_desc"] = setting_driven
        scan["Tantivy Query"] = json.dumps(
            {
                "boolean": {
                    "must": [
                        {"postgres_expression": {"expr": expression}},
                        {"heap_filter": {"field_filters": []}},
                    ]
                }
            }
        )
        with self.assertRaisesRegex(ValueError, "heap filtering"):
            verify(plan, filtered=True, neighbors=10, fields=["text"])


if __name__ == "__main__":
    unittest.main()

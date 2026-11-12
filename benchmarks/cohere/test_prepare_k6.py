"""Regression checks for the pdbench-to-k6 handoff."""

import copy
import json
import unittest
from pathlib import Path

from prepare_k6 import (
    PROBE,
    QUERIES,
    SUITES,
    fixed_operating_points,
    operating_points,
    summary,
)
from verify_plans import verify


def reports(dataset="cohere"):
    """Build reports with shared probes and an unreached r99 target."""
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

    def test_summary_collapses_the_table_and_its_paragraph(self):
        """Keep the table and paragraph inside one details block that GitHub renders."""
        text = summary(operating_points(reports()))
        self.assertTrue(
            text.startswith(
                "<details><summary>Operating points</summary>\n\n| Workload / recall target |"
            )
        )
        self.assertTrue(text.endswith("results.\n\n</details>\n"))
        self.assertEqual((text.count("<details>"), text.count("</details>")), (1, 1))
        self.assertIn("\n\npdbench selects max_probe separately", text)
        self.assertEqual(text.count("| `knn_top10_"), 9)

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


class FixedOperatingPointsTest(unittest.TestCase):
    """Keep fixed HN probes measurable without overstating achieved recall."""

    def setUp(self):
        path = Path(__file__).parents[1] / "hn" / "pg_vectorsearch-probes.json"
        self.probes = json.loads(path.read_text())["nprobe"]
        self.inputs = []
        for report in reports("hn"):
            report["config"]["engine"] = "pg_vectorsearch"
            report["config"]["dataset"]["dims"] = 1536
            report["data"]["sweep_param"] = "prism.nprobe"
            query = report["config"]["search"]["query"]
            for probe in sorted(set(self.probes[query].values())):
                fixed = copy.deepcopy(report)
                fixed["data"]["fixed_probe"] = probe
                result = fixed["data"]["variants"][query]
                result["query"] = result["query"].replace("1024", "1536")
                result["targets"] = {}
                point = result["points"][0]
                point["value"] = str(probe)
                point["metrics"]["recall"].update(samples=100, recall=0.8)
                self.inputs.append(fixed)

    def test_all_missed_targets_still_have_the_pinned_probe_and_measurements(self):
        """An entirely below-target run must still produce all 18 k6 inputs."""
        original = copy.deepcopy(self.inputs)
        points = fixed_operating_points(self.inputs, self.probes)
        self.assertEqual(len(self.inputs), 9)
        self.assertEqual(len(points), 18)
        for point in points:
            self.assertTrue(point["fixed_probe"])
            self.assertFalse(point["reached"])
            self.assertFalse(point["require_full_neighbors"])
            self.assertEqual(
                point["probe"], self.probes[point["query"]][str(point["target"])]
            )
            self.assertEqual(point["recall"], 0.8)
            self.assertEqual(point["buffers"]["shared_hit_mean"], 1234.5)
            self.assertEqual(point["param"], "prism.nprobe")
            self.assertIn("$1::vector(1536)", point["sql"])
        self.assertEqual(self.inputs, original)
        text = summary(points)
        self.assertIn("`4096` (target missed)", text)
        self.assertIn("80.0%", text)
        self.assertNotIn("Unreached targets have no latency", text)

    def test_reached_uses_current_recall_without_changing_the_pinned_probe(self):
        """A recall change must not start a new search or alter the pinned setting."""
        for report in self.inputs:
            for result in report["data"]["variants"].values():
                result["points"][0]["metrics"]["recall"]["recall"] = 0.95
        points = fixed_operating_points(self.inputs, self.probes)
        self.assertEqual(sum(point["reached"] for point in points), 12)
        self.assertTrue(
            all(point["probe"] == 4096 for point in points if point["target"] == 0.99)
        )

    def test_rejects_missing_duplicate_or_partial_measurements(self):
        """Do not publish partial fixed-probe validation as a complete run."""
        with self.assertRaisesRegex(ValueError, "Missing fixed probe"):
            fixed_operating_points(self.inputs[:-1], self.probes)
        with self.assertRaisesRegex(ValueError, "duplicate fixed probe"):
            fixed_operating_points(self.inputs + self.inputs[:1], self.probes)
        result = next(iter(self.inputs[0]["data"]["variants"].values()))
        result["points"][0]["metrics"]["recall"]["samples"] = 25
        with self.assertRaisesRegex(ValueError, "all 100 query vectors"):
            fixed_operating_points(self.inputs, self.probes)

    def test_rejects_invalid_probe_maps_and_mixed_sessions(self):
        """Fail before exporting out-of-range probes or inconsistent measurements."""
        for value in (0, 4097, 1.5, True):
            probes = copy.deepcopy(self.probes)
            probes["range_10pct"]["0.9"] = value
            with self.assertRaisesRegex(ValueError, "integer nprobe"):
                fixed_operating_points(self.inputs, probes)
        self.inputs[-1]["config"]["search"]["session"]["jit"] = "on"
        with self.assertRaisesRegex(ValueError, "same dataset, index, and session"):
            fixed_operating_points(self.inputs, self.probes)


if __name__ == "__main__":
    unittest.main()

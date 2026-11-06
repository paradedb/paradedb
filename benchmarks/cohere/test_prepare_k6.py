"""Regression checks for the pdbench-to-k6 handoff."""

import copy
import unittest

from prepare_k6 import PROBES, QUERIES, operating_points, summary
from verify_plans import verify


def reports():
    """Build three reports with shared probes and an unreached r99 target."""
    return [
        {
            "kind": "search",
            "config": {
                "dataset": {"size": "1m", "table": "cohere_wiki"},
                "search": {
                    "query": query,
                    "sweep": {"tolerance": 0.000001, "min": 0.0001, "max": 1.0},
                    "session": {"cohere.titles_10pct": "year", "jit": "off"},
                },
            },
            "data": {
                "sweep_param": PROBES["pg_search"],
                "query_vectors": 100,
                "variants": {
                    query: {
                        "query": "SELECT _id FROM cohere_wiki ORDER BY emb <=> "
                        "current_setting('cohere.qvec')::vector(1024) LIMIT 10",
                        "targets": {
                            "0.9": {"value": "0.001", "recall": 0.96},
                            "0.95": {"value": "0.001", "recall": 0.96},
                        },
                        "points": [
                            {
                                "value": "0.001",
                                "metrics": {
                                    "recall": {"recall": 0.96},
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
        for query in QUERIES
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

    def test_prism_preserves_integer_probe_and_backend(self):
        """Hand off nprobe unchanged and reject fractional or out-of-range values."""
        inputs = reports()
        for report in inputs:
            report["config"]["engine"] = "pg_vectorsearch"
            report["config"]["search"]["sweep"].update(min=1, max=4096)
            report["data"]["sweep_param"] = "prism.nprobe"
            result = next(iter(report["data"]["variants"].values()))
            result["points"][0]["value"] = "95"
            for target in result["targets"].values():
                target["value"] = "95"
        points = operating_points(inputs)
        for point in points:
            self.assertEqual(point["probe_param"], "prism.nprobe")
            self.assertEqual(point["backend"], "pg_vectorsearch")
            self.assertEqual(point["index_name"], "cohere_wiki_prism_idx")
            if point["reached"]:
                self.assertEqual(point["probe"], "95")
        self.assertIn("not pushed", summary(points))
        for value in ["95.5", "4097", "0"]:
            invalid = copy.deepcopy(inputs)
            result = next(iter(invalid[0]["data"]["variants"].values()))
            result["points"][0]["value"] = value
            for target in result["targets"].values():
                target["value"] = value
            with self.assertRaisesRegex(
                ValueError, "invalid selected point|integer probe"
            ):
                operating_points(invalid)
        inputs[0]["data"]["sweep_param"] = PROBES["pg_search"]
        with self.assertRaisesRegex(ValueError, "selected engine"):
            operating_points(inputs)

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

    def test_prism_plan_guard_allows_postfilter_but_rejects_other_indexes(self):
        """PRISM must handle ordering while PostgreSQL applies the text filter."""
        scan = {
            "Node Type": "Index Scan",
            "Index Name": "cohere_wiki_prism_idx",
            "Filter": "to_tsvector('english', text) @@ 'year'::tsquery",
        }
        plan = [{"Plan": {"Node Type": "Limit", "Plans": [scan]}}]
        verify(plan, True, "pg_vectorsearch", "cohere_wiki_prism_idx")
        scan.pop("Filter")
        verify(plan, False, "pg_vectorsearch", "cohere_wiki_prism_idx")
        with self.assertRaisesRegex(ValueError, "full-text filtering"):
            verify(plan, True, "pg_vectorsearch", "cohere_wiki_prism_idx")
        for index in ["another_index", None]:
            with self.assertRaisesRegex(ValueError, "PRISM Index Scan"):
                verify(plan, False, "pg_vectorsearch", index)
        scan["Node Type"] = "Seq Scan"
        with self.assertRaisesRegex(ValueError, "PRISM Index Scan"):
            verify(plan, False, "pg_vectorsearch", "cohere_wiki_prism_idx")


if __name__ == "__main__":
    unittest.main()

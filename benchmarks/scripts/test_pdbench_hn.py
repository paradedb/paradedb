"""Check that benchmark plans reject heap filters and non-native vector sorting."""

import copy
import json
import unittest

import pdbench_hn as bench


class HNPlanTests(unittest.TestCase):
    """Exercise the EXPLAIN gate with the extension's JSON plan shape."""

    def setUp(self):
        self.scan = {
            "Node Type": "Custom Scan",
            "Custom Plan Provider": "ParadeDB Base Scan",
            "   TopK Order By": "emb <=> vector asc",
            "   TopK Limit": 10,
            "Tantivy Query": json.dumps(
                {
                    "boolean": {
                        "must": [
                            {"with_index": {"query": "all"}},
                            {"term": {"field": "type", "value": "story"}},
                            {
                                "range": {
                                    "field": "score",
                                    "lower_bound": {"included": 10},
                                }
                            },
                        ]
                    }
                }
            ),
        }
        self.plan = [{"Plan": {"Node Type": "Limit", "Plans": [self.scan]}}]

    def test_accepts_native_topk_and_literal_equality(self):
        """Require all referenced metadata fields inside the index query."""
        bench.check_plan(self.plan, "type = 'story' AND score >= 10")
        with self.assertRaisesRegex(ValueError, "Filter fields are missing"):
            bench.check_plan(self.plan, "\"by\" = 'author'")

    def test_rejects_postfilter_and_heap_filter(self):
        """A custom scan alone is insufficient evidence of indexed pushdown."""
        for node in (self.scan, self.plan[0]["Plan"]):
            node["Filter"] = "type = 'story'"
            with self.assertRaisesRegex(ValueError, "outside the ParadeDB index"):
                bench.check_plan(self.plan, "type = 'story'")
            del node["Filter"]
        self.scan["Tantivy Query"] = json.dumps({"heap_filter": {"query": "all"}})
        with self.assertRaisesRegex(ValueError, "heap evaluation"):
            bench.check_plan(self.plan, "type = 'story'")

    def test_rejects_external_vector_sort_and_multiple_scans(self):
        """The native ParadeDB scan must own the vector ordering and top-10 limit."""
        del self.scan["   TopK Order By"]
        with self.assertRaisesRegex(ValueError, "Vector TopK"):
            bench.check_plan(self.plan, "TRUE")
        self.plan[0]["Plan"]["Plans"].append(copy.deepcopy(self.scan))
        with self.assertRaisesRegex(ValueError, "one ParadeDB Base Scan"):
            bench.check_plan(self.plan, "TRUE")


if __name__ == "__main__":
    unittest.main()

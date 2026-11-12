"""Keep PRISM ANN results distinct from PostgreSQL exact search."""

import unittest

from verify_plans import verify_prism


class PrismPlanTest(unittest.TestCase):
    """Validate the execution paths accepted by the vector benchmarks."""

    def test_prism_requires_the_expected_ordered_index(self):
        """A B-tree scan cannot masquerade as an unfiltered PRISM benchmark."""
        scan = {
            "Node Type": "Index Scan",
            "Index Name": "msmarco_prism_idx",
            "Order By": "emb <=> '[1,2,3]'::vector",
        }
        plan = [{"Plan": {"Node Type": "Limit", "Plans": [scan]}}]
        self.assertEqual(verify_prism(plan, "msmarco_prism_idx", []), "prism")
        for key, value in (("Index Name", "other_idx"), ("Order By", "id")):
            changed = [{"Plan": scan | {key: value}}]
            with self.assertRaisesRegex(ValueError, "Expected a cosine PRISM"):
                verify_prism(changed, "msmarco_prism_idx", [])

    def test_prism_postfilter_requires_the_scalar_predicate(self):
        """The vector scan must retain the exact-author predicate."""
        scan = {
            "Node Type": "Index Scan",
            "Index Name": "hn_items_prism_idx",
            "Order By": "emb <=> '[1,2,3]'::vector",
            "Filter": "(\"by\" = 'DanBC'::text)",
        }
        self.assertEqual(
            verify_prism([{"Plan": scan}], "hn_items_prism_idx", ["by"]),
            "prism-postfilter",
        )
        del scan["Filter"]
        with self.assertRaisesRegex(ValueError, "every HN predicate"):
            verify_prism([{"Plan": scan}], "hn_items_prism_idx", ["by"])

    def test_exact_filtering_is_rejected(self):
        """Do not measure B-tree or sequential filtering as a PRISM workload."""
        for scan in (
            {
                "Node Type": "Index Scan",
                "Index Name": "hn_items_time_idx",
                "Index Cond": "time >= '2021-09-30'::timestamptz",
            },
            {
                "Node Type": "Seq Scan",
                "Relation Name": "hn_items",
                "Filter": "time >= '2021-09-30'::timestamptz",
            },
        ):
            sort = {
                "Node Type": "Sort",
                "Sort Key": ["(emb <=> '[1,2,3]'::vector)"],
                "Plans": [scan],
            }
            with self.assertRaisesRegex(ValueError, "Expected a cosine PRISM"):
                verify_prism([{"Plan": sort}], "hn_items_prism_idx", ["time"])
            with self.assertRaisesRegex(ValueError, "Expected a cosine PRISM"):
                verify_prism([{"Plan": sort}], "hn_items_prism_idx", [])

    def test_rejects_paradedb_and_unordered_fallbacks(self):
        """Do not publish another engine's work as PRISM."""
        with self.assertRaisesRegex(ValueError, "custom scan"):
            verify_prism(
                [{"Plan": {"Custom Plan Provider": "ParadeDB Base Scan"}}],
                "hn_items_prism_idx",
                [],
            )
        with self.assertRaisesRegex(ValueError, "Expected a cosine PRISM"):
            verify_prism(
                [{"Plan": {"Node Type": "Seq Scan", "Filter": "time > now()"}}],
                "hn_items_prism_idx",
                ["time"],
            )


if __name__ == "__main__":
    unittest.main()

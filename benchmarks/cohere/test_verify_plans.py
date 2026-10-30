"""Ensure router verification detects a benchmark silently rebuilding without quantization."""

import unittest

from verify_plans import routing_metrics, routing_summary, verify


def segment(**overrides):
    return {
        "routing_kind": "quantized_stacked",
        "routing_lists_scanned": 4,
        "routing_approximate_scored": 32,
        "routing_approximate_members_scored": 24,
        "routing_approximate_pruned": 18,
        "routing_exact_scored": 6,
        "routing_exact_pruned": 3,
        "routing_candidate_count": 3,
        **overrides,
    }


class VerifyRouterTest(unittest.TestCase):
    def test_router_funnel_sums_segments_and_preserves_parent_scores(self):
        scan = {
            "Custom Plan Provider": "ParadeDB Scan",
            "Segment Info": {"a": segment(), "b": segment()},
        }
        scans = verify([{"Plan": {"Plans": [scan]}}], filtered=False)
        totals = routing_metrics(scans, require_routing=True)
        self.assertEqual(totals["routed_segments"], 2)
        self.assertEqual(totals["routing_approximate_scored"], 64)
        self.assertEqual(totals["routing_approximate_members_scored"], 48)
        self.assertEqual(totals["routing_approximate_pruned"], 36)
        self.assertEqual(totals["routing_exact_scored"], 12)
        self.assertEqual(totals["routing_exact_pruned"], 6)
        self.assertEqual(totals["routing_candidate_count"], 6)
        summary = routing_summary({"unfiltered@r95": totals})
        self.assertIn("| 2 | 16 | 48 | 36 | 12 | 6 |", summary)
        self.assertIn("not workload averages", summary)

    def test_full_precision_router_or_missing_stats_cannot_pass(self):
        with self.assertRaisesRegex(ValueError, "found stacked"):
            routing_metrics(
                [{"Segment Info": {"a": segment(routing_kind="stacked")}}], True
            )
        for scans in ([], [{}], [{"Segment Info": {"a": {}}}]):
            with self.assertRaisesRegex(ValueError, "did not exercise"):
                routing_metrics(scans, True)

    def test_inconsistent_or_absent_counts_cannot_pass(self):
        for bad in (
            segment(routing_approximate_pruned=19),
            segment(routing_exact_pruned=4),
            segment(routing_approximate_scored=1),
            segment(routing_exact_scored=-1),
            segment(routing_exact_scored=None),
        ):
            with self.assertRaises(ValueError):
                routing_metrics([{"Segment Info": {"a": bad}}], True)
        empty = segment(**{key: 0 for key in segment() if key != "routing_kind"})
        with self.assertRaisesRegex(ValueError, "did not exercise"):
            routing_metrics([{"Segment Info": {"a": empty}}], True)

    def test_located_filter_can_bypass_router(self):
        scans = [{"Segment Info": {"a": {"access_path": "Located", "exact_rows": 10}}}]
        counts = routing_metrics(scans, require_routing=False)
        self.assertEqual(counts["routed_segments"], 0)
        self.assertEqual(counts["routing_exact_scored"], 0)


if __name__ == "__main__":
    unittest.main()

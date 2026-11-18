"""Read the router, worker and filter path from executed pg_search plans."""

import json
import unittest

from verify_plans import execution_path


def plan(scan, workers=None):
    """An EXPLAIN (FORMAT JSON) plan with the scan under a Gather when `workers` is set."""
    node = {
        "Node Type": "Custom Scan",
        "Custom Plan Provider": "ParadeDB Base Scan",
    } | scan
    if workers is not None:
        planned, launched = workers
        node = {
            "Node Type": "Gather Merge",
            "Workers Planned": planned,
            "Workers Launched": launched,
            "Plans": [node],
        }
    return [{"Plan": {"Node Type": "Limit", "Plans": [node]}, "Execution Time": 1.0}]


class ExecutionPathTest(unittest.TestCase):
    def test_parallel_mixed_router_and_filter(self):
        info = {
            "a": {"access_path": "Located"},
            "b": {"access_path": "Located"},
            "c": {"access_path": "Routed"},
        }
        scan = {
            "Segment Info": json.dumps(info),
            "Vector Filtering": "Pre (fast fields)",
        }
        self.assertEqual(
            execution_path(plan(scan, workers=(4, 3))),
            {
                "router": "located 2/3, routed 1/3",
                "execution": "parallel 3/4",
                "filter": "Pre (fast fields)",
            },
        )

    def test_serial_unfiltered(self):
        info = {"a": {"access_path": "Routed"}, "b": {"access_path": "Routed"}}
        self.assertEqual(
            execution_path(plan({"Segment Info": info})),
            {"router": "routed 2/2", "execution": "serial", "filter": "—"},
        )

    def test_without_vector_stats(self):
        scan = {"Vector Filtering": "Post (before quantized scoring)"}
        self.assertEqual(
            execution_path(plan(scan, workers=(4, 4))),
            {
                "router": "—",
                "execution": "parallel 4/4",
                "filter": "Post (before quantized scoring)",
            },
        )


if __name__ == "__main__":
    unittest.main()

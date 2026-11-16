"""Exercise the Cohere comparison handoff from recall reports to k6 plans."""

import copy
import unittest
from pathlib import Path

import tomllib
from prepare_k6 import QUERIES, TARGETS, operating_points
from verify_plans import verify_ann

HERE = Path(__file__).parent
QUERY_COUNT = 100
NEIGHBORS = 10
VALIDATED_RECALL = 0.995
SELECTED_PROBE = 128


def reports_for(profile):
    """Render the selected engine's SQL into complete pdbench search reports."""
    config = tomllib.loads((HERE / f"10m-{profile}.toml").read_text())
    assets = HERE.parent / "assets" / "cohere" / config["engine"] / "queries"
    reports = []
    for query in QUERIES:
        sql = (assets / f"{query}.sql").read_text()
        sql = sql.replace("{{ table }}", config["dataset"]["table"])
        sql = sql.replace("{{ dims }}", str(config["dataset"]["dims"]))
        reports.append(
            {
                "kind": "search",
                "config": copy.deepcopy(config),
                "data": {
                    "sweep_param": config["search"]["sweep"]["param"],
                    "query_vectors": QUERY_COUNT,
                    "fast_search": True,
                    "variants": {
                        query: {
                            "query": sql,
                            "points": [
                                {
                                    "value": SELECTED_PROBE,
                                    "query_vectors": QUERY_COUNT,
                                    "metrics": {
                                        "recall": {
                                            "k": NEIGHBORS,
                                            "recall": VALIDATED_RECALL,
                                            "samples": QUERY_COUNT,
                                        },
                                        "buffers": {"shared_hit_mean": 12},
                                    },
                                }
                            ],
                            "targets": {
                                str(target): {
                                    "value": SELECTED_PROBE,
                                    "recall": VALIDATED_RECALL,
                                }
                                for target in TARGETS
                            },
                        }
                    },
                },
            }
        )
    return reports


class ComparisonHandoffTest(unittest.TestCase):
    """Keep competitor recall selections and execution modes intact."""

    def test_all_modes_preserve_filters_sessions_and_index_identity(self):
        """The real profiles and query assets retain their engine and predicates."""
        for profile, method, mode in (
            ("pg_vectorsearch", "prism", "postfilter"),
            ("vectorchord", "vchordrq", "postfilter"),
            ("vectorchord-prefilter", "vchordrq", "prefilter"),
        ):
            with self.subTest(profile=profile):
                points = operating_points(reports_for(profile))
                self.assertEqual(len(points), len(QUERIES) * len(TARGETS))
                for point in points:
                    self.assertIn("$1::vector(1024)", point["sql"])
                    self.assertEqual(point["index_name"], f"cohere_wiki_{method}_idx")
                    filtered = not point["query"].endswith("unfiltered")
                    self.assertEqual(
                        point["filter_fields"], ["text"] if filtered else []
                    )
                    scan = {
                        "Node Type": "Index Scan",
                        "Index Name": point["index_name"],
                        "Order By": "emb <=> '[0,1]'::vector",
                    }
                    if filtered:
                        predicate = (
                            point["sql"].split("WHERE ", 1)[1].split("ORDER BY")[0]
                        )
                        self.assertIn("to_tsvector('english', text)", predicate)
                        scan["Filter"] = predicate
                    if method == "vchordrq":
                        self.assertEqual(
                            point["session"]["vchordrq.prefilter"],
                            "on" if mode == "prefilter" else "off",
                        )
                    self.assertEqual(
                        verify_ann(
                            [{"Plan": scan}],
                            point["index_name"],
                            point["filter_fields"],
                            method,
                            prefilter=mode == "prefilter",
                        ),
                        f"{method}-{mode}" if filtered else method,
                    )
                    if filtered:
                        del scan["Filter"]
                        with self.assertRaisesRegex(
                            ValueError, "every filter predicate"
                        ):
                            verify_ann(
                                [{"Plan": scan}], point["index_name"], ["text"], method
                            )

    def test_pilot_results_or_missing_workloads_cannot_enter_k6(self):
        """Only complete workload sets validated on every vector are measurable."""
        reports = reports_for("vectorchord-prefilter")
        with self.assertRaisesRegex(ValueError, "Missing recall sweeps"):
            operating_points(reports[:-1])
        variant = next(iter(reports[0]["data"]["variants"].values()))
        variant["points"][0]["query_vectors"] = QUERY_COUNT // 4
        with self.assertRaisesRegex(ValueError, "full 100-vector validation"):
            operating_points(reports)

    def test_mixed_prefilter_modes_are_rejected(self):
        """A mixed session configuration cannot produce a single result series."""
        reports = reports_for("vectorchord-prefilter")
        reports[-1]["config"]["search"]["session"]["vchordrq.prefilter"] = "off"
        with self.assertRaisesRegex(ValueError, "same dataset, index, and session"):
            operating_points(reports)


if __name__ == "__main__":
    unittest.main()

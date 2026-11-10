"""Regression checks for the router-quality comparison."""

import copy
import json
import tempfile
import tomllib
import unittest
from pathlib import Path

import router_quality as rq
from prepare_k6 import PROBE, QUERIES, TARGETS

PROFILE = Path(__file__).parent / "1m.toml"


def segment(**overrides):
    """One routed segment's statistics, consistent with the checks."""
    stats = {
        "access_path": "Routed",
        "routing_kind": "quantized_stacked",
        "routing_mode": "hierarchical",
        "termination": "Ceiling",
        "postings_row": 10,
        "postings_skipped": 2,
        "clusters_skipped_empty": 2,
        "vectors_visited": 1000,
        "layer0_eligible": 900,
        "pruned_filter": 90,
        "pruned_dead": 10,
        "candidates_scored": 800,
        "exact_rows": 40,
        "routing_ns": 1_500_000,
        "routing_approximate_scored": 300,
        "routing_exact_scored": 24,
        "segment_rows": 50_000,
        "segment_clusters": 100,
    }
    return stats | overrides


def plan(segments, execution_ms=10.0, ids=None, workers=0):
    """An EXPLAIN (FORMAT JSON) plan whose scan carries Segment Info as a JSON string, under a
    Gather when `workers` are planned."""
    ids = ids or [f"seg{i}" for i in range(len(segments))]
    info = dict(zip(ids, segments))
    node = {"Node Type": "Custom Scan", "Segment Info": json.dumps(info)}
    if workers:
        node = {
            "Node Type": "Gather Merge",
            "Workers Planned": workers,
            "Workers Launched": workers - 1,
            "Plans": [node],
        }
    return [{"Plan": {"Node Type": "Limit", "Plans": [node]}, "Execution Time": execution_ms}]


def arm_cell(reached=True, **values):
    if not reached:
        return {"reached": False}
    cell = {
        "reached": True,
        "budget": "100",
        "recall": 0.95,
        "clusters": 100.0,
        "vectors_probed": 40_000.0,
        "valid_vectors": 30_000.0,
        "empty_clusters": 1.5,
        "layer0_scored": 20_000.0,
        "exact_rows": 50.0,
        "routing_ms": 0.8,
        "routing_ms_p95": 1.2,
        "centroids_1bit": 1200.0,
        "centroids_exact": 240.0,
        "query_ms": 8.0,
        "routing_share": 0.1,
        "adaptive_stops": 0,
    }
    return cell | values


def measurements(cells=None, fingerprint="abc"):
    """Every arm x query x target reached with equal work unless `cells` overrides one."""
    cells = cells or {}
    results = {
        query: {
            str(target): {
                arm: cells.get((query, target, arm), arm_cell()) for arm in rq.ARMS
            }
            for target in TARGETS
        }
        for query in QUERIES
    }
    return {
        "dataset": {"name": "cohere", "size": "1m"},
        "fingerprint": fingerprint,
        "query_vectors": 100,
        "arms": {arm: {"session": {rq.AUDIT: rq.ARMS[arm][0]}} for arm in rq.ARMS},
        "index": {
            "segments": 8,
            "clusters": 2445,
            "rows_per_cluster": 409.0,
            "routing_kind": "quantized_stacked",
            "rerank_factor": "2",
        },
        "checks": {name: {"failed": 0} for name in rq.CHECKS},
        "checks_passed": True,
        "results": results,
    }


class ConfigsTest(unittest.TestCase):
    def test_writes_four_arms_differing_only_in_audit_and_adaptive_settings(self):
        source = tomllib.loads(PROFILE.read_text())
        with tempfile.TemporaryDirectory() as directory:
            rq.configs(PROFILE, Path(directory))
            written = sorted(path.name for path in Path(directory).iterdir())
            self.assertEqual(written, sorted(f"router-{arm}.toml" for arm in rq.ARMS))
            for arm, (audit, adaptive) in rq.ARMS.items():
                config = tomllib.loads((Path(directory) / f"router-{arm}.toml").read_text())
                session = config["search"]["session"]
                self.assertEqual(session.pop(rq.AUDIT), audit)
                self.assertEqual(session.pop(rq.ADAPTIVE), adaptive)
                self.assertEqual(config, source)
        self.assertEqual(rq.ARMS["stored-adaptive"], ("stored", "on"))
        self.assertEqual(rq.ARMS["flat-adaptive"], ("flat", "on"))

    def test_refuses_a_profile_that_already_sets_the_audit(self):
        text = PROFILE.read_text().replace(
            "[search.session]\n", f'[search.session]\n"{rq.AUDIT}" = "flat"\n'
        )
        with self.assertRaises(ValueError):
            rq.arm_profile(text, "stored")


class StatsTest(unittest.TestCase):
    def test_sums_two_segments_and_flags_the_adaptive_stop(self):
        first = segment()
        second = segment(
            postings_row=5,
            routing_ns=500_000,
            termination=rq.ADAPTIVE_STOP,
            routing_approximate_scored=None,
        )
        del second["routing_approximate_scored"]
        totals, segments = rq.query_totals(plan([first, second], execution_ms=20.0))
        self.assertEqual(len(segments), 2)
        self.assertEqual(totals["clusters"], 12 + 7)
        self.assertEqual(totals["vectors_probed"], 2000)
        self.assertEqual(totals["valid_vectors"], 1800)
        self.assertEqual(totals["empty_clusters"], 4)
        self.assertEqual(totals["layer0_scored"], 1600)
        self.assertEqual(totals["exact_rows"], 80)
        self.assertEqual(totals["centroids_1bit"], 300)
        self.assertEqual(totals["centroids_exact"], 48)
        self.assertAlmostEqual(totals["routing_ms"], 2.0)
        self.assertEqual(totals["query_ms"], 20.0)
        self.assertTrue(totals["adaptive_stop"])

    def test_reads_parallel_workers_from_the_plan(self):
        serial, _ = rq.query_totals(plan([segment()]))
        parallel, _ = rq.query_totals(plan([segment()], workers=8))
        self.assertEqual((serial["parallel"], serial["workers_launched"]), (False, 0))
        self.assertEqual((parallel["parallel"], parallel["workers_launched"]), (True, 7))
        summary = rq.summarize([serial, parallel])
        self.assertEqual((summary["parallel_plans"], summary["workers_launched"]), (1, 3.5))

    def test_index_facts_cover_every_segment_id_across_plans(self):
        seen = {}
        seen.update(rq.segment_entries(plan([segment(), segment()], ids=["a", "b"])))
        seen.update(rq.segment_entries(plan([segment(segment_clusters=50)], ids=["c"])))
        facts = rq.index_facts(seen.values())
        self.assertEqual(facts["segments"], 3)
        self.assertEqual(facts["clusters"], 250)
        self.assertEqual(facts["rows_per_cluster"], 150_000 / 250)

    def test_summary_reports_means_tail_share_and_adaptive_stops(self):
        per_query = [
            rq.query_totals(plan([segment(routing_ns=ns, termination=t)], execution_ms=10.0))[0]
            for ns, t in [(1_000_000, "Ceiling")] * 19 + [(9_000_000, rq.ADAPTIVE_STOP)]
        ]
        summary = rq.summarize(per_query)
        self.assertAlmostEqual(summary["routing_ms"], 1.4)
        self.assertAlmostEqual(summary["routing_ms_p95"], 1.0)
        self.assertAlmostEqual(summary["routing_share"], 0.14)
        self.assertEqual(summary["adaptive_stops"], 1)

    def test_checks_accept_consistent_stats_and_name_each_violation(self):
        checks = rq.Checks()
        checks.observe("stored", "knn_top10_10pct", [segment(), segment()])
        checks.observe("flat", "knn_top10_10pct", [segment(routing_mode="flat")])
        checks.observe(
            "stored-adaptive", "knn_top10_unfiltered", [segment(pruned_filter=0, termination=rq.ADAPTIVE_STOP)]
        )
        self.assertTrue(checks.passed())
        for arm, query, bad, name in [
            ("stored", "knn_top10_1pct", segment(access_path="Located"), "routed"),
            ("stored", "knn_top10_1pct", segment(routing_mode="flat"), "flat_mode"),
            ("flat-adaptive", "knn_top10_1pct", segment(), "flat_mode"),
            ("flat", "knn_top10_1pct", segment(routing_mode="flat", pruned_dead=0), "filtered_rows"),
            ("stored", "knn_top10_unfiltered", segment(), "unfiltered_rows"),
            ("stored", "knn_top10_1pct", segment(termination=rq.ADAPTIVE_STOP), "fixed_no_adaptive_stop"),
        ]:
            checks = rq.Checks()
            checks.observe(arm, query, [bad])
            failed = {key for key, result in checks.results.items() if result["failed"]}
            self.assertEqual(failed, {name}, (arm, query))
        checks = rq.Checks()
        checks.observe("stored", "knn_top10_unfiltered", [segment(vectors_visited=0, pruned_filter=0)])
        self.assertEqual(checks.results["unfiltered_rows"]["failed"], 1)

    def test_parses_consecutive_explain_documents(self):
        documents = [plan([segment()]), plan([segment(), segment()])]
        text = "\n".join(json.dumps(doc, indent=2) for doc in documents) + "\n"
        self.assertEqual(rq.parse_plans(text), documents)

    def test_script_discards_one_warm_pass_before_the_measured_pass(self):
        script = rq.explain_script(
            {"jit": "off", PROBE: "42"},
            "SELECT id FROM t ORDER BY emb <=> $1::vector(3) LIMIT 10;",
            ["[1,2,3]", "[4,5,6]"],
            3,
        )
        self.assertIn("SET paradedb.vector_cluster_max_probe = '42';", script)
        self.assertIn("PREPARE router_quality(vector(3)) AS SELECT", script)
        warm, measured = script.split("\\o /dev/null\n")[1].split("\\o\n")
        self.assertEqual(warm, measured)
        self.assertEqual(measured.count("EXECUTE router_quality("), 2)


class MeasureTest(unittest.TestCase):
    def setUp(self):
        self.parallel = {"stored", "flat"}
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.runs = Path(self.temp.name)
        base = tomllib.loads(PROFILE.read_text())
        for arm, (audit, adaptive) in rq.ARMS.items():
            directory = self.runs / arm / f"router-{arm}"
            directory.mkdir(parents=True)
            for index, query in enumerate(QUERIES):
                config = copy.deepcopy(base)
                config["search"]["query"] = query
                config["search"]["session"] |= {rq.AUDIT: audit, rq.ADAPTIVE: adaptive}
                targets = {str(t): {"value": "50", "recall": t} for t in TARGETS}
                if arm == "stored-adaptive" and query == "knn_top10_1pct":
                    del targets["0.99"]
                report = {
                    "kind": "search",
                    "config": config,
                    "data": {
                        "sweep_param": PROBE,
                        "variants": {
                            query: {
                                "query": f"SELECT _id FROM cohere ORDER BY emb <=> "
                                f"{rq.QVEC}::vector(1024) LIMIT 10",
                                "targets": targets,
                            }
                        },
                    },
                }
                (directory / f"2026_{index}_search.json").write_text(json.dumps(report))

    def fake_run(self, script):
        if "SHOW paradedb.vector_router_rerank_factor" in script:
            return "2\n"
        audit = "flat" if f"{rq.AUDIT} = 'flat'" in script else "stored"
        adaptive = f"{rq.ADAPTIVE} = 'on'" in script
        unfiltered = "WHERE" not in script and "cohere.titles" not in script
        stats = segment(
            routing_mode="flat" if audit == "flat" else "hierarchical",
            termination=rq.ADAPTIVE_STOP if adaptive else "Ceiling",
            postings_row=8 if audit == "flat" else 10,
        )
        if unfiltered:
            stats |= {"pruned_filter": 0}
        count = script.split("\\o\n")[1].count("EXECUTE")
        workers = 8 if audit in self.parallel else 0
        ids = ["seg0", "seg1"] if "'50'" in script else ["seg1", "seg2"]
        return "\n".join(
            json.dumps(plan([stats, stats], ids=ids, workers=workers)) for _ in range(count)
        )

    def test_measures_every_arm_with_adaptive_stop_counts(self):
        result = rq.measure(self.runs, ["[1]", "[2]"], run=self.fake_run)
        self.assertEqual(result["index"]["segments"], 2)
        self.assertEqual(result["plan_shapes"], {query: ["parallel"] for query in QUERIES})
        self.assertEqual(result["checks"]["plan_length_rule"]["failed"], 0)
        self.assertEqual(result["results"]["knn_top10_10pct"]["0.9"]["flat"]["parallel_plans"], 2)
        self.assertEqual(result["index"]["rerank_factor"], "2")
        self.assertEqual(set(result["arms"]), set(rq.ARMS))
        cell = result["results"]["knn_top10_10pct"]["0.9"]
        self.assertEqual(cell["stored"]["clusters"], 24)
        self.assertEqual(cell["flat"]["clusters"], 20)
        self.assertEqual(cell["stored"]["adaptive_stops"], 0)
        self.assertEqual(cell["stored-adaptive"]["adaptive_stops"], 2)
        self.assertFalse(result["results"]["knn_top10_1pct"]["0.99"]["stored-adaptive"]["reached"])
        self.assertEqual(result["checks"]["fixed_no_adaptive_stop"]["failed"], 0)
        self.assertEqual(result["checks"]["routed"]["failed"], 0)
        self.assertEqual(result["terminations"]["stored"], {"Ceiling": 2 * 2 * 9})
        self.assertEqual(result["terminations"]["stored-adaptive"], {rq.ADAPTIVE_STOP: 2 * 2 * 8})

    def test_index_facts_union_segment_ids_and_plan_shapes_must_agree(self):
        for path in (self.runs / "stored").rglob("*_search.json"):
            report = json.loads(path.read_text())
            (variant,) = report["data"]["variants"].values()
            variant["targets"]["0.95"]["value"] = "51"
            path.write_text(json.dumps(report))
        self.parallel = {"stored"}
        result = rq.measure(self.runs, ["[1]"], run=self.fake_run)
        self.assertEqual(result["index"]["segments"], 3)
        self.assertEqual(result["plan_shapes"]["knn_top10_1pct"], ["parallel", "serial"])
        self.assertEqual(result["checks"]["plan_length_rule"]["failed"], 3)
        self.assertFalse(result["checks_passed"])

    def test_rejects_arms_that_differ_beyond_the_audit_settings(self):
        for path in (self.runs / "flat").rglob("*_search.json"):
            report = json.loads(path.read_text())
            report["config"]["search"]["session"]["jit"] = "on"
            path.write_text(json.dumps(report))
        with self.assertRaises(ValueError):
            rq.measure(self.runs, ["[1]"], run=self.fake_run)


class RenderTest(unittest.TestCase):
    def render(self, measured, baseline=None):
        with tempfile.TemporaryDirectory() as directory:
            report = rq.render(measured, Path(directory), "c" * 40, "p" * 40, baseline)
            text = (Path(directory) / "router_quality.md").read_text()
            stored = json.loads((Path(directory) / "router_quality.json").read_text())
        self.assertEqual(stored["verdict"], report["verdict"])
        self.assertEqual(set(stored["arms"]), set(rq.ARMS))
        return report, text

    def test_matching_router_passes_without_a_baseline(self):
        report, text = self.render(measurements())
        self.assertEqual(report["verdict"], "✅")
        self.assertIn("**✅ Router matches flat at r90 and r95.**", text)
        self.assertIn("#### Router quality: Cohere 1m, 8 segments", text)
        self.assertIn("K = 2,445 · 409 rows per cluster (mean) · quantized_stacked router", text)
        self.assertIn("| r90 | 100 | 100 | 1.00× | 40.0k | 40.0k | 1.00× | ✅ |", text)
        self.assertIn("| r90 | 100 | 100 | 1.00× | 30.0k | 30.0k | 1.00× | 1.5 | ✅ |", text)
        self.assertIn("| Unfiltered | 0.8 (10%) | 0.8 |", text)
        self.assertIn(rq.ROUTING_NOTE, text)
        self.assertIn("No main baseline yet.", text)
        self.assertEqual(text.count("**With adaptive probing on** (clusters probed per query)"), 3)
        self.assertIn("| r90 | 100 | 100 | 1.00× | 0% | ✅ |", text)
        self.assertIn("| Unfiltered | r90 | stored-adaptive | `100` |", text)
        self.assertIn("| 0/100 |", text)

    def test_ratios_status_and_verdict(self):
        cells = {
            ("knn_top10_10pct", 0.95, "stored"): arm_cell(clusters=130.0),
            ("knn_top10_1pct", 0.99, "stored"): arm_cell(clusters=150.0),
            ("knn_top10_unfiltered", 0.9, "stored"): arm_cell(vectors_probed=43_000.0),
        }
        report, text = self.render(measurements(cells))
        statuses = {(row["query"], row["target"]): row["status"] for row in report["rows"]}
        self.assertEqual(statuses[("knn_top10_10pct", 0.95)], "❌")
        self.assertEqual(statuses[("knn_top10_1pct", 0.99)], "⚠️")
        self.assertEqual(statuses[("knn_top10_unfiltered", 0.9)], "❌")
        self.assertEqual(statuses[("knn_top10_unfiltered", 0.95)], "✅")
        self.assertEqual(report["verdict"], "❌")
        self.assertIn(
            "**❌ Router needs 1.30× flat's clusters at r95 (Filter 10%).** "
            "⚠️ r99 needs 1.50× flat's clusters (Filter 1%, reported, not gated).",
            text,
        )
        self.assertIn("| r95 | 130 | 100 | 1.30× |", text)

    def test_unreached_targets_print_not_reached_and_no_ratio(self):
        cells = {
            ("knn_top10_unfiltered", 0.99, "stored"): arm_cell(reached=False),
            ("knn_top10_1pct", 0.99, "flat"): arm_cell(reached=False),
        }
        report, text = self.render(measurements(cells))
        self.assertIn(
            "| r99 | not reached | 100 |  | not reached | 40.0k |  | ⚠️ |", text
        )
        self.assertIn("| r99 | 100 | not reached |  | 30.0k | not reached |  | 1.5 | — |", text)
        self.assertIn("Router does not reach r99 (Unfiltered); flat does (reported, not gated).", text)
        self.assertEqual(report["verdict"], "✅")

    def test_adaptive_table_status_and_verdict_suffix(self):
        cells = {
            ("knn_top10_unfiltered", 0.9, "stored-adaptive"): arm_cell(clusters=89.0),
            ("knn_top10_unfiltered", 0.95, "stored-adaptive"): arm_cell(clusters=120.0),
            ("knn_top10_10pct", 0.99, "stored-adaptive"): arm_cell(reached=False),
            ("knn_top10_1pct", 0.99, "flat-adaptive"): arm_cell(reached=False),
        }
        report, text = self.render(measurements(cells))
        self.assertIn("| r90 | 89 | 100 | 0.89× | −11% | ✅ |", text)
        self.assertIn("| r95 | 120 | 100 | 1.20× | +20% | ⚠️ |", text)
        self.assertIn("| r99 | not reached | 100 |  |  | ⚠️ |", text)
        self.assertIn("| r99 | 100 | not reached |  | 0% | — |", text)
        self.assertEqual(report["verdict"], "✅")
        self.assertIn(
            "**✅ Router matches flat at r90 and r95.** "
            "⚠️ Adaptive probing stops short of r99 on Filter 10%.",
            text,
        )
        self.assertNotIn("❌", "".join(row["status"] for row in report["adaptive_rows"]))

    def test_against_main_with_matching_and_differing_baselines(self):
        baseline_cells = {
            ("knn_top10_unfiltered", 0.9, "stored"): arm_cell(clusters=110.0, routing_ms=1.0),
            ("knn_top10_unfiltered", 0.9, "stored-adaptive"): arm_cell(clusters=80.0),
        }
        baseline, _ = self.render(measurements(baseline_cells))
        baseline["commit"] = "b" * 40
        _, text = self.render(measurements(), baseline)
        self.assertIn(f"**Against main** (`{'b' * 40}`)", text)
        self.assertIn("| Router / flat clusters (unfiltered) | 1.10× → 1.00× |", text)
        self.assertIn("| Routing ms (unfiltered) | 1.0 → 0.8 (-20.0%) |", text)
        self.assertIn("| Routing share of query time (unfiltered) | 10% → 10% |", text)
        self.assertIn(
            "| Router clusters, adaptive probing (unfiltered) | 80 → 100 (+25.0%) |", text
        )
        self.assertIn("| Flat vectors probed (clustering) | 40.0k → 40.0k (+0.0%) |", text)
        _, text = self.render(measurements(fingerprint="other"), baseline)
        self.assertIn("baseline configuration differs", text)
        self.assertNotIn("| Router / flat clusters", text)


if __name__ == "__main__":
    unittest.main()

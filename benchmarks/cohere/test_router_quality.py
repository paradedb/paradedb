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
        "checks": {
            name: {"description": text, "checked": 1, "failed": 0, "example": None}
            for name, text in rq.CHECKS.items()
        },
        "checks_passed": True,
        "results": results,
    }


class ConfigsTest(unittest.TestCase):
    def test_writes_three_arms_differing_only_in_audit_and_adaptive_settings(self):
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
        self.assertEqual(
            rq.ARMS,
            {"stored": ("stored", "off"), "flat": ("flat", "off"), "stored-adaptive": ("stored", "on")},
        )

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
            ("flat", "knn_top10_1pct", segment(), "flat_mode"),
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
        self.assertEqual(result["tolerated"], [])
        self.assertEqual(result["terminations"]["stored"], {"Ceiling": 2 * 2 * 9})
        self.assertEqual(result["terminations"]["stored-adaptive"], {rq.ADAPTIVE_STOP: 2 * 2 * 8})

    def test_reads_tolerated_runs(self):
        (self.runs / rq.TOLERATED).write_text(
            "flat knn_top10_10pct benchmark-artifacts/router/flat/x_search.json\n\n"
        )
        result = rq.measure(self.runs, ["[1]"], run=self.fake_run)
        self.assertEqual(
            result["tolerated"],
            [
                {
                    "arm": "flat",
                    "query": "knn_top10_10pct",
                    "report": "benchmark-artifacts/router/flat/x_search.json",
                }
            ],
        )

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


HEADER = (
    "| | Flat clusters | Router clusters | Ratio | Flat vectors | Router vectors | Ratio "
    "| Adaptive on vs off | Routing ms |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|\n"
)
ROW_ORDER = [
    f"{level} {target}"
    for level in ("Unfiltered", "Filter 10%", "Filter 1%")
    for target in ("r90", "r95", "r99")
]
U = "knn_top10_unfiltered"


def table(text):
    """The comment's one table as {row label: {column id: cell}}, in row order."""
    tables = text.count("|---|")
    assert tables == 1, f"expected one table, found {tables}"
    rows = {}
    for line in text.split(HEADER)[1].split("\n\n")[0].splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        rows[cells[0]] = dict(zip([column[0] for column in rq.COLUMNS], cells[1:], strict=True))
    return rows


class RenderTest(unittest.TestCase):
    URL = "https://paradedb.github.io/paradedb/benchmarker-runs/c/cohere-1m/sweep/router_quality.html"

    def render(self, measured, baseline=None):
        with tempfile.TemporaryDirectory() as directory:
            report = rq.render(measured, Path(directory), "c" * 40, "p" * 40, baseline, self.URL)
            files = {
                name: (Path(directory) / name).read_text()
                for name in (
                    "router_quality.md",
                    "router_quality.json",
                    "router_quality.html",
                    "router-metrics.json",
                    "router-detail-metrics.json",
                )
            }
        self.assertEqual(json.loads(files["router_quality.json"])["verdict"], report["verdict"])
        return report, files

    def baseline(self, cells=None):
        report, _ = self.render(measurements(cells))
        return report | {"commit": "b" * 40}

    def assertVerdict(self, files, line):
        self.assertIn(f"\n**{line}**\n", files["router_quality.md"])

    def test_one_table_in_row_order_with_footnote_and_link(self):
        report, files = self.render(measurements())
        text = files["router_quality.md"]
        self.assertTrue(text.startswith("\n#### Router quality · Cohere 1m\n\n**✅ "))
        self.assertVerdict(files, "✅ Router matches flat ordering (worst 1.00x at r90) · no main baseline yet")
        self.assertIn(f"**\n\n{HEADER}| Unfiltered r90 |", text)
        rows = table(text)
        self.assertEqual(list(rows), ROW_ORDER)
        self.assertEqual(
            list(rows["Unfiltered r90"].values()),
            ["100", "100", "1.00x", "40.0k", "40.0k", "1.00x", "1.00x", "0.8 (10%)"],
        )
        # Filtered rows count filter-valid vectors.
        for level in ("Filter 10%", "Filter 1%"):
            self.assertEqual(
                list(rows[f"{level} r95"].values()),
                ["100", "100", "1.00x", "30.0k", "30.0k", "1.00x", "1.00x", "0.8 (10%)"],
            )
        self.assertNotIn("(main", text)
        self.assertNotIn("×", text)
        self.assertTrue(text.endswith(f" |\n\n{rq.FOOTNOTE}\n[Full report]({self.URL})\n"))
        for phrase in (
            "Each Ratio is Router divided by Flat for the two columns before it.",
            "Adaptive on vs off is the router's vectors probed with adaptive probing on, divided by off.",
            "Vectors in filtered rows count only vectors that pass the filter.",
            "with its share of query time in brackets.",
            "❌ marks an unfiltered ratio above 1.05x; filtered rows are informational.",
        ):
            self.assertIn(phrase, rq.FOOTNOTE)
        self.assertEqual(report["flags"], [])

    def test_routing_cell_and_adaptive_column_and_what_stays_out(self):
        cells = {
            (U, 0.9, "stored"): arm_cell(routing_ms=12.4, routing_share=0.25),
            (U, 0.9, "flat"): arm_cell(routing_ms=7.7),
            (U, 0.95, "stored-adaptive"): arm_cell(vectors_probed=36_000.0, clusters=77.0),
            ("knn_top10_1pct", 0.99, "stored-adaptive"): arm_cell(valid_vectors=33_000.0),
        }
        _, files = self.render(measurements(cells))
        rows = table(files["router_quality.md"])
        self.assertEqual(rows["Unfiltered r90"]["router:routing"], "12 (25%)")
        self.assertEqual(rows["Unfiltered r95"]["router:routing"], "0.8 (10%)")
        self.assertEqual(rows["Unfiltered r95"]["ratio:adaptive"], "0.90x")
        self.assertEqual(rows["Unfiltered r95"]["router:clusters"], "100")
        self.assertEqual(rows["Filter 1% r99"]["ratio:adaptive"], "1.10x")
        # Flat routing time and adaptive-on absolute numbers are in the full report only.
        self.assertNotIn("7.7", files["router_quality.md"])
        self.assertNotIn("| 77 |", files["router_quality.md"])
        self.assertNotIn("36.0k", files["router_quality.md"])
        self.assertIn("<td>7.7</td>", files["router_quality.html"])

    def test_either_ratio_flags_and_the_verdict_names_clusters_when_it_flags(self):
        cells = {(U, 0.95, "stored"): arm_cell(clusters=108.0, vectors_probed=43_200.0)}
        report, files = self.render(measurements(cells))
        row = table(files["router_quality.md"])["Unfiltered r95"]
        self.assertEqual(row["ratio:clusters"], "**1.08x** ❌")
        self.assertEqual(row["ratio:vectors"], "**1.08x** ❌")
        self.assertEqual(row["router:clusters"], "108")
        self.assertEqual(report["verdict"], "❌")
        self.assertVerdict(files, "❌ Router needs 1.08x flat's clusters at r95 · no main baseline yet.")
        # A worse vectors ratio elsewhere still names the flagged clusters ratio.
        cells = {
            (U, 0.9, "stored"): arm_cell(vectors_probed=44_000.0),
            (U, 0.95, "stored"): arm_cell(clusters=106.0),
        }
        _, files = self.render(measurements(cells))
        rows = table(files["router_quality.md"])
        self.assertEqual(rows["Unfiltered r90"]["ratio:vectors"], "**1.10x** ❌")
        self.assertEqual(rows["Unfiltered r90"]["ratio:clusters"], "1.00x")
        self.assertVerdict(files, "❌ Router needs 1.06x flat's clusters at r95 · no main baseline yet.")
        cells = {(U, 0.9, "stored"): arm_cell(vectors_probed=44_000.0)}
        _, files = self.render(measurements(cells))
        self.assertVerdict(files, "❌ Router needs 1.10x flat's vectors at r90 · no main baseline yet.")
        cells = {(U, 0.99, "stored"): arm_cell(clusters=110.0)}
        report, files = self.render(measurements(cells))
        self.assertEqual(table(files["router_quality.md"])["Unfiltered r99"]["ratio:clusters"], "**1.10x** ⚠️")
        self.assertEqual(report["verdict"], "⚠️")
        self.assertVerdict(files, "⚠️ Router needs 1.10x flat's clusters at r99 · no main baseline yet.")

    def test_with_a_baseline_a_ratio_flags_only_past_main_by_more_than_005(self):
        high = {(U, 0.95, "stored"): arm_cell(clusters=113.0)}
        base = self.baseline(high)
        report, files = self.render(measurements(high), base)
        self.assertEqual(report["flags"], [])
        self.assertEqual(table(files["router_quality.md"])["Unfiltered r95"]["ratio:clusters"], "1.13x")
        self.assertVerdict(
            files, f"✅ No regression against main {'b' * 7}. Router is at 1.13x flat at r95 (main 1.13x)."
        )
        worse = {(U, 0.95, "stored"): arm_cell(clusters=120.0)}
        report, files = self.render(measurements(worse), base)
        self.assertEqual(
            table(files["router_quality.md"])["Unfiltered r95"]["ratio:clusters"], "**1.20x** ❌ (main 1.13x)"
        )
        self.assertVerdict(files, "❌ Router needs 1.20x flat's clusters at r95.")
        # A cell main did not reach keeps the absolute rule.
        missed = self.baseline({(U, 0.9, "stored"): arm_cell(reached=False)})
        cells = {(U, 0.9, "stored"): arm_cell(vectors_probed=44_000.0)}
        _, files = self.render(measurements(cells), missed)
        self.assertEqual(table(files["router_quality.md"])["Unfiltered r90"]["ratio:vectors"], "**1.10x** ❌")
        # At r99 too, a ratio within 0.05 of main's stays unflagged.
        base = self.baseline({(U, 0.99, "stored"): arm_cell(vectors_probed=44_800.0)})
        cells = {(U, 0.99, "stored"): arm_cell(vectors_probed=46_000.0)}
        report, files = self.render(measurements(cells), base)
        self.assertEqual(report["flags"], [])
        self.assertVerdict(
            files, f"✅ No regression against main {'b' * 7}. Router is at 1.15x flat at r99 (main 1.12x)."
        )

    def test_a_clean_baseline_adds_no_brackets_and_names_main(self):
        _, files = self.render(measurements(), self.baseline())
        self.assertVerdict(
            files, f"✅ Router matches flat ordering (worst 1.00x at r90) · no regression against main {'b' * 7}"
        )
        self.assertNotIn("(main", files["router_quality.md"])

    def test_flat_columns_against_main_flag_inline(self):
        base = self.baseline()
        cells = {(U, 0.9, "flat"): arm_cell(clusters=115.0, vectors_probed=44_400.0)}
        report, files = self.render(measurements(cells), base)
        row = table(files["router_quality.md"])["Unfiltered r90"]
        self.assertEqual(row["flat:clusters"], "**115** ⚠️ (main 100)")
        self.assertEqual(row["flat:vectors"], "**44.4k** ⚠️ (main 40.0k)")
        self.assertEqual(row["ratio:clusters"], "0.87x")
        self.assertVerdict(
            files, "⚠️ Flat clusters at r90 up 15% against main (100 → 115). Look at: clustering."
        )
        cells = {(U, 0.95, "flat"): arm_cell(vectors_probed=43_900.0)}
        report, _ = self.render(measurements(cells), base)
        self.assertEqual(report["flags"], [])

    def test_adaptive_column_flags_a_rise_past_005(self):
        base = self.baseline()
        cells = {(U, 0.95, "stored-adaptive"): arm_cell(vectors_probed=42_400.0, clusters=200.0)}
        report, files = self.render(measurements(cells), base)
        self.assertEqual(
            table(files["router_quality.md"])["Unfiltered r95"]["ratio:adaptive"], "**1.06x** ⚠️ (main 1.00x)"
        )
        self.assertEqual([f["column"] for f in report["flags"]], ["ratio:adaptive"])
        self.assertVerdict(
            files,
            "⚠️ Adaptive on vs off at r95 up 6% against main (1.00x → 1.06x). Look at: adaptive stopping.",
        )
        cells = {(U, 0.9, "stored-adaptive"): arm_cell(vectors_probed=41_600.0)}
        report, _ = self.render(measurements(cells), base)
        self.assertEqual(report["flags"], [])

    def test_routing_ms_flags_only_past_both_thresholds(self):
        base = self.baseline()
        cells = {(U, 0.9, "stored"): arm_cell(routing_ms=1.2)}
        _, files = self.render(measurements(cells), base)
        self.assertEqual(
            table(files["router_quality.md"])["Unfiltered r90"]["router:routing"], "**1.2** (10%) ⚠️ (main 0.8)"
        )
        self.assertVerdict(
            files, "⚠️ Routing ms at r90 up 50% against main (0.8 → 1.2). Look at: router speed."
        )
        # Up 19% (below 25%); up 80% but only 0.08 ms (below 0.1 ms).
        for before, after in ((0.8, 0.95), (0.1, 0.18)):
            base = self.baseline({(U, 0.9, "stored"): arm_cell(routing_ms=before)})
            cells = {(U, 0.9, "stored"): arm_cell(routing_ms=after)}
            report, _ = self.render(measurements(cells), base)
            self.assertEqual(report["flags"], [], (before, after))

    def test_where_to_look_lists_every_flagged_area(self):
        base = self.baseline()
        cells = {
            (U, 0.9, "flat"): arm_cell(clusters=120.0),
            (U, 0.95, "stored"): arm_cell(routing_ms=1.2),
            (U, 0.99, "stored"): arm_cell(clusters=115.0),
        }
        report, files = self.render(measurements(cells), base)
        self.assertEqual(
            sorted((f["column"], f["target"]) for f in report["flags"]),
            [("flat:clusters", 0.9), ("ratio:clusters", 0.99), ("router:routing", 0.95)],
        )
        self.assertVerdict(
            files,
            "⚠️ Routing ms at r95 up 50% against main (0.8 → 1.2). "
            "Look at: router, clustering, router speed.",
        )

    def test_r99_changes_against_main_do_not_flag(self):
        base = self.baseline()
        cells = {
            (U, 0.99, "flat"): arm_cell(clusters=150.0, vectors_probed=60_000.0),
            (U, 0.99, "stored"): arm_cell(clusters=150.0, vectors_probed=60_000.0, routing_ms=5.0),
            (U, 0.99, "stored-adaptive"): arm_cell(vectors_probed=80_000.0),
        }
        report, _ = self.render(measurements(cells), base)
        self.assertEqual(report["flags"], [])
        self.assertEqual(report["verdict"], "✅")

    def test_filtered_rows_never_flag(self):
        base = self.baseline()
        cells = {
            ("knn_top10_10pct", 0.9, "stored"): arm_cell(clusters=300.0, valid_vectors=90_000.0),
            ("knn_top10_10pct", 0.9, "flat"): arm_cell(routing_ms=9.0, clusters=200.0),
            ("knn_top10_1pct", 0.95, "stored"): arm_cell(routing_ms=4.0),
            ("knn_top10_1pct", 0.95, "stored-adaptive"): arm_cell(valid_vectors=60_000.0),
        }
        report, files = self.render(measurements(cells), base)
        self.assertEqual(report["flags"], [])
        self.assertEqual(report["verdict"], "✅")
        rows = table(files["router_quality.md"])
        self.assertEqual(
            [rows["Filter 10% r90"][c] for c in ("ratio:clusters", "ratio:vectors", "flat:clusters")],
            ["1.50x", "3.00x", "200"],
        )
        self.assertEqual(rows["Filter 1% r95"]["ratio:adaptive"], "2.00x")
        self.assertEqual(rows["Filter 1% r95"]["router:routing"], "4.0 (10%)")
        for row in rows.values():
            self.assertFalse(any("❌" in c or "⚠️" in c or "**" in c for c in row.values()), row)

    def test_not_reached_prints_in_that_arms_cells_and_leaves_ratios_empty(self):
        cells = {
            (U, 0.9, "stored-adaptive"): arm_cell(reached=False),
            (U, 0.95, "flat"): arm_cell(reached=False),
            (U, 0.99, "stored"): arm_cell(reached=False),
        }
        _, files = self.render(measurements(cells))
        rows = table(files["router_quality.md"])
        self.assertEqual(
            list(rows["Unfiltered r90"].values()),
            ["100", "100", "1.00x", "40.0k", "40.0k", "1.00x", "", "0.8 (10%)"],
        )
        self.assertEqual(
            list(rows["Unfiltered r95"].values()),
            ["not reached", "100", "", "not reached", "40.0k", "", "1.00x", "0.8 (10%)"],
        )
        self.assertEqual(
            list(rows["Unfiltered r99"].values()),
            ["100", "not reached", "", "40.0k", "not reached", "", "", "not reached"],
        )

    def test_a_baseline_with_another_configuration_is_not_compared(self):
        base = self.baseline() | {"fingerprint": "other"}
        report, files = self.render(measurements(), base)
        self.assertVerdict(
            files, "✅ Router matches flat ordering (worst 1.00x at r90) · main baseline configuration differs"
        )
        self.assertIsNone(report["baseline_commit"])
        self.assertIn("main baseline configuration differs", files["router_quality.html"])

    def test_history_metrics_are_fifteen_unfiltered_values_in_two_suites(self):
        cells = {
            (U, 0.9, "stored"): arm_cell(clusters=120.0, vectors_probed=48_000.0, routing_ms=0.5),
            (U, 0.9, "stored-adaptive"): arm_cell(vectors_probed=36_000.0),
        }
        _, files = self.render(measurements(cells))
        ratios = json.loads(files["router-metrics.json"])
        details = json.loads(files["router-detail-metrics.json"])
        self.assertEqual((len(ratios), len(details)), (6, 9))
        self.assertEqual({m["unit"] for m in ratios}, {"ratio"})
        self.assertEqual({m["unit"] for m in details}, {"vectors/query", "ms"})
        values = {m["name"]: m["value"] for m in ratios + details}
        tag = f"{U}@r90"
        self.assertEqual(values[f"{tag} router vs flat, clusters"], 1.2)
        self.assertEqual(values[f"{tag} adaptive on vs off, vectors probed"], 0.75)
        self.assertEqual(values[f"{tag} flat vectors probed"], 40_000.0)
        self.assertEqual(values[f"{tag} router vectors probed, adaptive on"], 36_000.0)
        self.assertEqual(values[f"{tag} router routing time"], 0.5)
        self.assertNotIn(f"{tag} router vs flat, vectors probed", values)
        cells = {(U, 0.99, "flat"): arm_cell(reached=False)}
        _, files = self.render(measurements(cells))
        self.assertEqual(len(json.loads(files["router-metrics.json"])), 2 + 2 + 1)
        self.assertEqual(len(json.loads(files["router-detail-metrics.json"])), 3 + 3 + 2)

    def test_full_report_holds_main_side_by_side_and_what_left_the_comment(self):
        measured = measurements({(U, 0.9, "flat"): arm_cell(clusters=90.0)})
        measured["tolerated"] = [
            {"arm": "flat", "query": "knn_top10_10pct", "report": "router/flat/r_search.json"}
        ]
        measured["plan_shapes"] = {q: ["parallel"] for q in QUERIES}
        _, files = self.render(measured, self.baseline())
        page = files["router_quality.html"]
        self.assertTrue(page.startswith("<!doctype html>"))
        for text in (
            "Per query, target and arm",
            "Layer-0 scored",
            "Routing ms p95",
            "Centroids scored, 1-bit",
            "Adaptive stops",
            "Against main",
            f"Main <code>{'b' * 40}</code>",
            "<td>Flat clusters</td><td>90</td><td>100</td><td>-10.0%</td>",
            "<td>Clusters ratio</td><td>1.11x</td><td>1.00x</td><td>+11.1%</td>",
            "<td>Vectors ratio</td>",
            "<td>Router, adaptive on vectors</td>",
            "<td>Flat routing ms</td>",
            "Checks",
            "Plan shapes",
            "router/flat/r_search.json",
            "paradedb.vector_router_audit = stored",
            "cohere-1m-router-quality",
        ):
            self.assertIn(text, page)
        # Nine data rows and the config row.
        self.assertEqual(page.count("<td>stored-adaptive</td>"), 10)
        self.assertNotIn("router/flat/r_search.json", files["router_quality.md"])
        _, files = self.render(measurements())
        self.assertIn("no main baseline yet", files["router_quality.html"])


if __name__ == "__main__":
    unittest.main()

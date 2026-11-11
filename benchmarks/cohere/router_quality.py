"""Compare the index's stored router with a flat ranking of every centroid.

Every arm runs on the same index with every segment routed. The flat arm scores every centroid of
every segment exactly (`paradedb.vector_router_audit = flat`), so the clusters and vectors it needs
at a recall target are the reference the stored router (Router) is measured against; Router vs
flat is Router / Flat. Both run with adaptive probing off; a third arm repeats the stored router
with adaptive probing on.

    configs  write router-<arm>.toml for every arm from a Cohere profile
    measure  rerun pdbench's selected budgets under EXPLAIN and record per-segment work
    render   write router_quality.{json,md,html} and the router history metrics
"""

import argparse
import collections
import hashlib
import html
import json
import math
import os
import subprocess
import tomllib
from pathlib import Path

from compare_search import search_reports
from prepare_k6 import PROBE, QUERIES, TARGETS

AUDIT = "paradedb.vector_router_audit"
ADAPTIVE = "paradedb.vector_adaptive_probing"
# Arm name -> (router audit, adaptive probing).
ARMS = {
    "stored": ("stored", "off"),
    "flat": ("flat", "off"),
    "stored-adaptive": ("stored", "on"),
}
FIXED = ("stored", "flat")
# The `termination` value of a segment the adaptive stop ended; `Ceiling` and `Exhausted` are not.
ADAPTIVE_STOP = "QuantizedRecall"
MAX_RATIO = 1.05
LABELS = {
    "knn_top10_unfiltered": "Unfiltered",
    "knn_top10_10pct": "Filter 10%",
    "knn_top10_1pct": "Filter 1%",
}
QVEC = "current_setting('cohere.qvec')"
TOLERATED = "tolerated.txt"

# Per-query sums over segments and workers, from each segment's vector statistics.
SUMS = {
    "clusters": ("postings_row", "postings_skipped"),
    "vectors_probed": ("vectors_visited",),
    "valid_vectors": ("layer0_eligible",),
    "empty_clusters": ("clusters_skipped_empty",),
    "layer0_scored": ("candidates_scored",),
    "exact_rows": ("exact_rows",),
    "routing_ns": ("routing_ns",),
    "centroids_1bit": ("routing_approximate_scored",),
    "centroids_exact": ("routing_exact_scored",),
}
# Router counters a segment omits when its router does not produce them.
OPTIONAL = {"routing_approximate_scored", "routing_exact_scored"}
MEANS = (
    "clusters",
    "vectors_probed",
    "valid_vectors",
    "empty_clusters",
    "layer0_scored",
    "exact_rows",
    "routing_ms",
    "centroids_1bit",
    "centroids_exact",
    "query_ms",
)
CHECKS = {
    "routed": "every segment reports access_path Routed in both arms",
    "flat_mode": "routing_mode is flat on every flat-arm segment and never in the stored arm",
    "filtered_rows": (
        "filtered queries: vectors_visited = layer0_eligible + pruned_filter + pruned_dead"
    ),
    "unfiltered_rows": "unfiltered query: vectors_visited > 0 and pruned_filter = 0",
    "fixed_no_adaptive_stop": f"fixed-probe arms: no segment reports termination {ADAPTIVE_STOP}",
    "plan_length_rule": (
        "per query, every arm and target runs all-parallel or all-serial plans, so every arm "
        "gets the same candidate-list bound"
    ),
}


def label(target):
    return f"r{round(target * 100)}"


# --- configs ---


def arm_profile(text, arm):
    """The profile with the arm's router-audit and adaptive-probing settings added to
    [search.session], otherwise unchanged."""
    lines = text.splitlines(keepends=True)
    header = next(
        (i for i, line in enumerate(lines) if line.strip() == "[search.session]"), None
    )
    if header is None:
        raise ValueError("profile has no [search.session] table")
    expected = tomllib.loads(text)
    session = expected["search"]["session"]
    if AUDIT in session or ADAPTIVE in session:
        raise ValueError("profile already sets the router-audit session settings")
    audit, adaptive = ARMS[arm]
    session.update({AUDIT: audit, ADAPTIVE: adaptive})
    added = [f'"{AUDIT}" = "{audit}"\n', f'"{ADAPTIVE}" = "{adaptive}"\n']
    result = "".join(lines[: header + 1] + added + lines[header + 1 :])
    if tomllib.loads(result) != expected:
        raise ValueError("arm profile differs from the source beyond its session settings")
    return result


def configs(profile, output):
    output.mkdir(parents=True, exist_ok=True)
    text = profile.read_text()
    for arm in ARMS:
        (output / f"router-{arm}.toml").write_text(arm_profile(text, arm))


# --- measure ---


def plan_nodes(plan):
    nodes = [plan[0]["Plan"]]
    while nodes:
        node = nodes.pop()
        yield node
        nodes.extend(node.get("Plans", []))


def segment_entries(plan):
    """(segment id, statistics) for every segment of every ParadeDB scan in one EXPLAIN
    (FORMAT JSON) plan. The leader's Segment Info already holds the segments its parallel workers
    scanned."""
    entries = []
    for node in plan_nodes(plan):
        info = node.get("Segment Info")
        if info is not None:
            entries.extend((json.loads(info) if isinstance(info, str) else info).items())
    return entries


def segment_stats(plan):
    return [stats for _, stats in segment_entries(plan)]


def workers(plan):
    """(planned, launched) parallel workers. A planned Gather makes the scan parallel, which
    caps the routed candidate list at the router's k; a serial scan takes the whole ranking."""
    planned = launched = 0
    for node in plan_nodes(plan):
        planned += node.get("Workers Planned", 0)
        launched += node.get("Workers Launched", 0)
    return planned, launched


def query_totals(plan):
    """One query's work summed over its segments, and those segments."""
    segments = segment_stats(plan)
    totals = {
        name: sum(
            segment.get(key, 0) if key in OPTIONAL else segment[key]
            for segment in segments
            for key in keys
        )
        for name, keys in SUMS.items()
    }
    totals["routing_ms"] = totals.pop("routing_ns") / 1e6
    totals["query_ms"] = plan[0]["Execution Time"]
    totals["adaptive_stop"] = any(s.get("termination") == ADAPTIVE_STOP for s in segments)
    planned, launched = workers(plan)
    totals["parallel"] = planned > 0
    totals["workers_launched"] = launched
    return totals, segments


def summarize(per_query):
    """Means over the query vectors, the routing-time p95, and routing's share of query time."""
    count = len(per_query)
    summary = {key: sum(query[key] for query in per_query) / count for key in MEANS}
    routing = sorted(query["routing_ms"] for query in per_query)
    summary["routing_ms_p95"] = routing[math.ceil(0.95 * count) - 1]
    summary["routing_share"] = (
        summary["routing_ms"] / summary["query_ms"] if summary["query_ms"] else None
    )
    summary["adaptive_stops"] = sum(query["adaptive_stop"] for query in per_query)
    summary["parallel_plans"] = sum(query["parallel"] for query in per_query)
    summary["workers_launched"] = sum(query["workers_launched"] for query in per_query) / count
    return summary


class Checks:
    """Confirms each statistic means what the comment's tables say it means."""

    def __init__(self):
        self.results = {
            name: {"description": text, "checked": 0, "failed": 0, "example": None}
            for name, text in CHECKS.items()
        }

    def record(self, name, ok, example):
        result = self.results[name]
        result["checked"] += 1
        if not ok:
            result["failed"] += 1
            result["example"] = result["example"] or example

    def observe(self, arm, query, segments):
        filtered = not query.endswith("_unfiltered")
        where = {"arm": arm, "query": query}
        for segment in segments:
            example = where | {"segment": segment}
            self.record("routed", segment.get("access_path") == "Routed", example)
            self.record(
                "flat_mode",
                (segment.get("routing_mode") == "flat") == (ARMS[arm][0] == "flat"),
                example,
            )
            if arm in FIXED:
                self.record(
                    "fixed_no_adaptive_stop", segment.get("termination") != ADAPTIVE_STOP, example
                )
            if filtered:
                self.record(
                    "filtered_rows",
                    segment["vectors_visited"]
                    == segment["layer0_eligible"]
                    + segment["pruned_filter"]
                    + segment["pruned_dead"],
                    example,
                )
            else:
                self.record("unfiltered_rows", segment["pruned_filter"] == 0, example)
        if not filtered:
            visited = sum(segment["vectors_visited"] for segment in segments)
            self.record("unfiltered_rows", visited > 0, where | {"vectors_visited": visited})

    def passed(self):
        return all(result["failed"] == 0 for result in self.results.values())


def explain_script(session, sql, vectors, dims):
    """psql input: the arm's settings, one discarded warm pass, then the measured pass."""
    settings = "".join(f"SET {key} = '{value}';\n" for key, value in session.items())
    explains = "".join(
        f"EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON) EXECUTE router_quality('{vector}');\n"
        for vector in vectors
    )
    return (
        f"{settings}PREPARE router_quality(vector({dims})) AS {sql.rstrip().rstrip(';')};\n"
        f"\\o /dev/null\n{explains}\\o\n{explains}"
    )


def parse_plans(text):
    """Consecutive EXPLAIN (FORMAT JSON) documents from psql's unaligned output."""
    decoder = json.JSONDecoder()
    plans = []
    index = 0
    while True:
        while index < len(text) and text[index].isspace():
            index += 1
        if index == len(text):
            return plans
        plan, index = decoder.raw_decode(text, index)
        plans.append(plan)


def psql(script):
    return subprocess.run(
        ["psql", os.environ["PARADEDB_URL"], "-XqAt", "-v", "ON_ERROR_STOP=1"],
        input=script,
        capture_output=True,
        text=True,
        check=True,
    ).stdout


def arm_reports(runs, arm):
    """The arm's pdbench search report per query."""
    directories = [path.parent for path in (runs / arm).rglob("*_search.json")]
    if len(set(directories)) != 1:
        raise ValueError(f"expected one pdbench profile directory under {runs / arm}")
    reports = search_reports(directories[0])
    if set(reports) != set(QUERIES):
        raise ValueError(f"{arm}: missing sweeps for {set(QUERIES) - set(reports)}")
    audit, adaptive = ARMS[arm]
    for query, report in reports.items():
        if report["data"]["sweep_param"] != PROBE:
            raise ValueError(f"{arm}/{query}: expected a {PROBE} sweep")
        session = report["config"]["search"]["session"]
        if session.get(AUDIT) != audit or session.get(ADAPTIVE) != adaptive:
            raise ValueError(f"{arm}/{query}: report was not run with the arm's settings")
    return reports


def comparable_config(report):
    config = json.loads(json.dumps(report["config"]))
    config["search"].pop("query", None)
    config["search"].pop("k", None)
    return config


def fingerprint(configs_by_arm):
    canonical = json.dumps(configs_by_arm, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(canonical.encode()).hexdigest()


def measure(runs, vectors, run=psql):
    """Every arm x query x target at pdbench's selected budget, plus index facts and checks."""
    reports = {arm: arm_reports(runs, arm) for arm in ARMS}
    configs_by_arm = {}
    for arm in ARMS:
        found = {json.dumps(comparable_config(r), sort_keys=True) for r in reports[arm].values()}
        if len(found) != 1:
            raise ValueError(f"{arm}: sweeps used different configurations")
        configs_by_arm[arm] = json.loads(found.pop())
    shared = []
    for config in configs_by_arm.values():
        config = json.loads(json.dumps(config))
        config["search"]["session"].pop(AUDIT)
        config["search"]["session"].pop(ADAPTIVE)
        shared.append(config)
    if any(config != shared[0] for config in shared):
        raise ValueError(f"the arms differ beyond {AUDIT} and {ADAPTIVE}")

    checks = Checks()
    terminations = {arm: collections.Counter() for arm in ARMS}
    segments_seen = {}
    rerank_factor = None
    shapes = collections.defaultdict(set)
    results = {}
    for arm in ARMS:
        for query in QUERIES:
            report = reports[arm][query]
            variant = report["data"]["variants"][query]
            if variant["query"].count(QVEC) != 1:
                raise ValueError(f"{arm}/{query}: expected exactly one query-vector operand")
            sql = variant["query"].replace(QVEC, "$1")
            dims = report["config"]["dataset"].get("dims", 1024)
            for target in TARGETS:
                selected = variant["targets"].get(str(target))
                cell = {"reached": selected is not None}
                if selected:
                    session = report["config"]["search"]["session"] | {
                        PROBE: selected["value"],
                        "paradedb.vector_stats": "on",
                    }
                    plans = parse_plans(run(explain_script(session, sql, vectors, dims)))
                    if len(plans) != len(vectors):
                        raise ValueError(f"{arm}/{query}: expected {len(vectors)} plans")
                    per_query = []
                    for plan in plans:
                        totals, segments = query_totals(plan)
                        checks.observe(arm, query, segments)
                        terminations[arm].update(s.get("termination") for s in segments)
                        per_query.append(totals)
                        if arm == "stored":
                            segments_seen.update(segment_entries(plan))
                    if rerank_factor is None and arm == "stored":
                        rerank_factor = run(
                            "".join(f"SET {k} = '{v}';\n" for k, v in session.items())
                            + "SHOW paradedb.vector_router_rerank_factor;\n"
                        ).strip()
                    cell |= {
                        "budget": selected["value"],
                        "recall": selected["recall"],
                    } | summarize(per_query)
                    shapes[query].add(plan_shape(cell["parallel_plans"], len(vectors)))
                results.setdefault(query, {}).setdefault(str(target), {})[arm] = cell
    for query in QUERIES:
        checks.record(
            "plan_length_rule",
            len(shapes[query]) == 1 and "mixed" not in shapes[query],
            {"query": query, "shapes": sorted(shapes[query])},
        )
    tolerated = []
    if (runs / TOLERATED).exists():
        for line in (runs / TOLERATED).read_text().splitlines():
            if line.strip():
                arm, query, report_path = line.split(maxsplit=2)
                tolerated.append({"arm": arm, "query": query, "report": report_path})
    index = index_facts(segments_seen.values()) if segments_seen else None
    if index is not None:
        index["rerank_factor"] = rerank_factor
    return {
        "dataset": configs_by_arm["stored"]["dataset"],
        "fingerprint": fingerprint(configs_by_arm),
        "query_vectors": len(vectors),
        "arms": {arm: {"session": configs_by_arm[arm]["search"]["session"]} for arm in ARMS},
        "index": index,
        "checks": checks.results,
        "checks_passed": checks.passed(),
        # Segment `termination` values seen per arm; only ADAPTIVE_STOP counts as the adaptive stop.
        "adaptive_stop": ADAPTIVE_STOP,
        "terminations": {arm: dict(counter) for arm, counter in terminations.items()},
        "plan_shapes": {query: sorted(shapes[query]) for query in QUERIES},
        # pdbench runs whose console projection failed after their report was saved.
        "tolerated": tolerated,
        "results": results,
    }


def plan_shape(parallel_plans, count):
    if parallel_plans == count:
        return "parallel"
    return "serial" if parallel_plans == 0 else "mixed"


def index_facts(segments):
    """Segment count, K and rows per cluster over every segment id the stored arm reported."""
    segments = list(segments)
    clusters = sum(segment["segment_clusters"] for segment in segments)
    return {
        "segments": len(segments),
        "clusters": clusters,
        "rows_per_cluster": sum(segment["segment_rows"] for segment in segments) / clusters,
        "routing_kind": "/".join(
            sorted({segment.get("routing_kind", "none") for segment in segments})
        ),
    }


# --- render ---

FLAT, ROUTER, ROUTER_ADAPTIVE = "flat", "stored", "stored-adaptive"
UNFILTERED = "knn_top10_unfiltered"
# Policy thresholds for flagging the unfiltered table, set for review rather than derived.
MAIN_TARGETS = (0.9, 0.95)  # Against-main flags apply at these targets only.
RATIO_RISE = 0.05  # With a baseline, Router vs flat above MAX_RATIO flags only past main's by this.
VECTORS_WORSE = 0.10  # Flat or adaptive-on vectors probed, relative increase against main.
ADAPTIVE_RATIO_RISE = 0.05  # Adaptive on vs off, absolute increase against main.
ROUTING_WORSE = 0.25  # Router routing time, relative increase against main ...
ROUTING_WORSE_MS = 0.1  # ... and absolute increase in ms.
FIXED_NOTE = (
    "Flat and Router rows use fixed probes (adaptive off). Routing time shows the router's share "
    "of query time in brackets."
)
FILTERED_NOTE = (
    "Filtered tables are informational: every segment is forced to route, plans are parallel, "
    "and routing time includes row gating."
)
AREAS = {"ratio": "router", "flat": "clustering", "routing": "router speed"}


def vectors_key(query):
    """Unfiltered tables count vectors probed; filtered tables count filter-valid vectors."""
    return "vectors_probed" if query == UNFILTERED else "valid_vectors"


def vectors_label(query):
    return "Vectors probed" if query == UNFILTERED else "Valid vectors"


def value(report, query, target, arm, key):
    """An arm's mean at a target, or None when the report lacks it or the arm missed the target."""
    if report is None:
        return None
    cell = report["results"].get(query, {}).get(str(target), {}).get(arm)
    return cell.get(key) if cell and cell.get("reached") else None


def ratio_of(report, query, target, key, numerator=ROUTER, denominator=FLAT):
    top = value(report, query, target, numerator, key)
    bottom = value(report, query, target, denominator, key)
    if top is None or bottom is None:
        return None
    if bottom == 0:
        return 1.0 if top == 0 else math.inf
    return top / bottom


def router_vs_flat(report, query, target):
    """Router / Flat on the level's vectors."""
    return ratio_of(report, query, target, vectors_key(query))


def adaptive_vs_fixed(report, query, target):
    """Router with adaptive probing on / off, on the level's vectors."""
    return ratio_of(report, query, target, vectors_key(query), ROUTER_ADAPTIVE, ROUTER)


def routing_share(report, query, target):
    share = value(report, query, target, ROUTER, "routing_share")
    return "" if share is None else f" ({share:.0%})"


def table_rows(query):
    """(row id, label, getter, formatter, bold, suffix) per row of one filter level; a None getter
    is a heading."""
    vk = vectors_label(query)
    key = vectors_key(query)

    def arm(name, column):
        return lambda r, t: value(r, query, t, name, column)

    return [
        (None, f"Ratios, {vk.lower()}", None, None, False, None),
        ("ratio:router", "Router vs flat", lambda r, t: router_vs_flat(r, query, t), ratio_text, True, None),
        (
            "ratio:adaptive",
            "Adaptive on vs off",
            lambda r, t: adaptive_vs_fixed(r, query, t),
            ratio_text,
            True,
            None,
        ),
        (None, "Clusters probed", None, None, False, None),
        ("flat:clusters", "Flat", arm(FLAT, "clusters"), count, False, None),
        ("router:clusters", "Router", arm(ROUTER, "clusters"), count, False, None),
        ("adaptive:clusters", "Router, adaptive on", arm(ROUTER_ADAPTIVE, "clusters"), count, False, None),
        (None, vk, None, None, False, None),
        ("flat:vectors", "Flat", arm(FLAT, key), count, False, None),
        ("router:vectors", "Router", arm(ROUTER, key), count, False, None),
        ("adaptive:vectors", "Router, adaptive on", arm(ROUTER_ADAPTIVE, key), count, False, None),
        (None, "Routing time, ms", None, None, False, None),
        ("flat:routing", "Flat", arm(FLAT, "routing_ms"), ms, False, None),
        (
            "router:routing",
            "Router",
            arm(ROUTER, "routing_ms"),
            ms,
            False,
            lambda r, t: routing_share(r, query, t),
        ),
    ]


def flags(current, baseline):
    """Flags on the unfiltered table, keyed by (row id, target). Filtered tables never flag.

    Router vs flat above MAX_RATIO is ❌ at r90 and r95 and ⚠️ at r99; with a main baseline it must
    also exceed main's ratio by more than RATIO_RISE, and a cell main did not reach keeps the
    absolute rule. Against main, at r90 and r95, a cell is ⚠️ when Flat or adaptive-on vectors
    probed rose by more than VECTORS_WORSE, when adaptive on vs off rose by more than
    ADAPTIVE_RATIO_RISE, or when Router routing time rose by more than ROUTING_WORSE and
    ROUTING_WORSE_MS.
    """
    query, found = UNFILTERED, {}
    for target in TARGETS:
        current_ratio = router_vs_flat(current, query, target)
        main_ratio = router_vs_flat(baseline, query, target)
        if (
            current_ratio is not None
            and current_ratio > MAX_RATIO
            and (main_ratio is None or current_ratio - main_ratio > RATIO_RISE)
        ):
            found[("ratio:router", target)] = {
                "row": "ratio:router",
                "target": target,
                "mark": "⚠️" if target >= 0.99 else "❌",
                "kind": "ratio",
                "value": current_ratio,
                "before": main_ratio,
            }
    if baseline is None:
        return found

    def main_flag(row, target, kind, name, before, after):
        found[(row, target)] = {
            "row": row,
            "target": target,
            "mark": "⚠️",
            "kind": kind,
            "name": name,
            "before": before,
            "after": after,
            "change": (after - before) / before if before else math.inf,
        }

    for target in MAIN_TARGETS:
        before = adaptive_vs_fixed(baseline, query, target)
        after = adaptive_vs_fixed(current, query, target)
        if before is not None and after is not None and after - before > ADAPTIVE_RATIO_RISE:
            main_flag("ratio:adaptive", target, "adaptive", "Adaptive on vs off", before, after)
        for row, arm, kind, name in (
            ("flat:vectors", FLAT, "flat", "Flat vectors probed"),
            ("adaptive:vectors", ROUTER_ADAPTIVE, "adaptive", "Router, adaptive on vectors probed"),
        ):
            before = value(baseline, query, target, arm, "vectors_probed")
            after = value(current, query, target, arm, "vectors_probed")
            if before is not None and after is not None and after > before * (1 + VECTORS_WORSE):
                main_flag(row, target, kind, name, before, after)
        before = value(baseline, query, target, ROUTER, "routing_ms")
        after = value(current, query, target, ROUTER, "routing_ms")
        if (
            before is not None
            and after is not None
            and after > before * (1 + ROUTING_WORSE)
            and after - before > ROUTING_WORSE_MS
        ):
            main_flag("router:routing", target, "routing", "Router routing time", before, after)
    return found


def flag_renderer(flag):
    if flag["row"].startswith("ratio:"):
        return ratio_text
    return ms if flag["kind"] == "routing" else count


def worst_above_max(current, baseline):
    """The worst r90 / r95 Router vs flat above MAX_RATIO, with its target and main's, or None."""
    cells = [
        (router_vs_flat(current, UNFILTERED, t), t, router_vs_flat(baseline, UNFILTERED, t))
        for t in MAIN_TARGETS
    ]
    cells = [cell for cell in cells if cell[0] is not None and cell[0] > MAX_RATIO]
    return max(cells, key=lambda cell: (cell[0], -cell[1])) if cells else None


def verdict(current, flagged, baseline_note, baseline_commit, baseline=None):
    """The verdict line and its mark. `baseline_note` ends the line when main has no comparable
    baseline."""
    ratio_failures = [f for f in flagged.values() if f["kind"] == "ratio" and f["mark"] == "❌"]
    ratio_warnings = [f for f in flagged.values() if f["kind"] == "ratio" and f["mark"] == "⚠️"]
    against_main = [f for f in flagged.values() if f["kind"] != "ratio"]
    period = True
    if ratio_failures:
        worst_flag = max(ratio_failures, key=lambda f: f["value"])
        text = (
            f"❌ Router needs {ratio_text(worst_flag['value'])} flat's vectors at "
            f"{label(worst_flag['target'])}"
        )
        mark = "❌"
    elif against_main:
        worst_flag = max(against_main, key=lambda f: f["change"])
        kinds = {f["kind"] for f in flagged.values()}
        areas = [AREAS[kind] for kind in ("ratio", "flat", "routing") if kind in kinds]
        render_value = flag_renderer(worst_flag)
        text = (
            f"⚠️ {worst_flag['name']} at {label(worst_flag['target'])} up "
            f"{worst_flag['change']:.0%} against main ({render_value(worst_flag['before'])} → "
            f"{render_value(worst_flag['after'])}). Look at: {', '.join(areas or ['adaptive stopping'])}"
        )
        mark = "⚠️"
    elif ratio_warnings:
        worst_flag = max(ratio_warnings, key=lambda f: f["value"])
        text = (
            f"⚠️ Router needs {ratio_text(worst_flag['value'])} flat's vectors at "
            f"{label(worst_flag['target'])}"
        )
        mark = "⚠️"
    elif baseline_note is None and (above := worst_above_max(current, baseline)) is not None:
        current_ratio, target, main_ratio = above
        text = (
            f"✅ No regression against main {baseline_commit[:7]}. Router is at "
            f"{ratio_text(current_ratio)} flat at {label(target)} (main {ratio_text(main_ratio)})"
        )
        mark = "✅"
    else:
        ratios = [(router_vs_flat(current, UNFILTERED, t), t) for t in TARGETS]
        ratios = [(value_, t) for value_, t in ratios if value_ is not None]
        # Ties go to the lowest target.
        worst_pair = max(ratios, key=lambda pair: (pair[0], -pair[1])) if ratios else None
        worst_ratio = (
            f"worst {ratio_text(worst_pair[0])} at {label(worst_pair[1])}" if worst_pair else "no ratios"
        )
        text = f"✅ Router matches flat ordering ({worst_ratio})"
        if baseline_note is None:
            text += f" · no regression against main {baseline_commit[:7]}"
        mark, period = "✅", False
    if baseline_note is not None:
        text += f" · {baseline_note}"
    return text + ("." if period else ""), mark


def count(value_):
    if value_ >= 10_000:
        return f"{value_ / 1000:.1f}k"
    if value_ >= 10:
        return f"{value_:,.0f}"
    return f"{value_:.1f}"


def ms(value_):
    return f"{value_:.1f}" if value_ < 10 else f"{value_:.0f}"


def ratio_text(value_):
    return "∞" if value_ == math.inf else f"{value_:.2f}×"


def table_cell(value_, render_value, bold, suffix, flag):
    """A cell; a flagged cell is bold, marked and, with a baseline, shows main's value."""
    if value_ is None:
        return "" if render_value is ratio_text else "not reached"
    text = render_value(value_)
    if bold or flag:
        text = f"**{text}**"
    text += suffix
    if flag:
        text += f" {flag['mark']}"
        if flag.get("before") is not None:
            text += f" (main {render_value(flag['before'])})"
    return text


def markdown(report):
    """The PR comment section: verdict, one table per filter level, footnotes and report link."""
    size = report["dataset"].get("size", "")
    flagged = {(f["row"], f["target"]): f for f in report["flags"]}
    lines = ["", f"#### Router quality · Cohere {size}", "", f"**{report['verdict_line']}**", ""]
    for query in QUERIES:
        lines += [
            f"| {LABELS[query]} | " + " | ".join(label(t) for t in TARGETS) + " |",
            "|---|" + "---:|" * len(TARGETS),
        ]
        for row, name, getter, render_value, bold, suffix in table_rows(query):
            if getter is None:
                lines.append(f"| **{name}** |" + " |" * len(TARGETS))
                continue
            cells = [
                table_cell(
                    getter(report, target),
                    render_value,
                    bold,
                    suffix(report, target) if suffix else "",
                    flagged.get((row, target)) if query == UNFILTERED else None,
                )
                for target in TARGETS
            ]
            lines.append(f"| {name} | " + " | ".join(cells) + " |")
        lines.append("")
    lines += [FIXED_NOTE, FILTERED_NOTE]
    if report.get("report_url"):
        lines.append(f"[Full report]({report['report_url']})")
    return "\n".join(lines) + "\n"


RATIO_HISTORY = ("router vs flat, vectors probed", "adaptive on vs off, vectors probed")


def history_metrics(report):
    """Unfiltered metrics for the benchmark history, lower is better: `{name, unit, value}`. The
    two ratios form the alerting `(router)` suite; the rest form `(router detail)`."""
    metrics = []
    for target in TARGETS:
        tag = f"{UNFILTERED}@{label(target)}"
        for name, unit, metric in (
            ("router vs flat, vectors probed", "ratio", router_vs_flat(report, UNFILTERED, target)),
            (
                "adaptive on vs off, vectors probed",
                "ratio",
                adaptive_vs_fixed(report, UNFILTERED, target),
            ),
            (
                "flat vectors probed",
                "vectors/query",
                value(report, UNFILTERED, target, FLAT, "vectors_probed"),
            ),
            (
                "router vectors probed, adaptive on",
                "vectors/query",
                value(report, UNFILTERED, target, ROUTER_ADAPTIVE, "vectors_probed"),
            ),
            ("router routing time", "ms", value(report, UNFILTERED, target, ROUTER, "routing_ms")),
        ):
            if metric is not None and math.isfinite(metric):
                metrics.append({"name": f"{tag} {name}", "unit": unit, "value": metric})
    return metrics


REPORT_COLUMNS = (
    ("budget", "Budget", str),
    ("recall", "Recall", lambda v: f"{v:.3f}"),
    ("clusters", "Clusters", count),
    ("vectors_probed", "Vectors probed", count),
    ("valid_vectors", "Valid vectors", count),
    ("empty_clusters", "Empty clusters", count),
    ("layer0_scored", "Layer-0 scored", count),
    ("exact_rows", "Exact rows", count),
    ("routing_ms", "Routing ms mean", ms),
    ("routing_ms_p95", "Routing ms p95", ms),
    ("centroids_1bit", "Centroids scored, 1-bit", count),
    ("centroids_exact", "Centroids scored, exact", count),
    ("adaptive_stops", "Adaptive stops", str),
    ("parallel_plans", "Parallel plans", str),
    ("workers_launched", "Workers launched", lambda v: f"{v:.1f}"),
)


MAIN_ROWS = (
    ("Router vs flat", lambda r, q, t: router_vs_flat(r, q, t), ratio_text),
    ("Adaptive on vs off", lambda r, q, t: adaptive_vs_fixed(r, q, t), ratio_text),
    ("Flat clusters", lambda r, q, t: value(r, q, t, FLAT, "clusters"), count),
    ("Router clusters", lambda r, q, t: value(r, q, t, ROUTER, "clusters"), count),
    ("Router, adaptive on clusters", lambda r, q, t: value(r, q, t, ROUTER_ADAPTIVE, "clusters"), count),
    ("Flat vectors", lambda r, q, t: value(r, q, t, FLAT, vectors_key(q)), count),
    ("Router vectors", lambda r, q, t: value(r, q, t, ROUTER, vectors_key(q)), count),
    (
        "Router, adaptive on vectors",
        lambda r, q, t: value(r, q, t, ROUTER_ADAPTIVE, vectors_key(q)),
        count,
    ),
    ("Flat routing ms", lambda r, q, t: value(r, q, t, FLAT, "routing_ms"), ms),
    ("Router routing ms", lambda r, q, t: value(r, q, t, ROUTER, "routing_ms"), ms),
)


def html_report(report, baseline=None):
    """Everything the comment leaves out, as one self-contained page, with main side by side."""
    e = html.escape

    def table(head, body):
        return (
            "<table><thead><tr>"
            + "".join(f"<th>{e(str(h))}</th>" for h in head)
            + "</tr></thead><tbody>"
            + "".join("<tr>" + "".join(f"<td>{e(str(c))}</td>" for c in row) + "</tr>" for row in body)
            + "</tbody></table>"
        )

    size = report["dataset"].get("size", "")
    parts = [
        "<!doctype html><html><head><meta charset='utf-8'>",
        f"<title>Router quality · Cohere {e(size)}</title>",
        "<style>body{font:14px system-ui,sans-serif;margin:24px;color:#222}"
        "table{border-collapse:collapse;margin:8px 0 24px}"
        "th,td{border:1px solid #ccc;padding:3px 8px;text-align:right}"
        "th:first-child,td:first-child,th:nth-child(2),td:nth-child(2),th:nth-child(3),"
        "td:nth-child(3){text-align:left}code{font-size:12px}</style></head><body>",
        f"<h1>Router quality · Cohere {e(size)}</h1>",
        f"<p><b>{e(report['verdict_line'])}</b></p>",
        f"<p>pg_search <code>{e(report['commit'])}</code> · pdbench <code>{e(report['pdbench'])}</code>"
        f" · main baseline <code>{e(report.get('baseline_commit') or 'none')}</code></p>",
    ]
    index = report.get("index") or {}
    if index:
        parts.append(
            f"<p>K = {index['clusters']:,} · {index['rows_per_cluster']:,.0f} rows per cluster (mean) · "
            f"{e(index['routing_kind'])} router · rerank factor {e(str(index['rerank_factor']))} · "
            f"{index['segments']} segments</p>"
        )
    parts.append("<h2>Per query, target and arm</h2>")
    for query in QUERIES:
        body = []
        for target in TARGETS:
            for arm in ARMS:
                cell = report["results"][query][str(target)][arm]
                body.append(
                    [label(target), arm, "reached" if cell["reached"] else "not reached"]
                    + [
                        render_value(cell[key]) if cell["reached"] and key in cell else ""
                        for key, _, render_value in REPORT_COLUMNS
                    ]
                )
        parts += [
            f"<h3>{e(LABELS[query])}</h3>",
            table(["Target", "Arm", "Target reached"] + [h for _, h, _ in REPORT_COLUMNS], body),
        ]
    parts.append("<h2>Against main</h2>")
    if baseline is None:
        parts.append(f"<p>{e(report.get('baseline_note') or 'No main baseline.')}</p>")
    else:
        parts.append(f"<p>Main <code>{e(baseline['commit'])}</code>. Vectors are vectors probed "
                     "for Unfiltered and filter-valid vectors for the filtered levels.</p>")
        for query in QUERIES:
            body = []
            for target in TARGETS:
                for name, getter, render_value in MAIN_ROWS:
                    now, before = getter(report, query, target), getter(baseline, query, target)
                    change = (
                        f"{(now - before) / before:+.1%}"
                        if now is not None and before not in (None, 0)
                        else ""
                    )
                    body.append(
                        [
                            label(target),
                            name,
                            "not reached" if now is None else render_value(now),
                            "not reached" if before is None else render_value(before),
                            change,
                        ]
                    )
            parts += [
                f"<h3>{e(LABELS[query])}</h3>",
                table(["Target", "Value", "This commit", "Main", "Change"], body),
            ]
    parts += [
        "<h2>Checks</h2>",
        table(
            ["Check", "Failed", "Checked", "Description"],
            [
                [name, check["failed"], check["checked"], check["description"]]
                for name, check in report["checks"].items()
            ],
        ),
        "<h2>Plan shapes</h2>",
        table(
            ["Query", "Shape"],
            [[LABELS[q], "/".join(report.get("plan_shapes", {}).get(q, []))] for q in QUERIES],
        ),
        "<p>Parallel plans cap each arm's candidate list at the router's k; serial plans take the "
        "whole ranking.</p>",
        "<h2>Tolerated pdbench runs</h2>",
        table(
            ["Arm", "Query", "Report"],
            [[run["arm"], run["query"], run["report"]] for run in report.get("tolerated", [])],
        )
        if report.get("tolerated")
        else "<p>None.</p>",
        "<h2>Segment terminations</h2>",
        table(
            ["Arm", "Termination", "Segments"],
            [
                [arm, name, number]
                for arm, counts in report.get("terminations", {}).items()
                for name, number in sorted(counts.items())
            ],
        ),
        f"<p>{e(ADAPTIVE_STOP)} counts as the adaptive stop.</p>",
        "<h2>Config</h2>",
        table(
            ["Arm", "Session settings"],
            [
                [arm, ", ".join(f"{k} = {v}" for k, v in report["arms"][arm]["session"].items())]
                for arm in ARMS
            ],
        ),
        f"<p>Artifact <code>{e(report['artifact'])}</code> · config fingerprint "
        f"<code>{e(report['fingerprint'][:12])}</code></p>",
        "</body></html>",
    ]
    return "\n".join(parts) + "\n"


def render(measurements, output, commit, pdbench, baseline=None, report_url=None):
    """Writes router_quality.{json,md,html}, router-metrics.json and router-detail-metrics.json."""
    report = dict(measurements)
    size = report["dataset"].get("size", "")
    note = None
    if baseline is None:
        note = "no main baseline yet"
    elif baseline.get("fingerprint") != report["fingerprint"]:
        note, baseline = "main baseline configuration differs", None
    flagged = flags(report, baseline)
    line, mark = verdict(report, flagged, note, baseline["commit"] if baseline else "", baseline)
    report |= {
        "commit": commit,
        "pdbench": pdbench,
        "artifact": f"cohere-{size}-router-quality",
        "report_url": report_url,
        "baseline_commit": baseline["commit"] if baseline else None,
        "max_ratio": MAX_RATIO,
        "flags": sorted(flagged.values(), key=lambda f: (f["row"], f["target"])),
        "baseline_note": note,
        "verdict": mark,
        "verdict_line": line,
    }
    output.mkdir(parents=True, exist_ok=True)
    (output / "router_quality.json").write_text(json.dumps(report, indent=2, default=str) + "\n")
    (output / "router_quality.md").write_text(markdown(report))
    (output / "router_quality.html").write_text(html_report(report, baseline))
    metrics = history_metrics(report)
    ratios = [m for m in metrics if m["name"].split(" ", 1)[1] in RATIO_HISTORY]
    details = [m for m in metrics if m not in ratios]
    (output / "router-metrics.json").write_text(json.dumps(ratios, indent=2) + "\n")
    (output / "router-detail-metrics.json").write_text(json.dumps(details, indent=2) + "\n")
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    command = commands.add_parser("configs")
    command.add_argument("--profile", type=Path, required=True)
    command.add_argument("--output", type=Path, required=True)
    command = commands.add_parser("measure")
    command.add_argument("--runs", type=Path, required=True)
    command.add_argument("--vectors", type=Path, required=True)
    command.add_argument("--output", type=Path, required=True)
    command = commands.add_parser("render")
    command.add_argument("--measurements", type=Path, required=True)
    command.add_argument("--output", type=Path, required=True)
    command.add_argument("--commit", required=True)
    command.add_argument("--pdbench", required=True)
    command.add_argument("--baseline", type=Path)
    command.add_argument("--report-url")
    args = parser.parse_args()
    if args.command == "configs":
        configs(args.profile, args.output)
    elif args.command == "measure":
        result = measure(args.runs, json.loads(args.vectors.read_text()))
        args.output.write_text(json.dumps(result, indent=2) + "\n")
        for name, check in result["checks"].items():
            state = "ok" if check["failed"] == 0 else f"FAILED {check['failed']}"
            print(f"check {name}: {state} of {check['checked']} ({check['description']})")
    else:
        previous = None
        if args.baseline is not None:
            path = args.baseline / "router_quality.json"
            previous = json.loads(path.read_text()) if path.exists() else None
        render(
            json.loads(args.measurements.read_text()),
            args.output,
            args.commit,
            args.pdbench,
            previous,
            args.report_url,
        )

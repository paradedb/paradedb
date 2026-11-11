"""Compare the index's stored router with a flat ranking of every centroid.

Every arm runs on the same index with every segment routed. The flat arm scores every centroid of
every segment exactly (`paradedb.vector_router_audit = flat`), so the clusters and vectors it needs
at a recall target are the reference the stored router (Router) is measured against; each Ratio
is Router / Flat. Both run with adaptive probing off; a third arm repeats the stored router with
adaptive probing on.

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
# Policy thresholds for flagging unfiltered rows, set for review rather than derived.
MAIN_TARGETS = (0.9, 0.95)  # Against-main flags apply at these targets only.
RATIO_RISE = 0.05  # With a baseline, a Ratio above MAX_RATIO flags only past main's by this.
FLAT_WORSE = 0.10  # Flat clusters or Flat vectors, relative increase against main.
ADAPTIVE_RATIO_RISE = 0.05  # Adaptive on vs off, absolute increase against main.
ROUTING_WORSE = 0.25  # Routing ms, relative increase against main ...
ROUTING_WORSE_MS = 0.1  # ... and absolute increase in ms.
# Where to look, per kind of flag, in the order the verdict lists them.
AREAS = {
    "ratio": "router",
    "flat": "clustering",
    "adaptive": "adaptive stopping",
    "routing": "router speed",
}


def count(value_):
    if value_ >= 10_000:
        return f"{value_ / 1000:.1f}k"
    if value_ >= 10:
        return f"{value_:,.0f}"
    return f"{value_:.1f}"


def ms(value_):
    return f"{value_:.1f}" if value_ < 10 else f"{value_:.0f}"


def ratio_text(value_):
    return "∞" if value_ == math.inf else f"{value_:.2f}x"


def vectors_key(query):
    """Unfiltered rows count vectors probed; filtered rows count filter-valid vectors."""
    return "vectors_probed" if query == UNFILTERED else "valid_vectors"


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


def clusters_ratio(report, query, target):
    """Router / Flat on clusters probed."""
    return ratio_of(report, query, target, "clusters")


def vectors_ratio(report, query, target):
    """Router / Flat on the row's vectors."""
    return ratio_of(report, query, target, vectors_key(query))


def adaptive_vs_fixed(report, query, target):
    """Router with adaptive probing on / off, on the row's vectors."""
    return ratio_of(report, query, target, vectors_key(query), ROUTER_ADAPTIVE, ROUTER)


def arm_value(arm, key):
    return lambda report, query, target: value(report, query, target, arm, key)


def arm_vectors(arm):
    return lambda report, query, target: value(report, query, target, arm, vectors_key(query))


# The comment's columns: (id, header, getter, formatter, arm). A column with an arm prints
# `not reached` when that arm missed the target; a ratio column (no arm) is then empty.
COLUMNS = (
    ("flat:clusters", "Flat clusters", arm_value(FLAT, "clusters"), count, FLAT),
    ("router:clusters", "Router clusters", arm_value(ROUTER, "clusters"), count, ROUTER),
    ("ratio:clusters", "Ratio", clusters_ratio, ratio_text, None),
    ("flat:vectors", "Flat vectors", arm_vectors(FLAT), count, FLAT),
    ("router:vectors", "Router vectors", arm_vectors(ROUTER), count, ROUTER),
    ("ratio:vectors", "Ratio", vectors_ratio, ratio_text, None),
    ("ratio:adaptive", "Adaptive on vs off", adaptive_vs_fixed, ratio_text, None),
    ("router:routing", "Routing ms", arm_value(ROUTER, "routing_ms"), ms, ROUTER),
)
HEADERS = {column: header for column, header, *_ in COLUMNS}
GETTERS = {column: getter for column, _, getter, *_ in COLUMNS}
FORMATTERS = {column: formatter for column, _, _, formatter, _ in COLUMNS}
# The Router / Flat columns, with the work each one counts.
RATIOS = (("ratio:clusters", "clusters"), ("ratio:vectors", "vectors"))
FOOTNOTE = (
    "Each Ratio is Router divided by Flat for the two columns before it. Flat and Router use fixed "
    "probes (adaptive off). Adaptive on vs off is the router's vectors probed with adaptive probing "
    "on, divided by off. Vectors in filtered rows count only vectors that pass the filter. Routing "
    "ms is the router's time, with its share of query time in brackets. ❌ marks an unfiltered "
    f"ratio above {ratio_text(MAX_RATIO)}; filtered rows are informational."
)


def flags(current, baseline):
    """Flags on unfiltered rows, keyed by (column id, target). Filtered rows never flag.

    Either Ratio above MAX_RATIO is ❌ at r90 and r95 and ⚠️ at r99; with a main baseline it must
    also exceed main's by more than RATIO_RISE, and a cell main did not reach keeps the absolute
    rule. Against main, at MAIN_TARGETS, a cell is ⚠️ when Flat clusters or Flat vectors rose by
    more than FLAT_WORSE, Adaptive on vs off by more than ADAPTIVE_RATIO_RISE, or Routing ms by more
    than ROUTING_WORSE and ROUTING_WORSE_MS.
    """
    query, found = UNFILTERED, {}
    for target in TARGETS:
        for column, noun in RATIOS:
            now = GETTERS[column](current, query, target)
            before = GETTERS[column](baseline, query, target)
            if (
                now is not None
                and now > MAX_RATIO
                and (before is None or now - before > RATIO_RISE)
            ):
                found[(column, target)] = {
                    "column": column,
                    "target": target,
                    "mark": "⚠️" if target >= 0.99 else "❌",
                    "kind": "ratio",
                    "noun": noun,
                    "value": now,
                    "before": before,
                }
    if baseline is None:
        return found

    def worse(column, kind, target, rule):
        before = GETTERS[column](baseline, query, target)
        after = GETTERS[column](current, query, target)
        if before is not None and after is not None and rule(before, after):
            found[(column, target)] = {
                "column": column,
                "target": target,
                "mark": "⚠️",
                "kind": kind,
                "before": before,
                "after": after,
                "change": (after - before) / before if before else math.inf,
            }

    for target in MAIN_TARGETS:
        for column in ("flat:clusters", "flat:vectors"):
            worse(column, "flat", target, lambda a, b: b > a * (1 + FLAT_WORSE))
        worse("ratio:adaptive", "adaptive", target, lambda a, b: b - a > ADAPTIVE_RATIO_RISE)
        worse(
            "router:routing",
            "routing",
            target,
            lambda a, b: b > a * (1 + ROUTING_WORSE) and b - a > ROUTING_WORSE_MS,
        )
    return found


def router_needs(ratio_flags):
    """The worst flagged Ratio as `Router needs <x>x flat's <work> at <target>`, naming clusters
    whenever the clusters ratio flagged. Ties go to the lowest target."""
    clusters = [f for f in ratio_flags if f["noun"] == "clusters"]
    worst = max(clusters or ratio_flags, key=lambda f: (f["value"], -f["target"]))
    return (
        f"{worst['mark']} Router needs {ratio_text(worst['value'])} flat's {worst['noun']} at "
        f"{label(worst['target'])}"
    )


def worst_ratio(report):
    """The highest unfiltered Ratio over both ratio columns and every target, as (ratio, target,
    column), or None. Ties go to the lowest target."""
    cells = [
        (GETTERS[column](report, UNFILTERED, target), target, column)
        for target in TARGETS
        for column, _ in RATIOS
    ]
    cells = [cell for cell in cells if cell[0] is not None]
    return max(cells, key=lambda cell: (cell[0], -cell[1])) if cells else None


def verdict(current, flagged, baseline_note, baseline_commit, baseline=None):
    """The verdict line and its mark. `baseline_note` ends the line when main has no comparable
    baseline."""
    ratio_flags = [f for f in flagged.values() if f["kind"] == "ratio"]
    failures = [f for f in ratio_flags if f["mark"] == "❌"]
    warnings = [f for f in ratio_flags if f["mark"] == "⚠️"]
    against_main = [f for f in flagged.values() if f["kind"] != "ratio"]
    worst = worst_ratio(current)
    period = True
    if failures:
        text, mark = router_needs(failures), "❌"
    elif against_main:
        flag = max(against_main, key=lambda f: (f["change"], -f["target"]))
        kinds = {f["kind"] for f in flagged.values()}
        areas = ", ".join(area for kind, area in AREAS.items() if kind in kinds)
        render_value = FORMATTERS[flag["column"]]
        text = (
            f"⚠️ {HEADERS[flag['column']]} at {label(flag['target'])} up {flag['change']:.0%} "
            f"against main ({render_value(flag['before'])} → {render_value(flag['after'])}). "
            f"Look at: {areas}"
        )
        mark = "⚠️"
    elif warnings:
        text, mark = router_needs(warnings), "⚠️"
    elif baseline_note is None and worst is not None and worst[0] > MAX_RATIO:
        # Above MAX_RATIO without a flag: within RATIO_RISE of main's ratio for the same cell.
        now, target, column = worst
        before = GETTERS[column](baseline, UNFILTERED, target)
        main_text = f" (main {ratio_text(before)})" if before is not None else ""
        text = (
            f"✅ No regression against main {baseline_commit[:7]}. Router is at "
            f"{ratio_text(now)} flat at {label(target)}{main_text}"
        )
        mark = "✅"
    else:
        worst_text = (
            f"worst {ratio_text(worst[0])} at {label(worst[1])}" if worst else "no ratios"
        )
        text = f"✅ Router matches flat ordering ({worst_text})"
        if baseline_note is None:
            text += f" · no regression against main {baseline_commit[:7]}"
        mark, period = "✅", False
    if baseline_note is not None:
        text += f" · {baseline_note}"
    return text + ("." if period else ""), mark


def routing_share(report, query, target):
    share = value(report, query, target, ROUTER, "routing_share")
    return "" if share is None else f" ({share:.0%})"


def table_cell(value_, render_value, suffix, flag, is_ratio):
    """A cell; a flagged cell is bold, marked and, with a baseline, shows main's value."""
    if value_ is None:
        return "" if is_ratio else "not reached"
    text = render_value(value_)
    if flag:
        text = f"**{text}**"
    text += suffix
    if flag:
        text += f" {flag['mark']}"
        if flag.get("before") is not None:
            text += f" (main {render_value(flag['before'])})"
    return text


def markdown(report):
    """The PR comment section: verdict, one row per filter level and target, footnote and report
    link."""
    size = report["dataset"].get("size", "")
    flagged = {(f["column"], f["target"]): f for f in report["flags"]}
    lines = [
        "",
        f"#### Router quality · Cohere {size}",
        "",
        f"**{report['verdict_line']}**",
        "",
        "| | " + " | ".join(HEADERS.values()) + " |",
        "|---|" + "---:|" * len(COLUMNS),
    ]
    for query in QUERIES:
        for target in TARGETS:
            cells = [
                table_cell(
                    getter(report, query, target),
                    render_value,
                    routing_share(report, query, target) if column == "router:routing" else "",
                    flagged.get((column, target)) if query == UNFILTERED else None,
                    arm is None,
                )
                for column, _, getter, render_value, arm in COLUMNS
            ]
            lines.append(f"| {LABELS[query]} {label(target)} | " + " | ".join(cells) + " |")
    lines += ["", FOOTNOTE]
    if report.get("report_url"):
        lines.append(f"[Full report]({report['report_url']})")
    return "\n".join(lines) + "\n"


RATIO_HISTORY = ("router vs flat, clusters", "adaptive on vs off, vectors probed")


def history_metrics(report):
    """Unfiltered metrics for the benchmark history, lower is better: `{name, unit, value}`. The
    RATIO_HISTORY pair forms the alerting `(router)` suite; the rest form `(router detail)`."""
    metrics = []
    for target in TARGETS:
        tag = f"{UNFILTERED}@{label(target)}"
        for name, unit, metric in (
            ("router vs flat, clusters", "ratio", clusters_ratio(report, UNFILTERED, target)),
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
    ("Flat clusters", arm_value(FLAT, "clusters"), count),
    ("Router clusters", arm_value(ROUTER, "clusters"), count),
    ("Clusters ratio", clusters_ratio, ratio_text),
    ("Flat vectors", arm_vectors(FLAT), count),
    ("Router vectors", arm_vectors(ROUTER), count),
    ("Vectors ratio", vectors_ratio, ratio_text),
    ("Adaptive on vs off", adaptive_vs_fixed, ratio_text),
    ("Router, adaptive on clusters", arm_value(ROUTER_ADAPTIVE, "clusters"), count),
    ("Router, adaptive on vectors", arm_vectors(ROUTER_ADAPTIVE), count),
    ("Flat routing ms", arm_value(FLAT, "routing_ms"), ms),
    ("Router routing ms", arm_value(ROUTER, "routing_ms"), ms),
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
        "flags": sorted(flagged.values(), key=lambda f: (f["column"], f["target"])),
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

"""Compare the index's stored router with a flat ranking of every centroid.

Every arm runs on the same index with every segment routed. The flat arms score every centroid of
every segment exactly (`paradedb.vector_router_audit = flat`), so the cluster and vector counts
they need at a recall target are the reference the stored router is measured against. Ratios are
stored / flat. The fixed-probe arms run with adaptive probing off and are gated; the adaptive arms
repeat them with adaptive probing on and are reported only.

    configs  write router-<arm>.toml for every arm from a Cohere profile
    measure  rerun pdbench's selected budgets under EXPLAIN and record per-segment work
    render   write router_quality.json and the router_quality.md comment section
"""

import argparse
import collections
import hashlib
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
    "flat-adaptive": ("flat", "on"),
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
ROUTING_NOTE = "Routing time includes row gating."
FILTERED_NOTE = "Filtered cells are informational: forced routing, parallel plans."
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


def second_metric(query):
    """Unfiltered rows compare vectors probed; filtered rows compare filter-valid vectors."""
    return "vectors_probed" if query.endswith("_unfiltered") else "valid_vectors"


def ratio(stored, flat, key):
    if not (stored["reached"] and flat["reached"]):
        return None
    if flat[key] == 0:
        return 1.0 if stored[key] == 0 else math.inf
    return stored[key] / flat[key]


def status(target, stored, flat, ratios):
    """✅ within MAX_RATIO; otherwise ❌ at r90 and r95, ⚠️ at r99. A row flat cannot reach has none."""
    if not flat["reached"]:
        return "—"
    if stored["reached"] and all(value <= MAX_RATIO for value in ratios.values()):
        return "✅"
    return "⚠️" if target >= 0.99 else "❌"


def gated(query):
    """Only the unfiltered query carries a status: filtered rows run with the located path forced
    off and on parallel plans, so they are reported, not gated."""
    return query.endswith("_unfiltered")


def rows(results):
    out = []
    for query in QUERIES:
        for target in TARGETS:
            cell = results[query][str(target)]
            stored, flat = cell["stored"], cell["flat"]
            ratios = {
                key: ratio(stored, flat, key) for key in ("clusters", second_metric(query))
            }
            out.append(
                {
                    "query": query,
                    "target": target,
                    "ratios": ratios,
                    "status": status(target, stored, flat, ratios) if gated(query) else "",
                }
            )
    return out


def adaptive_status(fixed, stored, flat, value):
    """✅ within MAX_RATIO, ⚠️ beyond it or when the router reaches the target with fixed probes
    but not with adaptive probing. Never ❌: adaptive probing is reported, not gated."""
    if fixed["reached"] and not stored["reached"]:
        return "⚠️"
    if value is None:
        return "—"
    return "✅" if value <= MAX_RATIO else "⚠️"


def adaptive_rows(results):
    """Clusters per query with adaptive probing on, against flat and against fixed probes."""
    out = []
    for query in QUERIES:
        for target in TARGETS:
            cell = results[query][str(target)]
            fixed, stored, flat = cell["stored"], cell["stored-adaptive"], cell["flat-adaptive"]
            value = ratio(stored, flat, "clusters")
            out.append(
                {
                    "query": query,
                    "target": target,
                    "ratio": value,
                    "vs_fixed": (
                        stored["clusters"] / fixed["clusters"] - 1
                        if stored["reached"] and fixed["reached"] and fixed["clusters"]
                        else None
                    ),
                    "stops_short": fixed["reached"] and not stored["reached"],
                    "status": adaptive_status(fixed, stored, flat, value) if gated(query) else "",
                }
            )
    return out


def worst(candidates):
    return max(
        candidates,
        key=lambda row: math.inf if row["ratios"]["clusters"] is None else row["ratios"]["clusters"],
    )


def verdict(table, adaptive=()):
    failed = [row for row in table if row["status"] == "❌"]
    if failed:
        row = worst(failed)
        where = f"{label(row['target'])} ({LABELS[row['query']]})"
        if row["ratios"]["clusters"] is None:
            headline = f"❌ Router does not reach {where}; flat does."
        else:
            headline = f"❌ Router needs {ratio_text(row['ratios']['clusters'])} flat's clusters at {where}."
    else:
        headline = "✅ Router matches flat at r90 and r95."
    line = f"**{headline}**"
    warned = [row for row in table if row["status"] == "⚠️"]
    if warned:
        row = worst(warned)
        query = LABELS[row["query"]]
        if row["ratios"]["clusters"] is None:
            line += f" ⚠️ Router does not reach r99 ({query}); flat does (reported, not gated)."
        else:
            line += (
                f" ⚠️ r99 needs {ratio_text(row['ratios']['clusters'])} flat's clusters"
                f" ({query}, reported, not gated)."
            )
    for row in adaptive:
        if row["stops_short"] and gated(row["query"]):
            line += (
                f" ⚠️ Adaptive probing stops short of {label(row['target'])}"
                f" on {LABELS[row['query']]}."
            )
    return line, "❌" if failed else "✅"


def count(value):
    if value >= 10_000:
        return f"{value / 1000:.1f}k"
    if value >= 10:
        return f"{value:,.0f}"
    return f"{value:.1f}"


def ms(value):
    return f"{value:.1f}" if value < 10 else f"{value:.0f}"


def percent(value):
    """Signed percentage with a typographic minus; a change that rounds to zero has no sign."""
    text = f"{value:+.0%}"
    return "0%" if text in ("+0%", "-0%") else text.replace("-", "−")


def ratio_text(value):
    return "∞" if value == math.inf else f"{value:.2f}×"


def cell_text(arm, key, render=count):
    return render(arm[key]) if arm["reached"] else "not reached"


def change(before, after, render):
    if before is None or after is None:
        return f"{'not reached' if before is None else render(before)} → " + (
            "not reached" if after is None else render(after)
        )
    delta = f" ({(after - before) / before:+.1%})" if before else ""
    return f"{render(before)} → {render(after)}{delta}"


def against_main(current, baseline):
    if baseline is None:
        return ["**Against main**", "", "No main baseline yet."]
    head = f"**Against main** (`{baseline['commit']}`)"
    if baseline["fingerprint"] != current["fingerprint"]:
        return [head, "", "baseline configuration differs"]
    query = "knn_top10_unfiltered"

    def value(report, target, arm, key):
        cell = report["results"].get(query, {}).get(str(target), {}).get(arm)
        return cell[key] if cell and cell["reached"] else None

    def clusters_ratio(report, target):
        stored = value(report, target, "stored", "clusters")
        flat = value(report, target, "flat", "clusters")
        return stored / flat if stored is not None and flat else None

    def share(report, target):
        result = value(report, target, "stored", "routing_share")
        return None if result is None else result * 100

    lines = [
        head,
        "",
        "| | " + " | ".join(label(t) for t in TARGETS) + " |",
        "|---|" + "---:|" * len(TARGETS),
    ]
    for name, get, render in (
        ("Router / flat clusters (unfiltered)", clusters_ratio, ratio_text),
        (
            "Flat vectors probed (clustering)",
            lambda report, t: value(report, t, "flat", "vectors_probed"),
            count,
        ),
        (
            "Routing ms (unfiltered)",
            lambda report, t: value(report, t, "stored", "routing_ms"),
            ms,
        ),
        ("Routing share of query time (unfiltered)", share, lambda v: f"{v:.0f}%"),
        (
            "Router clusters, adaptive probing (unfiltered)",
            lambda report, t: value(report, t, "stored-adaptive", "clusters"),
            count,
        ),
    ):
        cells = []
        for target in TARGETS:
            before, after = get(baseline, target), get(current, target)
            if name.startswith(("Router /", "Routing share")):
                cells.append(
                    f"{'not reached' if before is None else render(before)} → "
                    f"{'not reached' if after is None else render(after)}"
                )
            else:
                cells.append(change(before, after, render))
        lines.append(f"| {name} | " + " | ".join(cells) + " |")
    return lines


def markdown(report, baseline):
    index = report["index"] or {}
    table = report["rows"]
    by_key = {(row["query"], row["target"]): row for row in table}
    adaptive_by_key = {(row["query"], row["target"]): row for row in report["adaptive_rows"]}
    size = report["dataset"].get("size", "")
    lines = [
        "",
        f"#### Router quality: Cohere {size}, {index.get('segments', '?')} segments",
        "",
        report["verdict_line"],
        "",
    ]
    if index:
        lines += [
            f"K = {index['clusters']:,} · {index['rows_per_cluster']:,.0f} rows per cluster (mean) · "
            f"{index['routing_kind']} router · rerank factor {index['rerank_factor']} · "
            "fixed probes, every segment routed",
            "",
        ]
    if not report["checks_passed"]:
        failed = [name for name, result in report["checks"].items() if result["failed"]]
        lines += [f"⚠️ Statistic checks failed: {', '.join(failed)}. See router_quality.json.", ""]
    for query in QUERIES:
        filtered = not query.endswith("_unfiltered")
        second = "Valid vectors" if filtered else "Vectors probed"
        header = f"| Target | Clusters: router | flat | ratio | {second}: router | flat | ratio |"
        align = "|---|---:|---:|---:|---:|---:|---:|"
        if filtered:
            header += " Empty clusters |"
            align += "---:|"
        else:
            header += " |"
            align += ":-:|"
        lines += [f"**{LABELS[query]}**", "", header, align]
        for target in TARGETS:
            cell = report["results"][query][str(target)]
            stored, flat = cell["stored"], cell["flat"]
            row = by_key[(query, target)]
            cells = [label(target)]
            for key in ("clusters", second_metric(query)):
                value = row["ratios"][key]
                cells += [
                    cell_text(stored, key),
                    cell_text(flat, key),
                    "" if value is None else ratio_text(value),
                ]
            if filtered:
                cells.append(cell_text(stored, "empty_clusters"))
            else:
                cells.append(row["status"])
            lines.append("| " + " | ".join(cells) + " |")
        lines += [
            "",
            "**With adaptive probing on** (clusters probed per query)",
            "",
            "| Target | Router | Flat | ratio | Router vs fixed probes |" + ("" if filtered else " |"),
            "|---|---:|---:|---:|---:|" + ("" if filtered else ":-:|"),
        ]
        for target in TARGETS:
            cell = report["results"][query][str(target)]
            row = adaptive_by_key[(query, target)]
            cells = [
                label(target),
                cell_text(cell["stored-adaptive"], "clusters"),
                cell_text(cell["flat-adaptive"], "clusters"),
                "" if row["ratio"] is None else ratio_text(row["ratio"]),
                "" if row["vs_fixed"] is None else percent(row["vs_fixed"]),
            ] + ([] if filtered else [row["status"]])
            lines.append("| " + " | ".join(cells) + " |")
        lines.append("")
    lines += [FILTERED_NOTE, ""]

    lines += [
        "**Routing time** (ms per query, mean; router's share of query time in brackets)",
        "",
        "| | " + " | ".join(f"{label(t)}: router | flat" for t in TARGETS) + " |",
        "|---|" + "---:|" * (2 * len(TARGETS)),
    ]
    for query in QUERIES:
        cells = []
        for target in TARGETS:
            cell = report["results"][query][str(target)]
            stored, flat = cell["stored"], cell["flat"]
            if stored["reached"]:
                share = stored["routing_share"]
                bracket = "" if share is None else f" ({share:.0%})"
                cells.append(f"{ms(stored['routing_ms'])}{bracket}")
            else:
                cells.append("not reached")
            cells.append(cell_text(flat, "routing_ms", ms))
        lines.append(f"| {LABELS[query]} | " + " | ".join(cells) + " |")
    lines += ["", ROUTING_NOTE, ""]
    lines += against_main(report, baseline) + [""]

    lines += [
        "<details><summary>Router work</summary>",
        "",
        "| Query | Target | Arm | Routing ms: mean | p95 | Centroids scored: 1-bit | exact |",
        "|---|---|---|---:|---:|---:|---:|",
    ]
    for query, target, arm, cell in cells_in_order(report):
        values = (
            [ms(cell["routing_ms"]), ms(cell["routing_ms_p95"])]
            + [count(cell["centroids_1bit"]), count(cell["centroids_exact"])]
            if cell["reached"]
            else ["not reached"] * 4
        )
        lines.append(f"| {LABELS[query]} | {label(target)} | {arm} | " + " | ".join(values) + " |")
    lines += [
        "",
        "</details>",
        "<details><summary>Scan work</summary>",
        "",
        "| Query | Target | Arm | Budget | Vectors probed | Valid vectors | Layer-0 scored "
        "| Exact rows | Adaptive stops |",
        "|---|---|---|---:|---:|---:|---:|---:|---:|",
    ]
    for query, target, arm, cell in cells_in_order(report):
        values = (
            [f"`{cell['budget']}`"]
            + [count(cell[k]) for k in ("vectors_probed", "valid_vectors", "layer0_scored", "exact_rows")]
            + [f"{cell['adaptive_stops']}/{report['query_vectors']}"]
            if cell["reached"]
            else ["not reached"] * 6
        )
        lines.append(f"| {LABELS[query]} | {label(target)} | {arm} | " + " | ".join(values) + " |")
    lines += ["", "</details>", "<details><summary>Config</summary>", ""]
    for arm in ARMS:
        settings = ", ".join(
            f"`{key} = {value}`" for key, value in report["arms"][arm]["session"].items()
        )
        lines.append(f"- {arm}: {settings}")
    shapes = report.get("plan_shapes", {})
    lines.append(
        "- Plans: "
        + " · ".join(f"{LABELS[q]} {'/'.join(shapes.get(q, [])) or '?'}" for q in QUERIES)
        + " (parallel plans cap each arm's candidate list at the router's k; serial plans take "
        "the whole ranking)"
    )
    for run in report.get("tolerated", []):
        lines.append(
            f"- Tolerated pdbench exit for `{run['arm']}` `{run['query']}`: "
            f"\"representative layer0 has no scan stage\"; report saved to `{run['report']}`"
        )
    lines += [
        f"- pg_search `{report['commit']}`, pdbench `{report['pdbench']}`",
        f"- Artifact `{report['artifact']}`, config fingerprint `{report['fingerprint'][:12]}`",
        "",
        "</details>",
        "",
    ]
    return "\n".join(lines)


def cells_in_order(report):
    for query in QUERIES:
        for target in TARGETS:
            for arm in ARMS:
                yield query, target, arm, report["results"][query][str(target)][arm]


def render(measurements, output, commit, pdbench, baseline=None):
    report = dict(measurements)
    size = report["dataset"].get("size", "")
    table = rows(report["results"])
    adaptive = adaptive_rows(report["results"])
    line, overall = verdict(table, adaptive)
    report |= {
        "commit": commit,
        "pdbench": pdbench,
        "artifact": f"cohere-{size}-router-quality",
        "max_ratio": MAX_RATIO,
        "rows": table,
        "adaptive_rows": adaptive,
        "verdict": overall,
        "verdict_line": line,
        "routing_time_note": ROUTING_NOTE,
        "filtered_note": FILTERED_NOTE,
    }
    output.mkdir(parents=True, exist_ok=True)
    (output / "router_quality.json").write_text(json.dumps(report, indent=2, default=str) + "\n")
    (output / "router_quality.md").write_text(markdown(report, baseline))
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
        )

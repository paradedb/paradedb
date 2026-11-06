#!/usr/bin/env python3
"""Stage telemetry at each pdbench operating point.

Reads each rerank factor's pdbench search report, then runs the query set at every recall target
with paradedb.vector_stats on: each query once to warm, then once under EXPLAIN (ANALYZE, VERBOSE,
FORMAT JSON). Writes perf.json (per-point means) and points.json (per-query router requests and
cluster counts, for the offline recall check).
"""
import argparse
import glob
import json
import math
import os
import re
import subprocess
import tempfile
import tomllib

DELIMITER = "@@@explain"
STAGES = {
    "routing": ("routing_ns", "routing_bytes_read", "routing_reads"),
    "routing_exact": ("routing_exact_ns", "routing_exact_bytes_read", "routing_exact_reads"),
    "layer0_scan": ("layer0_scan_ns", "layer0_bytes_read", "layer0_reads"),
    "layer1_scan": ("layer1_scan_ns", "layer1_bytes_read", "layer1_reads"),
}


def psql(url, sql_path):
    return subprocess.run(
        ["psql", url, "-X", "-q", "-At", "-v", "ON_ERROR_STOP=1", "-f", sql_path],
        check=True,
        capture_output=True,
        text=True,
    ).stdout


def latest_report(runs_dir, profile):
    reports = sorted(glob.glob(f"{runs_dir}/{profile}/*_search.json"))
    if not reports:
        raise SystemExit(f"no search report under {runs_dir}/{profile}")
    with open(reports[-1]) as handle:
        return json.load(handle)


def segment_info(node):
    """The Segment Info object anywhere under a plan node."""
    if "Segment Info" in node:
        info = node["Segment Info"]
        return json.loads(info) if isinstance(info, str) else info
    for child in node.get("Plans", []):
        found = segment_info(child)
        if found is not None:
            return found
    return None


def run_point(args, session, query_sql, vectors, factor, value):
    settings = dict(session)
    settings["paradedb.vector_router_rerank_factor"] = str(factor)
    settings["paradedb.vector_cluster_max_probe"] = value
    settings["paradedb.vector_stats"] = "on"
    lines = [f"SET {name} = '{setting}';" for name, setting in settings.items()]
    for _, vector in vectors:
        lines.append(f"SET cohere.qvec = '{vector}';")
        lines.append(f"\\o /dev/null\n{query_sql.rstrip().rstrip(';')};\n\\o")
        lines.append(f"\\qecho {DELIMITER}")
        lines.append(f"EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON) {query_sql.rstrip().rstrip(';')};")
    with tempfile.NamedTemporaryFile("w", suffix=".sql", delete=False) as script:
        script.write("\n".join(lines) + "\n")
    try:
        output = psql(args.url, script.name)
    finally:
        os.unlink(script.name)
    explains = [json.loads(piece) for piece in output.split(DELIMITER)[1:]]
    if len(explains) != len(vectors):
        raise SystemExit(f"{len(explains)} plans for {len(vectors)} queries")
    return explains


def summarize(explains, query_ids, clusters_total):
    rows = []
    for query_id, explain in zip(query_ids, explains):
        info = segment_info(explain[0]["Plan"])
        if info is None or len(info) != 1:
            raise SystemExit(f"expected one segment's stats, got {info!r}")
        stats = next(iter(info.values()))
        rows.append((query_id, stats, explain[0]["Execution Time"]))

    def mean(field):
        return sum(stats.get(field, 0) for _, stats, _ in rows) / len(rows)

    clusters = [stats["postings_row"] + stats["postings_skipped"] for _, stats, _ in rows]
    router_k = [
        min(
            clusters_total,
            max(1, stats["routing_candidate_count"], math.ceil(stats["work_budget"] - 1e-3)),
        )
        for _, stats, _ in rows
    ]
    ordered = sorted(clusters)
    stages = {
        stage: {
            "ns": mean(ns),
            "bytes_read": mean(bytes_read),
            "reads": mean(reads),
        }
        for stage, (ns, bytes_read, reads) in STAGES.items()
    }
    stages["exact_phase"] = {
        "ns": mean("exact_fetch_ns") + mean("exact_score_ns") + mean("exact_scan_ns"),
        "bytes_read": mean("exact_bytes_read"),
        "reads": mean("exact_reads"),
    }
    execution_ms = sum(row[2] for row in rows) / len(rows)
    routing_ns = stages["routing"]["ns"] + stages["routing_exact"]["ns"]
    layer0_bytes = stages["layer0_scan"]["bytes_read"]
    router_bytes = stages["routing"]["bytes_read"] + stages["routing_exact"]["bytes_read"]

    def per(numerator, denominator):
        return numerator / denominator if denominator else None

    return (
        {
            "clusters_mean": sum(clusters) / len(clusters),
            "clusters_p95": ordered[max(0, math.ceil(0.95 * len(ordered)) - 1)],
            "router_k_mean": sum(router_k) / len(router_k),
            "execution_ms_mean": execution_ms,
            "stages": stages,
            "one_bit_scored": mean("routing_approximate_scored"),
            "exact_scored": mean("routing_exact_scored"),
            "layer0_scored": mean("layer0_scored"),
            "exact_rows": mean("exact_rows"),
            "router_ns_per_one_bit": per(
                stages["routing"]["ns"], mean("routing_approximate_scored")
            ),
            "layer0_ns_per_row": per(stages["layer0_scan"]["ns"], mean("layer0_scored")),
            "router_ns_per_exact": per(
                stages["routing_exact"]["ns"], mean("routing_exact_scored")
            ),
            "exact_ns_per_row": per(stages["exact_phase"]["ns"], mean("exact_rows")),
            "routing_share": per(routing_ns, execution_ms * 1e6),
            "router_bytes_over_layer0": per(router_bytes, layer0_bytes),
        },
        router_k,
        clusters,
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--url", required=True)
    parser.add_argument("--profile", required=True, help="the profile the sweeps ran under")
    parser.add_argument("--assets", required=True)
    parser.add_argument("--runs-dir", required=True)
    parser.add_argument("--queries", required=True, help="queries.jsonl")
    parser.add_argument("--clusters", type=int, required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--query-stem", default="knn_top10_unfiltered")
    args = parser.parse_args()

    with open(args.profile, "rb") as handle:
        profile = tomllib.load(handle)
    session = dict(profile["search"]["session"])
    table, dims = profile["dataset"]["table"], profile["dataset"]["dims"]
    with open(f"{args.assets}/cohere/queries/{args.query_stem}.sql") as handle:
        query_sql = re.sub(r"\{\{\s*table\s*\}\}", table, handle.read())
    query_sql = re.sub(r"\{\{\s*dims\s*\}\}", str(dims), query_sql)

    held_out = []
    with open(args.queries) as handle:
        for line in handle:
            row = json.loads(line)
            if row["set"] == "held_out":
                held_out.append(row)
    held_out.sort(key=lambda row: row["query_id"])
    vectors = [(row["query_id"], json.dumps(row["emb"], separators=(",", ":"))) for row in held_out]
    query_ids = [query_id for query_id, _ in vectors]

    perf, points = [], []
    for factor in (2, 4):
        report = latest_report(args.runs_dir, f"k3-{factor}")
        variant = report["data"]["variants"][args.query_stem]
        by_value = {point["value"]: point for point in variant["points"]}
        for target, chosen in sorted(variant["targets"].items(), key=lambda kv: float(kv[0])):
            value = chosen["value"]
            label = f"K3-{factor} r{target}"
            explains = run_point(args, session, query_sql, vectors, factor, value)
            summary, router_k, clusters = summarize(explains, query_ids, args.clusters)
            latency = by_value[value]["metrics"]["latency"]
            perf.append(
                {
                    "label": label,
                    "rerank_factor": factor,
                    "target": float(target),
                    "max_probe": value,
                    "recall": chosen["recall"],
                    "p50_ms": latency.get("p50_ms"),
                    "p99_ms": latency.get("p99_ms"),
                    **summary,
                }
            )
            points.append(
                {
                    "label": label,
                    "rerank_factor": factor,
                    "query_ids": query_ids,
                    "router_k": router_k,
                    "clusters": clusters,
                }
            )
            print(f"{label}: max_probe {value}, clusters {summary['clusters_mean']:.1f}", flush=True)
    with open(f"{args.out}/perf.json", "w") as handle:
        json.dump(perf, handle, indent=2)
    with open(f"{args.out}/points.json", "w") as handle:
        json.dump(points, handle)


if __name__ == "__main__":
    main()

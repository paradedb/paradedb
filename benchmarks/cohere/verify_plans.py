"""Check the parameterized k6 query plans before measuring latency."""

import argparse
import json
import os
import subprocess
from pathlib import Path

ROUTING_COUNTERS = (
    "routing_lists_scanned",
    "routing_approximate_scored",
    "routing_approximate_members_scored",
    "routing_approximate_pruned",
    "routing_exact_scored",
    "routing_exact_pruned",
    "routing_candidate_count",
)


def verify(plan, filtered, neighbors=None, engine="pg_search", index_name=None):
    """Require ParadeDB execution and index-level text filtering."""
    nodes = [plan[0]["Plan"]]
    scans = []
    while nodes:
        node = nodes.pop()
        if engine == "pg_vectorsearch":
            if node.get("Custom Plan Provider"):
                raise ValueError(
                    "Expected a pg_vectorsearch index scan, found a custom scan"
                )
            if (
                node.get("Node Type") == "Index Scan"
                and node.get("Index Name") == index_name
            ):
                scans.append(node)
        if engine == "pg_search" and node.get("Filter"):
            raise ValueError("Filter runs outside the ParadeDB index")
        if "ParadeDB" in node.get("Custom Plan Provider", ""):
            scans.append({key.strip(): value for key, value in node.items()})
        nodes.extend(node.get("Plans", []))
    if engine == "pg_vectorsearch":
        if not scans:
            raise ValueError("Expected a pg_vectorsearch index scan")
        return scans
    if not scans or (
        filtered
        and not any("text" in str(scan.get("Tantivy Query", "")) for scan in scans)
    ):
        raise ValueError("Expected a ParadeDB scan with the text predicate pushed down")
    if neighbors is not None and not any(
        scan.get("TopK Limit") == neighbors
        and "<=> vector" in str(scan.get("TopK Order By", ""))
        for scan in scans
    ):
        raise ValueError(
            f"Expected cosine vector TopK with limit {neighbors} inside ParadeDB"
        )

    return scans


def routing_metrics(scans, require_routing, expected_mode=None):
    """Verify the measured router and sum its flat per-segment counters."""
    totals = dict.fromkeys(ROUTING_COUNTERS, 0)
    routed_segments = 0
    for scan in scans:
        segments = scan.get("Segment Info", {})
        # EXPLAIN's add_json uses a JSON-valued text property. Accept decoded
        # objects too, as used by callers that already normalize plan fields.
        if isinstance(segments, str):
            segments = json.loads(segments)
        for segment in segments.values():
            kind = segment.get("routing_kind")
            if kind is None:
                continue  # Located and flat scans do not invoke a router.
            if kind != "quantized_stacked":
                raise ValueError(f"Expected quantized_stacked router, found {kind}")
            if (
                expected_mode is not None
                and segment.get("routing_mode") != expected_mode
            ):
                raise ValueError(f"Expected {expected_mode} routing mode")
            counts = {key: segment.get(key) for key in ROUTING_COUNTERS}
            if any(
                not isinstance(value, int) or isinstance(value, bool) or value < 0
                for value in counts.values()
            ):
                raise ValueError("Missing or invalid router counters")
            if (
                counts["routing_approximate_members_scored"]
                != counts["routing_approximate_pruned"] + counts["routing_exact_scored"]
                or counts["routing_exact_scored"]
                != counts["routing_exact_pruned"] + counts["routing_candidate_count"]
                or counts["routing_approximate_scored"]
                < counts["routing_approximate_members_scored"]
            ):
                raise ValueError("Router scoring and pruning counters do not reconcile")
            routed_segments += 1
            for key, value in counts.items():
                totals[key] += value
    if require_routing and (
        not routed_segments
        or not totals["routing_approximate_members_scored"]
        or not totals["routing_exact_scored"]
    ):
        raise ValueError(
            "Unfiltered query did not exercise quantized routing and exact reranking"
        )
    return {"routed_segments": routed_segments, **totals}


def routing_summary(records):
    """Routing summary."""
    lines = [
        "### Router verification",
        "",
        (
            "| Point | Routed segments | Centroid scores (1-bit) | Member scores "
            "(1-bit) | Members pruned before rerank | Full-precision scores | After "
            "exact rerank |"
        ),
        "|---|---:|---:|---:|---:|---:|---:|",
    ]
    for name, counts in records.items():
        centroids = (
            counts["routing_approximate_scored"]
            - counts["routing_approximate_members_scored"]
        )
        values = [
            counts[key]
            for key in (
                "routing_approximate_members_scored",
                "routing_approximate_pruned",
                "routing_exact_scored",
                "routing_candidate_count",
            )
        ]
        lines.append(
            f"| `{name}` | {counts['routed_segments']} | {centroids} | "
            + " | ".join(map(str, values))
            + " |"
        )
    lines += [
        "",
        (
            "Counts sum over segments for the first fixture vector at each selected "
            "operating point; they are not workload averages. "
            "Centroid scores include parent traversal. Pruning counts only scored "
            "bottom-router members rejected by the padded shortlist; "
            "unopened lists and APS-skipped members are not counted as pruned. Located "
            "scans can bypass routing."
        ),
        "",
    ]
    return "\n".join(lines)


def main():
    """Verify selected operating points and save diagnostic plans."""
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    parser.add_argument("--quantized", action="store_true")
    args = parser.parse_args()
    points = json.loads((args.directory / "operating_points.json").read_text())
    vector = json.loads((args.directory / "query_vectors.json").read_text())[0]
    output = args.directory / "plans"
    output.mkdir(exist_ok=True)
    records = {}
    for point in points:
        if not point["reached"]:
            continue
        session = point["session"] | {
            point.get("param", "paradedb.vector_cluster_max_probe"): point["probe"]
        }
        if args.quantized:
            session["paradedb.vector_stats"] = "on"
        settings = "\n".join(
            f"SET {key} = '{value}';" for key, value in session.items()
        )
        sql = f"""{settings}
PREPARE vector_ci(vector({point["dimensions"]})) AS {point["sql"].rstrip(";")};
EXPLAIN (ANALYZE, VERBOSE, BUFFERS, FORMAT JSON) EXECUTE vector_ci('{vector}');
"""
        result = subprocess.run(
            [
                "psql",
                os.environ["PARADEDB_URL"],
                "-XqAt",
                "-v",
                "ON_ERROR_STOP=1",
            ],
            input=sql,
            capture_output=True,
            text=True,
            check=True,
        )
        path = output / f"{point['name']}.json"
        path.write_text(result.stdout)
        scans = verify(
            json.loads(result.stdout),
            point["filtered"],
            point["neighbors"],
            point.get("engine", "pg_search"),
            point.get("index_name"),
        )
        if args.quantized:
            records[point["name"]] = routing_metrics(scans, require_routing=True)
        print(f"Verified {point['name']}: {path}")
    if args.quantized:
        (args.directory / "router-metrics.json").write_text(
            json.dumps(records, indent=2) + "\n"
        )
        report = routing_summary(records)
        (args.directory / "router-metrics.md").write_text(report)
        if os.environ.get("GITHUB_STEP_SUMMARY"):
            with Path(os.environ["GITHUB_STEP_SUMMARY"]).open(
                "a", encoding="utf-8"
            ) as step_summary:
                step_summary.write("\n" + report)


if __name__ == "__main__":
    main()

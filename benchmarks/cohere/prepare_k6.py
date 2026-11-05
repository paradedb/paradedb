"""Export pdbench recall selections for Benchmarker k6 and CI comments."""

import argparse
import json
from pathlib import Path

QUERIES = ("knn_top10_unfiltered", "knn_top10_10pct", "knn_top10_1pct")
TARGETS = (0.9, 0.95, 0.99)
PROBE = "paradedb.vector_cluster_max_probe"
EFFORT = "paradedb.vector_search_effort"
QVEC = "current_setting('cohere.qvec')"


def sweep_settings(report):
    """Normalize query-specific settings while preserving one shared effort sweep."""
    current = json.loads(json.dumps(report["config"]))
    current["search"].pop("query", None)
    param = report["data"]["sweep_param"]
    session = dict(current["search"]["session"])
    targets = current["search"]["sweep"].get("targets", TARGETS)
    if list(targets) != list(TARGETS):
        raise ValueError("Recall sweeps must include all three recall targets")
    if param == EFFORT and float(session.get(PROBE, -1)) != 0:
        raise ValueError("Adaptive sweeps require automatic max_probe=0")
    return current, session, targets, param


def operating_points(reports):
    """Validate and export pdbench selections without choosing new probes."""
    points = []
    seen = set()
    config = None
    for report in reports:
        current, session, targets, param = sweep_settings(report)
        if config is not None and config != current:
            raise ValueError(
                "Recall sweeps must use the same dataset, index, and session settings"
            )
        config = current
        if report["kind"] != "search" or param not in (PROBE, EFFORT):
            raise ValueError(
                "Expected a pdbench probe or adaptive effort search report"
            )
        if (
            report["data"]["query_vectors"] != 100
            or not 0 < current["search"]["sweep"]["tolerance"] < 0.001
        ):
            raise ValueError(
                "CI requires all 100 query vectors and strict recall targets"
            )
        for query, result in report["data"]["variants"].items():
            if query not in QUERIES:
                raise ValueError(f"Unexpected query: {query}")
            if result["query"].count(QVEC) != 1:
                raise ValueError(f"{query}: expected exactly one query-vector operand")
            measured = {point["value"]: point["metrics"] for point in result["points"]}
            for target in targets:
                if (query, target) in seen:
                    raise ValueError(f"Unexpected or duplicate query: {query}@{target}")
                seen.add((query, target))
                selected = result["targets"].get(str(target))
                metrics = measured[selected["value"]] if selected else None
                if selected and (
                    not 0 < float(selected["value"]) <= (10 if param == EFFORT else 1)
                    or not target - 1e-12 <= selected["recall"] <= 1
                    or metrics["recall"]["recall"] != selected["recall"]
                ):
                    raise ValueError(f"{query}: invalid selected point for {target}")
                points.append(
                    {
                        "name": f"{query}@r{round(target * 100)}",
                        "query": query,
                        "target": target,
                        "reached": selected is not None,
                        "probe": selected["value"] if selected else None,
                        "param": param,
                        "recall": selected["recall"]
                        if selected
                        else max(
                            point["recall"]["recall"] for point in measured.values()
                        ),
                        "sql": result["query"].replace(QVEC, "$1"),
                        "session": session,
                        "buffers": metrics["buffers"] if metrics else None,
                    }
                )
    if seen != {(query, target) for query in QUERIES for target in TARGETS}:
        raise ValueError(
            "Missing recall sweeps: expected every filter and recall target"
        )
    return sorted(points, key=lambda point: point["name"])


def summary(points):
    """Render recall and buffer diagnostics for the shared publisher."""
    parameter = "search effort" if points[0]["param"] == EFFORT else "max_probe"
    lines = [
        (
            f"| Filter / recall target | {parameter} | max_probe setting | Achieved recall@10 "
            "| Shared hits/query | Shared reads/query |"
        ),
        "|---|---:|---:|---:|---:|---:|",
    ]
    for point in points:
        buffers = point["buffers"]
        hits = (
            f"{buffers['shared_hit_mean']:.1f}"
            if buffers and buffers["shared_hit_mean"] is not None
            else "—"
        )
        reads = (
            f"{buffers['shared_read_mean']:.1f}"
            if buffers and buffers["shared_read_mean"] is not None
            else "—"
        )
        probe = f"`{point['probe']}`" if point["reached"] else "⚠️ Not reached"
        budget = point["session"].get(PROBE, "—")
        lines.append(
            f"| `{point['name']}` | {probe} | {budget} | {point['recall']:.1%} | {hits} | {reads} |"
        )
    lines.append(
        f"\npdbench selects {parameter} separately for each filter and recall target "
        "on this index. Adaptive points use only search effort with automatic max_probe. "
        "Buffer counts are means from its EXPLAIN pass. k6 measures p50/p95/p99 and QPS at the "
        "selected settings with 1 VU, after 10 seconds of warmup, for 50 seconds per point. "
        "Unreached targets have no latency or throughput results.\n"
    )
    return "\n".join(lines)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    selected_points = operating_points(
        json.loads(path.read_text())
        for path in sorted(args.runs.rglob("*_search.json"))
    )
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / "operating_points.json").write_text(
        json.dumps(selected_points, indent=2) + "\n"
    )
    (args.output / "operating_points.md").write_text(summary(selected_points))

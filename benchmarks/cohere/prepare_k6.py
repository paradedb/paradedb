"""Export pdbench recall selections for Benchmarker k6 and CI comments."""

import argparse
import json
from pathlib import Path

QUERIES = ("knn_top10_unfiltered", "knn_top10_10pct", "knn_top10_1pct")
TARGETS = (0.9, 0.95, 0.99)
RECALL_ROUNDOFF = 1e-12
PROBE = "paradedb.vector_cluster_max_probe"
QVEC = "current_setting('cohere.qvec')"


def operating_points(reports):
    """Validate and export pdbench selections without choosing new probes."""
    points = []
    seen = set()
    config = None
    for report in reports:
        current = json.loads(json.dumps(report["config"]))
        current["search"].pop("query", None)
        if config is not None and config != current:
            raise ValueError(
                "Recall sweeps must use the same dataset, index, and session settings"
            )
        config = current
        if report["kind"] != "search" or report["data"]["sweep_param"] != PROBE:
            raise ValueError("Expected a pdbench max_probe search report")
        if (
            report["data"]["query_vectors"] != 100
            or not 0 < current["search"]["sweep"]["tolerance"] < 0.001
        ):
            raise ValueError(
                "CI requires all 100 query vectors and strict recall targets"
            )
        for query, result in report["data"]["variants"].items():
            if query not in QUERIES or query in seen:
                raise ValueError(f"Unexpected or duplicate query: {query}")
            seen.add(query)
            if result["query"].count(QVEC) != 1:
                raise ValueError(f"{query}: expected exactly one query-vector operand")
            measured = {point["value"]: point["metrics"] for point in result["points"]}
            for target in TARGETS:
                selected = result["targets"].get(str(target))
                metrics = measured[selected["value"]] if selected else None
                if selected and (
                    not 0 < float(selected["value"]) <= 1
                    or not target - RECALL_ROUNDOFF <= selected["recall"] <= 1
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
                        "recall": selected["recall"]
                        if selected
                        else max(
                            point["recall"]["recall"] for point in measured.values()
                        ),
                        "sql": result["query"].replace(QVEC, "$1"),
                        "session": current["search"]["session"],
                        "buffers": metrics["buffers"] if metrics else None,
                    }
                )
    if seen != set(QUERIES):
        raise ValueError(f"Missing recall sweeps: {set(QUERIES) - seen}")
    return sorted(points, key=lambda point: point["name"])


def summary(points):
    """Render recall and buffer diagnostics for the shared publisher."""
    lines = [
        (
            "| Filter / recall target | max_probe | Achieved recall@10 "
            "| Shared hits/query | Shared reads/query |"
        ),
        "|---|---:|---:|---:|---:|",
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
        lines.append(
            f"| `{point['name']}` | {probe} | {point['recall']:.1%} | {hits} | {reads} |"
        )
    lines.append(
        "\npdbench selects max_probe separately for each filter and recall target on this index. "
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

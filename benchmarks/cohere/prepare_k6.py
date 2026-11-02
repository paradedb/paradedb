"""Export pdbench recall selections for Benchmarker k6 and CI comments."""

import argparse
import json
from pathlib import Path

QUERIES = ("knn_top10_unfiltered", "knn_top10_10pct", "knn_top10_1pct")
SUITES = {
    "cohere": dict.fromkeys(QUERIES, 10),
    "msmarco": {"knn_top10_unfiltered": 10, "knn_top100_unfiltered": 100},
}
TARGETS = (0.9, 0.95, 0.99)
PROBE = "paradedb.vector_cluster_max_probe"
EFFORT = "paradedb.vector_search_effort"
NPROBE = "prism.nprobe"


def validated_config(report, dataset, config):
    """Check report identity and shared sweep settings before exporting results."""
    current = json.loads(json.dumps(report["config"]))
    current["search"].pop("query", None)
    param = report["data"]["sweep_param"]
    engine = current.get("engine", "pg_search")
    allowed = {"pg_search": (PROBE, EFFORT), "pg_vectorsearch": (NPROBE,)}
    if list(current["search"]["sweep"].get("targets", TARGETS)) != list(TARGETS):
        raise ValueError("Recall sweeps must include all three recall targets")
    if param == EFFORT and float(current["search"]["session"].get(PROBE, -1)) != 0:
        raise ValueError("Adaptive sweeps require automatic max_probe=0")
    if config is not None and config != current:
        raise ValueError(
            "Recall sweeps must use the same dataset, index, and session settings"
        )
    if (
        current["dataset"]["name"] != dataset
        or report["kind"] != "search"
        or param not in allowed.get(engine, ())
    ):
        raise ValueError(f"Expected {dataset} pdbench probe or effort search report")
    if report["data"]["query_vectors"] != 100:
        raise ValueError(
            "CI requires all 100 selected query vectors and strict recall targets"
        )
    return current


def selected_metrics(selected, measured, sweep, param, target):
    """Validate a reported selection and preserve its measured diagnostics."""
    if selected is None:
        return None
    value = float(selected["value"])
    metrics = measured[selected["value"]]
    upper = sweep.get("max", 10 if param == EFFORT else 1)
    if not 0 < value <= upper or value < sweep.get("min", 0):
        raise ValueError(f"invalid selected point for {target}: outside sweep range")
    if param == NPROBE and not value.is_integer():
        raise ValueError(
            f"invalid selected point for {target}: PRISM requires an integer"
        )
    if (
        not target - 1e-12 <= selected["recall"] <= 1
        or metrics["recall"]["recall"] != selected["recall"]
    ):
        raise ValueError(f"invalid selected point for {target}: recall mismatch")
    return metrics


def operating_points(reports, dataset="cohere"):
    """Validate and export pdbench selections without choosing new probes."""
    points = []
    seen = set()
    config = None
    qvec = f"current_setting('{dataset}.qvec')"
    for report in reports:
        config = validated_config(report, dataset, config)
        param = report["data"]["sweep_param"]
        for query, result in report["data"]["variants"].items():
            if query not in SUITES[dataset] or query in seen:
                raise ValueError(f"Unexpected or duplicate query: {query}")
            seen.add(query)
            neighbors = SUITES[dataset][query]
            if not 0 < config["search"]["sweep"]["tolerance"] < 1 / (100 * neighbors):
                raise ValueError("CI requires strict recall targets")
            if result["query"].count(qvec) != 1:
                raise ValueError(f"{query}: expected exactly one query-vector operand")
            measured = {point["value"]: point["metrics"] for point in result["points"]}
            if any(point["recall"]["k"] != neighbors for point in measured.values()):
                raise ValueError(f"{query}: expected recall@{neighbors}")
            for target in TARGETS:
                selected = result["targets"].get(str(target))
                metrics = selected_metrics(
                    selected, measured, config["search"]["sweep"], param, target
                )
                points.append(
                    {
                        "name": f"{query}@r{round(target * 100)}",
                        "query": query,
                        "neighbors": neighbors,
                        "query_vectors": 100,
                        "dimensions": config["dataset"]["dims"],
                        "filtered": not query.endswith("_unfiltered"),
                        "target": target,
                        "reached": selected is not None,
                        "probe": selected["value"] if selected else None,
                        "param": param,
                        "engine": config.get("engine", "pg_search"),
                        "backend": "pg_vectorsearch"
                        if config.get("engine") == "pg_vectorsearch"
                        else "paradedb",
                        "index_name": f"{config['dataset']['table']}_prism_idx"
                        if config.get("engine") == "pg_vectorsearch"
                        else None,
                        "recall": selected["recall"]
                        if selected
                        else max(
                            point["recall"]["recall"] for point in measured.values()
                        ),
                        "sql": result["query"].replace(qvec, "$1"),
                        "session": config["search"]["session"],
                        "buffers": metrics["buffers"] if metrics else None,
                    }
                )
    if seen != set(SUITES[dataset]):
        raise ValueError(f"Missing recall sweeps: {set(SUITES[dataset]) - seen}")
    return sorted(points, key=lambda point: point["name"])


def summary(points):
    """Render recall and buffer diagnostics for the shared publisher."""
    parameter = {EFFORT: "search effort", PROBE: "max_probe", NPROBE: "nprobe"}[
        points[0]["param"]
    ]
    lines = [
        (
            f"| Workload / recall target | k | {parameter} | Achieved recall@k "
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
        lines.append(
            f"| `{point['name']}` | {point['neighbors']} | {probe} | "
            f"{point['recall']:.1%} | {hits} | {reads} |"
        )
    lines.append(
        f"\npdbench selects {parameter} separately for each workload and recall target "
        "on this index. "
        "Buffer counts are means from its EXPLAIN pass. k6 measures p50/p95/p99 and QPS at the "
        "selected settings with 1 VU, after 10 seconds of warmup, for 50 seconds per point. "
        "Unreached targets have no latency or throughput results.\n"
    )
    return "\n".join(lines)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--dataset", choices=SUITES, default="cohere")
    args = parser.parse_args()
    selected_points = operating_points(
        (
            json.loads(path.read_text())
            for path in sorted(args.runs.rglob("*_search.json"))
        ),
        dataset=args.dataset,
    )
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / "operating_points.json").write_text(
        json.dumps(selected_points, indent=2) + "\n"
    )
    (args.output / "operating_points.md").write_text(summary(selected_points))

"""Export pdbench recall selections for Benchmarker k6 and CI comments."""

import argparse
import json
from pathlib import Path

QUERIES = ("knn_top10_unfiltered", "knn_top10_10pct", "knn_top10_1pct")
HN_FILTER_FIELDS = {
    "range_10pct": ["time"],
    "range_1pct": ["time"],
    "range_0.1pct": ["time"],
    "single_term_0.1pct": ["by"],
    "single_term_0.01pct": ["by"],
    "single_term_0.001pct": ["by"],
}
SUITES = {
    "cohere": dict.fromkeys(QUERIES, 10),
    "msmarco": {"knn_top10_unfiltered": 10, "knn_top100_unfiltered": 100},
    "hn": dict.fromkeys(HN_FILTER_FIELDS, 10),
}
DIMENSIONS = {"cohere": 1024, "msmarco": 1024, "hn": 1536}
TARGETS = (0.9, 0.95, 0.99)
PROBE = "paradedb.vector_cluster_max_probe"
NPROBE = "prism.nprobe"
VCHORD_PROBES = "vchordrq.probes"
ENGINES = {
    "pg_search": (PROBE, "paradedb", None),
    "pg_vectorsearch": (NPROBE, "pg_vectorsearch", "prism"),
    "vectorchord": (VCHORD_PROBES, "postgres", "vchordrq"),
}


def validated_config(report, dataset, config):
    """Validate the engine, dataset, and shared recall settings."""
    current = json.loads(json.dumps(report["config"]))
    current["search"].pop("query", None)
    current["search"].pop("k", None)
    if config is not None and config != current:
        raise ValueError(
            "Recall sweeps must use the same dataset, index, and session settings"
        )
    param = ENGINES.get(current.get("engine", "pg_search"), (None, None, None))[0]
    if (
        report["kind"] != "search"
        or param is None
        or report["data"]["sweep_param"] != param
        or current["dataset"]["name"] != dataset
        or current["dataset"]["dims"] != DIMENSIONS[dataset]
    ):
        raise ValueError(
            "Expected a pdbench probe search report with "
            f"{DIMENSIONS[dataset]}-dimensional query vectors"
        )
    if current.get("engine") == "vectorchord" and current["search"]["session"].get(
        "vchordrq.prefilter"
    ) not in ("on", "off"):
        raise ValueError(
            "VectorChord requires an explicit prefilter setting: on or off"
        )
    if report["data"]["query_vectors"] != 100:
        raise ValueError("CI requires all 100 query vectors and strict recall targets")
    return current


def selected_metrics(selected, measured, sweep, param, target):
    """Preserve valid recall selections without changing the chosen probes."""
    if selected is None:
        return None
    value = float(selected["value"])
    if not sweep.get("min", 1) <= value <= sweep.get("max", 100_000):
        raise ValueError(f"invalid selected point for {target}: outside sweep range")
    if param in (NPROBE, VCHORD_PROBES) and not value.is_integer():
        raise ValueError(
            f"invalid selected point for {target}: {param} requires an integer"
        )
    metrics = measured[selected["value"]]
    if (
        not target - 1e-12 <= selected["recall"] <= 1
        or metrics["recall"]["recall"] != selected["recall"]
    ):
        raise ValueError(f"invalid selected point for {target}: recall mismatch")
    return metrics


def operating_points(reports, dataset="cohere"):
    """Validate and export pdbench selections without choosing new probes."""
    # pylint: disable=too-many-locals
    points = []
    seen = set()
    config = None
    qvec = f"current_setting('{dataset}.qvec')"
    for report in reports:
        config = validated_config(report, dataset, config)
        param = report["data"]["sweep_param"]
        engine = config.get("engine", "pg_search")
        _, backend, index_suffix = ENGINES[engine]
        for query, result in report["data"]["variants"].items():
            if query not in SUITES[dataset] or query in seen:
                raise ValueError(f"Unexpected or duplicate query: {query}")
            seen.add(query)
            neighbors = SUITES[dataset][query]
            if not 0 < config["search"]["sweep"]["tolerance"] < 1 / (100 * neighbors):
                raise ValueError("CI requires strict recall targets")
            if result["query"].count(qvec) != 1:
                raise ValueError(f"{query}: expected exactly one query-vector operand")
            if report["data"].get("fast_search") and any(
                point.get("query_vectors") != 100
                or point["metrics"]["recall"].get("samples") != 100
                for point in result["points"]
            ):
                raise ValueError(
                    f"{query}: fast search requires full 100-vector validation"
                )
            measured = {point["value"]: point["metrics"] for point in result["points"]}
            if report["config"]["search"].get("k", 10) != neighbors or any(
                point["recall"]["k"] != neighbors for point in measured.values()
            ):
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
                        "require_full_neighbors": not (
                            dataset == "hn"
                            and engine in ("pg_vectorsearch", "vectorchord")
                        ),
                        "dimensions": DIMENSIONS[dataset],
                        "filter_fields": HN_FILTER_FIELDS[query]
                        if dataset == "hn"
                        else ([] if query.endswith("_unfiltered") else ["text"]),
                        "target": target,
                        "reached": selected is not None,
                        "status": result.get("target_status", {})
                        .get(str(target), {})
                        .get("status", "reached" if selected else "unreachable"),
                        "reason": result.get("target_status", {})
                        .get(str(target), {})
                        .get("reason"),
                        "probe": selected["value"] if selected else None,
                        "param": param,
                        "engine": engine,
                        "backend": backend,
                        "index_name": f"{config['dataset']['table']}_{index_suffix}_idx"
                        if index_suffix
                        else None,
                        "recall": selected["recall"]
                        if selected
                        else max(
                            (point["recall"]["recall"] for point in measured.values()),
                            default=None,
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
    parameter = points[0]["param"].rsplit(".", 1)[-1].removeprefix("vector_cluster_")
    lines = [
        # GitHub renders Markdown inside <details> only with a blank line after the summary.
        "<details><summary>Operating points</summary>",
        "",
        (
            f"| Workload / recall target | k | {parameter} | Achieved recall@k "
            "| Shared hits/query | Shared reads/query | EXPLAIN queries |"
        ),
        "|---|---:|---:|---:|---:|---:|---:|",
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
        status = {
            "budget_exceeded": "⚠️ Time budget exceeded",
            "probe_budget_exceeded": "⚠️ Probe budget exceeded",
            "unreachable": "⚠️ Not reached",
        }.get(point["status"], point["status"])
        probe = f"`{point['probe']}`" if point["reached"] else status
        recall = f"{point['recall']:.1%}" if point["recall"] is not None else "—"
        samples = buffers.get("queries_measured", 0) if buffers else 0
        lines.append(
            f"| `{point['name']}` | {point['neighbors']} | {probe} | "
            f"{recall} | {hits} | {reads} | {samples} |"
        )
    lines.append(
        f"\npdbench selects {parameter} separately for each workload and recall target. "
        "Every selected point is validated on all 100 query vectors. "
        "Buffer counts are means over the listed EXPLAIN sample count; "
        "zero means diagnostics were not collected. "
        "Time/probe budget exhaustion does not establish that a target is unreachable. "
        "k6 measures p50/p95/p99 and QPS at the "
        "selected settings with 1 VU, after 10 seconds of warmup, for 50 seconds per point. "
        "Unreached targets have no latency or throughput results.\n"
    )
    if any(point.get("require_full_neighbors") is False for point in points):
        lines.append(
            f"HN {points[0]['engine']} reports short result counts without failing k6. "
            "Query errors still fail the run; the 100-vector recall gate remains strict.\n"
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

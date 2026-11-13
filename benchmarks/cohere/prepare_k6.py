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


def operating_points(reports, dataset="cohere"):
    """Validate and export pdbench selections without choosing new probes."""
    points = []
    seen = set()
    config = None
    qvec = f"current_setting('{dataset}.qvec')"
    for report in reports:
        current = json.loads(json.dumps(report["config"]))
        current["search"].pop("query", None)
        current["search"].pop("k", None)
        if config is not None and config != current:
            raise ValueError(
                "Recall sweeps must use the same dataset, index, and session settings"
            )
        config = current
        if (
            report["kind"] != "search"
            or report["data"]["sweep_param"] != PROBE
            or current["dataset"]["name"] != dataset
            or current["dataset"]["dims"] != DIMENSIONS[dataset]
        ):
            raise ValueError(
                "Expected a pdbench max_probe search report with "
                f"{DIMENSIONS[dataset]}-dimensional query vectors"
            )
        if report["data"]["query_vectors"] != 100:
            raise ValueError(
                "CI requires all 100 query vectors and strict recall targets"
            )
        for query, result in report["data"]["variants"].items():
            if query not in SUITES[dataset] or query in seen:
                raise ValueError(f"Unexpected or duplicate query: {query}")
            seen.add(query)
            neighbors = SUITES[dataset][query]
            if not 0 < current["search"]["sweep"]["tolerance"] < 1 / (100 * neighbors):
                raise ValueError("CI requires strict recall targets")
            if result["query"].count(qvec) != 1:
                raise ValueError(f"{query}: expected exactly one query-vector operand")
            measured = {point["value"]: point["metrics"] for point in result["points"]}
            if report["config"]["search"].get("k", 10) != neighbors or any(
                point["recall"]["k"] != neighbors for point in measured.values()
            ):
                raise ValueError(f"{query}: expected recall@{neighbors}")
            for target in TARGETS:
                selected = result["targets"].get(str(target))
                metrics = measured[selected["value"]] if selected else None
                if selected and (
                    not 1 <= float(selected["value"]) <= 100_000
                    or not target - 1e-12 <= selected["recall"] <= 1
                    or metrics["recall"]["recall"] != selected["recall"]
                ):
                    raise ValueError(f"{query}: invalid selected point for {target}")
                points.append(
                    {
                        "name": f"{query}@r{round(target * 100)}",
                        "query": query,
                        "neighbors": neighbors,
                        "dimensions": DIMENSIONS[dataset],
                        "filter_fields": HN_FILTER_FIELDS[query]
                        if dataset == "hn"
                        else ([] if query.endswith("_unfiltered") else ["text"]),
                        "target": target,
                        "reached": selected is not None,
                        "probe": selected["value"] if selected else None,
                        "recall": selected["recall"]
                        if selected
                        else max(
                            point["recall"]["recall"] for point in measured.values()
                        ),
                        "sql": result["query"].replace(qvec, "$1"),
                        "session": current["search"]["session"],
                        "buffers": metrics["buffers"] if metrics else None,
                    }
                )
    if seen != set(SUITES[dataset]):
        raise ValueError(f"Missing recall sweeps: {set(SUITES[dataset]) - seen}")
    return sorted(points, key=lambda point: point["name"])


def fixed_measurements(reports, probes):
    """Require consistent full-query measurements at the configured HN probes."""
    measured = {}
    config = None
    for report in reports:
        current = json.loads(json.dumps(report["config"]))
        current["search"].pop("query", None)
        current["search"].pop("k", None)
        if config is not None and current != config:
            raise ValueError(
                "Fixed probes must use the same dataset, index, and session settings"
            )
        config = current
        if (
            report["kind"] != "search"
            or (
                current.get("engine"),
                current["dataset"]["name"],
                current["dataset"]["dims"],
            )
            != ("pg_vectorsearch", "hn", DIMENSIONS["hn"])
            or report["data"]["sweep_param"] != "prism.nprobe"
            or report["data"]["query_vectors"] != 100
            or report["data"].get("fixed_probe") is None
        ):
            raise ValueError("Expected HN PRISM measurements on all 100 query vectors")
        for query, result in report["data"]["variants"].items():
            if (
                query not in probes
                or result["query"].count("current_setting('hn.qvec')") != 1
            ):
                raise ValueError(f"Unexpected HN query: {query}")
            for point in result["points"]:
                key = (query, float(point["value"]))
                if key in measured or key[1] not in probes[query].values():
                    raise ValueError(f"Unexpected or duplicate fixed probe: {key}")
                recall = point["metrics"]["recall"]
                if (
                    recall["k"] != 10
                    or recall["samples"] != 100
                    or not 0 <= recall["recall"] <= 1
                ):
                    raise ValueError("Expected recall@10 over all 100 query vectors")
                measured[key] = (result["query"], point["metrics"])
    return config, measured


def fixed_operating_points(reports, probes):
    """Export HN PRISM measurements at pinned probes, including missed targets."""
    if set(probes) != set(HN_FILTER_FIELDS) or any(
        set(targets) != {str(target) for target in TARGETS}
        or any(
            not isinstance(value, int)
            or isinstance(value, bool)
            or not 1 <= value <= 4096
            for value in targets.values()
        )
        for targets in probes.values()
    ):
        raise ValueError(
            "Expected all HN targets with integer nprobe values in [1, 4096]"
        )
    config, measured = fixed_measurements(reports, probes)
    points = []
    for query, targets in probes.items():
        for target, probe in targets.items():
            if (query, probe) not in measured:
                raise ValueError(f"Missing fixed probe: {query} at {probe}")
            sql, metrics = measured[(query, probe)]
            recall = metrics["recall"]["recall"]
            points.append(
                {
                    "name": f"{query}@r{round(float(target) * 100)}-prism-postfilter",
                    "query": query,
                    "neighbors": 10,
                    "dimensions": DIMENSIONS["hn"],
                    "filter_fields": HN_FILTER_FIELDS[query],
                    "target": float(target),
                    "reached": recall + 1e-12 >= float(target),
                    "fixed_probe": True,
                    "probe": probe,
                    "param": "prism.nprobe",
                    "engine": "pg_vectorsearch",
                    "backend": "pg_vectorsearch",
                    "index_name": "hn_items_prism_idx",
                    "require_full_neighbors": False,
                    "recall": recall,
                    "sql": sql.replace("current_setting('hn.qvec')", "$1"),
                    "session": config["search"]["session"],
                    "buffers": metrics["buffers"],
                }
            )
    return sorted(points, key=lambda point: point["name"])


def summary(points):
    """Render recall and buffer diagnostics for the shared publisher, collapsed under a summary
    line so the PR comment leads with regressions."""
    fixed = any(point.get("fixed_probe") for point in points)
    parameter = "nprobe" if fixed else "max_probe"
    lines = [
        # GitHub renders Markdown inside <details> only with a blank line after the summary.
        "<details><summary>Operating points</summary>",
        "",
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
        if fixed:
            probe = f"`{point['probe']}`" + (
                " (target missed)" if not point["reached"] else ""
            )
        lines.append(
            f"| `{point['name']}` | {point['neighbors']} | {probe} | "
            f"{point['recall']:.1%} | {hits} | {reads} |"
        )
    if fixed:
        lines.append(
            "\nFixed nprobe values come from the reference sweep; missed targets use 4096. "
            "Recall and buffers are measured again on this run without searching for new probes. "
            "All points, including missed targets, run with 1 VU, 10 seconds of warmup, and "
            "50 seconds of measurement. Short result counts are reported; "
            "query errors fail the run.\n"
        )
    else:
        lines.append(
            "\npdbench selects max_probe separately for each workload and recall target "
            "on this index. "
            "Buffer counts are means from its EXPLAIN pass. k6 measures p50/p95/p99 and QPS at the "
            "selected settings with 1 VU, after 10 seconds of warmup, for 50 seconds per point. "
            "Unreached targets have no latency or throughput results.\n"
        )
    lines.append("</details>\n")
    return "\n".join(lines)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--dataset", choices=SUITES, default="cohere")
    parser.add_argument("--fixed-probes", type=Path)
    args = parser.parse_args()
    input_reports = (
        json.loads(path.read_text())
        for path in sorted(args.runs.rglob("*_search.json"))
    )
    if args.fixed_probes:
        if args.dataset != "hn":
            parser.error("--fixed-probes requires --dataset hn")
        selected_points = fixed_operating_points(
            input_reports, json.loads(args.fixed_probes.read_text())["nprobe"]
        )
    else:
        selected_points = operating_points(input_reports, dataset=args.dataset)
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / "operating_points.json").write_text(
        json.dumps(selected_points, indent=2) + "\n"
    )
    (args.output / "operating_points.md").write_text(summary(selected_points))

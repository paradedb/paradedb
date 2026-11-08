"""Compare four-copy MS MARCO selections with the pinned two-copy SOAR run."""

import argparse
import hashlib
import json
import math
from pathlib import Path

import tomllib

BASELINE_SHA = "3105167ae0c5dcd514a558f535286578a5416a32"
BASELINE_URL = "https://github.com/paradedb/paradedb/actions/runs/42477854482"
QUERY_SHA = "cda4591e197108fcc6778352a2de649811f0aac3a30e8edafecf76d38101cc6d"


def k6_metrics(path):
    """Use the shared publisher's floor quantiles and observed measurement window."""
    if not path.exists():
        return ["missing"] * 4
    runs = json.loads(path.read_text())["runs"]
    if len(runs) != 1:
        raise ValueError("Expected one k6 backend")
    queries = next(iter(runs.values()))["queries"].values()
    samples = []
    for query in queries:
        latencies, timestamps = query["latencies"], query["timestamps"]
        if len(latencies) != len(timestamps):
            raise ValueError("Missing k6 timestamps")
        samples.extend(zip(latencies, timestamps))
    if not samples or any(
        not math.isfinite(latency) or latency < 0 or not math.isfinite(timestamp)
        for latency, timestamp in samples
    ):
        raise ValueError("Invalid k6 samples")
    seconds = (
        max(timestamp for _, timestamp in samples)
        - min(timestamp - latency for latency, timestamp in samples)
    ) / 1000
    if seconds <= 0:
        raise ValueError("Non-positive k6 measurement window")
    latencies = sorted(latency for latency, _ in samples)
    return [
        *(
            f"{latencies[math.floor((len(latencies) - 1) * p)]:.3f}"
            for p in (0.5, 0.95, 0.99)
        ),
        f"{len(samples) / seconds:.3f}",
    ]


def build_comparison(roots):
    """Display the same observed build metrics as the shared publisher."""
    metrics = [
        {metric["name"]: metric for metric in json.loads(path.read_text())}
        if (path := root / "index-metrics.json").exists()
        else {}
        for root in roots
    ]
    lines = ["| Build metric | Unit | SOAR 2x | SOAR 4x |", "|---|---|---:|---:|"]
    for name in sorted(metrics[0].keys() | metrics[1].keys()):
        values = [metric.get(name) for metric in metrics]
        units = {value["unit"] for value in values if value is not None}
        if len(units) != 1:
            raise ValueError(f"Build metric units differ: {name}")
        row = [name, units.pop()]
        row += [f"{value['value']:.3f}" if value else "missing" for value in values]
        lines.append("| " + " | ".join(row) + " |")
    return lines


def verify_replicas(roots):
    """Require the measured indexes to contain two and four copies respectively."""
    for root, replicas in zip(roots, (2, 4)):
        info = json.loads((root / "vector-info.json").read_text())
        if not info or any(
            segment["vector_total_rows"] != replicas * segment["vector_num_vectors"]
            for segment in info
        ):
            raise ValueError(f"Expected {replicas} postings per vector")


def comparison(baseline, current):
    """Retain unreached targets and distinguish EXPLAIN means from k6 percentiles."""
    roots = [baseline / "benchmark-artifacts", current / "benchmark-artifacts"]
    if (roots[0] / "extension-commit.txt").read_text().strip() != BASELINE_SHA:
        raise ValueError("Unexpected baseline revision")
    configs = [tomllib.loads((root / "config.toml").read_text()) for root in roots]
    if not configs[0]["index"].get("spill") or configs[0] != configs[1]:
        raise ValueError("Expected identical spill=true benchmark configuration")
    verify_replicas(roots)
    for root in roots:
        if (
            hashlib.sha256((root / "query_vectors.json").read_bytes()).hexdigest()
            != QUERY_SHA
        ):
            raise ValueError("Query-set hash differs from the reference")
    points = [
        {
            point["name"]: point
            for point in json.loads((root / "operating_points.json").read_text())
        }
        for root in roots
    ]
    if set(points[0]) != set(points[1]) or len(points[0]) != 6:
        raise ValueError("Expected all six recall targets in both runs")
    lines = [
        "### Comparison: SOAR 2x versus 4x",
        "",
        (
            f"Baseline: [{BASELINE_SHA[:12]}]({BASELINE_URL}). "
            "Each run selects its own absolute cluster-work budget at each recall target."
        ),
        "",
        *build_comparison(roots),
        "",
        (
            "| Point | Run | Recall | Max probe | Shared hits | Shared reads | "
            "p50 ms | p95 ms | p99 ms | QPS |"
        ),
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for name in sorted(points[0]):
        for label, mapping, root in zip(
            ("SOAR 2x", "SOAR 4x"), points, (baseline, current)
        ):
            point = mapping[name]
            if point["reached"]:
                buffers = point["buffers"] or {}
                values = [
                    f"{buffers[key]:.1f}" if buffers.get(key) is not None else "missing"
                    for key in ("shared_hit_mean", "shared_read_mean")
                ]
                values += k6_metrics(root / "benchmarker/results" / f"{name}.json")
                row = [
                    name,
                    label,
                    f"{point['recall'] * 100:.2f}%",
                    str(point["probe"]),
                    *values,
                ]
            else:
                row = [
                    name,
                    label,
                    f"{point['recall'] * 100:.2f}%",
                    "unreached",
                    *(["—"] * 6),
                ]
            lines.append("| " + " | ".join(row) + " |")
    lines += [
        "",
        (
            "Buffer counts are means from the separate EXPLAIN pass. Latencies are k6 percentiles; "
            "QPS uses the observed timed window. Unreached points have no k6 measurement. "
            "Build workers are observed PostgreSQL parallel workers; summed RSS includes shared "
            "mappings. Both runs use V6 exact-row storage. "
            "Separate hosts, index builds, "
            "and selected budgets limit causal attribution."
        ),
        "",
    ]
    return "\n".join(lines)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for option in ("baseline", "current", "output"):
        parser.add_argument(f"--{option}", type=Path, required=True)
    args = parser.parse_args()
    args.output.write_text(comparison(args.baseline, args.current))

"""Calibrate one admission knob shared across filters at each recall goal."""

import json
import os
import subprocess
from pathlib import Path

from prepare_k6 import (
    EXPONENT,
    PROBE,
    QUERIES,
    QVEC,
    RECALL_TARGET,
    TARGETS,
    operating_points,
)

SHARED_EXPONENT = 1.25
AUTO_CEILING = 0.0


def fixed_point(report, control, exponent, profile):
    """Export one measured setting without searching for a replacement target."""
    query = control["query"]
    variant = report["data"]["variants"][query]
    (measured,) = variant["points"]
    if report["data"]["query_vectors"] != 100:
        raise ValueError("Admission experiments require all 100 queries")
    if float(measured["value"]) != control["target"]:
        raise ValueError("The measured setting must equal the requested recall target")
    metrics = measured["metrics"]
    recall = metrics["recall"]["recall"]
    session = report["config"]["search"]["session"]
    if float(session[EXPONENT]) != exponent:
        raise ValueError("Unexpected admission calibration")
    return {
        "name": control["name"],
        "query": query,
        "target": control["target"],
        "reached": recall >= control["target"] - 1e-12,
        "probe": control["target"],
        "param": RECALL_TARGET,
        "recall": recall,
        "sql": variant["query"].replace(QVEC, "$1"),
        "session": session,
        "buffers": metrics["buffers"],
        "metrics": metrics,
        "exponent": exponent,
        "profile": profile,
    }


def choose_calibration(points):
    """Select one measured knob per recall goal; never select per filter."""
    groups = {}
    for point in points:
        groups.setdefault(point["probe"], []).append(point)
    if not groups:
        raise ValueError("No admission measurements")
    for group in groups.values():
        if (
            len(group) != len(QUERIES)
            or {p["query"] for p in group} != set(QUERIES)
            or {p["exponent"] for p in group} != {SHARED_EXPONENT}
            or {float(p["session"][PROBE]) for p in group} != {AUTO_CEILING}
        ):
            raise ValueError(
                "Every knob must cover all filters with the shared exponent and ceiling"
            )
    selected = []
    for target in TARGETS:
        passing = [
            knob
            for knob, group in groups.items()
            if all(p["recall"] >= target - 1e-12 for p in group)
        ]
        knob = min(passing) if passing else max(groups)
        for point in groups[knob]:
            selected.append(
                point
                | {
                    "name": f"{point['query']}@r{round(target * 100)}",
                    "target": target,
                    "reached": point["recall"] >= target - 1e-12,
                }
            )
    return sorted(selected, key=lambda point: point["name"])


def summary(points, selected):
    """Separate empirical recall goals from configured knob values."""
    lines = [
        f"Admission experiment: exponent **{SHARED_EXPONENT:g}**, automatic ceiling **0**.",
        "",
        (
            "Only the recall-target knob is calibrated. Each selected value is shared "
            "by all three filters at that recall goal. The knob is an empirical control, "
            "not a guarantee of measured recall. "
            "The separately calibrated nonadaptive controls are retained in fixed-budget/. "
            "Calibration and evaluation use the same 100 queries."
        ),
        "",
        "| Query / recall goal | Configured knob | Recall | Reached | pdbench p50 ms |",
        "|---|---:|---:|---|---:|",
    ]
    for point in selected:
        lines.append(
            f"| {point['name']} | {point['probe']:g} | {point['recall']:.2%} | "
            f"{'yes' if point['reached'] else 'NO'} | {point['metrics']['latency']['p50_ms']:.3f} |"
        )
    lines.extend(
        [
            "",
            "All measured settings, including misses:",
            "",
            "| Query | Configured knob | Recall | pdbench p50 ms |",
            "|---|---:|---:|---:|",
        ]
    )
    for point in sorted(points, key=lambda p: (p["probe"], p["query"])):
        lines.append(
            f"| {point['query']} | {point['probe']:g} | {point['recall']:.2%} | "
            f"{point['metrics']['latency']['p50_ms']:.3f} |"
        )
    return "\n".join(lines) + "\n"


def main():
    """Run controls, then bisect one shared knob on a 0.001 grid."""
    root = Path("benchmark-artifacts")
    root.mkdir(exist_ok=True)
    common = [
        "pdbench",
        "search",
        "--url",
        os.environ["PARADEDB_URL"],
        "--config",
        f"benchmarks/cohere/{os.environ['COHERE_SIZE']}.toml",
        "--assets-dir",
        "benchmarks/cohere/assets",
        "--no-compare",
        "--set",
        "search.runs=3",
    ]
    baseline = root / "fixed-budget"
    for query in QUERIES:
        subprocess.run(
            common
            + [
                "--query",
                query,
                "--runs-dir",
                str(baseline),
                "--set",
                "search.session.paradedb.vector_adaptive_probing=off",
                "--set",
                f"search.sweep.param={PROBE}",
                "--set",
                "search.sweep.min=0.0001",
                "--set",
                "search.sweep.max=1.0",
                "--set",
                "search.sweep.start=0.005",
            ],
            check=True,
        )
    controls = operating_points(
        json.loads(p.read_text()) for p in sorted(baseline.rglob("*_search.json"))
    )
    if not all(p["reached"] for p in controls):
        raise ValueError("The nonadaptive control did not reach every target")
    (baseline / "operating_points.json").write_text(
        json.dumps(controls, indent=2) + "\n"
    )
    points = []

    def measure(query, knob):
        control = {"query": query, "target": knob, "name": f"{query}@knob{knob:g}"}
        output = root / "admission" / control["name"]
        subprocess.run(
            common
            + [
                "--query",
                control["query"],
                "--runs-dir",
                str(output),
                "--no-prewarm",
                "--set",
                "search.session.paradedb.vector_adaptive_probing=on",
                "--probe",
                str(control["target"]),
                "--set",
                f"search.sweep.param={RECALL_TARGET}",
                "--set",
                "search.sweep.min=0.01",
                "--set",
                "search.sweep.max=1.0",
                "--set",
                "search.sweep.start=0.95",
                "--set",
                f"search.session.{PROBE}={AUTO_CEILING}",
                "--set",
                f"search.session.{EXPONENT}={SHARED_EXPONENT}",
            ],
            check=True,
        )
        (report_path,) = output.rglob("*_search.json")
        point = fixed_point(
            json.loads(report_path.read_text()), control, SHARED_EXPONENT, "automatic"
        )
        points.append(point)
        (root / "admission_points.json").write_text(json.dumps(points, indent=2) + "\n")
        print(
            f"{point['name']}: recall={point['recall']:.4f}",
            flush=True,
        )
        return point

    measured = {}
    for target in TARGETS:
        low, high = 10, 1000
        while True:
            midpoint = (low + high) // 2
            if midpoint not in measured:
                measured[midpoint] = [
                    measure(query, midpoint / 1000) for query in QUERIES
                ]
            if low == high:
                break
            if all(p["recall"] >= target - 1e-12 for p in measured[midpoint]):
                high = midpoint
            else:
                low = midpoint + 1
    selected = choose_calibration(points)
    (root / "operating_points.json").write_text(json.dumps(selected, indent=2) + "\n")
    (root / "operating_points.md").write_text(summary(points, selected))


if __name__ == "__main__":
    main()

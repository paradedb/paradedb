"""Compare one admission calibration across all Cohere operating points."""

import json
import os
import subprocess
from pathlib import Path

from prepare_k6 import EXPONENT, PROBE, QUERIES, QVEC, RECALL_TARGET, operating_points

EXPONENTS = (0.25, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0)
SHARED_BUDGET = 0.125


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
    """Select one exponent for the entire matrix; never select per filter."""
    groups = {}
    for point in points:
        groups.setdefault(point["exponent"], []).append(point)
    expected = {(query, target) for query in QUERIES for target in (0.9, 0.95, 0.99)}
    for group in groups.values():
        if (
            len(group) != len(expected)
            or {(p["query"], p["target"]) for p in group} != expected
        ):
            raise ValueError(
                "Every calibration must cover the complete operating-point matrix"
            )

    def rank(exponent):
        group = groups[exponent]
        deficits = [max(0, p["target"] - p["recall"]) for p in group]
        return (max(deficits), sum(deficits), exponent)

    return min(groups, key=rank)


def summary(points, selected):
    """Report the full calibration matrix, including missed targets."""
    lines = [
        f"Admission experiment: shared exponent **{selected:g}** across all filters and targets.",
        "",
        (
            "Matched-budget runs use the budgets selected with adaptive probing off. "
            f"Shared-budget runs use {SHARED_BUDGET} everywhere with that same exponent. "
            "Matched-budget results alone do not establish a one-knob interface. "
            "Recall is measured at the requested target, without per-case recalibration."
        ),
        "",
        "| Profile | Exponent | Query / target | Budget | Recall | Reached | pdbench p50 ms |",
        "|---|---:|---|---:|---:|---|---:|",
    ]
    for point in points:
        lines.append(
            f"| {point['profile']} | {point['exponent']:g} | {point['name']} | "
            f"{point['session'][PROBE]} | {point['recall']:.2%} | "
            f"{'yes' if point['reached'] else 'NO'} | {point['metrics']['latency']['p50_ms']:.3f} |"
        )
    return "\n".join(lines) + "\n"


def main():
    """Run controls, calibrate once, then evaluate a shared probe ceiling."""
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

    def measure(control, exponent, profile, budget):
        output = root / "admission" / profile / str(exponent) / control["name"]
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
                f"search.session.{PROBE}={budget}",
                "--set",
                f"search.session.{EXPONENT}={exponent}",
            ],
            check=True,
        )
        (report_path,) = output.rglob("*_search.json")
        point = fixed_point(
            json.loads(report_path.read_text()), control, exponent, profile
        )
        points.append(point)
        (root / "admission_points.json").write_text(json.dumps(points, indent=2) + "\n")
        print(
            f"{profile} exponent={exponent} {point['name']}: recall={point['recall']:.4f}",
            flush=True,
        )

    # Interleave calibrations across operating points to avoid one arm always running first.
    for index, control in enumerate(controls):
        exponents = (
            EXPONENTS[index % len(EXPONENTS) :] + EXPONENTS[: index % len(EXPONENTS)]
        )
        for exponent in exponents:
            measure(control, exponent, "matched-budget", control["probe"])
    selected = choose_calibration(points)
    for control in controls:
        measure(control, selected, "shared-budget", SHARED_BUDGET)
    exported = []
    for point in points:
        if point["exponent"] != selected:
            continue
        point = dict(point)
        if point["profile"] == "shared-budget":
            point["name"] = "shared-budget_" + point["name"]
        exported.append(point)
    (root / "operating_points.json").write_text(json.dumps(exported, indent=2) + "\n")
    (root / "operating_points.md").write_text(summary(points, selected))


if __name__ == "__main__":
    main()

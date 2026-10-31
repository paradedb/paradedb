"""Calibrate the exponent at each nominal recall target with an automatic ceiling."""

import json
import os
import subprocess
from pathlib import Path

from prepare_k6 import (
    EXPONENT,
    PROBE,
    QUERIES,
    RECALL_TARGET,
    TARGETS,
    operating_points,
    summary,
)


def main():
    """Measure nonadaptive controls, then calibrate only the adaptive exponent."""
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
    adaptive = root / "admission"
    for target in TARGETS:
        for query in QUERIES:
            subprocess.run(
                common
                + [
                    "--query",
                    query,
                    "--runs-dir",
                    str(adaptive / f"r{round(target * 100)}"),
                    "--set",
                    "search.session.paradedb.vector_adaptive_probing=on",
                    "--set",
                    f"search.session.{RECALL_TARGET}={target}",
                    "--set",
                    f"search.session.{PROBE}=0",
                    "--set",
                    f"search.sweep.param={EXPONENT}",
                    "--set",
                    f"search.sweep.targets=[{target}]",
                ],
                check=True,
            )
    selected = operating_points(
        json.loads(p.read_text()) for p in sorted(adaptive.rglob("*_search.json"))
    )
    (root / "operating_points.json").write_text(json.dumps(selected, indent=2) + "\n")
    (root / "operating_points.md").write_text(
        "Admission experiment: nominal recall targets **90%, 95%, 99%**; "
        "only exponent is calibrated per filter/target; **max_probe=0**.\n\n"
        "Nonadaptive controls are retained in fixed-budget/. "
        "Calibration and evaluation use the same 100 queries.\n\n" + summary(selected)
    )


if __name__ == "__main__":
    main()

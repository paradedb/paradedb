"""Compare CI recall sweeps against mainline using pdbench's iso-recall report."""

import argparse
import json
import subprocess
from pathlib import Path

from prepare_k6 import QUERIES


def search_reports(directory):
    """Index by query identity, since filenames contain run-specific timestamps."""
    reports = {}
    for path in sorted(directory.glob("*_search.json")):
        report = json.loads(path.read_text())
        variants = report["data"]["variants"]
        if report["kind"] != "search" or len(variants) != 1:
            raise ValueError(f"Expected a single-query search report: {path}")
        query = next(iter(variants))
        if query not in QUERIES or query in reports:
            raise ValueError(f"Unexpected or duplicate query in {path}: {query}")
        reports[query] = report
    return reports


def compare(baseline, current, output, baseline_commit, baseline_url, current_commit):
    """Keep labeled inputs, provenance, text deltas, and offline HTML together."""
    baseline_reports = search_reports(baseline)
    current_reports = search_reports(current)
    if set(current_reports) != set(QUERIES):
        raise ValueError("Current run is missing recall sweeps")
    output.mkdir(parents=True, exist_ok=True)
    metadata = {
        "baseline_commit": baseline_commit,
        "baseline_url": baseline_url,
        "current_commit": current_commit,
        "queries": {},
    }
    lines = [
        "### pdbench mainline comparison",
        "",
        (
            f"Baseline: [main `{baseline_commit}`]({baseline_url}). "
            f"Current: `{current_commit}`."
        ),
        (
            "Reports compare independently selected probes at matched recall. "
            "HTML, text deltas, and labeled JSON inputs are in `comparison/` "
            "inside the diagnostics artifact."
        ),
        "",
    ]
    for query in QUERIES:
        base = baseline_reports.get(query)
        candidate = current_reports[query]
        if base is None:
            reason = "query missing from mainline diagnostics"
        elif base["fingerprint"] != candidate["fingerprint"]:
            reason = "benchmark configuration fingerprints differ"
        else:
            reason = None
        if reason:
            metadata["queries"][query] = {"skipped": reason}
            lines.append(f"- `{query}`: skipped ({reason}).")
            continue

        inputs = []
        for label, commit, report in (
            ("main", baseline_commit, base),
            ("current", current_commit, candidate),
        ):
            path = output / label / f"{query}_search.json"
            path.parent.mkdir(parents=True, exist_ok=True)
            # Profile is a display label, outside the config fingerprint. Make
            # the two commits distinguishable in pdbench's charts and tables.
            labeled = dict(report, profile=f"{label}@{commit[:12]}")
            path.write_text(json.dumps(labeled, indent=2) + "\n")
            inputs.append(str(path))
        html = output / f"{query}.html"
        with (output / f"{query}.txt").open("w") as log:
            subprocess.run(
                ["pdbench", "compare", *inputs, "--html", str(html)],
                stdout=log,
                check=True,
            )
        metadata["queries"][query] = {"html": html.name}
        lines.append(f"- `{query}`: `{html.name}`.")

    (output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    (output / "summary.md").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--current", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--baseline-commit", required=True)
    parser.add_argument("--baseline-url", required=True)
    parser.add_argument("--current-commit", required=True)
    args = parser.parse_args()
    compare(**vars(args))

"""Compare router compositions on one built index using the same held-out queries."""

import argparse
import copy
import json
import os
import subprocess
import tomllib
from pathlib import Path

from compare_search import search_reports
from prepare_k6 import QVEC, TARGETS
from verify_plans import verify_point

QUERY = "knn_top10_unfiltered"
KEYS = (
    "paradedb.vector_router_min_beam_fraction",
    "paradedb.vector_router_rerank_factor",
    "paradedb.vector_router_aps",
    "paradedb.vector_router_flat",
)
VARIANTS = (
    ("quarter-c2", "0.25", "2", "on", "off"),
    ("half-c2", "0.5", "2", "on", "off"),
    ("half-c4", "0.5", "4", "on", "off"),
    ("full-c2-noaps", "1.0", "2", "off", "off"),
    ("flat-c2", "0.25", "2", "off", "on"),
    ("flat-c4", "0.25", "4", "off", "on"),
)


def settings(variant):
    return dict(zip(KEYS, variant[1:], strict=True))


def render_profile(source, variant):
    base = tomllib.loads(source)
    if any(key in base["search"]["session"] for key in KEYS):
        raise ValueError("Base profile already overrides router experiment settings")
    lines = "".join(f"{json.dumps(k)} = {json.dumps(v)}\n" for k, v in settings(variant).items())
    return source.replace("[search.session]\n", "[search.session]\n" + lines, 1)


def validate_experiment(base, candidate, variant):
    """Only the named query-time router controls may differ; no rebuild is run."""
    for key, value in settings(variant).items():
        if candidate["config"]["search"]["session"].get(key) != value:
            raise ValueError(f"Experiment did not apply {key}={value}")
    configs = []
    for report in (base, candidate):
        config = copy.deepcopy(report["config"])
        for key in KEYS:
            config["search"]["session"].pop(key, None)
        configs.append(config)
    if configs[0] != configs[1] or base["data"]["build"] != candidate["data"]["build"]:
        raise ValueError("Router experiments must reuse the same dataset, build, and query settings")


def selected_points(report):
    """Use the sweep's actual target selections, including their achieved recall."""
    result = report["data"]["variants"][QUERY]
    measured = {point["value"]: point["metrics"] for point in result["points"]}
    rows = []
    for target in TARGETS:
        selection = result["targets"].get(str(target))
        row = {"target": target, "probe": None, "recall": None, "p50_ms": None}
        if selection:
            metrics = measured[selection["value"]]
            recall = metrics["recall"]["recall"]
            if recall + 1e-12 < target or abs(recall - selection["recall"]) > 1e-12:
                raise ValueError("Selected point does not meet its reported recall target")
            row.update(probe=selection["value"], recall=recall, p50_ms=metrics["latency"]["p50_ms"])
        rows.append(row)
    return rows


def summary(records):
    lines = [
        "### Router compositions on the same index",
        "",
        "All experiments reuse the built index and the same 100 query vectors. "
        "Only the listed query-time router controls change. Values below use the "
        "sweep's selected recall targets; unreached targets remain visible.",
        "",
        "| Composition | Target | max_probe | Achieved recall | p50 (ms) |",
        "|---|---:|---:|---:|---:|",
    ]
    for name, record in records.items():
        for row in record["targets"]:
            if row["probe"] is None:
                lines.append(f"| `{name}` | r{round(row['target'] * 100)} | unreached | — | — |")
            else:
                lines.append(
                    f"| `{name}` | r{round(row['target'] * 100)} | {row['probe']} "
                    f"| {row['recall']:.1%} | {row['p50_ms']:.3f} |"
                )
    return "\n".join(lines) + "\n"


def run_tuning(profile, directory, assets, variants=VARIANTS):
    runs = directory / "search"
    base = search_reports(runs / profile.stem)[QUERY]
    vector = json.loads((directory / "query_vectors.json").read_text())[0]
    output = directory / "router-tuning"
    for name in ("configs", "reports", "plans", "logs"):
        (output / name).mkdir(parents=True, exist_ok=True)
    comparisons = directory / "comparison"
    comparisons.mkdir(exist_ok=True)
    records = {}
    baseline = None
    for variant in variants:
        name = variant[0]
        generated = output / "configs" / f"{profile.stem}-router-{name}.toml"
        generated.write_text(render_profile(profile.read_text(), variant))
        with (output / "logs" / f"{name}.log").open("w") as log:
            subprocess.run(
                ["pdbench", "search", "--url", os.environ["PARADEDB_URL"],
                 "--config", str(generated), "--assets-dir", str(assets),
                 "--runs-dir", str(runs), "--query", QUERY, "--no-compare", "--html"],
                stdout=log, stderr=subprocess.STDOUT, check=True,
            )
        report = search_reports(runs / generated.stem)[QUERY]
        validate_experiment(base, report, variant)
        targets = selected_points(report)
        candidate = output / "reports" / f"{name}.json"
        candidate.write_text(json.dumps(report, indent=2) + "\n")
        if baseline is None:
            baseline = candidate
        else:
            # Config differences are intentional and validated above. Keep the
            # actual GUC values in each report and let pdbench display them.
            with (output / "logs" / f"{name}-compare.log").open("w") as log:
                subprocess.run(
                    ["pdbench", "compare", str(baseline), str(candidate), "--force", "--html",
                     str(comparisons / f"router_{name}.html")],
                    stdout=log, stderr=subprocess.STDOUT, check=True,
                )
        selected = next((p for p in targets if p["target"] == 0.95 and p["probe"] is not None), None)
        if selected is None:
            best = max(report["data"]["variants"][QUERY]["points"], key=lambda p: p["metrics"]["recall"]["recall"])
            probe = best["value"]
        else:
            probe = selected["probe"]
        point = {
            "name": name, "query": QUERY, "probe": probe,
            "session": report["config"]["search"]["session"],
            "sql": report["data"]["variants"][QUERY]["query"].replace(QVEC, "$1"),
        }
        counters = verify_point(point, vector, output / "plans",
                                expected_mode="flat" if variant[-1] == "on" else "hierarchical")
        records[name] = {"settings": settings(variant), "targets": targets, "verification": counters}
        (output / "results.json").write_text(json.dumps(records, indent=2) + "\n")
        (output / "summary.md").write_text(summary(records))
        print(f"Verified router experiment: {name}", flush=True)
    text = summary(records)
    with (directory / "operating_points.md").open("a") as context:
        context.write("\n" + text)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with Path(os.environ["GITHUB_STEP_SUMMARY"]).open("a") as step_summary:
            step_summary.write("\n" + text)
    return records


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--assets", type=Path, required=True)
    args = parser.parse_args()
    run_tuning(args.profile, args.directory, args.assets)

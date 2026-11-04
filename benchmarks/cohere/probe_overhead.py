"""Compare compatible extension libraries on one index, with fixed work budgets."""

import argparse
import hashlib
import json
import os
import random
import statistics
import subprocess
from pathlib import Path

import tomllib
from compare_search import search_reports


def measure_k6(root, destination, point, session):
    """Use the standard full-fixture workload and retain its timing samples."""
    destination.mkdir(parents=True, exist_ok=True)
    point = point | {
        "session": session,
        "probe": session["paradedb.vector_cluster_max_probe"],
    }
    points_file = destination / "operating_points.json"
    points_file.write_text(json.dumps([point], indent=2) + "\n")
    with (destination / "k6.log").open("w") as log:
        result = subprocess.run(
            [
                str(root / "benchmarker/k6"),
                "run",
                "--out",
                "dashboard=json,html",
                "-e",
                f"PARADEDB_URL={os.environ['PARADEDB_URL']}",
                "-e",
                f"OPERATING_POINTS={points_file}",
                "-e",
                f"OPERATING_POINT={point['name']}",
                "-e",
                f"QUERY_VECTORS={root / 'benchmark-artifacts/query_vectors.json'}",
                str(root / "benchmarks/cohere/k6/search.js"),
            ],
            cwd=destination,
            stdout=log,
            stderr=subprocess.STDOUT,
            check=False,
        )
    if result.returncode:
        print((destination / "k6.log").read_text(), flush=True)
        result.check_returncode()
    dashboards = list(destination.glob("dashboard_*.json"))
    if len(dashboards) != 1:
        raise ValueError(f"Expected one k6 dashboard in {destination}")
    dashboard = json.loads(dashboards[0].read_text())
    for suffix in (".json", ".html"):
        dashboards[0].with_suffix(suffix).rename(destination / f"k6{suffix}")
    samples = dashboard["runs"]["paradedb"]["queries"]["search"]["latencies"]
    return {
        "p50_ms": statistics.median(samples),
        "p95_ms": statistics.quantiles(samples, n=100)[94],
        "samples": len(samples),
        "start_time": dashboard["startTime"],
    }


def record_machine(destination, installed):
    """Retain restart and memory-placement evidence without collecting secrets."""
    destination.mkdir(parents=True, exist_ok=True)
    pid = (Path.home() / ".pgrx/data-18/postmaster.pid").read_text().splitlines()[0]
    with installed.open("rb") as library:
        digest = hashlib.file_digest(library, "sha256").hexdigest()
    metadata = {"postmaster_pid": pid, "library_sha256": digest}
    for name in ("status", "stat", "maps", "numa_maps", "smaps", "smaps_rollup"):
        source = Path(f"/proc/{pid}/{name}")
        try:
            (destination / f"postmaster-{name}.txt").write_text(
                source.read_text(encoding="utf-8")
            )
        except OSError as error:
            metadata[name] = str(error)
    for name, command in {
        "cpu": ["lscpu", "-J"],
        "processes": ["ps", "-C", "postgres", "-o", "pid,ppid,psr,pcpu,pmem,rss,comm"],
        "kernel": ["uname", "-a"],
        "libc": ["ldd", "--version"],
    }.items():
        result = subprocess.run(command, capture_output=True, text=True, check=False)
        (destination / f"{name}.txt").write_text(result.stdout + result.stderr)
    (destination / "meminfo.txt").write_text(
        Path("/proc/meminfo").read_text(encoding="utf-8")
    )
    (destination / "vmstat.txt").write_text(
        Path("/proc/vmstat").read_text(encoding="utf-8")
    )
    thp = Path("/sys/kernel/mm/transparent_hugepage")
    metadata["thp"] = {
        str(setting.relative_to(thp)): setting.read_text().strip()
        for setting in [thp / "enabled", *thp.glob("hugepages-*/enabled")]
        if setting.exists()
    }
    metadata["allocator_environment"] = [
        value
        for value in Path(f"/proc/{pid}/environ")
        .read_text(encoding="utf-8", errors="replace")
        .split("\0")
        if value.startswith("GLIBC_TUNABLES=")
    ]
    (destination / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")


def probe_config(template, arm, workers, raw_probe, selected_probe):
    """Keep the budget out of pdbench's three-significant-digit sweep rounding."""
    session = {}
    if arm == "normalized":
        probe = selected_probe
    elif arm == "scaled":
        probe = 0.125 * (raw_probe / 0.125) ** 0.25 if raw_probe < 0.125 else raw_probe
    else:
        probe = raw_probe
    if arm != "baseline":
        session = {
            "paradedb.vector_probe_scale_power": "1" if arm == "disabled" else "4",
            "paradedb.vector_probe_selectivity_exponent": "-0.86"
            if arm == "normalized"
            else "0",
            "paradedb.vector_probe_selectivity_slope": "-0.73"
            if arm == "normalized"
            else "0",
        }
    session["paradedb.vector_cluster_max_probe"] = repr(probe)
    settings = "".join(f'"{key}" = "{value}"\n' for key, value in session.items())
    return (
        template.replace("[search.session]\n", "[search.session]\n" + settings)
        .replace(
            'max_parallel_workers_per_gather = "8"',
            f'max_parallel_workers_per_gather = "{workers}"',
        )
        .replace("runs = 1\n", "runs = 3\n", 1)
        .replace(
            'param = "paradedb.vector_cluster_max_probe"',
            'param = "cohere.diagnostic_point"',
        )
    )


def summarize(directory, manifest):  # pylint: disable=too-many-locals
    """Retain every measurement and summarize the two repetitions of each arm."""
    rows = []
    for case in manifest:
        paths = list((directory / case["path"]).glob("**/*_search.json"))
        if len(paths) != 1:
            raise ValueError(f"Expected one fixed-probe report: {case}")
        report = json.loads(paths[0].read_text())
        point = report["data"]["variants"][case["query"]]["points"][0]
        metrics = point["metrics"]
        rows.append(case | {"metrics": metrics})
    (directory / "measurements.json").write_text(json.dumps(rows, indent=2) + "\n")
    lines = [
        "### Same-index probe overhead",
        "",
        (
            "The index is built once. Library order is candidate, baseline, baseline, candidate; "
            "each switch restarts PostgreSQL. Each point uses all 100 fixture vectors, "
            "one warmup and three timing passes, plus a separate EXPLAIN pass."
        ),
        "",
        (
            "Baseline and disabled arms use the same effective unnormalized fraction. "
            "Normalized uses the candidate's selected point; budgets can differ between segments. "
            "The following latency values are pdbench client timings; separate k6 results "
            "use the standard 10-second warmup and 50-second measurement."
        ),
        "",
        "| Workers allowed | Point | Arm | p50 ms | vs baseline | Work/query | Recall |",
        "|---:|---|---|---:|---:|---:|---:|",
    ]
    for workers in [0, 8]:
        for name in sorted({row["name"] for row in rows}):
            subset = [r for r in rows if r["workers"] == workers and r["name"] == name]
            base = statistics.mean(
                r["metrics"]["latency"]["p50_ms"]
                for r in subset
                if r["arm"] == "baseline"
            )
            for arm in ["baseline", "disabled", "normalized"]:
                samples = [r["metrics"] for r in subset if r["arm"] == arm]
                latency = statistics.mean(m["latency"]["p50_ms"] for m in samples)
                work = statistics.mean(
                    m["probes"]["per_query"]["work_charged_mean"] for m in samples
                )
                recall = statistics.mean(m["recall"]["recall"] for m in samples)
                lines.append(
                    f"| {workers} | {name} | {arm} | {latency:.3f} | "
                    f"{latency / base:.3f}x | {work:.3f} | {recall:.3f} |"
                )
    (directory / "summary.md").write_text("\n".join(lines) + "\n")
    k6 = json.loads((directory / "k6_measurements.json").read_text())
    lines = [
        "### Full-fixture k6 binary comparison",
        "",
        (
            "Initial measurements precede any diagnostic restart. All other rounds use "
            "the same index, 100 fixture vectors, and the standard k6 workload."
        ),
        "",
        "| Round | Arm | Point | p50 ms | p95 ms | Samples |",
        "|---:|---|---|---:|---:|---:|",
    ]
    for row in k6:
        lines.append(
            f"| {row['round']} | {row['arm']} | {row['name']} | "
            f"{row['p50_ms']:.3f} | {row['p95_ms']:.3f} | {row['samples']} |"
        )
    with (directory / "summary.md").open("a") as summary:
        summary.write("\n" + "\n".join(lines) + "\n")


def run(args):  # pylint: disable=too-many-locals,too-many-statements
    """Swap compatible libraries around measurements without rebuilding the index."""
    root = Path.cwd()
    output = args.directory.resolve() / "probe-overhead"
    output.mkdir(parents=True, exist_ok=True)
    template = (root / f"benchmarks/cohere/{args.size}.toml").read_text()
    reports = search_reports(args.directory / "search" / args.size)
    points = json.loads((args.directory / "operating_points.json").read_text())
    points = [
        p
        for p in points
        if p["reached"]
        and (p["target"] == 0.95 or p["name"] == "knn_top10_unfiltered@r99")
    ]
    for point in points:
        report = reports[point["query"]]
        measured = next(
            p
            for p in report["data"]["variants"][point["query"]]["points"]
            if p["value"] == point["probe"]
        )
        stats = measured["metrics"]["probes"]["per_query"]
        mean_rows = stats["segment_rows_mean"] / stats["segment_clusters_mean"]
        open_share = 1.64 / (1.64 + mean_rows)
        point["raw_probe"] = (
            stats["located_cap_mean"] * (1 - open_share) / stats["segment_rows_mean"]
        )
        if point["query"] == "knn_top10_unfiltered":
            probe = float(point["probe"])
            point["raw_probe"] = probe * min(probe / 0.125, 1) ** 3
    library_dir = subprocess.check_output(
        ["/usr/lib/postgresql/18/bin/pg_config", "--pkglibdir"], text=True
    ).strip()
    installed = Path(library_dir) / "pg_search.so"
    manifest = []
    k6_measurements = []
    provenance = {
        key: os.environ[key] for key in ["PROBE_BASELINE_REF", "PROBE_CANDIDATE_REF"]
    }
    (output / "commits.json").write_text(json.dumps(provenance, indent=2) + "\n")
    record_machine(output / "initial/machine", installed)
    for point in points:
        session = tomllib.loads(
            probe_config(
                template, "normalized", 8, point["raw_probe"], float(point["probe"])
            )
        )["search"]["session"]
        case_path = Path("initial") / point["name"]
        measured = measure_k6(root, output / case_path, point, session)
        k6_measurements.append(
            {
                "round": "initial",
                "arm": "normalized",
                "name": point["name"],
                "path": str(case_path),
            }
            | measured
        )
        (output / "k6_measurements.json").write_text(
            json.dumps(k6_measurements, indent=2) + "\n"
        )

    def switch(library):
        subprocess.run(
            ["cargo", "pgrx", "stop", "pg18"], cwd=root / "pg_search", check=True
        )
        subprocess.run(
            ["sudo", "install", "-m", "755", str(library), str(installed)], check=True
        )
        subprocess.run(
            ["cargo", "pgrx", "start", "pg18"], cwd=root / "pg_search", check=True
        )

    try:
        for iteration, binary in enumerate(
            ["candidate", "baseline", "baseline", "candidate"]
        ):
            switch(
                args.candidate_library
                if binary == "candidate"
                else args.baseline_library
            )
            record_machine(output / f"round-{iteration}/machine", installed)
            arms = ["disabled", "normalized"] if binary == "candidate" else ["baseline"]
            cases = [
                (workers, point, arm)
                for workers in [0, 8]
                for point in points
                for arm in arms
            ]
            random.Random(iteration).shuffle(cases)
            for index, (workers, point, arm) in enumerate(cases):
                case_path = (
                    Path(f"round-{iteration}") / str(workers) / arm / point["name"]
                )
                destination = output / case_path
                destination.mkdir(parents=True)
                config = destination / "config.toml"
                config.write_text(
                    probe_config(
                        template,
                        arm,
                        workers,
                        point["raw_probe"],
                        float(point["probe"]),
                    )
                )
                command = [
                    "pdbench",
                    "search",
                    "--url",
                    os.environ["PARADEDB_URL"],
                    "--config",
                    str(config),
                    "--assets-dir",
                    str(root / "benchmarks/cohere/assets"),
                    "--runs-dir",
                    str(destination),
                    "--query",
                    point["query"],
                    "--probe",
                    "1",
                    "--queries",
                    "100",
                    "--explain-sample",
                    "100",
                    "--no-compare",
                ]
                if index:
                    command.append("--no-prewarm")
                print(
                    f"Round {iteration}: workers={workers} {arm} {point['name']}",
                    flush=True,
                )
                with (destination / "search.log").open("w") as log:
                    result = subprocess.run(
                        command, stdout=log, stderr=subprocess.STDOUT, check=False
                    )
                if result.returncode:
                    print((destination / "search.log").read_text(), flush=True)
                    result.check_returncode()
                manifest.append(
                    {
                        "round": iteration,
                        "arm": arm,
                        "workers": workers,
                        "name": point["name"],
                        "query": point["query"],
                        "raw_probe": point["raw_probe"],
                        "path": str(case_path),
                    }
                )
                (output / "manifest.json").write_text(
                    json.dumps(manifest, indent=2) + "\n"
                )
                if workers:
                    session = tomllib.loads(config.read_text())["search"]["session"]
                    measured = measure_k6(root, destination, point, session)
                    k6_measurements.append(manifest[-1] | measured)
                    (output / "k6_measurements.json").write_text(
                        json.dumps(k6_measurements, indent=2) + "\n"
                    )
    finally:
        switch(args.candidate_library)
    summarize(output, manifest)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--size", choices=["1m", "10m"], required=True)
    parser.add_argument("--candidate-library", type=Path, required=True)
    parser.add_argument("--baseline-library", type=Path, required=True)
    run(parser.parse_args())

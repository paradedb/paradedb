"""CI-only, same-index CPU and scheduler profiles; never publishes a baseline."""

import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

from paired_stats_once import ROOT, OUT, URL, identity, psql, run


GROUPS = ("join_semi_filter", "join_permissioned_search", "join_top_k_score_desc_high_selectivity")
SIZE = sys.argv[1] if len(sys.argv) > 1 else "20m"
assert SIZE in ("1m", "20m")
VARIANTS = ("hash_partitioned", "range_partitioned")


def selected_queries():
    directory = ROOT / "benchmarks/datasets/stackoverflow/queries"
    selected = {}
    for group in GROUPS:
        for variant in VARIANTS:
            source = directory / group / f"{variant}.sql"
            # These six source files contain no semicolons or comments in literals.
            statements = [s.strip() for s in re.sub(r"--[^\n]*", "", source.read_text()).split(";") if s.strip()]
            assert all(s.upper().startswith("SET ") for s in statements[:-1])
            assert statements[-1].upper().startswith("SELECT")
            selected[f"{group} - {variant}"] = statements
    return selected


def benchmark(label, *, initialize):
    args = [str(ROOT / "target/release/benchmarks"), "benchmark", "--url", URL,
            "--dataset", "stackoverflow", "--index", "bm25", "--size", SIZE,
            "--runs", "30", "--output", "json", "--fail-on-error", "true"]
    if not initialize:
        args += ["--skip-index", "--vacuum", "false"]
    with (OUT / f"{label}-unprofiled.log").open("w") as log:
        subprocess.run(args, cwd=ROOT / "benchmarks", stdout=log, stderr=subprocess.STDOUT, check=True)
    text = (OUT / f"{label}-unprofiled.log").read_text()
    observed = {}
    for name, samples, rows in re.findall(
        r"Query Type: ([^\n]+).*?Results: \[cold: [^\]]*\] \[([^\]]*)\] \| Rows Returned: (\d+)", text, re.S
    ):
        assert name not in observed
        observed[name] = {"samples_ms": json.loads("[" + samples + "]"), "rows": int(rows)}
        assert len(observed[name]["samples_ms"]) == 30
    assert set(observed) == set(selected_queries()), "missing benchmark result"
    assert "EXPLAIN failed:" not in text
    (OUT / f"{label}-samples.json").write_text(json.dumps(observed, indent=2))
    return observed


def query_script(path, statements, repetitions):
    settings, query = statements[:-1], statements[-1]
    # Separate statements preserve ordinary planning on every execution. No PREPARE,
    # server-side loop, or EXPLAIN is inside the sampled workload.
    path.write_text("\\set ON_ERROR_STOP on\n\\o /dev/null\n" +
                    ";\n".join(settings) + ";\n" + (query + ";\n") * repetitions)


def record(label, statements, mode):
    capability = json.loads((OUT / "perf-capability.json").read_text())
    repeats = 200 if mode == "cpu" else 10
    sql_path = OUT / f"{label}-{mode}.sql"
    query_script(sql_path, statements, repeats)
    data = OUT / f"{label}-{mode}.data"
    workload = ["psql", URL, "-Xq", "-v", "ON_ERROR_STOP=1", "-f", str(sql_path)]
    # -a covers the already-running leader and newly forked MPP workers. Profiling
    # psql alone, or one backend PID, would miss most of the server execution.
    if mode == "cpu":
        args = ["sudo", "perf", "record", "-a", "-e", capability["event"], "-F", "199",
                "--call-graph", "dwarf,16384", "-o", str(data), "--", *workload]
    else:
        args = ["sudo", "perf", "sched", "record", "-a", "-o", str(data), "--", *workload]
    with (OUT / f"{label}-{mode}-record.log").open("w") as log:
        subprocess.run(args, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
    assert data.stat().st_size > 0
    if mode == "cpu":
        for kind, options in (
            ("self", ["--no-children", "--sort", "comm,pid,dso,symbol", "-g", "none"]),
            ("stacks", ["--children", "--sort", "comm,pid,dso,symbol", "-g", "graph,0.5,caller"]),
        ):
            with (OUT / f"{label}-{kind}.txt").open("w") as report:
                subprocess.run(["sudo", "perf", "report", "--stdio", "--stdio-color", "never",
                                "--percent-limit", "0.1", "-i", str(data), *options],
                               stdout=report, check=True)
        # Resolve symbols before swapping the shared library. Preserve raw samples
        # too, so reporting thresholds cannot hide a later candidate.
        with gzip.open(OUT / f"{label}-stacks.txt.gz", "wb") as compressed:
            process = subprocess.Popen(["sudo", "perf", "script", "-i", str(data)], stdout=subprocess.PIPE)
            try:
                shutil.copyfileobj(process.stdout, compressed)
            finally:
                process.stdout.close()
                assert process.wait() == 0
    else:
        with (OUT / f"{label}-scheduler.txt").open("w") as report:
            subprocess.run(["sudo", "perf", "sched", "timehist", "-i", str(data), "--summary"],
                           stdout=report, check=True)
    return {"mode": mode, "repetitions": repeats, "command": args, "event": capability["event"]}


def main():
    OUT.mkdir(exist_ok=True)
    selected = selected_queries()
    subset = ROOT / "benchmarks/datasets/stackoverflow/queries/bm25"
    assert not subset.exists(), "refuse to replace an existing query suite"
    pkglibdir = Path(run(["/usr/lib/postgresql/18/bin/pg_config", "--pkglibdir"], capture=True).strip())
    manifest = {"baseline": os.environ["BASELINE_SHA"], "branch": os.environ["FIXED_SHA"],
                "size": SIZE, "cpu_sample_hz": 199, "rounds": []}
    expected = expected_rows = None
    try:
        # Use the normal Rust benchmark runner for index creation and unprofiled
        # timings. Its index-specific directory selects only these six queries.
        for group in GROUPS:
            (subset / group).mkdir(parents=True)
            for variant in VARIANTS:
                shutil.copy2(subset.parent / group / f"{variant}.sql", subset / group)
        for number, version in enumerate(("baseline", "fixed", "fixed", "baseline"), 1):
            label = f"r{number}-{version}"
            run(["cargo", "pgrx", "stop", "pg18"], cwd=ROOT / "pg_search")
            library = OUT / f"{version}.so"
            run(["sudo", "install", "-m", "755", str(library), str(pkglibdir / "pg_search.so")])
            run(["sudo", "perf", "buildid-cache", "--add", str(pkglibdir / "pg_search.so")])
            run(["cargo", "pgrx", "start", "pg18"], cwd=ROOT / "pg_search")
            if expected is not None:
                assert identity() == expected, "indexes changed across restart"
            observed = benchmark(label, initialize=expected is None)
            if expected is None:
                expected = identity()
                (OUT / "index-identity.json").write_text(json.dumps(expected, indent=2))
            assert identity() == expected
            rows = {name: result["rows"] for name, result in observed.items()}
            if expected_rows is None:
                expected_rows = rows
            assert rows == expected_rows, "returned row counts differ"
            captures = []
            manifest["rounds"].append({"label": label, "library_sha256": hashlib.sha256(library.read_bytes()).hexdigest(),
                                       "captures": captures, "index_identity_unchanged": True, "row_counts_match": True})
            (OUT / "profile-manifest.json").write_text(json.dumps(manifest, indent=2))
            for name, statements in selected.items():
                prefix = label + "-" + name.replace(" - ", "-")
                warmup = OUT / f"{prefix}-warmup.sql"
                query_script(warmup, statements, 10)
                run(["psql", URL, "-Xq", "-f", str(warmup)])
                plan = psql(";\n".join([*statements[:-1], "SET track_io_timing = on",
                            "EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, BUFFERS, SUMMARY ON) " + statements[-1]]))
                (OUT / f"{prefix}-plan.txt").write_text(plan)
                assert "DistributedExec" in plan and re.search(r"MPP Launch: workers=[1-9]", plan), "MPP did not execute"
                if name.endswith("range_partitioned"):
                    assert "partition=owner_user_id[" in plan, "range assignment did not execute"
                captures.append({"query": name, **record(prefix, statements, "cpu")})
                if not name.startswith("join_top_k") and json.loads((OUT / "perf-capability.json").read_text())["scheduler"]:
                    captures.append({"query": name, **record(prefix, statements, "scheduler")})
                (OUT / "profile-manifest.json").write_text(json.dumps(manifest, indent=2))
            assert identity() == expected
            (OUT / "profile-manifest.json").write_text(json.dumps(manifest, indent=2))
            print(f"Finished {label}: unchanged indexes; six CPU profiles plus supported scheduler traces", flush=True)
    finally:
        if subset.exists():
            shutil.rmtree(subset)
        run(["cargo", "pgrx", "stop", "pg18"], cwd=ROOT / "pg_search")


if __name__ == "__main__":
    main()

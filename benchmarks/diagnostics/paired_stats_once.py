"""CI-only comparison of two libraries against one unchanged physical index build."""

import hashlib
import getpass
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import quote


ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "diagnostic-output"
# SQLx defaults to `anonymous` when the URI omits a user; psql defaults to the OS user.
# Use the same explicit OS-user URI as the repository's benchmark action.
URL = f"postgresql://{quote(getpass.getuser(), safe='')}@localhost:28818/postgres"


def run(args, *, cwd=ROOT, capture=False):
    return subprocess.check_output(args, cwd=cwd, text=True) if capture else subprocess.run(
        args, cwd=cwd, check=True
    )


def psql(sql):
    return run(["psql", URL, "-XAt", "-v", "ON_ERROR_STOP=1", "-c", sql], capture=True)


def queries():
    selected = {}
    directory = ROOT / "benchmarks/datasets/stackoverflow/queries"
    # These three existing files contain no semicolons or SQL comments within literals.
    for stem, alternative in [
        ("join_aggregate_topk_count", 2),
        ("join_semi_filter", 1),
        ("regex-and-heap", 0),
    ]:
        sql = re.sub(r"--[^\n]*", "", (directory / f"{stem}.sql").read_text())
        groups = []
        prefix = []
        for statement in sql.split(";"):
            statement = statement.strip()
            if not statement:
                continue
            if statement.upper().startswith("SET "):
                prefix.append(statement)
            else:
                assert statement.split()[0].upper() == "SELECT", statement
                groups.append((prefix, statement))
                prefix = []
        assert not prefix
        settings, query = groups[alternative]
        name = f"{stem}-alt{alternative}"
        selected[name] = (settings, query)
    subset = directory / "bm25"
    assert not subset.exists(), "do not replace an existing index-specific suite"
    subset.mkdir()
    for name, (settings, query) in selected.items():
        sql = ";\n".join([*settings, query]) + ";\n"
        (subset / f"{name}.sql").write_text(sql)
        (OUT / f"{name}.sql").write_text(sql)
    return selected


def identity():
    tables = ["stackoverflow_posts", "users", "badges", "comments"]
    result = {}
    for table in tables:
        index = "stackoverflow_posts_idx" if table == "stackoverflow_posts" else f"{table}_idx"
        result[table] = psql(
            f"SELECT relname, relfilenode, pg_relation_size(oid) FROM pg_class "
            f"WHERE oid IN ('{table}'::regclass, '{index}'::regclass) ORDER BY relname;"
        )
        result[index] = psql(
            f"SELECT segno, mutable, num_docs, num_deleted "
            f"FROM paradedb.index_info('{index}') ORDER BY segno;"
        )
    return result


def benchmark(size, label, *, initialize=False):
    args = [str(ROOT / "target/release/benchmarks"), "benchmark", "--url", URL,
            "--dataset", "stackoverflow", "--index", "bm25", "--size", size,
            "--runs", "1" if initialize else "10", "--output", "json",
            "--fail-on-error", "true"]
    if not initialize:
        args += ["--skip-index", "--vacuum", "false"]
    # Keep the benchmark's existing cache clearing, prewarming, and warmup policy.
    with (OUT / f"{label}-benchmark.log").open("w") as log:
        subprocess.run(args, cwd=ROOT / "benchmarks", stdout=log,
                       stderr=subprocess.STDOUT, check=True)
    (OUT / f"{label}-results.json").write_bytes((ROOT / "benchmarks/results.json").read_bytes())
    print(f"Completed {label}", flush=True)


def main():
    size = sys.argv[1]
    assert size in {"1m", "20m"}
    OUT.mkdir(exist_ok=True)
    selected = queries()
    benchmark(size, "initialize", initialize=True)
    expected = identity()
    (OUT / "index-identity.json").write_text(json.dumps(expected, indent=2))
    pkglibdir = Path(run(["/usr/lib/postgresql/18/bin/pg_config", "--pkglibdir"], capture=True).strip())
    manifest = {"size": size, "baseline_sha": os.environ["BASELINE_SHA"],
                "fixed_sha": os.environ["FIXED_SHA"], "rounds": []}
    for number, version in enumerate(["baseline", "fixed", "fixed", "baseline"], 1):
        label = f"r{number}-{version}"
        run(["cargo", "pgrx", "stop", "pg18"], cwd=ROOT / "pg_search")
        library = OUT / f"{version}.so"
        run(["sudo", "install", "-m", "755", str(library), str(pkglibdir / "pg_search.so")])
        run(["cargo", "pgrx", "start", "pg18"], cwd=ROOT / "pg_search")
        assert identity() == expected, f"physical data changed before {label}"
        benchmark(size, label)
        # These are additional diagnostic executions, separate from ordinary SELECT timings.
        # Every plan has its own total time and worker/task metrics, so imbalance is measurable.
        for name, (settings, query) in selected.items():
            statements = [*settings, "SET track_io_timing = on",
                          f"EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF, BUFFERS, SUMMARY ON) {query}"]
            sql = ";\n".join(statements) + ";"
            for repeat in range(6):
                output = psql(sql)
                assert "Execution Time:" in output, output
                if name.startswith("join_"):
                    assert "DistributedExec" in output and "workers=7" in output, output
                (OUT / f"{label}-{name}-plan-{repeat}.txt").write_text(output)
        assert identity() == expected, f"physical data changed after {label}"
        manifest["rounds"].append({"round": number, "version": version,
                                   "library_sha256": hashlib.sha256(library.read_bytes()).hexdigest(),
                                   "physical_data_unchanged": True})
        (OUT / "manifest.json").write_text(json.dumps(manifest, indent=2))
        print(f"Recorded all plans for {label}; physical data unchanged", flush=True)
    run(["cargo", "pgrx", "stop", "pg18"], cwd=ROOT / "pg_search")


if __name__ == "__main__":
    main()

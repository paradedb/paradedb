"""CI-only comparison of two libraries against one unchanged physical index build."""

import hashlib
import getpass
import json
import math
import statistics
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
    assert not (directory / "bm25").exists(), "the full flat suite must remain selected"
    for name, (settings, query) in selected.items():
        (OUT / f"{name}.sql").write_text("; ".join([*settings, query]) + ";\n")
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
            "--runs", "10", "--output", "json",
            "--fail-on-error", "true"]
    if not initialize:
        args += ["--skip-index", "--vacuum", "false"]
    # Keep the benchmark's existing cache clearing, prewarming, and warmup policy.
    with (OUT / f"{label}-benchmark.log").open("w") as log:
        subprocess.run(args, cwd=ROOT / "benchmarks", stdout=log,
                       stderr=subprocess.STDOUT, check=True)
    (OUT / f"{label}-results.json").write_bytes((ROOT / "benchmarks/results.json").read_bytes())
    log_text = (OUT / f"{label}-benchmark.log").read_text()
    observed = {}
    for name, samples, rows in re.findall(
        r"Query Type: ([^\n]+).*?Results: \[cold: [^\]]*\] \[([^\]]*)\] \| Rows Returned: (\d+)",
        log_text, re.S,
    ):
        assert name not in observed, f"duplicate result: {name}"
        samples = json.loads("[" + samples + "]")
        assert len(samples) == 10 and all(math.isfinite(x) and x >= 0 for x in samples)
        observed[name] = {"samples_ms": samples, "rows": int(rows)}
    assert set(observed) == set(suite_inventory()), f"incomplete suite in {label}"
    assert "EXPLAIN failed:" not in log_text, f"missing executed plan in {label}"
    (OUT / f"{label}-samples.json").write_text(json.dumps(observed, indent=2))
    print(f"Completed {label}: {len(observed)} query alternatives", flush=True)
    return observed


def suite_inventory():
    """Match the repository parser's group boundaries; the Rust runner executes the SQL."""
    directory = ROOT / "benchmarks/datasets/stackoverflow/queries"
    assert not (directory / "bm25").exists()
    result = {}
    for path in sorted(directory.glob("*.sql")):
        groups = []
        for group in path.read_text().split(";\n"):
            parts = group.split("$$")
            group = "$$".join(
                part if i % 2 else " ".join(line.split("--", 1)[0].strip() for line in part.split("\n"))
                for i, part in enumerate(parts)
            ).strip()
            if group:
                groups.append(group)
        for alternative, group in enumerate(groups):
            name = path.stem + (f" - alternative {alternative}" if alternative else "")
            result[name] = group
    assert result
    return result


def main():
    size = sys.argv[1]
    assert size in {"100k", "1m", "20m"}
    OUT.mkdir(exist_ok=True)
    selected = queries()
    inventory = suite_inventory()
    (OUT / "suite-inventory.json").write_text(json.dumps(inventory, indent=2))
    pkglibdir = Path(run(["/usr/lib/postgresql/18/bin/pg_config", "--pkglibdir"], capture=True).strip())
    manifest = {"size": size, "baseline_sha": os.environ["BASELINE_SHA"],
                "fixed_sha": os.environ["FIXED_SHA"], "query_alternatives": len(inventory),
                "samples_per_query_per_round": 10, "rounds": []}
    expected = None
    expected_rows = None
    results = {"baseline": {}, "fixed": {}}
    try:
        for number, version in enumerate(["baseline", "fixed", "fixed", "baseline"], 1):
            label = f"r{number}-{version}"
            run(["cargo", "pgrx", "stop", "pg18"], cwd=ROOT / "pg_search")
            library = OUT / f"{version}.so"
            run(["sudo", "install", "-m", "755", str(library), str(pkglibdir / "pg_search.so")])
            run(["cargo", "pgrx", "start", "pg18"], cwd=ROOT / "pg_search")
            if expected is not None:
                assert identity() == expected, f"physical data changed before {label}"
            observed = benchmark(size, label, initialize=expected is None)
            if expected is None:
                expected = identity()
                (OUT / "index-identity.json").write_text(json.dumps(expected, indent=2))
            assert identity() == expected, f"physical data changed after {label}"
            rows = {name: item["rows"] for name, item in observed.items()}
            if expected_rows is None:
                expected_rows = rows
            assert rows == expected_rows, f"returned row counts changed in {label}"
            for name, item in observed.items():
                results[version].setdefault(name, []).extend(item["samples_ms"])
            # The full suite logs an executed plan for every alternative. These selected
            # extra profiles retain buffers and total time, outside ordinary SELECT timing.
            for name, (settings, query) in selected.items():
                statements = [*settings, "SET track_io_timing = on",
                              f"EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF, BUFFERS, SUMMARY ON) {query}"]
                sql = ";\n".join(statements) + ";"
                for repeat in range(6):
                    output = psql(sql)
                    assert "Execution Time:" in output, output
                    # Preserve native/serial fallbacks in the evidence; small datasets may
                    # legitimately decline MPP. The actual plan records workers and mode.
                    (OUT / f"{label}-{name}-plan-{repeat}.txt").write_text(output)
            assert identity() == expected, f"physical data changed after profiles for {label}"
            manifest["rounds"].append({"round": number, "version": version,
                                       "library_sha256": hashlib.sha256(library.read_bytes()).hexdigest(),
                                       "index_initialized": number == 1,
                                       "matches_first_round_index_identity": True,
                                       "row_counts_match": True})
            (OUT / "manifest.json").write_text(json.dumps(manifest, indent=2))
            print(f"Recorded all plans for {label}; physical data unchanged", flush=True)
        comparison = []
        for name in sorted(inventory):
            baseline = statistics.mean(results["baseline"][name])
            fixed = statistics.mean(results["fixed"][name])
            comparison.append({"query": name, "main_ms": baseline, "fixed_ms": fixed,
                               "change_percent": (fixed / baseline - 1) * 100 if baseline else None})
        comparison.sort(key=lambda r: r["change_percent"] or 0, reverse=True)
        (OUT / "comparison.json").write_text(json.dumps(comparison, indent=2))
        report = [f"Stack Overflow {size}: pinned main vs latest local fixes", "",
                  "Twenty ordinary SELECT samples per version/query; four rounds in main/fixed/fixed/main order.",
                  "Identical physical indexes and returned row counts verified. Row counts do not prove full result-content equality.",
                  "Positive percentages are slower; the 15% threshold flags candidates for investigation, not statistical significance.", "",
                  "| Query | Main ms | Fixed ms | Change |", "|---|---:|---:|---:|"]
        for item in comparison:
            change = f"{item['change_percent']:+.1f}%" if item['change_percent'] is not None else "n/a"
            report.append(f"| {item['query']} | {item['main_ms']:.3f} | {item['fixed_ms']:.3f} | {change} |")
        report = "\n".join(report) + "\n"
        (OUT / "comparison.md").write_text(report)
        if os.environ.get("GITHUB_STEP_SUMMARY"):
            with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as summary:
                summary.write(report)
    finally:
        run(["cargo", "pgrx", "stop", "pg18"], cwd=ROOT / "pg_search")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Benchmark runner comparing Hash-partitioned vs Range-partitioned join queries in ParadeDB.

Accounts for cold and warm runs, identifies cases where Hash is faster than Range,
and captures detailed EXPLAIN (ANALYZE, BUFFERS, TIMING, COSTS, SUMMARY, VERBOSE) query plans.
"""

import argparse
import glob
import json
import os
import re
import statistics
import subprocess
import sys
import time
from typing import Any


def discover_queries(
    queries_dir: str, filter_str: str | None = None
) -> list[tuple[str, str, str]]:
    """Discover all query directories containing both hash_partitioned.sql and range_partitioned.sql."""
    subdirs = sorted(glob.glob(os.path.join(queries_dir, "join_*")))
    valid = []
    for d in subdirs:
        qname = os.path.basename(d)
        if filter_str and filter_str not in qname:
            continue
        hash_sql = os.path.join(d, "hash_partitioned.sql")
        range_sql = os.path.join(d, "range_partitioned.sql")
        if os.path.isfile(hash_sql) and os.path.isfile(range_sql):
            valid.append((qname, hash_sql, range_sql))
    return valid


def make_explain_sql(sql_content: str, analyze: bool = False) -> str:
    """Prepend EXPLAIN (with optional ANALYZE/BUFFERS/etc.) to the SELECT statement."""
    m = re.search(r"\bSELECT\b", sql_content, re.IGNORECASE)
    if not m:
        raise ValueError("Could not find SELECT keyword in SQL content")
    prelude = sql_content[: m.start()]
    select_part = sql_content[m.start() :]

    if analyze:
        explain_clause = "EXPLAIN (ANALYZE, BUFFERS, TIMING, COSTS, SUMMARY, VERBOSE) "
    else:
        explain_clause = "EXPLAIN "

    return r"\timing on" + "\n" + prelude + explain_clause + select_part


def run_psql(db_url: str, sql: str, timeout: int = 120) -> tuple[int, str, str, float]:
    """Execute SQL via psql and measure wall clock time."""
    start_t = time.perf_counter()
    try:
        res = subprocess.run(
            [
                "psql",
                db_url,
                "-X",
                "-q",
                "-v",
                "ON_ERROR_STOP=1",
            ],
            input=sql,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        elapsed_ms = (time.perf_counter() - start_t) * 1000.0
        return res.returncode, res.stdout, res.stderr, elapsed_ms
    except subprocess.TimeoutExpired:
        elapsed_ms = (time.perf_counter() - start_t) * 1000.0
        return -1, "", f"Timed out after {timeout} seconds", elapsed_ms


def parse_psql_timing(stdout: str) -> float | None:
    """Parse the last 'Time: <ms> ms' from psql timing output."""
    matches = re.findall(r"Time:\s+([\d\.]+)\s+ms", stdout)
    if matches:
        return float(matches[-1])
    return None


def parse_explain_metrics(stdout: str) -> dict[str, Any]:
    """Extract key metrics from EXPLAIN (ANALYZE) output."""
    metrics: dict[str, Any] = {}

    plan_time = re.search(r"Planning Time:\s+([\d\.]+)\s+ms", stdout)
    if plan_time:
        metrics["planning_time_ms"] = float(plan_time.group(1))

    exec_time = re.search(r"Execution Time:\s+([\d\.]+)\s+ms", stdout)
    if exec_time:
        metrics["execution_time_ms"] = float(exec_time.group(1))

    buf_hit = re.search(r"Buffers:\s+shared hit=(\d+)", stdout)
    if buf_hit:
        metrics["shared_hit_buffers"] = int(buf_hit.group(1))

    buf_read = re.search(r"Buffers:.*read=(\d+)", stdout)
    if buf_read:
        metrics["shared_read_buffers"] = int(buf_read.group(1))

    mpp_workers = re.search(r"MPP Launch:\s+workers=(\d+)", stdout)
    if mpp_workers:
        metrics["mpp_workers"] = int(mpp_workers.group(1))

    metrics["has_repartition_exec"] = "RepartitionExec" in stdout
    metrics["has_network_shuffle"] = "NetworkShuffleExec" in stdout
    metrics["has_hash_join"] = "HashJoinExec" in stdout
    metrics["has_visibility_filter"] = "VisibilityFilterExec" in stdout

    return metrics


def execute_variant_run(
    db_url: str, sql_path: str, timeout: int
) -> tuple[float, float, str]:
    """Run a query file with timing enabled."""
    with open(sql_path, "r") as f:
        content = f.read()
    sql_with_timing = r"\timing on" + "\n" + content
    rc, stdout, stderr, wall_ms = run_psql(db_url, sql_with_timing, timeout)
    if rc != 0:
        raise RuntimeError(f"psql failed with code {rc}: {stderr}")
    timing_ms = parse_psql_timing(stdout)
    if timing_ms is None:
        timing_ms = wall_ms
    return timing_ms, wall_ms, stdout


def run_explain_plan(
    db_url: str, sql_path: str, timeout: int, analyze: bool = True
) -> tuple[str, dict[str, Any]]:
    """Generate EXPLAIN or EXPLAIN ANALYZE plan."""
    with open(sql_path, "r") as f:
        content = f.read()
    explain_sql = make_explain_sql(content, analyze=analyze)
    rc, stdout, stderr, _ = run_psql(db_url, explain_sql, timeout)
    if rc != 0:
        raise RuntimeError(
            f"psql EXPLAIN failed with code {rc}: {stderr}\nSQL:\n{explain_sql}"
        )
    metrics = parse_explain_metrics(stdout)
    return stdout, metrics


def benchmark_query_pair(
    db_url: str,
    query_name: str,
    hash_path: str,
    range_path: str,
    warm_runs: int,
    timeout: int,
    speedup_threshold: float,
    verbose: bool = True,
) -> dict[str, Any]:
    """Benchmark one query pair with cold and warm runs."""
    if verbose:
        print(f"\n[QUERY] {query_name}")

    # Cold runs: Run hash cold, then range cold
    if verbose:
        print("  Running cold run: Hash...", end="", flush=True)
    hash_cold_ms, _, _ = execute_variant_run(db_url, hash_path, timeout)
    if verbose:
        print(f" {hash_cold_ms:.2f} ms")

    if verbose:
        print("  Running cold run: Range...", end="", flush=True)
    range_cold_ms, _, _ = execute_variant_run(db_url, range_path, timeout)
    if verbose:
        print(f" {range_cold_ms:.2f} ms")

    # Warm runs: Alternate between hash and range
    hash_warm_runs: list[float] = []
    range_warm_runs: list[float] = []

    for i in range(1, warm_runs + 1):
        if verbose:
            print(f"  Warm run #{i}...", end="", flush=True)
        h_ms, _, _ = execute_variant_run(db_url, hash_path, timeout)
        r_ms, _, _ = execute_variant_run(db_url, range_path, timeout)
        hash_warm_runs.append(h_ms)
        range_warm_runs.append(r_ms)
        if verbose:
            print(f" Hash: {h_ms:.2f} ms, Range: {r_ms:.2f} ms")

    hash_warm_median = statistics.median(hash_warm_runs)
    range_warm_median = statistics.median(range_warm_runs)
    hash_warm_mean = statistics.mean(hash_warm_runs)
    range_warm_mean = statistics.mean(range_warm_runs)
    hash_warm_std = statistics.stdev(hash_warm_runs) if len(hash_warm_runs) > 1 else 0.0
    range_warm_std = (
        statistics.stdev(range_warm_runs) if len(range_warm_runs) > 1 else 0.0
    )

    warm_speedup = range_warm_median / hash_warm_median if hash_warm_median > 0 else 1.0
    cold_speedup = range_cold_ms / hash_cold_ms if hash_cold_ms > 0 else 1.0

    # Hash is faster if range took longer (warm_speedup > threshold)
    hash_faster = warm_speedup >= speedup_threshold

    if verbose:
        winner = (
            "HASH FASTER"
            if hash_faster
            else ("RANGE FASTER" if warm_speedup < 1.0 else "TIE")
        )
        print(
            f"  --> Median Warm: Hash={hash_warm_median:.2f} ms vs Range={range_warm_median:.2f} ms | "
            f"Ratio={warm_speedup:.2f}x ({winner})"
        )

    return {
        "query": query_name,
        "hash_cold_ms": hash_cold_ms,
        "range_cold_ms": range_cold_ms,
        "cold_speedup": cold_speedup,
        "hash_warm_runs_ms": hash_warm_runs,
        "range_warm_runs_ms": range_warm_runs,
        "hash_warm_median_ms": hash_warm_median,
        "range_warm_median_ms": range_warm_median,
        "hash_warm_mean_ms": hash_warm_mean,
        "range_warm_mean_ms": range_warm_mean,
        "hash_warm_std_ms": hash_warm_std,
        "range_warm_std_ms": range_warm_std,
        "warm_speedup": warm_speedup,
        "hash_faster": hash_faster,
    }


def capture_query_plans(
    db_url: str,
    query_name: str,
    hash_path: str,
    range_path: str,
    output_dir: str,
    timeout: int,
    analyze: bool = True,
    verbose: bool = True,
) -> dict[str, Any]:
    """Capture query plans into the output directory."""
    target_dir = os.path.join(output_dir, query_name)
    os.makedirs(target_dir, exist_ok=True)

    if verbose:
        print(
            f"  Capturing EXPLAIN {'(ANALYZE)' if analyze else ''} plans to {target_dir}..."
        )

    hash_plan_raw, hash_metrics = run_explain_plan(
        db_url, hash_path, timeout, analyze=analyze
    )
    range_plan_raw, range_metrics = run_explain_plan(
        db_url, range_path, timeout, analyze=analyze
    )

    with open(os.path.join(target_dir, "hash_plan.txt"), "w") as f:
        f.write(hash_plan_raw)

    with open(os.path.join(target_dir, "range_plan.txt"), "w") as f:
        f.write(range_plan_raw)

    plan_metrics = {
        "query": query_name,
        "hash_metrics": hash_metrics,
        "range_metrics": range_metrics,
    }
    with open(os.path.join(target_dir, "plan_metrics.json"), "w") as f:
        json.dump(plan_metrics, f, indent=2)

    return plan_metrics


def generate_summary(
    results: list[dict[str, Any]],
    output_dir: str,
    speedup_threshold: float,
    dry_run: bool = False,
) -> str:
    """Generate Markdown summary table and JSON results."""
    os.makedirs(output_dir, exist_ok=True)

    results_file = os.path.join(output_dir, "results.json")
    with open(results_file, "w") as f:
        json.dump(
            {
                "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
                "dry_run": dry_run,
                "speedup_threshold": speedup_threshold,
                "total_queries": len(results),
                "hash_faster_count": sum(1 for r in results if r.get("hash_faster")),
                "queries": results,
            },
            f,
            indent=2,
        )

    md_lines = [
        f"# Benchmark Results: Hash vs Range Partitioned Joins {'(DRY RUN)' if dry_run else ''}",
        "",
        f"- Generated: {time.strftime('%Y-%m-%d %H:%M:%S')}",
        f"- Total queries evaluated: {len(results)}",
        f"- Threshold for Hash faster: >= {speedup_threshold:.2f}x speedup",
        "",
        "## Summary Table",
        "",
        "| Query | Cold Hash (ms) | Cold Range (ms) | Warm Hash Med (ms) | Warm Range Med (ms) | Warm Ratio (Range/Hash) | Faster Variant |",
        "| :--- | :---: | :---: | :---: | :---: | :---: | :---: |",
    ]

    for r in results:
        q = r["query"]
        ch = f"{r['hash_cold_ms']:.1f}"
        cr = f"{r['range_cold_ms']:.1f}"
        wh = f"{r['hash_warm_median_ms']:.1f}"
        wr = f"{r['range_warm_median_ms']:.1f}"
        ratio = f"{r['warm_speedup']:.2f}x"
        winner = (
            "`HASH` 🚀"
            if r["hash_faster"]
            else ("`RANGE` ⚡" if r["warm_speedup"] < 0.98 else "`EQUIVALENT`")
        )
        md_lines.append(f"| `{q}` | {ch} | {cr} | {wh} | {wr} | {ratio} | {winner} |")

    hash_faster_queries = [r for r in results if r.get("hash_faster")]
    md_lines.extend(
        [
            "",
            f"## Queries Where Hash is Faster ({len(hash_faster_queries)})",
            "",
        ]
    )

    if not hash_faster_queries:
        md_lines.append(
            "None. Range partitioning outperformed or matched Hash across all tested queries."
        )
    else:
        for r in hash_faster_queries:
            q = r["query"]
            md_lines.extend(
                [
                    f"### `{q}`",
                    f"- Warm Median: Hash `{r['hash_warm_median_ms']:.2f} ms` vs Range `{r['range_warm_median_ms']:.2f} ms` ({r['warm_speedup']:.2f}x faster with Hash)",
                    f"- Cold Run: Hash `{r['hash_cold_ms']:.2f} ms` vs Range `{r['range_cold_ms']:.2f} ms`",
                    f"- Query plans captured under `{output_dir}/{q}/`",
                    "",
                ]
            )

    summary_file = os.path.join(output_dir, "summary.md")
    with open(summary_file, "w") as f:
        f.write("\n".join(md_lines) + "\n")

    return summary_file


def perform_dry_run(
    db_url: str,
    queries: list[tuple[str, str, str]],
    output_dir: str,
    timeout: int,
) -> None:
    """Run a thorough dry run validating syntax, planning, and 1 execution sample."""
    print("=" * 60)
    print("STARTING DRY RUN")
    print(f"Database URL: {db_url}")
    print(f"Total query pairs discovered: {len(queries)}")
    print(f"Output directory: {output_dir}")
    print("=" * 60)

    # 1. Connection check
    print("[1/3] Testing PostgreSQL connection...", end="", flush=True)
    rc, stdout, stderr, _ = run_psql(db_url, "SELECT 1;", timeout=10)
    if rc != 0:
        print(" FAILED")
        raise RuntimeError(f"Connection failed: {stderr}")
    print(" OK")

    # 2. Planning check on all discovered queries
    print(
        f"[2/3] Validating query syntax & EXPLAIN planning for all {len(queries)} pairs..."
    )
    for qname, hpath, rpath in queries:
        print(f"  Validating `{qname}`...", end="", flush=True)
        # Hash plan
        _, _ = run_explain_plan(db_url, hpath, timeout=timeout, analyze=False)
        # Range plan
        _, _ = run_explain_plan(db_url, rpath, timeout=timeout, analyze=False)
        print(" OK")

    # 3. Single query execution check (1 cold + 1 warm run)
    sample_query = next((q for q in queries if "permissioned" in q[0]), queries[0])
    qname, hpath, rpath = sample_query
    print(f"\n[3/3] Testing end-to-end execution on sample query `{qname}`...")
    sample_result = benchmark_query_pair(
        db_url,
        qname,
        hpath,
        rpath,
        warm_runs=1,
        timeout=timeout,
        speedup_threshold=1.0,
        verbose=True,
    )

    # Test plan capture into output directory
    print("  Testing plan capture...")
    capture_query_plans(
        db_url,
        qname,
        hpath,
        rpath,
        output_dir,
        timeout=timeout,
        analyze=True,
        verbose=True,
    )

    # Test summary generation
    summary_path = generate_summary(
        [sample_result], output_dir, speedup_threshold=1.0, dry_run=True
    )
    print(f"  Summary generated at: {summary_path}")

    print("\n" + "=" * 60)
    print("DRY RUN COMPLETED SUCCESSFULLY!")
    print("All queries passed EXPLAIN validation.")
    print("End-to-end measurement and plan capture verified.")
    print("=" * 60)


def main():
    parser = argparse.ArgumentParser(
        description="Benchmark Hash vs Range partitioned join queries"
    )
    parser.add_argument(
        "--db-url",
        default="postgresql://localhost:28818/postgres",
        help="Postgres connection URL",
    )
    parser.add_argument(
        "--queries-dir",
        default="benchmarks/datasets/stackoverflow/queries",
        help="Path to StackOverflow benchmark queries directory",
    )
    parser.add_argument(
        "--output-dir",
        default="tmp_query_plans",
        help="Directory to store captured query plans and results",
    )
    parser.add_argument(
        "--warm-runs",
        type=int,
        default=3,
        help="Number of warm benchmark runs per query variant (default: 3)",
    )
    parser.add_argument(
        "--speedup-threshold",
        type=float,
        default=1.0,
        help="Threshold ratio (Range/Hash) above which Hash is flagged as faster (default: 1.0)",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=180,
        help="Per-query timeout in seconds (default: 180s)",
    )
    parser.add_argument(
        "--query",
        type=str,
        default=None,
        help="Filter to run specific query matching substring",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Run in dry run mode (validates all queries with EXPLAIN + 1 sample execution)",
    )
    parser.add_argument(
        "--capture-all-plans",
        action="store_true",
        help="Capture EXPLAIN ANALYZE plans for ALL queries, not just Hash-faster ones",
    )

    args = parser.parse_args()

    queries = discover_queries(args.queries_dir, filter_str=args.query)
    if not queries:
        print(
            f"Error: No join queries found under '{args.queries_dir}' (matching filter '{args.query}')"
        )
        sys.exit(1)

    if args.dry_run:
        perform_dry_run(args.db_url, queries, args.output_dir, args.timeout)
        return

    print("=" * 60)
    print("STARTING FULL BENCHMARK RUN")
    print(f"Database URL: {args.db_url}")
    print(f"Total queries: {len(queries)}")
    print(f"Warm runs per variant: {args.warm_runs}")
    print(f"Speedup threshold: {args.speedup_threshold:.2f}x")
    print(f"Output directory: {args.output_dir}")
    print("=" * 60)

    results: list[dict[str, Any]] = []
    plans_captured: list[str] = []

    for qname, hpath, rpath in queries:
        try:
            res = benchmark_query_pair(
                args.db_url,
                qname,
                hpath,
                rpath,
                warm_runs=args.warm_runs,
                timeout=args.timeout,
                speedup_threshold=args.speedup_threshold,
                verbose=True,
            )
            results.append(res)

            if res["hash_faster"] or args.capture_all_plans:
                capture_query_plans(
                    args.db_url,
                    qname,
                    hpath,
                    rpath,
                    args.output_dir,
                    timeout=args.timeout,
                    analyze=True,
                    verbose=True,
                )
                plans_captured.append(qname)

        except Exception as e:
            print(f"\n[ERROR] Query `{qname}` failed: {e}")
            results.append(
                {
                    "query": qname,
                    "error": str(e),
                    "hash_faster": False,
                    "warm_speedup": 0.0,
                    "hash_cold_ms": 0.0,
                    "range_cold_ms": 0.0,
                    "hash_warm_median_ms": 0.0,
                    "range_warm_median_ms": 0.0,
                }
            )

    summary_path = generate_summary(
        results, args.output_dir, args.speedup_threshold, dry_run=False
    )

    print("\n" + "=" * 60)
    print("BENCHMARK COMPLETED")
    print(f"Summary report written to: {summary_path}")
    print(f"Plans captured for queries ({len(plans_captured)}): {plans_captured}")
    print("=" * 60)


if __name__ == "__main__":
    main()

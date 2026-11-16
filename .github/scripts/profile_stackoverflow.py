#!/usr/bin/env python3
"""Profile existing SO outliers after the standard suite, without publishing diagnostics."""

import argparse
import json
import re
import subprocess
from pathlib import Path

PROFILE_SAMPLES = 1000
QUERY_PATHS = {
    "100k": (
        "paging_string_min.sql",
        "paging_string_median.sql",
        "paging_string_max.sql",
        "bucket_string_filter/aggregate_scan_count_col.sql",
    ),
    "1m": (
        "join_disjunctive_permissioned_score_sort_low_selectivity/rank_join_range_partitioned.sql",
        "join_disjunctive_permissioned_score_sort_medium_selectivity/range_partitioned.sql",
    ),
    "20m": (
        "join_disjunctive_permissioned_score_sort_low_selectivity/hash_partitioned.sql",
        "top_k_string.sql",
    ),
}


def main():
    """Locate the existing cluster and benchmark binary, then profile each query."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--pgdata", type=Path, required=True)
    parser.add_argument("--url", required=True)
    parser.add_argument("--size", choices=QUERY_PATHS, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    args.source = args.source.resolve()
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=False)
    postmaster = (args.pgdata / "postmaster.pid").read_text().splitlines()[0]
    metadata = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--locked", "--no-deps", "--format-version=1"],
            cwd=args.source,
            text=True,
        )
    )
    binary = Path(metadata["target_directory"]) / "release/benchmarks"
    if not binary.is_file():
        raise FileNotFoundError(binary)
    commit = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=args.source, text=True
    ).strip()
    (args.output / "manifest.json").write_text(
        json.dumps(
            {
                "commit": commit,
                "size": args.size,
                "samples": PROFILE_SAMPLES,
                "query_paths": QUERY_PATHS[args.size],
                "purpose": "Diagnostic profiles, excluded from standard benchmark publication",
            },
            indent=2,
        )
        + "\n"
    )

    for relative in QUERY_PATHS[args.size]:
        profile_query(args, binary, postmaster, relative)


def profile_query(args, binary, postmaster, relative):
    """Retain the unchanged SQL, timed plan, profile and isolated benchmark result."""
    name = " - ".join(Path(relative).with_suffix("").parts)
    directory = args.output / name
    directory.mkdir()
    sql = (
        args.source / "benchmarks/datasets/stackoverflow/queries" / relative
    ).read_text()
    (directory / "query.sql").write_text(sql)
    statements = [
        s.strip() for s in re.sub(r"--[^\n]*", "", sql).split(";") if s.strip()
    ]
    if not re.match(r"SELECT\b", statements[-1], re.IGNORECASE) or any(
        not re.match(r"SET\b", s, re.IGNORECASE) for s in statements[:-1]
    ):
        raise ValueError(f"Expected SET preamble and SELECT query: {relative}")
    explain = ";\n".join(
        statements[:-1]
        + ["EXPLAIN (ANALYZE, VERBOSE, BUFFERS, FORMAT JSON) " + statements[-1]]
    )
    plan = subprocess.check_output(
        ["psql", args.url, "-XqAt", "-v", "ON_ERROR_STOP=1", "-c", explain],
        text=True,
    )
    (directory / "plan.json").write_text(json.dumps(json.loads(plan), indent=2) + "\n")
    command = [
        str(binary),
        "benchmark",
        "--url",
        args.url,
        "--dataset",
        "stackoverflow",
        "--index",
        "bm25",
        "--size",
        args.size,
        "--skip-index",
        "--vacuum",
        "false",
        "--dirty",
        "false",
        "--clear-caches",
        "false",
        "--prewarm",
        "false",
        "--runs",
        str(PROFILE_SAMPLES),
        "--output",
        "json",
        "--fail-on-error",
        "true",
        "--query-filter",
        f"^{re.escape(name)}$",
    ]
    (directory / "datasets").symlink_to(
        args.source / "benchmarks/datasets", target_is_directory=True
    )
    try:
        with (directory / "benchmark.log").open("w") as log:
            subprocess.run(
                [
                    "bash",
                    str(Path(__file__).with_name("profile-postgres.sh").resolve()),
                    f"pid:{postmaster}",
                    str(directory),
                    "--",
                    *command,
                ],
                cwd=directory,
                stdout=log,
                stderr=subprocess.STDOUT,
                check=True,
            )
        result = json.loads(
            (directory / f"results_stackoverflow_bm25_{args.size}.json").read_text()
        )
        measured = [row for row in result if row["unit"] == "mean ms"]
        if len(measured) != 1 or measured[0]["name"] != name:
            raise ValueError(f"Expected exactly one measured query: {name}")
    finally:
        (directory / "datasets").unlink()


if __name__ == "__main__":
    main()

"""Test allocator effects with one binary, index, and fixed k6 workload."""

import argparse
import json
import os
import re
import subprocess
from pathlib import Path

from probe_overhead import measure_k6, record_machine


def heap_pages():
    """Count resident heap pages before selecting a restart for measurement."""
    pid = (Path.home() / ".pgrx/data-18/postmaster.pid").read_text().splitlines()[0]
    return sum(
        int(match[1])
        for line in Path(f"/proc/{pid}/numa_maps")
        .read_text(encoding="utf-8")
        .splitlines()
        if " heap " in line and (match := re.search(r"\banon=(\d+)", line))
    )


def statement_totals():
    """Separate server planning and execution costs for the measured queries."""
    query = """
        SELECT row_to_json(s) FROM (
            SELECT sum(calls) AS calls, sum(plans) AS plans,
                   sum(total_plan_time) AS plan_ms, sum(total_exec_time) AS exec_ms
            FROM pg_stat_statements
            WHERE query LIKE '%cohere_wiki%' AND query LIKE '%ORDER BY%'
        ) s
    """
    return json.loads(
        subprocess.check_output(
            [
                "psql",
                os.environ["PARADEDB_URL"],
                "-XAt",
                "-v",
                "ON_ERROR_STOP=1",
                "-c",
                query,
            ],
            text=True,
        )
    )


def run(directory):  # pylint: disable=too-many-locals
    """Select small and inflated heaps independently of their query timings."""
    root = Path.cwd()
    output = directory / "probe-restart"
    output.mkdir(parents=True, exist_ok=True)
    point = next(
        point
        for point in json.loads((directory / "operating_points.json").read_text())
        if point["name"] == "knn_top10_unfiltered@r95" and point["reached"]
    )
    library_dir = subprocess.check_output(
        ["/usr/lib/postgresql/18/bin/pg_config", "--pkglibdir"], text=True
    ).strip()
    installed = Path(library_dir) / "pg_search.so"
    tunables = [
        value
        for value in os.environ.get("GLIBC_TUNABLES", "").split(":")
        if value and not value.startswith("glibc.malloc.hugetlb=")
    ]
    modes = [("small", "1", False), ("inflated", "1", True), ("disabled", "0", None)]
    rows = []
    try:
        for iteration, order in enumerate([modes, modes[::-1], modes[1:] + modes[:1]]):
            for name, hugetlb, inflated in order:
                destination = output / f"round-{iteration}" / name
                env = os.environ | {
                    "GLIBC_TUNABLES": ":".join(
                        tunables + [f"glibc.malloc.hugetlb={hugetlb}"]
                    )
                }
                attempts = []
                for attempt in range(12 if inflated is not None else 1):
                    subprocess.run(
                        ["cargo", "pgrx", "stop", "pg18"],
                        cwd=root / "pg_search",
                        check=True,
                    )
                    subprocess.run(
                        ["cargo", "pgrx", "start", "pg18"],
                        cwd=root / "pg_search",
                        env=env,
                        check=True,
                    )
                    pages = heap_pages()
                    attempts.append(pages)
                    record_machine(destination / f"startup-{attempt}", installed)
                    if inflated is None or (pages >= 512) == inflated:
                        break
                print(f"Round {iteration} {name}: heap pages {attempts}", flush=True)
                before = statement_totals()
                measured = measure_k6(
                    root,
                    destination,
                    point,
                    point["session"]
                    | {"paradedb.vector_cluster_max_probe": point["probe"]},
                )
                after = statement_totals()
                record_machine(destination / "after", installed)
                row = {
                    "round": iteration,
                    "mode": name,
                    "hugetlb": hugetlb,
                    "heap_pages": pages,
                    "attempts": attempts,
                    "selected": inflated is None or (pages >= 512) == inflated,
                    "server": {
                        key: (after[key] or 0) - (before[key] or 0) for key in after
                    },
                } | measured
                rows.append(row)
                (output / "measurements.json").write_text(
                    json.dumps(rows, indent=2) + "\n"
                )
    finally:
        subprocess.run(
            ["cargo", "pgrx", "stop", "pg18"], cwd=root / "pg_search", check=True
        )
        subprocess.run(
            ["cargo", "pgrx", "start", "pg18"], cwd=root / "pg_search", check=True
        )
    lines = [
        "### Same-binary allocator comparison",
        "",
        (
            "Three repetitions per mode, identical unfiltered r95 query and index. "
            "Small/inflated heaps are selected by resident pages before timing "
            "(up to 12 restarts). The disabled arm sets glibc.malloc.hugetlb=0; "
            "PostgreSQL's explicit huge_pages setting is unchanged."
        ),
        "",
        "| Round | Mode | Heap pages | Selected | p50 ms | Plan ms/call | Exec ms/call |",
        "|---:|---|---:|---|---:|---:|---:|",
    ]
    for row in rows:
        server = row["server"]
        calls = server["calls"] or 1
        lines.append(
            f"| {row['round']} | {row['mode']} | {row['heap_pages']} | {row['selected']} | "
            f"{row['p50_ms']:.3f} | {server['plan_ms'] / calls:.3f} | "
            f"{server['exec_ms'] / calls:.3f} |"
        )
    (output / "summary.md").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    run(parser.parse_args().directory.resolve())

"""Measure and profile StackExchange conjunctions against an existing CI index."""

import hashlib
import json
import os
import statistics
import subprocess
import sys
import time
from pathlib import Path

import psycopg  # pylint: disable=import-error

root = Path(sys.argv[1]).resolve()
rows = json.loads((root / "requests.json").read_text())
with psycopg.connect(
    "postgres://postgres:postgres@127.0.0.1:45432/benchmark",
    autocommit=True,
    prepare_threshold=None,
    application_name="bp128_profile",
) as connection:
    connection.execute(
        "SET default_transaction_read_only=on; SET statement_timeout='120s'"
    )
    metadata = {
        "commit": os.environ["PROFILE_COMMIT"],
        "perf_event": os.environ["PERF_EVENT"],
        "settings": connection.execute(
            "SELECT name, setting FROM pg_settings"
        ).fetchall(),
        "segments": connection.execute(
            "SELECT * FROM paradedb.index_info('documents_body_bm25_idx')"
        ).fetchall(),
        "index_bytes": connection.execute(
            "SELECT pg_relation_size('documents_body_bm25_idx')"
        ).fetchone()[0],
    }
    (root / "metadata.json").write_text(json.dumps(metadata, indent=2, default=str))
    for row in rows:
        row["samples_ms"] = []
        row["sql"] = row["request"][0].replace("$1", "%s")
    for cycle in range(5):
        for row in rows:
            start = time.perf_counter_ns()
            result = connection.execute(row["sql"], row["request"][1:]).fetchall()
            row["samples_ms"].append((time.perf_counter_ns() - start) / 1e6)
            row["hits"] = len(result)
            row["result_ids_scores"] = [(r[0], r[2]) for r in result]
            row["result_digest"] = hashlib.sha256(
                json.dumps(row["result_ids_scores"]).encode()
            ).hexdigest()
        (root / "timings.json").write_text(json.dumps(rows, indent=2, default=str))
        print(
            "cycle",
            cycle,
            "mean_ms",
            statistics.mean(r["samples_ms"][-1] for r in rows),
            flush=True,
        )
    sparse = [r for r in rows if r["hits"] < 10]
    dense = [r for r in rows if r["hits"] == 10]
    for name, pool in [("all", rows), ("sparse", sparse), ("dense", dense)]:
        assert pool, name
        with (
            (root / f"{name}.perf.log").open("w") as log,
            subprocess.Popen(
                [
                    "sudo",
                    os.environ["PERF_BIN"],
                    "record",
                    "-a",
                    "-C",
                    "0-7",
                    "-e",
                    os.environ["PERF_EVENT"],
                    "-F",
                    "99",
                    "--call-graph",
                    "dwarf,16384",
                    "-o",
                    str(root / f"{name}.data"),
                    "--",
                    "sleep",
                    "60",
                ],
                stdout=log,
                stderr=subprocess.STDOUT,
            ) as profiler,
        ):
            iteration = 0
            while profiler.poll() is None:
                row = pool[iteration % len(pool)]
                connection.execute(row["sql"], row["request"][1:]).fetchall()
                iteration += 1
            assert profiler.wait() == 0, name
        print("profile", name, "pool", len(pool), "iterations", iteration, flush=True)
    selected = sorted(
        rows, key=lambda r: statistics.median(r["samples_ms"][1:]), reverse=True
    )[:20]
    selected += sorted(
        sparse, key=lambda r: statistics.median(r["samples_ms"][1:]), reverse=True
    )[:20]
    for row in selected:
        row["plan"] = connection.execute(
            "EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS, TIMING OFF, FORMAT JSON) "
            + row["sql"],
            row["request"][1:],
        ).fetchone()[0]
    (root / "timings.json").write_text(json.dumps(rows, indent=2, default=str))

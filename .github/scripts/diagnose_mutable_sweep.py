"""Compare periodic sweeping on/off from identical data on one isolated Linux runner."""

import argparse
import hashlib
import json
import os
import pathlib
import shutil
import statistics
import subprocess
import threading
import time

parser = argparse.ArgumentParser()
parser.add_argument("--source", required=True)
parser.add_argument("--output", required=True)
parser.add_argument("--runtime-ms", type=int, default=300000)
args = parser.parse_args()
assert (
    os.environ.get("GITHUB_ACTIONS") == "true"
    and os.environ.get("RUNNER_OS") == "Linux"
), "Only run on an ephemeral Linux Actions runner"
r = pathlib.Path(args.output).resolve()
r.mkdir(parents=True, exist_ok=True)
source = pathlib.Path(args.source).resolve()
pg = pathlib.Path(subprocess.check_output(["pg_config", "--bindir"], text=True).strip())
stress = pathlib.Path(shutil.which("stressgres"))
psql = [
    str(pg / "psql"),
    "-X",
    "-h",
    "127.0.0.1",
    "-p",
    "28818",
    "-d",
    "stressgres",
    "-At",
    "-v",
    "ON_ERROR_STOP=1",
]


def query(sql):
    """Execute a control query against the current trial cluster."""
    return subprocess.check_output(psql + ["-c", sql], text=True).strip()


def snapshot():
    """Capture heap, index, WAL, worker-call, and SQL execution statistics."""
    return json.loads(
        query("""SELECT jsonb_build_object(
 'lsn',pg_current_wal_insert_lsn(),
 'table',(SELECT to_jsonb(s) FROM pg_stat_user_tables s WHERE relname='wide_table'),
 'wal',(SELECT to_jsonb(s) FROM pg_stat_wal s),
 'index_bytes',pg_relation_size('wide_table_idx'),
 'heap_bytes',pg_relation_size('wide_table'),
 'indexes',(SELECT jsonb_agg(to_jsonb(s)) FROM paradedb.index_info('wide_table_idx') s),
 'statements',(SELECT jsonb_agg(to_jsonb(s)) FROM (SELECT query,calls,total_exec_time,rows,shared_blks_hit,shared_blks_read,shared_blks_written,wal_records,wal_bytes FROM pg_stat_statements WHERE query LIKE '%wide_table%' OR query LIKE '%random_data%' ORDER BY total_exec_time DESC LIMIT 20) s),
 'sweep_calls',(SELECT jsonb_agg(to_jsonb(s)) FROM (SELECT query,calls,total_exec_time FROM pg_stat_statements WHERE query LIKE '%extversion%' OR query LIKE '%array_agg(i.indexrelid%' ORDER BY calls DESC) s),
 'settings',(SELECT jsonb_object_agg(name,setting) FROM pg_settings WHERE name IN ('paradedb.mutable_sweep_interval','paradedb.global_target_segment_count','max_worker_processes','max_parallel_workers','shared_buffers','jit')))
 """)
    )


def sample(path, stop, failed):
    """Observe transactions and lock waits without touching index metadata."""
    env = os.environ.copy()
    env["PGAPPNAME"] = "sweep_observer"
    sql = """SELECT jsonb_build_object(
      'time',clock_timestamp(),
      'activity',(SELECT jsonb_agg(jsonb_build_object(
        'pid',pid,'type',backend_type,'state',state,
        'wait_type',wait_event_type,'wait',wait_event,
        'query',left(query,160),'xmin',backend_xmin,'xact',xact_start))
        FROM pg_stat_activity WHERE application_name<>'sweep_observer'
        AND backend_type NOT IN ('checkpointer','background writer',
          'walwriter','autovacuum launcher','logical replication launcher')),
      'waiting_locks',(SELECT count(*) FROM pg_locks WHERE NOT granted));
    """
    with (
        (path.parent / "sampler-error.log").open("w") as errors,
        subprocess.Popen(
            psql,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=errors,
            text=True,
            env=env,
        ) as process,
    ):
        try:
            with path.open("w") as output:
                while not stop.is_set():
                    process.stdin.write(sql + "\n")
                    process.stdin.flush()
                    line = process.stdout.readline()
                    assert line
                    output.write(line)
                    stop.wait(0.2)
        except Exception:
            failed.set()
            raise
        finally:
            process.stdin.close()
            process.wait(timeout=5)


def run(name, interval, pool=8, ms=300000):
    """Run and verify one independently restored trial, preserving every artifact."""
    # pylint: disable=too-many-locals,too-many-statements
    case = r / name
    case.mkdir()
    data = case / "pgdata"
    subprocess.run(
        ["cp", "-a", "--reflink=auto", str(r / "seed"), str(data)], check=True
    )
    with (data / "postgresql.auto.conf").open("a") as f:
        f.write(
            f"\nmax_worker_processes={pool}\nparadedb.mutable_sweep_interval={interval}\n"
        )
    ctl = [str(pg / "pg_ctl"), "-D", str(data)]
    subprocess.run(ctl + ["-l", str(case / "postgres.log"), "-w", "start"], check=True)
    try:
        query("SELECT pg_stat_reset(); SELECT pg_stat_statements_reset();")
        before = snapshot()
        (case / "before.json").write_text(json.dumps(before, indent=2) + "\n")
        stop = threading.Event()
        failed = threading.Event()
        sampler = threading.Thread(
            target=sample, args=(case / "samples.jsonl", stop, failed)
        )
        sampler.start()
        try:
            with (case / "driver.log").open("w") as f:
                start = time.monotonic()
                with subprocess.Popen(
                    [
                        str(stress),
                        "headless",
                        str(r / "wide.toml"),
                        "--skip-setup",
                        "--runtime",
                        str(ms),
                        "--log-interval-ms",
                        "10",
                        "--log-file",
                        str(case / "metrics.jsonl"),
                    ],
                    stdout=f,
                    stderr=subprocess.STDOUT,
                ) as process:
                    try:
                        process.wait(timeout=ms / 1000 + 90)
                    except subprocess.TimeoutExpired:
                        process.terminate()
                        process.wait(timeout=10)
                        raise
                    elapsed = time.monotonic() - start
                    assert process.returncode == 0, (name, process.returncode)
        finally:
            stop.set()
            sampler.join(timeout=5)
            assert not failed.is_set() and not sampler.is_alive(), "sampler failed"
        time.sleep(1)
        after = snapshot()
        (case / "after.json").write_text(json.dumps(after, indent=2) + "\n")
        (case / "explain.txt").write_text(
            query(
                "EXPLAIN (VERBOSE,COSTS OFF) SELECT COUNT(*) FROM (SELECT * FROM wide_table "
                "WHERE field_1='123e4567-e89b-12d3-a456-426614174000'::uuid "
                "AND bool_1=true AND number_3 BETWEEN 100 AND 900 AND timestamp_3 IS NULL "
                "ORDER BY date_1 DESC,timestamp_1 DESC,id DESC LIMIT 100) s"
            )
        )
        heap = query("SELECT count(*),sum(id) FROM wide_table")
        indexed = query(
            "SELECT count(*),sum(id) FROM wide_table WHERE id @@@ pdb.all()"
        )
        assert heap == indexed, (heap, indexed)
        query("VACUUM (INDEX_CLEANUP ON) wide_table")
        verify = json.loads(
            query(
                "SELECT jsonb_agg(to_jsonb(s)) FROM "
                "pdb.verify_index('wide_table_idx',heapallindexed=>true) s"
            )
        )
        (case / "verification.json").write_text(
            json.dumps({"heap": heap, "indexed": indexed, "verify": verify}, indent=2)
            + "\n"
        )
        assert all(x["passed"] for x in verify), verify
        tps = {}
        sizes = []
        for line in (case / "metrics.jsonl").open():
            x = json.loads(line)
            m = x["metrics"]
            dur = x["duration"]
            secs = dur["secs"] + dur["nanos"] / 1e9
            if secs >= 10 and "tps" in m:
                tps.setdefault(x["job_title"], []).append(m["tps"])
            if secs >= 10 and "segment_count" in m:
                sizes.append(m["segment_count"])
        delta = {
            k: after["table"][k] - before["table"][k]
            for k in ["n_tup_ins", "n_tup_upd", "n_tup_hot_upd", "n_tup_newpage_upd"]
        }
        summary = {
            "name": name,
            "interval_ms": interval,
            "worker_pool": pool,
            "runtime_ms": ms,
            "elapsed": elapsed,
            "tps": {k: statistics.median(v) for k, v in tps.items()},
            "table_delta": delta,
            "hot_pct": 100 * delta["n_tup_hot_upd"] / delta["n_tup_upd"],
            "wal_bytes": after["wal"]["wal_bytes"] - before["wal"]["wal_bytes"],
            "index_bytes_after": after["index_bytes"],
            "median_segments": statistics.median(sizes),
            "verification": "PASS",
            "updated_rows_per_requested_second": delta["n_tup_upd"] / (ms / 1000),
            "sweep_calls": after["sweep_calls"],
        }
        (case / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
        print(json.dumps(summary), flush=True)
    finally:
        subprocess.run(ctl + ["-m", "fast", "-w", "stop"], check=True)


def prepare():
    """Build one seeded wide-table database and checkpoint it for every trial."""
    data = pathlib.Path.home() / ".pgrx/data-18"
    assert data.is_dir()
    ctl = [str(pg / "pg_ctl"), "-D", str(data)]
    admin = psql.copy()
    admin[admin.index("stressgres")] = "postgres"
    subprocess.run(
        admin
        + [
            "-c",
            "ALTER SYSTEM SET shared_preload_libraries='pg_search,pg_stat_statements'",
        ],
        check=True,
    )
    subprocess.run(
        admin + ["-c", "ALTER SYSTEM SET paradedb.mutable_sweep_interval=0"], check=True
    )
    subprocess.run(ctl + ["-m", "fast", "-w", "stop"], check=True)
    subprocess.run(
        ctl + ["-l", str(r / "prepare-postgres.log"), "-w", "start"], check=True
    )
    try:
        subprocess.run(admin + ["-c", "CREATE DATABASE stressgres"], check=True)
        query("CREATE EXTENSION pg_stat_statements")
        query("ALTER SYSTEM SET pg_stat_statements.track='all'")
        query("SELECT pg_reload_conf()")
        suite = (source / "stressgres/suites/wide-table.toml").read_text()
        suite = suite.replace(
            "[server.style.Automatic]",
            '[server.style.With]\nconnection_string = "postgresql://localhost:28818/stressgres"',
            1,
        )
        (r / "wide.toml").write_text(suite)
        with (r / "setup.log").open("w") as f:
            subprocess.run(
                [str(stress), "headless", str(r / "wide.toml"), "--setup-only"],
                stdout=f,
                stderr=subprocess.STDOUT,
                check=True,
            )
        (r / "seed-index-info.json").write_text(
            query(
                "SELECT jsonb_agg(to_jsonb(s)) FROM paradedb.index_info('wide_table_idx') s"
            )
        )
        (r / "provenance.json").write_text(
            json.dumps(
                {
                    "source": subprocess.check_output(
                        ["git", "rev-parse", "HEAD"], cwd=source, text=True
                    ).strip(),
                    "suite_sha256": hashlib.sha256(
                        (source / "stressgres/suites/wide-table.toml").read_bytes()
                    ).hexdigest(),
                    "version": query("SELECT version()"),
                    "settings": snapshot()["settings"],
                    "runtime_ms": args.runtime_ms,
                    "seed_rows": query("SELECT count(*) FROM wide_table"),
                    "hardware": subprocess.check_output(["lscpu"], text=True),
                },
                indent=2,
            )
        )
    finally:
        subprocess.run(ctl + ["-m", "fast", "-w", "stop"], check=True)
    subprocess.run(
        ["cp", "-a", "--reflink=auto", str(data), str(r / "seed")], check=True
    )


if __name__ == "__main__":
    prepare()
    for CASE_NAME, INTERVAL in [
        ("off-a", 0),
        ("on-a", 1000),
        ("on-b", 1000),
        ("off-b", 0),
    ]:
        run(CASE_NAME, INTERVAL, ms=args.runtime_ms)

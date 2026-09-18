"""Check exact COUNT results and the reader/VACUUM visibility barrier."""

import json
import os
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import psycopg  # pylint: disable=import-error

ROOT = Path(__file__).resolve().parent
DSN = os.environ["BENCHMARK_DSN"]
TABLE = "hn_count_visibility_fixture.docs"
CASES = (
    ("term", "body ||| 'alpha'", "body LIKE '%alpha%'"),
    ("or", "body ||| 'alpha beta'", "body LIKE '%alpha%' OR body LIKE '%beta%'"),
    ("and", "body &&& 'alpha beta'", "body LIKE '%alpha%' AND body LIKE '%beta%'"),
    ("phrase", "body @@@ pdb.phrase('alpha beta')", "body LIKE '%alpha beta%'"),
    ("absent", "body ||| 'absenttoken'", "body LIKE '%absenttoken%'"),
)
records = []
started = time.monotonic()


def save():
    """Persist completed comparisons and elapsed time."""
    (ROOT / "count-visible-fixture.json").write_text(
        json.dumps(
            {"seconds": time.monotonic() - started, "records": records}, indent=2
        )
        + "\n"
    )


def compare(conn, phase, cases=CASES):
    """Compare both COUNT modes against independent SQL predicates."""
    for name, predicate, oracle_predicate in cases:
        expected = conn.execute(
            f"SELECT count(*) FROM {TABLE} WHERE {oracle_predicate}"
        ).fetchone()[0]
        sql = f"SELECT count(*) FROM {TABLE} WHERE {predicate}"
        for enabled in [False, True]:
            conn.execute(
                "SELECT set_config('paradedb.experiment_count_all_visible', %s, false)",
                (str(enabled),),
            )
            actual = conn.execute(sql).fetchone()[0]
            assert actual == expected, (phase, name, enabled, expected, actual)
            plan = conn.execute(
                "EXPLAIN (ANALYZE, BUFFERS, TIMING OFF, FORMAT JSON) " + sql
            ).fetchone()[0][0]
            records.append(
                {
                    "phase": phase,
                    "name": name,
                    "enabled": enabled,
                    "count": actual,
                    "expected": expected,
                    "plan": plan,
                }
            )
            save()
    print(phase, "passed", len(cases) * 2, "comparisons", flush=True)


def vacuum_barrier(conn):
    """Verify VACUUM waits while a cursor retains its older index reader."""
    conn.execute(f"DELETE FROM {TABLE} WHERE id BETWEEN 200 AND 209")
    with (
        psycopg.connect(DSN, autocommit=True, prepare_threshold=None) as held,
        psycopg.connect(DSN, autocommit=True, prepare_threshold=None) as vacuum,
    ):
        held.execute("SET statement_timeout='5s'")
        held.execute("SET max_parallel_workers_per_gather=0")
        vacuum.execute("SET statement_timeout='5s'")
        pid = vacuum.execute("SELECT pg_backend_pid()").fetchone()[0]
        held.execute("BEGIN")
        held.execute(
            f"DECLARE held_reader NO SCROLL CURSOR FOR SELECT body FROM {TABLE} "
            "WHERE body ||| 'alpha beta'"
        )
        held.execute("FETCH 1 FROM held_reader").fetchone()
        observed = False
        with ThreadPoolExecutor(max_workers=1) as pool:
            pending = pool.submit(vacuum.execute, f"VACUUM (ANALYZE) {TABLE}")
            try:
                deadline = time.monotonic() + 3
                while time.monotonic() < deadline and not pending.done():
                    state = conn.execute(
                        "SELECT wait_event_type, wait_event FROM pg_stat_activity WHERE pid=%s",
                        (pid,),
                    ).fetchone()
                    if state and "BufferPin" in state:
                        observed = True
                        break
                    time.sleep(0.01)
            finally:
                held.execute("CLOSE held_reader")
                held.execute("ROLLBACK")
            pending.result(timeout=5)
        records.append(
            {
                "phase": "reader_vacuum_cleanup_barrier",
                "observed_buffer_pin_wait": observed,
            }
        )
        save()
        assert observed, (
            "VACUUM did not visibly wait for the held reader; "
            "inspect before claiming barrier coverage"
        )
    compare(conn, "after_concurrent_vacuum")


with psycopg.connect(DSN, autocommit=True, prepare_threshold=None) as fixture_conn:
    fixture_conn.execute("SET statement_timeout='5s'")
    fixture_conn.execute("SET max_parallel_workers_per_gather=0")
    fixture_conn.execute("CREATE SCHEMA IF NOT EXISTS hn_count_visibility_fixture")
    exists = fixture_conn.execute("SELECT to_regclass(%s)", (TABLE,)).fetchone()[0]
    assert exists is None, "fixture already exists; inspect it before rerunning"
    fixture_conn.execute(
        f"CREATE TABLE {TABLE} (id bigint PRIMARY KEY, body text) WITH (autovacuum_enabled=false)"
    )
    fixture_conn.execute(
        f"INSERT INTO {TABLE} SELECT i, CASE i%4 "
        "WHEN 0 THEN 'alpha beta' WHEN 1 THEN 'alpha gamma' "
        "WHEN 2 THEN 'beta delta' ELSE 'epsilon' END "
        "FROM generate_series(1,20000)i"
    )
    fixture_conn.execute(
        f"CREATE INDEX docs_bm25 ON {TABLE} USING bm25(id,body) "
        "WITH(key_field='id', target_segment_count=4)"
    )
    assert (
        fixture_conn.execute(
            "SELECT count(*) FROM pg_settings WHERE name='paradedb.experiment_count_all_visible'"
        ).fetchone()[0]
        == 1
    )
    fixture_conn.execute(f"VACUUM (ANALYZE) {TABLE}")
    compare(fixture_conn, "all_visible")
    fixture_conn.execute(f"DELETE FROM {TABLE} WHERE id <= 17")
    compare(fixture_conn, "committed_delete_nonvisible")
    fixture_conn.execute(f"VACUUM (ANALYZE) {TABLE}")
    compare(fixture_conn, "vacuum_tombstones")
    vacuum_barrier(fixture_conn)

    with psycopg.connect(DSN, autocommit=True, prepare_threshold=None) as old:
        old.execute("SET statement_timeout='5s'")
        old.execute("SET max_parallel_workers_per_gather=0")
        old.execute("BEGIN ISOLATION LEVEL REPEATABLE READ")
        old.execute(f"SELECT count(*) FROM {TABLE}").fetchone()
        fixture_conn.execute(
            f"UPDATE {TABLE} SET body='alpha beta' WHERE id BETWEEN 20 AND 29"
        )
        fixture_conn.execute(f"DELETE FROM {TABLE} WHERE id BETWEEN 30 AND 39")
        fixture_conn.execute(f"INSERT INTO {TABLE} VALUES (30001,'alpha beta')")
        compare(old, "old_snapshot_after_other_writes")
        compare(fixture_conn, "fresh_snapshot_after_other_writes")
        fixture_conn.execute(f"VACUUM (ANALYZE) {TABLE}")
        compare(old, "old_snapshot_after_vacuum")
        old.execute("ROLLBACK")
    fixture_conn.execute(f"VACUUM (ANALYZE) {TABLE}")
    compare(fixture_conn, "mutable_after_vacuum")
    fixture_conn.execute("BEGIN")
    fixture_conn.execute(
        f"UPDATE {TABLE} SET body='epsilon' WHERE id BETWEEN 100 AND 109"
    )
    fixture_conn.execute(f"DELETE FROM {TABLE} WHERE id BETWEEN 110 AND 119")
    compare(fixture_conn, "own_transaction_writes")
    fixture_conn.execute("ROLLBACK")
    fixture_conn.execute("REINDEX INDEX hn_count_visibility_fixture.docs_bm25")
    fixture_conn.execute(f"VACUUM (ANALYZE) {TABLE}")
    compare(fixture_conn, "reindexed_all_visible")
    fixture_conn.execute(f"DELETE FROM {TABLE}")
    fixture_conn.execute(f"VACUUM (ANALYZE) {TABLE}")
    compare(fixture_conn, "all_deleted")
    print(
        "Fixture retained at",
        TABLE,
        "; seconds",
        round(time.monotonic() - started, 3),
        flush=True,
    )

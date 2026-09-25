// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

//! A die (`pg_terminate_backend`, or the SIGTERM a parallel query's leader sends its workers on
//! cancel) acted on in the middle of a Rust call runs exit cleanup without unwinding, so that
//! cleanup must not drop scan state the call is still using: a tokio runtime inside `block_on`, a
//! scorer still being built. Dropping it panics into a second FATAL, and an assert-enabled server
//! aborts and resets the cluster. This test signals running scans from another connection and
//! checks that a witness connection open throughout survives.

use anyhow::Result;
use rstest::*;
use sqlx::{AssertSqlSafe, Executor, PgConnection};
use std::time::{Duration, Instant};
use tests::fixtures::*;
use tokio::time::sleep;

// Many small segments and selective queries, so a signal usually lands mid-scan.
const SEGMENTS: usize = 100;

const SETUP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
CREATE TABLE bs_cancel (id bigserial primary key, body text, grp int, extra int);
CREATE INDEX bs_cancel_idx ON bs_cancel USING bm25 (id, body, grp);
-- A heap-filter predicate that checks for interrupts itself, like any function calling pg_sleep.
CREATE FUNCTION bs_cancel_sleepy(x int) RETURNS bool LANGUAGE plpgsql IMMUTABLE AS
$$ BEGIN PERFORM pg_sleep(0); RETURN x = -1; END $$;
"#;

const INSERT_SEGMENT: &str = r#"
INSERT INTO bs_cancel (body, grp, extra)
SELECT md5(random()::text) || ' ' || md5(random()::text), g % 10, g
FROM generate_series(1, 2000) AS g
"#;

const PARALLEL_GUCS: &str = r#"
SET max_parallel_workers_per_gather TO 2;
SET max_parallel_workers TO 8;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
SET paradedb.min_rows_per_worker TO 0;
"#;

// Plans the serial cases as NormalScan (Aggregate Scan off), which runs no tokio runtime.
const SERIAL_GUCS: &str = r#"
SET max_parallel_workers_per_gather TO 0;
SET paradedb.enable_aggregate_custom_scan TO off;
"#;

struct Case {
    name: &'static str,
    gucs: &'static str,
    query: &'static str,
    /// Every line must appear in the query's EXPLAIN, so the case runs the scan it's meant to.
    plan_contains: &'static [&'static str],
}

// Each query matches nothing, so every segment is walked: md5 bodies are hex, and `extra` is
// never negative. `extra` is not indexed, so it is evaluated per doc as a heap filter. In the last
// case the heap filter's own `pg_sleep` acts on the die while the scorer is still being built.
static CASES: [Case; 4] = [
    Case {
        name: "regex",
        gucs: PARALLEL_GUCS,
        query: "SELECT grp FROM bs_cancel WHERE id @@@ paradedb.regex('body', '.*q.*z.*') AND grp = 1",
        plan_contains: &[
            "Parallel Custom Scan (ParadeDB Base Scan)",
            "ColumnarExecState",
            "regex",
        ],
    },
    Case {
        name: "heapfilter",
        gucs: PARALLEL_GUCS,
        query: "SELECT grp FROM bs_cancel WHERE id @@@ paradedb.all() AND extra = -1",
        plan_contains: &[
            "Parallel Custom Scan (ParadeDB Base Scan)",
            "ColumnarExecState",
            "heap_filter",
        ],
    },
    Case {
        name: "serial_heapfilter",
        gucs: SERIAL_GUCS,
        query: "SELECT count(*) FROM bs_cancel WHERE id @@@ paradedb.all() AND extra = -1",
        plan_contains: &[
            "Custom Scan (ParadeDB Base Scan)",
            "NormalScanExecState",
            "heap_filter",
        ],
    },
    Case {
        name: "serial_heapfilter_sleep",
        gucs: SERIAL_GUCS,
        query: "SELECT count(*) FROM bs_cancel WHERE id @@@ paradedb.all() AND bs_cancel_sleepy(extra)",
        plan_contains: &[
            "Custom Scan (ParadeDB Base Scan)",
            "NormalScanExecState",
            "heap_filter",
        ],
    },
];

// A scan runs for tens of milliseconds, so a longer wait would mostly land between queries.
const SIGNAL_DELAY: Duration = Duration::from_millis(20);
const ATTEMPTS: usize = 3;
const TARGET_LOOP_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
enum Signal {
    Cancel,
    Terminate,
}

impl Signal {
    fn name(self) -> &'static str {
        match self {
            Signal::Cancel => "cancel",
            Signal::Terminate => "terminate",
        }
    }

    fn function(self) -> &'static str {
        match self {
            Signal::Cancel => "pg_cancel_backend",
            Signal::Terminate => "pg_terminate_backend",
        }
    }

    /// The SQLSTATE the target's query must end with. A crash-triggered cluster reset reports
    /// `57P02` (crash_shutdown) instead, which fails the test.
    fn sqlstate(self) -> &'static str {
        match self {
            Signal::Cancel => "57014",    // query_canceled
            Signal::Terminate => "57P01", // admin_shutdown
        }
    }
}

async fn assert_plan(conn: &mut PgConnection, case: &Case) -> Result<()> {
    conn.execute(case.gucs).await?;
    let rows: Vec<(String,)> = sqlx::query_as(AssertSqlSafe(format!(
        "EXPLAIN (COSTS OFF, VERBOSE) {}",
        case.query
    )))
    .fetch_all(&mut *conn)
    .await?;
    let explain = rows
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n");
    for needle in case.plan_contains {
        assert!(
            explain.contains(needle),
            "Base Scan cancel test query must plan with `{needle}`:\n{explain}"
        );
    }
    Ok(())
}

/// Each time `target_app`'s backend shows up running the scan, wait briefly and send it `signal`,
/// until the target's loop ends. A cancel that lands between two queries finds the backend idle
/// and is ignored, so a single signal isn't enough when each query is this short.
async fn signal_until_stopped(
    signaller: &mut PgConnection,
    target_app: &str,
    signal: Signal,
    target: &tokio::task::JoinHandle<Result<()>>,
) -> Result<()> {
    let signal_fn = signal.function();
    let deadline = Instant::now() + TARGET_LOOP_TIMEOUT;
    while !target.is_finished() {
        if Instant::now() >= deadline {
            anyhow::bail!("{signal_fn} never stopped the target within {TARGET_LOOP_TIMEOUT:?}");
        }
        // Parallel workers inherit the leader's `application_name`, so pin to the client backend
        // to signal the leader; its abort is what sends SIGTERM to the workers.
        let pid: Option<i32> = sqlx::query_scalar(
            "SELECT pid FROM pg_stat_activity \
             WHERE application_name = $1 AND backend_type = 'client backend' \
             AND state = 'active' AND query LIKE '%FROM bs_cancel%'",
        )
        .bind(target_app)
        .fetch_optional(&mut *signaller)
        .await?;

        if let Some(pid) = pid {
            sleep(SIGNAL_DELAY).await;
            // The backend may have exited during the wait; signalling it only while it's still
            // listed avoids a "not a PostgreSQL backend process" warning.
            sqlx::query(AssertSqlSafe(format!(
                "SELECT {signal_fn}(pid) FROM pg_stat_activity WHERE pid = $1"
            )))
            .bind(pid)
            .execute(&mut *signaller)
            .await?;
        }
        sleep(Duration::from_millis(5)).await;
    }
    Ok(())
}

/// Run the case's scan in a loop on `target` until `signal` cuts it off, and check it ended with
/// the signal's SQLSTATE. `signal_until_stopped` bounds the loop.
async fn run_until_signalled(
    mut target: PgConnection,
    app_name: String,
    case: &'static Case,
    signal: Signal,
) -> Result<()> {
    target
        .execute(AssertSqlSafe(format!(
            "SET application_name = '{app_name}';"
        )))
        .await?;
    target.execute(case.gucs).await?;

    let err = loop {
        if let Err(e) = target.execute(case.query).await {
            break e;
        }
    };

    let ok = match &err {
        sqlx::Error::Database(db) => db.code().as_deref() == Some(signal.sqlstate()),
        // A terminate may surface as the connection closing under us instead of the FATAL.
        sqlx::Error::Io(_) => matches!(signal, Signal::Terminate),
        _ => false,
    };
    anyhow::ensure!(
        ok,
        "expected SQLSTATE {}, target ended with: {err}",
        signal.sqlstate()
    );
    Ok(())
}

async fn assert_cluster_alive(witness: &mut PgConnection) -> Result<()> {
    // A worker crash makes the postmaster reset the cluster, which would have killed this
    // long-lived connection. If it still answers, no reset happened.
    let one: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&mut *witness)
        .await?;
    assert_eq!(one, 1, "witness connection lost: a backend crashed");
    Ok(())
}

#[rstest]
#[tokio::test]
async fn basescan_signal_does_not_crash_workers(database: Db) -> Result<()> {
    let mut setup = database.connection().await;
    setup.execute(SETUP_SQL).await?;
    setup
        .execute("SET paradedb.global_mutable_segment_rows = 0")
        .await?;
    for _ in 0..SEGMENTS {
        setup.execute(INSERT_SEGMENT).await?;
    }
    setup
        .execute("RESET paradedb.global_mutable_segment_rows")
        .await?;
    setup.execute("ANALYZE bs_cancel").await?;

    let mut witness = database.connection().await;
    assert_cluster_alive(&mut witness).await?;
    for case in &CASES {
        assert_plan(&mut setup, case).await?;
    }

    for case in &CASES {
        for signal in [Signal::Cancel, Signal::Terminate] {
            for attempt in 0..ATTEMPTS {
                let app_name = format!("bs_{}_{}_{attempt}", case.name, signal.name());
                let target = database.connection().await;
                let loop_handle =
                    tokio::spawn(run_until_signalled(target, app_name.clone(), case, signal));

                let mut signaller = database.connection().await;
                signal_until_stopped(&mut signaller, &app_name, signal, &loop_handle).await?;

                let outcome = loop_handle.await?;
                // Check the witness first so a crash reports as a crash, not as the target's error.
                assert_cluster_alive(&mut witness).await?;
                outcome?;
            }
        }
    }

    Ok(())
}

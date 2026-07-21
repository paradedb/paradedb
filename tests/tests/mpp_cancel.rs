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

//! A cancel or `pg_terminate_backend` taken while an MPP query runs used to
//! crash the leader during teardown (a die serviced from inside the tokio
//! `block_on`, or DSM senders dropped after the parallel segment was gone),
//! which resets the whole cluster. These tests signal a running MPP query from
//! a second connection and assert a witness connection kept open the whole
//! time survives. A cluster reset would have dropped it.

use anyhow::Result;
use rstest::*;
use sqlx::{Executor, PgConnection};
use std::time::{Duration, Instant};
use tests::fixtures::*;
use tokio::time::sleep;

const SETUP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE mpp_sig_files (id SERIAL PRIMARY KEY, title TEXT, content TEXT);
CREATE TABLE mpp_sig_pages (id SERIAL PRIMARY KEY, file_id INTEGER, page_text TEXT);

INSERT INTO mpp_sig_files (title, content)
SELECT 'file-' || g, 'Section ' || g || ' has content for testing'
FROM generate_series(1, 200) AS g;

INSERT INTO mpp_sig_pages (file_id, page_text)
SELECT (g % 200) + 1, 'Page text for page ' || g
FROM generate_series(1, 200000) AS g;

CREATE INDEX mpp_sig_files_idx ON mpp_sig_files
USING bm25 (id, title, content)
WITH (key_field='id', text_fields='{"title": {"fast": true}, "content": {}}');

CREATE INDEX mpp_sig_pages_idx ON mpp_sig_pages
USING bm25 (id, file_id, page_text)
WITH (key_field='id', numeric_fields='{"file_id": {"fast": true}}', text_fields='{"page_text": {}}');

ANALYZE mpp_sig_files;
ANALYZE mpp_sig_pages;
"#;

// Forces the join through MPP and zeroes the parallel costs so the planner always picks it.
const MPP_GUCS: &str = r#"
SET paradedb.enable_join_custom_scan TO on;
SET paradedb.mpp_worker_count TO 4;
SET max_parallel_workers_per_gather TO 4;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
"#;

// Streams 200k rows out of an MPP join. No sort/aggregate, so it stays under `work_mem`
// (the point here is a live query to signal, not an overflow), but it runs long enough that
// the killer reliably catches the backend mid-execution.
const MPP_QUERY: &str = r#"
SELECT f.title, p.page_text
FROM mpp_sig_files f JOIN mpp_sig_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
"#;

/// Spin until `victim_app`'s backend is running the MPP query, then run `signal_fn(pid)`
/// (`pg_cancel_backend` or `pg_terminate_backend`) on it. Returns the pid that was signalled.
async fn signal_running_mpp_backend(
    killer: &mut PgConnection,
    victim_app: &str,
    signal_fn: &str,
) -> Result<i32> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        // Parallel workers inherit the leader's `application_name`, so pin to the client backend
        // to signal the leader (the connection running the top-level query), not a worker.
        let pid: Option<i32> = sqlx::query_scalar(
            "SELECT pid FROM pg_stat_activity \
             WHERE application_name = $1 AND backend_type = 'client backend' \
             AND state = 'active' AND query LIKE '%mpp_sig_pages%'",
        )
        .bind(victim_app)
        .fetch_optional(&mut *killer)
        .await?;

        if let Some(pid) = pid {
            sqlx::query(&format!("SELECT {signal_fn}($1)"))
                .bind(pid)
                .execute(&mut *killer)
                .await?;
            return Ok(pid);
        }
        if Instant::now() >= deadline {
            anyhow::bail!("victim backend never showed up active running the MPP query");
        }
        sleep(Duration::from_millis(5)).await;
    }
}

/// Run the MPP query in a loop on `victim` until its connection or query is cut off. The
/// loop keeps the backend busy so the killer has a wide window to land the signal mid-query.
async fn run_until_signalled(mut victim: PgConnection, app_name: &str) {
    if victim
        .execute(format!("SET application_name = '{app_name}';").as_str())
        .await
        .is_err()
    {
        return;
    }
    if victim.execute(MPP_GUCS).await.is_err() {
        return;
    }
    // A terminate drops the connection and a cancel errors the in-flight query; either way
    // the next iteration's error ends the loop.
    while victim.execute(MPP_QUERY).await.is_ok() {}
}

async fn assert_cluster_alive(witness: &mut PgConnection) -> Result<()> {
    // A backend crash during teardown makes the postmaster reset the cluster, which would have
    // killed this long-lived connection. If it still answers, no reset happened.
    let one: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&mut *witness)
        .await?;
    assert_eq!(one, 1, "witness connection lost: the backend crashed");
    Ok(())
}

#[rstest]
#[tokio::test]
async fn mpp_signal_does_not_crash_backend(database: Db) -> Result<()> {
    let mut setup = database.connection().await;
    setup.execute(SETUP_SQL).await?;

    // Opened before any signal and reused after each, so a cluster reset would surface as a
    // failure on this connection.
    let mut witness = database.connection().await;
    witness
        .execute("CREATE EXTENSION IF NOT EXISTS pg_search;")
        .await?;
    assert_cluster_alive(&mut witness).await?;

    for (app, signal_fn) in [
        ("mpp_sig_cancel", "pg_cancel_backend"),
        ("mpp_sig_terminate", "pg_terminate_backend"),
    ] {
        let victim = database.connection().await;
        let loop_handle = tokio::spawn(run_until_signalled(victim, app));

        let mut killer = database.connection().await;
        signal_running_mpp_backend(&mut killer, app, signal_fn).await?;

        let _ = loop_handle.await;
        assert_cluster_alive(&mut witness).await?;
    }

    Ok(())
}

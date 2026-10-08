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

use rstest::*;
use sqlx::Executor;
use std::time::{Duration, Instant};
use tests::fixtures::*;

/// A background merge holds the index's cleanup lock as a lock-manager page lock in
/// `ShareLock` mode, so it shows up in `pg_locks`, and a VACUUM that has to wait for it waits
/// in the lock manager, where `lock_timeout` applies and the wait can be cancelled.
#[rstest]
#[tokio::test]
async fn background_merge_holds_cleanup_page_lock(database: Db) -> anyhow::Result<()> {
    let mut conn = database.connection().await;
    let mut vacuum = database.connection().await;
    conn.execute(
        r#"
        CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
        DROP TABLE IF EXISTS cleanup_page_lock;
        CREATE TABLE cleanup_page_lock (id bigint, body text);
        CREATE INDEX cleanup_page_lock_idx ON cleanup_page_lock USING paradedb (id, body)
            WITH (background_layer_sizes = '1mb,10mb,100mb,1gb', target_segment_count = 1);
        "#,
    )
    .await?;

    // Each insert adds a segment; once enough accumulate the insert path launches a background
    // merger, which holds the cleanup lock shared for the length of the merge. The delete before
    // each insert leaves dead tuples older than any merger the insert launches, so a VACUUM
    // started while that merger runs has index work to do and must take the lock exclusive.
    let page_lock = "SELECT count(*) FROM pg_locks l JOIN pg_stat_activity a USING (pid) \
         WHERE l.locktype = 'page' AND l.relation = 'cleanup_page_lock_idx'::regclass \
           AND l.mode = 'ShareLock' AND l.granted AND a.backend_type LIKE 'background merger%'";
    let mut merger_lock_seen = false;
    let mut vacuum_waited = false;
    'batches: for _ in 0..8 {
        conn.execute("DELETE FROM cleanup_page_lock WHERE id % 50 = 0")
            .await?;
        conn.execute(
            "INSERT INTO cleanup_page_lock \
             SELECT i, 'alpha beta gamma ' || md5(i::text) || ' ' || md5((i * 7)::text) \
             FROM generate_series(1, 150000) s(i)",
        )
        .await?;
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            let (locks,): (i64,) = sqlx::query_as(page_lock).fetch_one(&mut conn).await?;
            if locks > 0 {
                merger_lock_seen = true;
                // lock_timeout only applies to lock-manager waits, so timing out here proves the
                // wait is interruptible; the merger may also finish first, in which case VACUUM
                // simply succeeds.
                vacuum.execute("SET lock_timeout = '300ms'").await?;
                match vacuum.execute("VACUUM cleanup_page_lock").await {
                    Ok(_) => {}
                    Err(err) => {
                        let code = err.as_database_error().and_then(|e| e.code());
                        assert_eq!(code.as_deref(), Some("55P03"), "{err}");
                        vacuum_waited = true;
                    }
                }
                vacuum.execute("RESET lock_timeout").await?;
                break 'batches;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
    assert!(
        merger_lock_seen,
        "no background merger ever held a ShareLock page lock on the index"
    );
    eprintln!("vacuum timed out behind the merger: {vacuum_waited}");

    // Once every merger is gone the exclusive side is free: VACUUM completes and no page locks
    // remain on the index.
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let (mergers,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM pg_stat_activity WHERE backend_type LIKE 'background merger%'",
        )
        .fetch_one(&mut conn)
        .await?;
        if mergers == 0 {
            break;
        }
        anyhow::ensure!(
            Instant::now() < deadline,
            "background mergers never finished"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    vacuum.execute("VACUUM cleanup_page_lock").await?;
    let (remaining,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM pg_locks WHERE locktype = 'page' \
         AND relation = 'cleanup_page_lock_idx'::regclass",
    )
    .fetch_one(&mut conn)
    .await?;
    assert_eq!(remaining, 0, "page locks left on the index after VACUUM");
    Ok(())
}

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

use anyhow::Result;
use rstest::*;
use sqlx::Executor;
use std::time::{Duration, Instant};
use tests::fixtures::*;

#[rstest]
#[async_std::test]
async fn mutable_flush_batches_without_holding_a_transaction(database: Db) -> Result<()> {
    let mut conn = database.connection().await;
    conn.execute("CREATE EXTENSION IF NOT EXISTS pg_search CASCADE")
        .await?;
    conn.execute("CREATE TABLE batch_flush (id bigint PRIMARY KEY, body text)")
        .await?;
    conn.execute(
        "CREATE INDEX batch_flush_idx ON batch_flush USING paradedb (id, body) \
         WITH (mutable_segment_rows = 10000, target_segment_count = 8)",
    )
    .await?;
    conn.execute("INSERT INTO batch_flush VALUES (1, 'batch')")
        .await?;

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let waiting: Option<(bool, bool, bool)> = sqlx::query_as(
            "SELECT a.xact_start IS NULL, a.backend_xmin IS NULL, \
                    NOT EXISTS (SELECT FROM pg_locks l WHERE l.pid = a.pid \
                                AND l.relation = 'batch_flush_idx'::regclass) \
             FROM pg_stat_activity a WHERE a.datid = (SELECT oid FROM pg_database WHERE datname = current_database()) \
               AND a.backend_type = 'background merger for public.batch_flush_idx' \
               AND a.wait_event_type = 'Extension'",
        ).fetch_optional(&mut conn).await?;
        if let Some(state) = waiting {
            assert_eq!(state, (true, true, true));
            break;
        }
        anyhow::ensure!(
            Instant::now() < deadline,
            "never observed a collection window"
        );
        conn.execute("INSERT INTO batch_flush VALUES (1, 'batch') ON CONFLICT (id) DO UPDATE SET body = EXCLUDED.body").await?;
        async_std::task::sleep(Duration::from_millis(5)).await;
    }

    let until = Instant::now() + Duration::from_secs(2);
    let mut id = 1_i64;
    let mut published_while_writing = false;
    while Instant::now() < until {
        id += 1;
        sqlx::query("INSERT INTO batch_flush VALUES ($1, 'batch')")
            .bind(id)
            .execute(&mut conn)
            .await?;
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM batch_flush WHERE body ||| 'batch'")
                .fetch_one(&mut conn)
                .await?;
        assert_eq!(count, id);
        let published: bool = sqlx::query_scalar("SELECT EXISTS (SELECT FROM paradedb.index_info('batch_flush_idx') WHERE visible AND NOT mutable)").fetch_one(&mut conn).await?;
        published_while_writing |= published;
        async_std::task::sleep(Duration::from_millis(5)).await;
    }
    assert!(
        published_while_writing,
        "continuous writes postponed the flush"
    );

    // The last write must drain without a later insert or VACUUM waking the worker.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let mutable: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM paradedb.index_info('batch_flush_idx') WHERE visible AND mutable",
        )
        .fetch_one(&mut conn)
        .await?;
        if mutable == 0 {
            break;
        }
        anyhow::ensure!(Instant::now() < deadline, "mutable rows did not drain");
        async_std::task::sleep(Duration::from_millis(10)).await;
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM batch_flush WHERE body ||| 'batch'")
        .fetch_one(&mut conn)
        .await?;
    assert_eq!(count, id);
    let passed: Option<bool> = sqlx::query_scalar(
        "SELECT bool_and(passed) FROM pdb.verify_index('batch_flush_idx', heapallindexed => true)",
    )
    .fetch_one(&mut conn)
    .await?;
    assert_eq!(passed, Some(true));
    Ok(())
}

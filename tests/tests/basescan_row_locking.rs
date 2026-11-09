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

//! LockRows above a TopK Base Scan can drop rows (SKIP LOCKED).
//! The scan must keep producing rows past K so the outer Limit is still filled (#7170).

use anyhow::Result;
use rstest::rstest;
use sqlx::{Connection, PgConnection};
use tests::fixtures::*;

const QUERY: &str = "SELECT id, rating FROM topk_locked
     WHERE body === 'alpha' ORDER BY rating DESC LIMIT 2";

async fn setup(database: &Db) -> (PgConnection, PgConnection) {
    let mut writer = database.connection().await;
    "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
     CREATE TABLE topk_locked (id int PRIMARY KEY, body text NOT NULL, rating int NOT NULL);
     INSERT INTO topk_locked SELECT i, 'alpha ' || i, 110 - 10 * i FROM generate_series(1, 8) i;
     CREATE INDEX topk_locked_idx ON topk_locked USING paradedb (id, body, rating);
     ANALYZE topk_locked;"
        .execute(&mut writer);

    let mut reader = database.connection().await;
    "SET max_parallel_workers_per_gather = 0;
     SET enable_indexscan = off;
     SET statement_timeout = '30s';"
        .execute(&mut reader);
    (writer, reader)
}

fn assert_topk_plan(reader: &mut PgConnection, query: &str) {
    let plan: Vec<(String,)> = format!("EXPLAIN (COSTS OFF) {query}").fetch(reader);
    assert!(
        plan.iter()
            .any(|(line,)| line.contains("TopKScanExecState")),
        "{plan:?}"
    );
}

#[rstest]
#[async_std::test]
async fn skip_locked_fills_limit(database: Db) -> Result<()> {
    let (mut writer, mut reader) = setup(&database).await;
    let query = format!("{QUERY} FOR UPDATE SKIP LOCKED");
    assert_topk_plan(&mut reader, &query);

    let mut locker = writer.begin().await?;
    sqlx::query("SELECT id FROM topk_locked WHERE id IN (1, 2) FOR UPDATE")
        .execute(&mut *locker)
        .await?;

    let rows: Vec<(i32, i32)> = query.fetch(&mut reader);
    assert_eq!(rows, vec![(3, 80), (4, 70)]);

    locker.rollback().await?;
    Ok(())
}

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
    // The all-locked case runs the scan to exhaustion; a regression should fail, not hang.
    "SET statement_timeout = '30s';".execute(&mut reader);
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
#[case::top_two(2, vec![(3, 80), (4, 70)])]
// The whole first window is locked, so the scan must query again.
#[case::first_window(6, vec![(7, 40), (8, 30)])]
#[case::all(8, vec![])]
#[async_std::test]
async fn skip_locked_fills_limit(
    database: Db,
    #[case] locked_up_to: i32,
    #[case] expected: Vec<(i32, i32)>,
) -> Result<()> {
    let (mut writer, mut reader) = setup(&database).await;
    let query = format!("{QUERY} FOR UPDATE SKIP LOCKED");
    assert_topk_plan(&mut reader, &query);

    let mut locker = writer.begin().await?;
    sqlx::query("SELECT id FROM topk_locked WHERE id <= $1 FOR UPDATE")
        .bind(locked_up_to)
        .execute(&mut *locker)
        .await?;

    let rows: Vec<(i32, i32)> = query.fetch(&mut reader);
    assert_eq!(rows, expected);

    locker.rollback().await?;
    Ok(())
}

#[rstest]
#[async_std::test]
async fn no_locks_limit_stops_scan(database: Db) {
    let (_writer, mut reader) = setup(&database).await;
    let plan: Vec<(String,)> =
        format!("EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF) {QUERY} FOR UPDATE SKIP LOCKED")
            .fetch(&mut reader);
    let plan = plan
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n");

    // The Limit above still stops the scan after K rows and one query.
    assert!(plan.contains("on topk_locked (actual rows=2"), "{plan}");
    assert!(plan.contains("Queries: 1"), "{plan}");
}

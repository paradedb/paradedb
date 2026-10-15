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

//! Write skew under `SERIALIZABLE`: two transactions read the rows matching a search, then
//! each writes a row the other one read, or would have read. One of them has to abort.

use rstest::*;
use sqlx::{AssertSqlSafe, Executor, PgConnection};
use tests::fixtures::*;

const SERIALIZATION_FAILURE: &str = "40001";

const SETUP: &str = r#"
    CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;

    -- No index covers `note`, so an update that changes only `note` can be HOT.
    CREATE TABLE ssi_doctors (id int PRIMARY KEY, name text, status text, note text);
    INSERT INTO ssi_doctors
    SELECT g, 'doc' || g, CASE WHEN g <= 2 THEN 'oncall' ELSE 'offcall' END
    FROM generate_series(1, 10) g;
    CREATE INDEX ssi_doctors_idx ON ssi_doctors USING bm25 (id, name, status)
    WITH (text_fields = '{"status": {"tokenizer": {"type": "keyword"}, "fast": true},
                          "name": {"fast": true}}');

    CREATE TABLE ssi_shifts (id int PRIMARY KEY, doctor_id int, ward text);
    INSERT INTO ssi_shifts SELECT g, g, 'ward' || g FROM generate_series(1, 10) g;
    CREATE INDEX ssi_shifts_idx ON ssi_shifts USING bm25 (id, doctor_id, ward)
    WITH (text_fields = '{"ward": {"tokenizer": {"type": "keyword"}, "fast": true}}');
"#;

async fn run(conn: &mut PgConnection, sql: &str) -> Result<(), sqlx::Error> {
    conn.execute(AssertSqlSafe(sql)).await.map(|_| ())
}

fn sqlstate(err: &sqlx::Error) -> String {
    err.as_database_error()
        .and_then(|err| err.code())
        .map(|code| code.to_string())
        .unwrap_or_else(|| format!("{err}"))
}

/// Fails the test if `query` does not plan to `node`. A planner decline is a warning, not an
/// error, so without this a declined scan would quietly retest the base scan.
async fn assert_plan_has(conn: &mut PgConnection, query: &str, node: &str) {
    let plan: Vec<(String,)> =
        sqlx::query_as(AssertSqlSafe(format!("EXPLAIN (COSTS OFF) {query}")))
            .fetch_all(conn)
            .await
            .expect("EXPLAIN should succeed");
    assert!(
        plan.iter().any(|(line,)| line.contains(node)),
        "expected `{node}` in plan:\n{}",
        plan.iter()
            .map(|(line,)| line.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Fails the test if `update` is not HOT when `hot` says it is, or the reverse. Postgres
/// decides that on its own, from the columns the indexes cover and the free space on the page.
async fn assert_update_hot(conn: &mut PgConnection, update: &str, hot: bool) {
    run(conn, "BEGIN").await.expect("BEGIN should succeed");
    run(conn, update).await.expect("the update should succeed");
    let (hot_updates,): (i64,) = sqlx::query_as(
        "SELECT n_tup_hot_upd FROM pg_stat_xact_user_tables WHERE relid = 'ssi_doctors'::regclass",
    )
    .fetch_one(&mut *conn)
    .await
    .expect("the table should have a row in pg_stat_xact_user_tables");
    run(conn, "ROLLBACK")
        .await
        .expect("ROLLBACK should succeed");
    assert_eq!(hot_updates > 0, hot, "HOT should be {hot} for `{update}`");
}

/// Interleaves the two transactions and returns the `SQLSTATE` of the first step that failed.
///
/// The conflict surfaces wherever SSI notices it first: on the second write, or on either
/// commit.
async fn write_skew(database: &Db, read: &str, write_one: &str, write_two: &str) -> Option<String> {
    let mut one = database.connection().await;
    let mut two = database.connection().await;

    macro_rules! step {
        ($conn:expr, $sql:expr) => {
            if let Err(err) = run(&mut $conn, $sql).await {
                return Some(sqlstate(&err));
            }
        };
    }

    step!(one, "BEGIN ISOLATION LEVEL SERIALIZABLE");
    step!(one, read);
    step!(two, "BEGIN ISOLATION LEVEL SERIALIZABLE");
    step!(two, read);
    step!(one, write_one);
    step!(two, write_two);
    step!(one, "COMMIT");
    step!(two, "COMMIT");
    None
}

const INSERT_ONE: &str = "INSERT INTO ssi_doctors VALUES (101, 'new1', 'oncall')";
const INSERT_TWO: &str = "INSERT INTO ssi_doctors VALUES (102, 'new2', 'oncall')";

/// Also makes the writes that follow in the same session find their row through the primary
/// key. A seq scan would lock the whole heap for the write itself, and the test would pass
/// with no lock from the search.
const COUNT_ONCALL: &str =
    "SET enable_seqscan = off; SELECT count(*) FROM ssi_doctors WHERE status @@@ 'oncall'";

#[rstest]
#[async_std::test]
async fn base_scan_phantom_insert(database: Db) {
    let mut conn = database.connection().await;
    run(&mut conn, SETUP).await.expect("setup should succeed");
    assert_plan_has(
        &mut conn,
        "SELECT id, name FROM ssi_doctors WHERE status @@@ 'oncall'",
        "ParadeDB Base Scan",
    )
    .await;

    let failure = write_skew(
        &database,
        "SELECT id, name FROM ssi_doctors WHERE status @@@ 'oncall'",
        INSERT_ONE,
        INSERT_TWO,
    )
    .await;
    assert_eq!(failure.as_deref(), Some(SERIALIZATION_FAILURE));
}

/// The columnar path answers from the index and never reaches the heap, so the row locks a
/// heap fetch leaves behind are not there to help.
#[rstest]
#[async_std::test]
async fn columnar_scan_phantom_insert(database: Db) {
    let mut conn = database.connection().await;
    run(&mut conn, SETUP).await.expect("setup should succeed");
    assert_plan_has(
        &mut conn,
        "SELECT name FROM ssi_doctors WHERE status @@@ 'oncall'",
        "ParadeDB Base Scan",
    )
    .await;

    let failure = write_skew(
        &database,
        "SELECT name FROM ssi_doctors WHERE status @@@ 'oncall'",
        INSERT_ONE,
        INSERT_TWO,
    )
    .await;
    assert_eq!(failure.as_deref(), Some(SERIALIZATION_FAILURE));
}

#[rstest]
#[async_std::test]
async fn aggregate_scan_phantom_insert(database: Db) {
    let mut conn = database.connection().await;
    run(&mut conn, SETUP).await.expect("setup should succeed");
    assert_plan_has(
        &mut conn,
        "SELECT count(*) FROM ssi_doctors WHERE status @@@ 'oncall'",
        "ParadeDB Aggregate Scan",
    )
    .await;

    let failure = write_skew(
        &database,
        "SELECT count(*) FROM ssi_doctors WHERE status @@@ 'oncall'",
        INSERT_ONE,
        INSERT_TWO,
    )
    .await;
    assert_eq!(failure.as_deref(), Some(SERIALIZATION_FAILURE));
}

#[rstest]
#[async_std::test]
async fn aggregate_function_phantom_insert(database: Db) {
    let mut conn = database.connection().await;
    run(&mut conn, SETUP).await.expect("setup should succeed");

    let failure = write_skew(
        &database,
        "SELECT paradedb.aggregate(index=>'ssi_doctors_idx', \
                                   query=>paradedb.term('status', 'oncall'), \
                                   agg=>'{\"matches\": {\"value_count\": {\"field\": \"id\"}}}')",
        INSERT_ONE,
        INSERT_TWO,
    )
    .await;
    assert_eq!(failure.as_deref(), Some(SERIALIZATION_FAILURE));
}

#[rstest]
#[async_std::test]
async fn join_scan_phantom_insert(database: Db) {
    let mut conn = database.connection().await;
    run(&mut conn, SETUP).await.expect("setup should succeed");

    let read = "SELECT d.id, s.ward FROM ssi_doctors d JOIN ssi_shifts s ON d.id = s.doctor_id \
                WHERE d.status @@@ 'oncall' AND s.ward @@@ 'ward1' ORDER BY d.id LIMIT 10";
    assert_plan_has(&mut conn, read, "ParadeDB Join Scan").await;

    let failure = write_skew(
        &database,
        read,
        "INSERT INTO ssi_shifts VALUES (101, 1, 'ward1')",
        "INSERT INTO ssi_shifts VALUES (102, 1, 'ward1')",
    )
    .await;
    assert_eq!(failure.as_deref(), Some(SERIALIZATION_FAILURE));
}

/// A concurrent delete already conflicted through the row locks the heap visibility check
/// leaves behind. The relation lock is coarser, so it has to keep that working.
#[rstest]
#[async_std::test]
async fn aggregate_scan_concurrent_delete(database: Db) {
    let mut conn = database.connection().await;
    run(&mut conn, SETUP).await.expect("setup should succeed");

    let failure = write_skew(
        &database,
        COUNT_ONCALL,
        "DELETE FROM ssi_doctors WHERE id = 1",
        "DELETE FROM ssi_doctors WHERE id = 2",
    )
    .await;
    assert_eq!(failure.as_deref(), Some(SERIALIZATION_FAILURE));
}

/// A HOT update writes no index entry, so a lock on the index would not see it. After `VACUUM`
/// every page is all-visible, so the read visits no heap row and leaves no row lock to catch
/// the update either. Only the lock on the heap is left.
#[rstest]
#[async_std::test]
async fn hot_update_conflicts(database: Db) {
    let mut conn = database.connection().await;
    run(&mut conn, SETUP).await.expect("setup should succeed");
    assert_update_hot(
        &mut conn,
        "UPDATE ssi_doctors SET note = 'busy' WHERE id = 1",
        true,
    )
    .await;
    run(&mut conn, "VACUUM ssi_doctors")
        .await
        .expect("VACUUM should succeed");

    let failure = write_skew(
        &database,
        COUNT_ONCALL,
        "UPDATE ssi_doctors SET note = 'busy' WHERE id = 1",
        "UPDATE ssi_doctors SET note = 'busy' WHERE id = 2",
    )
    .await;
    assert_eq!(failure.as_deref(), Some(SERIALIZATION_FAILURE));
}

/// The textbook write skew: each transaction sees two doctors on call and takes one off. The
/// update changes an indexed column, so it is not HOT.
#[rstest]
#[async_std::test]
async fn non_hot_update_conflicts(database: Db) {
    let mut conn = database.connection().await;
    run(&mut conn, SETUP).await.expect("setup should succeed");
    assert_update_hot(
        &mut conn,
        "UPDATE ssi_doctors SET status = 'offcall' WHERE id = 1",
        false,
    )
    .await;
    run(&mut conn, "VACUUM ssi_doctors")
        .await
        .expect("VACUUM should succeed");

    let failure = write_skew(
        &database,
        COUNT_ONCALL,
        "UPDATE ssi_doctors SET status = 'offcall' WHERE id = 1",
        "UPDATE ssi_doctors SET status = 'offcall' WHERE id = 2",
    )
    .await;
    assert_eq!(failure.as_deref(), Some(SERIALIZATION_FAILURE));
}

/// Relation granularity is the cost of the design: a delete of a row the search never
/// matched still conflicts. Pinned here so a finer-grained lock has something to flip.
#[rstest]
#[async_std::test]
async fn non_matching_delete_conflicts(database: Db) {
    let mut conn = database.connection().await;
    run(&mut conn, SETUP).await.expect("setup should succeed");
    // The deleted rows go on other heap pages than the two matches. On one shared page, the
    // tuple locks of each transaction go over `max_pred_locks_per_page`, and the page lock that
    // replaces them conflicts with the other delete under any lock granularity. Planning the
    // read heap-fetches every row that an `INSERT` added to the index, so `REINDEX` rebuilds
    // the index over them, and `VACUUM` makes their pages all-visible. Then the read leaves no
    // row or page lock of its own.
    run(
        &mut conn,
        "INSERT INTO ssi_doctors SELECT g, 'doc' || g, 'offcall' FROM generate_series(11, 1000) g",
    )
    .await
    .expect("insert should succeed");
    run(&mut conn, "REINDEX INDEX ssi_doctors_idx")
        .await
        .expect("REINDEX should succeed");
    run(&mut conn, "VACUUM ssi_doctors")
        .await
        .expect("VACUUM should succeed");

    let failure = write_skew(
        &database,
        COUNT_ONCALL,
        "DELETE FROM ssi_doctors WHERE id = 500",
        "DELETE FROM ssi_doctors WHERE id = 900",
    )
    .await;
    assert_eq!(failure.as_deref(), Some(SERIALIZATION_FAILURE));
}

/// The lock covers one table, so a write to another one still commits.
#[rstest]
#[async_std::test]
async fn unrelated_table_write_commits(database: Db) {
    let mut conn = database.connection().await;
    run(&mut conn, SETUP).await.expect("setup should succeed");

    let failure = write_skew(
        &database,
        "SELECT count(*) FROM ssi_doctors WHERE status @@@ 'oncall'",
        "INSERT INTO ssi_shifts VALUES (101, 101, 'ward101')",
        "INSERT INTO ssi_shifts VALUES (102, 102, 'ward102')",
    )
    .await;
    assert_eq!(failure, None);
}

/// Postgres' own plan for the same read already aborts one of the two, which is the behavior
/// the custom scans have to match.
#[rstest]
#[async_std::test]
async fn postgres_index_scan_phantom_insert(database: Db) {
    let mut conn = database.connection().await;
    run(&mut conn, SETUP).await.expect("setup should succeed");

    let failure = write_skew(
        &database,
        "SET paradedb.enable_custom_scan = off; \
         SET paradedb.enable_aggregate_custom_scan = off; \
         SET enable_seqscan = off; \
         SELECT id, name FROM ssi_doctors WHERE status @@@ 'oncall'",
        INSERT_ONE,
        INSERT_TWO,
    )
    .await;
    assert_eq!(failure.as_deref(), Some(SERIALIZATION_FAILURE));
}

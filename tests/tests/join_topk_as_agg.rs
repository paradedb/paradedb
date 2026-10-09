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

//! JoinScan's Top-K-as-aggregate path (`paradedb.joinscan_force_topk_as_agg`) must
//! return the same rows, in the same order, as the default `SortExec` Top-K path.
//! Each query runs with the GUC off and then on, and the row vectors are compared,
//! both serially and under MPP, where the aggregate splits into a Partial per
//! worker and a Final on the leader. DISTINCT queries run too, where the GUC-on
//! path absorbs the DISTINCT into the aggregate instead of a GROUP BY. Each
//! query's EXPLAIN is checked as well, so the comparison is known to be between
//! the two paths and not the SortExec path against itself.

use rstest::*;
use sqlx::PgConnection;
use tests::fixtures::*;

/// Small single-segment tables. `rating` carries ties and NULLs, so null placement
/// and the id tiebreaks both matter.
const SERIAL_SETUP: &str = r#"
SET paradedb.enable_custom_scan = on;
SET paradedb.enable_join_custom_scan = on;
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan = off;

DROP TABLE IF EXISTS tka_t1 CASCADE;
DROP TABLE IF EXISTS tka_t2 CASCADE;
CREATE TABLE tka_t1 (id INTEGER PRIMARY KEY, rating INTEGER, val TEXT);
CREATE TABLE tka_t2 (id INTEGER PRIMARY KEY, t1_id INTEGER, qty INTEGER, val TEXT);

INSERT INTO tka_t1
SELECT i, CASE WHEN i % 7 = 0 THEN NULL ELSE i % 5 END, 'val ' || i
FROM generate_series(1, 300) i;
INSERT INTO tka_t2
SELECT i, (i % 300) + 1, i % 11, 'val ' || i
FROM generate_series(1, 600) i;

CREATE INDEX tka_t1_idx ON tka_t1
USING paradedb (id, rating, (val::pdb.unicode_words('columnar=true')));
CREATE INDEX tka_t2_idx ON tka_t2 USING paradedb (id, t1_id, qty, val);

ANALYZE tka_t1;
ANALYZE tka_t2;
"#;

/// Multi-segment tables with parallel workers forced on, so the join distributes
/// and the Top-K aggregate runs as a Partial per worker merged by a Final on the
/// leader.
const MPP_SETUP: &str = r#"
SET paradedb.enable_custom_scan = on;
SET paradedb.enable_join_custom_scan = on;
SET enable_indexscan = off;
SET paradedb.mpp_min_rows = 0;
SET paradedb.min_rows_per_worker = 0;
SET max_parallel_workers = 4;
SET max_parallel_workers_per_gather = 3;
SET parallel_tuple_cost = 0;
SET parallel_setup_cost = 0;
SET min_parallel_table_scan_size = 0;
SET min_parallel_index_scan_size = 0;
SET parallel_leader_participation = off;

DROP TABLE IF EXISTS tka_t1 CASCADE;
DROP TABLE IF EXISTS tka_t2 CASCADE;
CREATE TABLE tka_t1 (id INTEGER PRIMARY KEY, rating INTEGER, val TEXT)
WITH (autovacuum_enabled = false);
CREATE TABLE tka_t2 (id INTEGER PRIMARY KEY, t1_id INTEGER, qty INTEGER, val TEXT)
WITH (autovacuum_enabled = false);

-- Several segments per index, so the scans fan out to more than one task.
SET paradedb.global_mutable_segment_rows = 0;
CREATE INDEX tka_t1_idx ON tka_t1
USING paradedb (id, rating, (val::pdb.unicode_words('columnar=true')))
WITH (target_segment_count = 8, background_layer_sizes = '0');
CREATE INDEX tka_t2_idx ON tka_t2 USING paradedb (id, t1_id, qty, val)
WITH (target_segment_count = 8, background_layer_sizes = '0');

INSERT INTO tka_t1
SELECT i, CASE WHEN i % 7 = 0 THEN NULL ELSE i % 5 END, 'val ' || i
FROM generate_series(1, 1500) i;
INSERT INTO tka_t1
SELECT i, CASE WHEN i % 7 = 0 THEN NULL ELSE i % 5 END, 'val ' || i
FROM generate_series(1501, 3000) i;
INSERT INTO tka_t2
SELECT i, (i % 3000) + 1, i % 11, 'val ' || i
FROM generate_series(1, 3000) i;
INSERT INTO tka_t2
SELECT i, (i % 3000) + 1, i % 11, 'val ' || i
FROM generate_series(3001, 6000) i;
RESET paradedb.global_mutable_segment_rows;

ANALYZE tka_t1;
ANALYZE tka_t2;
"#;

#[derive(Debug)]
enum Mode {
    Serial,
    Mpp,
}

/// The name the Top-K-as-aggregate path gives its aggregate (`TOPK_AGG_ROWS_COL_NAME`),
/// which is how it shows up in the rendered DataFusion plan. The SortExec path
/// never uses it.
const TOPK_AGG_ALIAS: &str = "__topk";

/// A one-column `float8` row, compared the way Postgres compares floats: NaN
/// equals NaN, and `-0` equals `0`. `f64`'s own `PartialEq` would make two
/// identical results unequal as soon as they held a NaN.
#[derive(Debug)]
struct Float8Row(f64);

impl PartialEq for Float8Row {
    fn eq(&self, other: &Self) -> bool {
        (self.0.is_nan() && other.0.is_nan()) || self.0 == other.0
    }
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for Float8Row {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self(row.try_get(0)?))
    }
}

/// The `EXPLAIN` of `query`, joined into one string.
fn explain(conn: &mut PgConnection, query: &str) -> String {
    format!("EXPLAIN {query}")
        .fetch::<(String,)>(conn)
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Runs `query` with the GUC off, then on, and requires identical rows in identical
/// order. `expected_rows` guards against a vacuous match between two empty results.
///
/// The plans are checked too: the GUC-off plan must not mention the Top-K
/// aggregate, and the GUC-on plan must run it as its only aggregate, so a DISTINCT
/// is absorbed into it rather than planned as a GROUP BY beside it.
fn assert_paths_agree<T>(conn: &mut PgConnection, query: &str, expected_rows: usize)
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
        + PartialEq
        + std::fmt::Debug
        + Send
        + Unpin,
{
    "SET paradedb.joinscan_force_topk_as_agg = off".execute(conn);
    let sort_exec_plan = explain(conn, query);
    let sort_exec: Vec<T> = query.fetch(conn);
    "SET paradedb.joinscan_force_topk_as_agg = on".execute(conn);
    let topk_agg_plan = explain(conn, query);
    let topk_agg: Vec<T> = query.fetch(conn);

    assert!(
        !sort_exec_plan.contains(TOPK_AGG_ALIAS),
        "GUC off must not plan the Top-K aggregate.\nquery: {query}\nplan:\n{sort_exec_plan}"
    );
    let aggregates: Vec<&str> = topk_agg_plan
        .lines()
        .filter(|line| line.contains("AggregateExec"))
        .collect();
    // LIMIT 0 folds to an EmptyRelation before physical planning, so there is no
    // aggregate node to find; every other query must run exactly this aggregate.
    if expected_rows > 0 {
        assert!(
            !aggregates.is_empty(),
            "GUC on must plan the Top-K aggregate.\nquery: {query}\nplan:\n{topk_agg_plan}"
        );
    }
    assert!(
        aggregates.iter().all(|line| line.contains(TOPK_AGG_ALIAS)),
        "GUC on must plan no aggregate but the Top-K one.\nquery: {query}\nplan:\n{topk_agg_plan}"
    );

    assert_eq!(sort_exec.len(), expected_rows, "{query}");
    assert_eq!(topk_agg, sort_exec, "{query}");
}

/// Every ORDER BY ends in a unique id tiebreak, so the two paths cannot legitimately
/// differ on ties.
#[rstest]
#[case::serial(Mode::Serial)]
#[case::mpp(Mode::Mpp)]
fn topk_as_agg_matches_sort_exec(#[case] mode: Mode, mut conn: PgConnection) {
    // Every t2 row joins exactly one t1 row and every t1 row matches `val`, so the
    // join has one row per t2 row.
    let join_rows: i64 = match mode {
        Mode::Serial => {
            SERIAL_SETUP.execute(&mut conn);
            600
        }
        Mode::Mpp => {
            MPP_SETUP.execute(&mut conn);
            6000
        }
    };

    // ASC on one side with OFFSET, so the aggregate keeps offset + limit rows.
    assert_paths_agree::<(i32, i32, String, String)>(
        &mut conn,
        r#"
        SELECT t1.id, t2.id, t1.val, t2.val
        FROM tka_t1 t1
        JOIN tka_t2 t2 ON t1.id = t2.t1_id
        WHERE t1.val ||| 'val'
        ORDER BY t1.id ASC, t2.id ASC
        OFFSET 5 LIMIT 10
        "#,
        10,
    );

    // DESC NULLS FIRST on a nullable key with ties, keys from both sides.
    assert_paths_agree::<(i32, Option<i32>, i32, i32)>(
        &mut conn,
        r#"
        SELECT t1.id, t1.rating, t2.qty, t2.id
        FROM tka_t1 t1
        JOIN tka_t2 t2 ON t1.id = t2.t1_id
        WHERE t1.val ||| 'val'
        ORDER BY t1.rating DESC NULLS FIRST, t2.qty ASC, t2.id ASC
        LIMIT 12
        "#,
        12,
    );

    // Score ordering; the extra terms give some rows a higher score.
    assert_paths_agree::<(i32, i32, f32)>(
        &mut conn,
        r#"
        SELECT t1.id, t2.id, paradedb.score(t1.id)
        FROM tka_t1 t1
        JOIN tka_t2 t2 ON t1.id = t2.t1_id
        WHERE t1.val ||| 'val 42 77'
        ORDER BY paradedb.score(t1.id) DESC, t1.id ASC, t2.id ASC
        LIMIT 8
        "#,
        8,
    );

    // The join condition written with t2's column first puts `t2.t1_id` ahead
    // of `t1.id` in their equivalence class, and the ORDER BY on `t1.id` reaches
    // JoinScan as `t2.t1_id`, a column nothing projects. Without DISTINCT that is
    // legal, and the aggregate path has to feed it to the aggregate's ORDER BY as
    // an extra input rather than resolve it through the select list.
    assert_paths_agree::<(i32, i32)>(
        &mut conn,
        r#"
        SELECT t1.id, t2.id
        FROM tka_t1 t1
        JOIN tka_t2 t2 ON t2.t1_id = t1.id
        WHERE t1.val ||| 'val'
        ORDER BY t1.id ASC, t2.id ASC
        LIMIT 5
        "#,
        5,
    );

    // A sort key that wraps an unselected column: Postgres adds `t1.rating IS NULL`
    // to the target list as resjunk, not `t1.rating` itself, so the aggregate path
    // has to feed `rating` to the aggregate's ORDER BY as an extra input and let
    // DataFusion evaluate the key.
    assert_paths_agree::<(i32, i32)>(
        &mut conn,
        r#"
        SELECT t1.id, t2.id
        FROM tka_t1 t1
        JOIN tka_t2 t2 ON t1.id = t2.t1_id
        WHERE t1.val ||| 'val'
        ORDER BY t1.rating IS NULL, t1.id ASC, t2.id ASC
        LIMIT 5
        "#,
        5,
    );

    // A score sum over both relations, neither score selected: the key is an
    // expression over two columns the payload lacks.
    //
    // Skipped on Postgres 15, where JoinScan fails on this query with or without
    // the Top-K aggregate. See https://github.com/paradedb/paradedb/issues/6596
    if pg_major_version(&mut conn) >= 16 {
        assert_paths_agree::<(i32, i32)>(
            &mut conn,
            r#"
            SELECT t1.id, t2.id
            FROM tka_t1 t1
            JOIN tka_t2 t2 ON t1.id = t2.t1_id
            WHERE t1.val ||| 'val 42 77' AND t2.val ||| 'val 7 99'
            ORDER BY paradedb.score(t1.id) + paradedb.score(t2.id) DESC, t1.id ASC, t2.id ASC
            LIMIT 8
            "#,
            8,
        );
    }

    // A projected ctid. JoinScan resolves `t1.ctid` to the same scan column it
    // carries to fetch t1's heap tuples, so that column is wanted twice: as a
    // select-list entry and as the heap-fetch handle. The aggregate path has to
    // carry it once, not as two payload columns with one name.
    //
    // The outer query only casts the `tid` to text, which sqlx can decode; the
    // inner query is the one JoinScan plans. Both paths return NULL for the ctid
    // today (JoinScan's slot fill reads no system attribute from a heap tuple),
    // so this checks that the aggregate path plans and agrees, not the value.
    assert_paths_agree::<(Option<String>, i32)>(
        &mut conn,
        r#"
        SELECT ctid::text, id FROM (
            SELECT t1.ctid, t1.id
            FROM tka_t1 t1
            JOIN tka_t2 t2 ON t1.id = t2.t1_id
            WHERE t1.val ||| 'val'
            ORDER BY t1.id ASC, t2.id ASC
            LIMIT 3
        ) q
        "#,
        3,
    );

    // DISTINCT: each t1 row joins two t2 rows, so every (id, rating) pair
    // appears twice before deduplication. With the GUC on, the DISTINCT is
    // absorbed into the Top-K aggregate instead of running as a GROUP BY.
    assert_paths_agree::<(i32, Option<i32>)>(
        &mut conn,
        r#"
        SELECT DISTINCT t1.id, t1.rating
        FROM tka_t1 t1
        JOIN tka_t2 t2 ON t1.id = t2.t1_id
        WHERE t1.val ||| 'val'
        ORDER BY t1.rating DESC NULLS FIRST, t1.id ASC
        OFFSET 3 LIMIT 10
        "#,
        10,
    );

    // DISTINCT over heavily repeated keys from both sides: six rating values
    // including NULL by eleven qty values, so the distinct set is far smaller
    // than the join and every group has many duplicates to collapse.
    assert_paths_agree::<(Option<i32>, i32)>(
        &mut conn,
        r#"
        SELECT DISTINCT t1.rating, t2.qty
        FROM tka_t1 t1
        JOIN tka_t2 t2 ON t1.id = t2.t1_id
        WHERE t1.val ||| 'val'
        ORDER BY t1.rating DESC NULLS FIRST, t2.qty ASC
        LIMIT 12
        "#,
        12,
    );

    // LIMIT 0 on the DISTINCT form: the aggregate still runs, with a k of zero
    // that admits no rows, rather than being skipped, so the sort above still
    // finds its `col_N` keys.
    assert_paths_agree::<(i32, Option<i32>)>(
        &mut conn,
        r#"
        SELECT DISTINCT t1.id, t1.rating
        FROM tka_t1 t1
        JOIN tka_t2 t2 ON t1.id = t2.t1_id
        WHERE t1.val ||| 'val'
        ORDER BY t1.rating DESC NULLS FIRST, t1.id ASC
        LIMIT 0
        "#,
        0,
    );

    // DISTINCT with a score in the key and in the ORDER BY.
    assert_paths_agree::<(i32, f32)>(
        &mut conn,
        r#"
        SELECT DISTINCT t1.id, paradedb.score(t1.id)
        FROM tka_t1 t1
        JOIN tka_t2 t2 ON t1.id = t2.t1_id
        WHERE t1.val ||| 'val 42 77'
        ORDER BY paradedb.score(t1.id) DESC, t1.id ASC
        LIMIT 8
        "#,
        8,
    );

    // A window aggregate as the whole select list, with no ORDER BY: the aggregate
    // has no sort key, and nothing to carry but the window's own column. Every
    // row holds the same count, so it does not matter which five come back. A
    // window aggregate takes the Top-K aggregate path whatever the GUC says, so
    // there is no SortExec plan to compare against.
    let window_only = r#"
        SELECT count(*) OVER ()
        FROM tka_t1 t1
        JOIN tka_t2 t2 ON t1.id = t2.t1_id
        WHERE t1.val ||| 'val'
        LIMIT 5
        "#;
    let rows: Vec<(i64,)> = window_only.fetch(&mut conn);
    assert_eq!(rows, vec![(join_rows,); 5], "{window_only}");

    // An expression as the whole select list, with no ORDER BY: no column is
    // selected and no heap tuple is fetched, so the evaluated expression is all
    // the aggregate carries. Without an ORDER BY any three rows may come back,
    // so the predicate keeps to rows that agree: ids 1 to 6 all have a rating.
    assert_paths_agree::<(bool,)>(
        &mut conn,
        r#"
        SELECT t1.rating IS NULL
        FROM tka_t1 t1
        JOIN tka_t2 t2 ON t1.id = t2.t1_id
        WHERE t1.val ||| 'val' AND t1.id < 7
        LIMIT 3
        "#,
        3,
    );

    // https://github.com/paradedb/paradedb/issues/6601: DISTINCT over a select
    // list that is only a window aggregate, on the default path. The output
    // projection took an empty column map to mean "no DISTINCT" and looked for
    // the pre-GROUP BY column name, failing at plan build. One row comes back:
    // the count of the whole join.
    "SET paradedb.joinscan_force_topk_as_agg = off".execute(&mut conn);
    let distinct_window = r#"
        SELECT DISTINCT count(*) OVER ()
        FROM tka_t1 t1
        JOIN tka_t2 t2 ON t1.id = t2.t1_id
        WHERE t1.val ||| 'val'
        LIMIT 5
        "#;
    let plan = explain(&mut conn, distinct_window);
    assert!(
        plan.contains("ParadeDB Join Scan"),
        "JoinScan must plan the query, or the check below is vacuous.\nplan:\n{plan}"
    );
    let rows: Vec<(i64,)> = distinct_window.fetch(&mut conn);
    assert_eq!(rows, vec![(join_rows,)], "{distinct_window}");

    // Float keys. The three values that separate float comparison rules: `0`
    // and `-0`, which Postgres and a GROUP BY treat as equal while a total order
    // does not, and NaN, which sorts above everything and equals itself. Each
    // joins two t2 rows.
    r#"
    DROP TABLE IF EXISTS tka_floats CASCADE;
    CREATE TABLE tka_floats (id INTEGER PRIMARY KEY, rating FLOAT8, val TEXT);
    INSERT INTO tka_floats VALUES
        (1, 0.0, 'val'), (2, '-0'::float8, 'val'), (3, 'NaN'::float8, 'val');
    CREATE INDEX tka_floats_idx ON tka_floats USING paradedb (id, rating, val);
    ANALYZE tka_floats;
    "#
    .execute(&mut conn);

    // Ordering on a float key: NaN sorts first under DESC on both paths.
    assert_paths_agree::<Float8Row>(
        &mut conn,
        r#"
        SELECT f.rating
        FROM tka_floats f
        JOIN tka_t2 t2 ON f.id = t2.t1_id
        WHERE f.val ||| 'val'
        ORDER BY f.rating DESC, t2.id ASC
        LIMIT 3
        "#,
        3,
    );

    // DISTINCT on a float key: the two zeros are one group, so LIMIT 2 has room
    // for NaN. A distinct that tells `-0` from `0` spends both slots on zeros.
    assert_paths_agree::<Float8Row>(
        &mut conn,
        r#"
        SELECT DISTINCT f.rating
        FROM tka_floats f
        JOIN tka_t2 t2 ON f.id = t2.t1_id
        WHERE f.val ||| 'val'
        ORDER BY f.rating
        LIMIT 2
        "#,
        2,
    );
}

/// `pdb.agg() OVER ()` is one more aggregate in the Top-K aggregate node. Under
/// MPP each worker hands its buckets to the leader, which merges the buckets
/// more than one worker saw. The aggregate scan's document over the same join
/// is the oracle.
///
/// The query has no SQL window aggregate: that one is a window operator beneath
/// the aggregate node, which then runs on the leader alone.
#[rstest]
#[case::serial(Mode::Serial)]
#[case::mpp(Mode::Mpp)]
fn pdb_agg_window_in_topk_agg(#[case] mode: Mode, mut conn: PgConnection) {
    match mode {
        Mode::Serial => SERIAL_SETUP.execute(&mut conn),
        Mode::Mpp => MPP_SETUP.execute(&mut conn),
    }
    "SET paradedb.enable_aggregate_custom_scan = on".execute(&mut conn);
    "SET paradedb.joinscan_force_topk_as_agg = off".execute(&mut conn);

    let spec = r#"{
        "terms": {"field": "rating"},
        "aggs": {
            "avg_qty": {"avg": {"field": "qty"}},
            "quantities": {
                "terms": {"field": "qty", "size": 3},
                "aggs": {"ids": {"cardinality": {"field": "t2.id"}}}
            }
        }
    }"#;
    let join = r#"
        FROM tka_t1 t1
        JOIN tka_t2 t2 ON t1.id = t2.t1_id
        WHERE t1.val ||| 'val'
    "#;
    let query = format!(
        r#"
        SELECT t1.id, t2.id, pdb.agg('{spec}') OVER ()
        {join}
        ORDER BY t1.rating DESC NULLS FIRST, t2.qty ASC, t2.id ASC
        OFFSET 3 LIMIT 10
        "#
    );

    let plan: Vec<String> = format!("EXPLAIN (COSTS OFF, VERBOSE) {query}").fetch_scalar(&mut conn);
    let plan = plan.join("\n");
    assert!(plan.contains("Custom Scan (ParadeDB Join Scan)"), "{plan}");
    if matches!(mode, Mode::Mpp) {
        let partial = plan
            .lines()
            .find(|line| line.contains("AggregateExec: mode=Partial"))
            .unwrap_or_else(|| panic!("no partial aggregate in plan:\n{plan}"));
        assert!(partial.contains("pdb_agg("), "{partial}");
    }

    let (expected,) =
        format!("SELECT pdb.agg('{spec}') {join}").fetch_one::<(serde_json::Value,)>(&mut conn);
    let rows: Vec<(i32, i32, serde_json::Value)> = query.fetch(&mut conn);
    assert_eq!(rows.len(), 10);
    for (_, _, document) in rows {
        assert_eq!(document, expected);
    }
}

/// Window aggregates are computed inside the Top-K aggregate node, so a query
/// with them takes this path with the GUC off. Under MPP every aggregate in the
/// node splits into a Partial per worker and a Final on the leader. PostgreSQL's
/// own WindowAgg plan is the oracle.
#[rstest]
#[case::serial(Mode::Serial)]
#[case::mpp(Mode::Mpp)]
fn window_aggregates_in_topk_agg(#[case] mode: Mode, mut conn: PgConnection) {
    match mode {
        Mode::Serial => SERIAL_SETUP.execute(&mut conn),
        Mode::Mpp => MPP_SETUP.execute(&mut conn),
    }
    "SET paradedb.joinscan_force_topk_as_agg = off".execute(&mut conn);

    let query = r#"
        SELECT t1.id, t2.id,
               COUNT(*) OVER () AS total,
               SUM(t2.qty) OVER () AS qty_sum,
               AVG(t1.rating) OVER ()::float8 AS rating_avg,
               MIN(t1.rating) OVER () AS rating_min,
               MAX(t2.qty) OVER () AS qty_max
        FROM tka_t1 t1
        JOIN tka_t2 t2 ON t1.id = t2.t1_id
        WHERE t1.val ||| 'val'
        ORDER BY t1.rating DESC NULLS FIRST, t2.qty ASC, t2.id ASC
        OFFSET 3 LIMIT 10
    "#;
    type Row = (i32, i32, i64, i64, f64, i32, i32);

    let plan: Vec<String> = format!("EXPLAIN (COSTS OFF, VERBOSE) {query}").fetch_scalar(&mut conn);
    let plan = plan.join("\n");
    assert!(plan.contains("Custom Scan (ParadeDB Join Scan)"), "{plan}");
    assert!(!plan.contains("WindowAggExec"), "{plan}");
    let join_scan: Vec<Row> = query.fetch(&mut conn);

    "SET paradedb.enable_join_custom_scan = off".execute(&mut conn);
    let postgres: Vec<Row> = query.fetch(&mut conn);

    assert_eq!(postgres.len(), 10);
    assert_eq!(join_scan, postgres);
}

/// DISTINCT folds into the same Top-K aggregate as the window aggregates,
/// `topk_as_agg(DISTINCT ...)`, and the windows still count the join before
/// DISTINCT. Only `COUNT(*) OVER ()`, and expressions over it, take this path
/// with DISTINCT today; a window over a column declines at planning (#6501).
#[rstest]
#[case::serial(Mode::Serial)]
#[case::mpp(Mode::Mpp)]
fn distinct_window_aggregates_in_topk_agg(#[case] mode: Mode, mut conn: PgConnection) {
    match mode {
        Mode::Serial => SERIAL_SETUP.execute(&mut conn),
        Mode::Mpp => MPP_SETUP.execute(&mut conn),
    }
    "SET paradedb.joinscan_force_topk_as_agg = off".execute(&mut conn);

    // Every t1 row joins two t2 rows, so DISTINCT halves the join while the
    // window counts all of it.
    let query = r#"
        SELECT DISTINCT t1.id,
               COUNT(*) OVER () AS total,
               COUNT(*) OVER () + 1 AS total_plus_one
        FROM tka_t1 t1
        JOIN tka_t2 t2 ON t1.id = t2.t1_id
        WHERE t1.val ||| 'val'
        ORDER BY t1.id DESC
        OFFSET 3 LIMIT 10
    "#;
    type Row = (i32, i64, i64);

    let plan: Vec<String> = format!("EXPLAIN (COSTS OFF, VERBOSE) {query}").fetch_scalar(&mut conn);
    let plan = plan.join("\n");
    assert!(plan.contains("Custom Scan (ParadeDB Join Scan)"), "{plan}");
    assert!(!plan.contains("WindowAggExec"), "{plan}");
    assert!(plan.contains("topk_as_agg(DISTINCT "), "{plan}");
    if matches!(mode, Mode::Mpp) {
        let partial = plan
            .lines()
            .find(|line| line.contains("AggregateExec: mode=Partial"))
            .unwrap_or_else(|| panic!("no partial aggregate in plan:\n{plan}"));
        assert!(
            partial.contains("topk_as_agg(DISTINCT ") && partial.contains("count("),
            "{partial}"
        );
    }
    let join_scan: Vec<Row> = query.fetch(&mut conn);

    let (join_rows,) =
        "SELECT COUNT(*) FROM tka_t1 t1 JOIN tka_t2 t2 ON t1.id = t2.t1_id WHERE t1.val ||| 'val'"
            .fetch_one::<(i64,)>(&mut conn);
    assert!(
        join_scan
            .iter()
            .all(|(_, total, plus_one)| *total == join_rows && *plus_one == join_rows + 1),
        "{join_scan:?}"
    );

    "SET paradedb.enable_join_custom_scan = off".execute(&mut conn);
    let postgres: Vec<Row> = query.fetch(&mut conn);

    assert_eq!(postgres.len(), 10);
    assert_eq!(join_scan, postgres);
}

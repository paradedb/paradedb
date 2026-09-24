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
//! worker and a Final on the leader.

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

/// Runs `query` with the GUC off, then on, and requires identical rows in identical
/// order. `expected_rows` guards against a vacuous match between two empty results.
fn assert_paths_agree<T>(conn: &mut PgConnection, query: &str, expected_rows: usize)
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
        + PartialEq
        + std::fmt::Debug
        + Send
        + Unpin,
{
    "SET paradedb.joinscan_force_topk_as_agg = off".execute(conn);
    let sort_exec: Vec<T> = query.fetch(conn);
    "SET paradedb.joinscan_force_topk_as_agg = on".execute(conn);
    let topk_agg: Vec<T> = query.fetch(conn);

    assert_eq!(sort_exec.len(), expected_rows, "{query}");
    assert_eq!(topk_agg, sort_exec, "{query}");
}

/// Every ORDER BY ends in a unique id tiebreak, so the two paths cannot legitimately
/// differ on ties.
#[rstest]
#[case::serial(Mode::Serial)]
#[case::mpp(Mode::Mpp)]
fn topk_as_agg_matches_sort_exec(#[case] mode: Mode, mut conn: PgConnection) {
    match mode {
        Mode::Serial => SERIAL_SETUP.execute(&mut conn),
        Mode::Mpp => MPP_SETUP.execute(&mut conn),
    }

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
}

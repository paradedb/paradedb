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

//! A heap filter on `ctid` or `tableoid` must see the fetched tuple's values
//! (issue #7037). Each query must return the same rows with the ParadeDB
//! custom scan on and off.

use rstest::*;
use sqlx::{PgConnection, Row};
use tests::fixtures::*;

const BASE_SCAN: &str = "Custom Scan (ParadeDB Base Scan)";
const JOIN_SCAN: &str = "Custom Scan (ParadeDB Join Scan)";

fn setup(conn: &mut PgConnection) {
    r#"
    SET max_parallel_workers_per_gather = 0;

    CREATE TABLE heap_filter_a (id int PRIMARY KEY, val text, note text);
    CREATE TABLE heap_filter_b (id int PRIMARY KEY, a_id int, val text);
    INSERT INTO heap_filter_a SELECT i, 'val ' || i, 'n' FROM generate_series(1, 5) i;
    -- In insertion order, b.id 14 is at (0,1), 13 at (0,2), 12 at (0,3) and 11 at (0,4).
    INSERT INTO heap_filter_b VALUES (14, 99, 'val 14'), (13, 3, 'val 13'), (12, 2, 'val 12'), (11, 1, 'val 11');

    CREATE INDEX heap_filter_a_idx ON heap_filter_a USING paradedb (id, val);
    CREATE INDEX heap_filter_b_idx ON heap_filter_b USING paradedb (id, a_id, val);
    ANALYZE heap_filter_a;
    ANALYZE heap_filter_b;
    "#
    .execute(conn);
}

/// Rows as raw column values. sqlx cannot decode `tid`, and the comparison
/// only needs equal bytes.
fn fetch_raw(query: &str, conn: &mut PgConnection) -> Vec<Vec<Option<Vec<u8>>>> {
    query
        .fetch_dynamic(conn)
        .iter()
        .map(|row| {
            (0..row.len())
                .map(|i| {
                    let value = row.try_get_raw(i).expect("column should exist");
                    // `as_bytes` fails only for NULL.
                    value.as_bytes().ok().map(<[u8]>::to_vec)
                })
                .collect()
        })
        .collect()
}

#[derive(Clone, Copy)]
enum HeapFilterCase {
    JoinCtidGreater,
    JoinCtidLess,
    JoinTableoid,
    BaseCtidGreater,
}

#[rstest]
#[case::join_ctid_greater(HeapFilterCase::JoinCtidGreater)]
#[case::join_ctid_less(HeapFilterCase::JoinCtidLess)]
#[case::join_tableoid(HeapFilterCase::JoinTableoid)]
#[case::base_ctid_greater(HeapFilterCase::BaseCtidGreater)]
fn heap_filter_system_columns_match_postgres(#[case] case: HeapFilterCase, mut conn: PgConnection) {
    setup(&mut conn);

    let (query, scan, scan_guc, expected_rows) = match case {
        HeapFilterCase::JoinCtidGreater => (
            // The query from the issue: b.id 13 is at (0,2), which the filter excludes.
            r#"
            SELECT a.id, b.id AS b_id, b.ctid AS b_ctid
            FROM heap_filter_a a JOIN heap_filter_b b ON a.id = b.a_id
            WHERE a.val ||| 'val' AND b.ctid > '(0,2)'::tid
            ORDER BY a.id LIMIT 10
            "#,
            JOIN_SCAN,
            "paradedb.enable_join_custom_scan",
            2,
        ),
        HeapFilterCase::JoinCtidLess => (
            // b.id 13 at (0,2) is the only matched row before (0,3).
            r#"
            SELECT a.id, b.id AS b_id, b.ctid AS b_ctid
            FROM heap_filter_a a JOIN heap_filter_b b ON a.id = b.a_id
            WHERE a.val ||| 'val' AND b.ctid < '(0,3)'::tid
            ORDER BY a.id LIMIT 10
            "#,
            JOIN_SCAN,
            "paradedb.enable_join_custom_scan",
            1,
        ),
        HeapFilterCase::JoinTableoid => (
            r#"
            SELECT a.id, b.id AS b_id
            FROM heap_filter_a a JOIN heap_filter_b b ON a.id = b.a_id
            WHERE a.val ||| 'val' AND b.tableoid = 'heap_filter_b'::regclass
            ORDER BY a.id LIMIT 10
            "#,
            JOIN_SCAN,
            "paradedb.enable_join_custom_scan",
            3,
        ),
        HeapFilterCase::BaseCtidGreater => (
            r#"
            SELECT id, ctid
            FROM heap_filter_b
            WHERE val ||| 'val' AND ctid > '(0,2)'::tid
            ORDER BY id
            "#,
            BASE_SCAN,
            "paradedb.enable_custom_scan",
            2,
        ),
    };

    format!("SET {scan_guc} = on; DISCARD PLANS;").execute(&mut conn);
    let plan: Vec<String> = format!("EXPLAIN (COSTS OFF) {query}").fetch_scalar(&mut conn);
    let plan = plan.join("\n");
    assert!(plan.contains(scan), "{plan}");
    let paradedb_rows = fetch_raw(query, &mut conn);

    format!("SET {scan_guc} = off; DISCARD PLANS;").execute(&mut conn);
    let plan: Vec<String> = format!("EXPLAIN (COSTS OFF) {query}").fetch_scalar(&mut conn);
    let plan = plan.join("\n");
    assert!(!plan.contains(scan), "{plan}");
    let postgres_rows = fetch_raw(query, &mut conn);

    assert_eq!(postgres_rows.len(), expected_rows, "{query}");
    assert_eq!(paradedb_rows, postgres_rows, "{query}");
}

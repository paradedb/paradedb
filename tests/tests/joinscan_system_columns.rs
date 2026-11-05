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

//! JoinScan must return system columns from each source's fetched heap tuple
//! (issue #6630). Each query must return the same rows with JoinScan on and
//! off, for all six system columns of both sources.

use rstest::*;
use sqlx::{PgConnection, Row};
use tests::fixtures::*;

const JOIN_SCAN: &str = "Custom Scan (ParadeDB Join Scan)";

const SYSTEM_COLUMNS: &str = r#"
    a.ctid AS a_ctid, a.tableoid AS a_tableoid, a.xmin AS a_xmin,
    a.cmin AS a_cmin, a.xmax AS a_xmax, a.cmax AS a_cmax,
    b.ctid AS b_ctid, b.tableoid AS b_tableoid, b.xmin AS b_xmin,
    b.cmin AS b_cmin, b.xmax AS b_xmax, b.cmax AS b_cmax
"#;

fn setup(conn: &mut PgConnection) {
    r#"
    SET max_parallel_workers_per_gather = 0;

    CREATE TABLE system_columns_a (id int PRIMARY KEY, next_id int, val text, note text);
    CREATE TABLE system_columns_b (id int PRIMARY KEY, a_id int, val text);
    INSERT INTO system_columns_a SELECT i, i + 1, 'val ' || i, 'original' FROM generate_series(1, 5) i;
    -- The reverse insertion order gives the two sources different CTIDs.
    INSERT INTO system_columns_b VALUES (14, 99, 'val 14'), (13, 3, 'val 13'), (12, 2, 'val 12'), (11, 1, 'val 11');

    CREATE INDEX system_columns_a_idx ON system_columns_a USING paradedb (id, next_id, val);
    CREATE INDEX system_columns_b_idx ON system_columns_b USING paradedb (id, a_id, val);
    ANALYZE system_columns_a;
    ANALYZE system_columns_b;
    "#
    .execute(conn);
}

/// Rows as raw column values. sqlx cannot decode `tid`, `xid` or `cid`, and
/// the comparison only needs equal bytes.
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
enum SystemColumnsCase {
    InnerJoin,
    LeftJoinUnmatchedRow,
    SelfJoin,
    OnlySystemColumns,
    UpdatedTuple,
}

#[rstest]
#[case::inner_join(SystemColumnsCase::InnerJoin)]
#[case::left_join_unmatched_row(SystemColumnsCase::LeftJoinUnmatchedRow)]
#[case::self_join(SystemColumnsCase::SelfJoin)]
#[case::only_system_columns(SystemColumnsCase::OnlySystemColumns)]
#[case::updated_tuple(SystemColumnsCase::UpdatedTuple)]
fn joinscan_system_columns_match_postgres(#[case] case: SystemColumnsCase, mut conn: PgConnection) {
    setup(&mut conn);

    let (query, expected_rows) = match case {
        SystemColumnsCase::InnerJoin => (
            format!(
                r#"
                SELECT a.id, b.id AS b_id, a.note, {SYSTEM_COLUMNS}
                FROM system_columns_a a JOIN system_columns_b b ON a.id = b.a_id
                WHERE a.val ||| 'val'
                ORDER BY a.id LIMIT 3
                "#
            ),
            3,
        ),
        SystemColumnsCase::LeftJoinUnmatchedRow => (
            // b.id 14 has no match, so its `a` system columns must be NULL and
            // must not repeat the previous row's values.
            format!(
                r#"
                SELECT b.id, a.id AS a_id, {SYSTEM_COLUMNS}
                FROM system_columns_b b LEFT JOIN system_columns_a a ON a.id = b.a_id
                WHERE b.val ||| 'val'
                ORDER BY b.id LIMIT 10
                "#
            ),
            4,
        ),
        SystemColumnsCase::SelfJoin => (
            // Both sources read the same table, but each row pairs two
            // different tuples.
            format!(
                r#"
                SELECT a.id, b.id AS b_id, {SYSTEM_COLUMNS}
                FROM system_columns_a a JOIN system_columns_a b ON a.next_id = b.id
                WHERE a.val ||| 'val'
                ORDER BY a.id LIMIT 3
                "#
            ),
            3,
        ),
        SystemColumnsCase::OnlySystemColumns => (
            format!(
                r#"
                SELECT {SYSTEM_COLUMNS}
                FROM system_columns_a a JOIN system_columns_b b ON a.id = b.a_id
                WHERE a.val ||| 'val'
                ORDER BY a.id LIMIT 3
                "#
            ),
            3,
        ),
        SystemColumnsCase::UpdatedTuple => {
            // Updating an unindexed column moves the visible heap tuple. After
            // VACUUM, system columns must describe that tuple, not the old CTID
            // in the index.
            let ctid_query = "SELECT ctid::text FROM system_columns_a WHERE id = 2";
            let old_ctid: Vec<String> = ctid_query.fetch_scalar(&mut conn);
            "UPDATE system_columns_a SET note = 'updated' WHERE id = 2".execute(&mut conn);
            "VACUUM (FREEZE, TRUNCATE false) system_columns_a".execute(&mut conn);
            let new_ctid: Vec<String> = ctid_query.fetch_scalar(&mut conn);
            assert_ne!(new_ctid, old_ctid, "the update should move the tuple");

            (
                format!(
                    r#"
                    SELECT a.id, b.id AS b_id, a.note, {SYSTEM_COLUMNS}
                    FROM system_columns_a a JOIN system_columns_b b ON a.id = b.a_id
                    WHERE a.val ||| 'val'
                    ORDER BY a.id LIMIT 3
                    "#
                ),
                3,
            )
        }
    };

    "SET paradedb.enable_join_custom_scan = on; DISCARD PLANS;".execute(&mut conn);
    let plan: Vec<String> = format!("EXPLAIN (COSTS OFF) {query}").fetch_scalar(&mut conn);
    let plan = plan.join("\n");
    assert!(plan.contains(JOIN_SCAN), "{plan}");
    let joinscan_rows = fetch_raw(&query, &mut conn);

    "SET paradedb.enable_join_custom_scan = off; DISCARD PLANS;".execute(&mut conn);
    let postgres_rows = fetch_raw(&query, &mut conn);

    assert_eq!(postgres_rows.len(), expected_rows, "{query}");
    assert_eq!(joinscan_rows, postgres_rows, "{query}");
}

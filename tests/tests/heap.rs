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
use sqlx::PgConnection;
use tests::fixtures::*;

#[rstest]
fn mvcc_heap_filter(mut conn: PgConnection) {
    r#"
        CALL paradedb.create_paradedb_test_table(table_name => 'heap_and_clauses_table', schema_name => 'public');

        CREATE INDEX heap_and_clauses_idx ON heap_and_clauses_table
        USING paradedb (id, description);
    "#.execute(&mut conn);

    // Ensure that heap filters continue to be applied correctly in the presence of updates.
    for _ in 0..128 {
        let results: Vec<(i32, String)> = r#"
            SELECT id, description
            FROM heap_and_clauses_table
            WHERE id @@@ paradedb.match('description', 'Sleek running', conjunction_mode := true)
            AND description ILIKE 'Sleek running shoes'
            ORDER BY id;
        "#
        .fetch(&mut conn);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 3);
        assert_eq!(results[0].1, "Sleek running shoes");

        r#"
            UPDATE heap_and_clauses_table SET last_updated_date = NOW();
        "#
        .execute(&mut conn);
    }
}

#[rstest]
fn mvcc_snippet(mut conn: PgConnection) {
    r#"
        CALL paradedb.create_paradedb_test_table(table_name => 'mock_items', schema_name => 'public');
        
        CREATE INDEX mock_items_idx ON mock_items
        USING paradedb (id, description);
    "#
    .execute(&mut conn);

    // Ensure that snippet lookups from the heap succeed in the presence of updates.
    for _ in 0..128 {
        let results: Vec<(i32, String)> = r#"
            SELECT id, pdb.snippet(description)
            FROM mock_items
            WHERE description ||| 'shoes'
            ORDER BY id
            LIMIT 5;
        "#
        .fetch(&mut conn);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, 3);
        assert_eq!(results[0].1, "Sleek running <b>shoes</b>");
        assert_eq!(results[1].0, 4);
        assert_eq!(results[1].1, "White jogging <b>shoes</b>");
        assert_eq!(results[2].0, 5);
        assert_eq!(results[2].1, "Generic <b>shoes</b>");

        r#"
            UPDATE mock_items SET last_updated_date = NOW();
        "#
        .execute(&mut conn);
    }
}

#[rstest]
#[case("ASC NULLS FIRST")]
#[case("DESC NULLS LAST")]
fn visibility_map_shortcuts(#[case] direction: &str, mut conn: PgConnection) {
    let default: (String,) = "SHOW paradedb.enable_visibility_map_shortcuts".fetch_one(&mut conn);
    assert_eq!(default.0, "on");
    format!(
        r#"
        SET max_parallel_workers_per_gather = 0;
        CREATE TABLE visibility_blocks (
            id int PRIMARY KEY, title text, value int, padding text
        ) WITH (autovacuum_enabled = false, fillfactor = 70);
        INSERT INTO visibility_blocks
        SELECT id, 'database', id, repeat('x', 500) FROM generate_series(1, 10000) id;
        CREATE INDEX visibility_blocks_idx ON visibility_blocks USING paradedb (id, title, value)
        WITH (sort_by = 'ctid {direction}', mutable_segment_rows = 0, target_segment_count = 1);
        "#
    )
    .execute(&mut conn);
    "VACUUM (INDEX_CLEANUP ON, ANALYZE) visibility_blocks".execute(&mut conn);

    for phase in 0..3 {
        if phase == 1 {
            // Leave dirty blocks, stale index entries, and HOT chains alongside visible blocks.
            "UPDATE visibility_blocks SET padding = 'changed' WHERE id % 997 = 0"
                .execute(&mut conn);
            "UPDATE visibility_blocks SET title = 'postgres', value = -id WHERE id % 991 = 0"
                .execute(&mut conn);
            "DELETE FROM visibility_blocks WHERE id % 983 = 0".execute(&mut conn);
        } else if phase == 2 {
            "VACUUM (INDEX_CLEANUP ON, ANALYZE) visibility_blocks".execute(&mut conn);
        }
        let expected_aggregate: (i64, i64) =
            "SELECT count(*), sum(value) FROM visibility_blocks WHERE title = 'database'"
                .fetch_one(&mut conn);
        let expected_rows: Vec<(i32, i32, String)> =
            "SELECT id, value, ctid::text FROM visibility_blocks WHERE title = 'database' ORDER BY id"
                .fetch(&mut conn);

        for enabled in [true, false] {
            format!("SET paradedb.enable_visibility_map_shortcuts = {enabled}").execute(&mut conn);
            let query =
                "SELECT count(*), sum(value) FROM visibility_blocks WHERE title === 'database'";
            let (plan,): (serde_json::Value,) =
                format!("EXPLAIN (FORMAT JSON) {query}").fetch_one(&mut conn);
            assert!(plan.to_string().contains("ParadeDB Aggregate Scan"));
            let aggregate: (i64, i64) = query.fetch_one(&mut conn);
            let rows: Vec<(i32, i32, String)> =
                "SELECT id, value, ctid::text FROM visibility_blocks WHERE title === 'database' ORDER BY id"
                    .fetch(&mut conn);
            assert_eq!(
                aggregate, expected_aggregate,
                "phase {phase}, shortcuts {enabled}"
            );
            assert_eq!(rows, expected_rows, "phase {phase}, shortcuts {enabled}");
        }
    }
}

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

mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn mvcc_heap_filter(mut conn: PgConnection) {
    r#"
        CALL paradedb.create_bm25_test_table(table_name => 'heap_and_clauses_table', schema_name => 'public');

        CREATE INDEX heap_and_clauses_idx ON heap_and_clauses_table
        USING bm25 (id, description)
        WITH (key_field = 'id');
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
    if pg_major_version(&mut conn) <= 14 {
        // TODO: See https://github.com/paradedb/paradedb/issues/3358.
        return;
    }

    r#"
        CALL paradedb.create_bm25_test_table(table_name => 'mock_items', schema_name => 'public');
        
        CREATE INDEX mock_items_idx ON mock_items
        USING bm25 (id, description)
        WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // Ensure that snippet lookups from the heap succeed in the presence of updates.
    for _ in 0..128 {
        let results: Vec<(i32, String)> = r#"
            SELECT id, pdb.snippet(description)
            FROM mock_items
            WHERE description @@@ 'shoes'
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

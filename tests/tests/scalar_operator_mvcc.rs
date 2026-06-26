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

//! Regression test for https://github.com/paradedb/paradedb/issues/5365
//!
//! The scalar `key @@@ query` operator (`search_with_query_input`) answers purely from the
//! index. It must apply the same heap MVCC visibility the heap-driven scan applies, otherwise a
//! committed-but-invisible row that matches the query in the index leaks through the scalar path
//! while the heap-driven path correctly hides it.

mod fixtures;

use anyhow::Result;
use fixtures::*;
use rstest::*;

#[rstest]
#[tokio::test]
async fn scalar_operator_respects_mvcc_visibility(database: Db) -> Result<()> {
    let mut writer = database.connection().await;
    let mut reader = database.connection().await;

    r#"
    CREATE EXTENSION IF NOT EXISTS pg_search;
    DROP TABLE IF EXISTS mvcc_items;
    CREATE TABLE mvcc_items (id text PRIMARY KEY, body text);
    CREATE INDEX mvcc_items_idx ON mvcc_items USING bm25 (id, body)
    WITH (
        key_field = 'id',
        text_fields = '{"id": {"tokenizer": {"type": "keyword"}}, "body": {"fast": true}}',
        mutable_segment_rows = 10000,
        background_layer_sizes = '0'
    );
    INSERT INTO mvcc_items VALUES ('visible', 'body before snapshot');
    "#
    .execute(&mut writer);

    // Establish a REPEATABLE READ snapshot on the reader *before* 'later' is inserted.
    "BEGIN ISOLATION LEVEL REPEATABLE READ".execute(&mut reader);
    let before: Vec<(String,)> = "SELECT id FROM mvcc_items ORDER BY id".fetch(&mut reader);
    assert_eq!(before, vec![("visible".to_string(),)]);

    // A different transaction commits a row that is invisible to the reader's snapshot.
    "INSERT INTO mvcc_items (id, body) VALUES ('later', NULL)".execute(&mut writer);

    // The reader's snapshot still only sees the pre-existing row.
    let after: Vec<(String,)> = "SELECT id FROM mvcc_items ORDER BY id".fetch(&mut reader);
    assert_eq!(after, vec![("visible".to_string(),)]);

    let query = r#"paradedb.with_index(
        'mvcc_items_idx'::regclass,
        paradedb.boolean(must => ARRAY[paradedb.all()], must_not => ARRAY[paradedb.exists('body')])
    )"#;

    // Heap-driven path: never returns the invisible 'later'.
    let heap_driven: Vec<(String,)> =
        format!("SELECT id FROM mvcc_items WHERE id @@@ {query} ORDER BY id").fetch(&mut reader);
    assert!(
        !heap_driven.iter().any(|(id,)| id == "later"),
        "heap-driven scan leaked an invisible row: {heap_driven:?}"
    );

    // Direct scalar key-set path: must obey the same snapshot. Before the fix this returned true.
    let (scalar_later,): (bool,) =
        format!("SELECT 'later'::text @@@ {query}").fetch_one(&mut reader);
    assert!(
        !scalar_later,
        "scalar `@@@` returned true for a row invisible to the snapshot"
    );

    "ROLLBACK".execute(&mut reader);

    // Once 'later' is visible (no snapshot pinning it away), both paths agree it matches.
    let (scalar_later_visible,): (bool,) =
        format!("SELECT 'later'::text @@@ {query}").fetch_one(&mut writer);
    assert!(
        scalar_later_visible,
        "scalar `@@@` should see 'later' once it is visible"
    );

    "DROP TABLE mvcc_items".execute(&mut writer);
    Ok(())
}

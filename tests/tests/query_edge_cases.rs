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
fn select_everything(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value text
    );
    INSERT INTO test_table (value) VALUES ('beer'), ('wine'), ('cheese');
    CREATE INDEX test_index ON test_table USING bm25 (id, value) WITH (key_field='id');
    "#
    .execute(&mut conn);

    r#"set paradedb.enable_custom_scan to off; set max_parallel_workers_per_gather = 0;"#
        .execute(&mut conn);
    let (count,) = r#"SELECT count(*) FROM test_table WHERE id @@@ paradedb.all() OR id > 0"#
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 3);
}

#[rstest]
fn query_empty_table(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS test_table;
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value text[]
    );

    CREATE INDEX test_index ON test_table
    USING bm25 (id, value) WITH (key_field='id', text_fields='{"value": {}}');
    "#
    .execute(&mut conn);

    "SET max_parallel_workers = 0;".execute(&mut conn);
    let (count,) =
        "SELECT count(*) FROM test_table WHERE value @@@ 'beer';".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 0);

    "SET max_parallel_workers = 8;".execute(&mut conn);
    if pg_major_version(&mut conn) >= 16 {
        "SET debug_parallel_query TO on".execute(&mut conn);
    }
    let (count,) =
        "SELECT count(*) FROM test_table WHERE value @@@ 'beer';".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 0);
}

#[rstest]
fn unary_not_issue2141(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value text[]
    );

    INSERT INTO test_table (value) VALUES (ARRAY['beer', 'cheese']), (ARRAY['beer', 'wine']), (ARRAY['beer']), (ARRAY['beer']);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value) WITH (key_field='id', text_fields='{"value": {}}');
    "#
    .execute(&mut conn);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE value @@@ 'beer';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 4);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE NOT value @@@ 'beer';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 0);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE value @@@ 'wine';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE NOT value @@@ 'wine';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 3);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE value @@@ 'wine' AND NOT value @@@ 'cheese';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 1);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE NOT value @@@ 'wine' OR NOT value @@@ 'missing';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 4);

    let (count,) = r#"
    SELECT count(*) FROM test_table WHERE NOT value @@@ 'wine' AND NOT value @@@ 'cheese';
    "#
    .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 2);
}

#[rstest]
fn not_operator_preserves_null_semantics_issue_5264(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS min_repro;
    CREATE TABLE min_repro (
        id INTEGER PRIMARY KEY,
        color TEXT
    );

    INSERT INTO min_repro (id, color) VALUES
        (1, 'blue'),
        (2, 'red'),
        (3, NULL);

    CREATE INDEX min_repro_idx ON min_repro
    USING bm25 (id, color) WITH (
        key_field = 'id',
        text_fields = '{"color": {"tokenizer": {"type": "keyword"}, "fast": true}}'
    );
    "#
    .execute(&mut conn);

    let postgres_rows: Vec<(i32,)> = r#"
    SELECT id FROM min_repro WHERE NOT (color = 'blue') ORDER BY id;
    "#
    .fetch(&mut conn);

    let indexed_rows: Vec<(i32,)> = r#"
    SELECT id FROM min_repro WHERE NOT (color @@@ 'blue') ORDER BY id;
    "#
    .fetch(&mut conn);

    assert_eq!(postgres_rows, vec![(2,)]);
    assert_eq!(indexed_rows, postgres_rows);
}

#[rstest]
fn negated_boolean_composition_preserves_null_semantics_issue_5264(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS bool_comp_repro;
    CREATE TABLE bool_comp_repro (
        id INTEGER PRIMARY KEY,
        color TEXT,
        shape TEXT
    );

    INSERT INTO bool_comp_repro (id, color, shape) VALUES
        (1, 'blue', 'square'),
        (2, 'red', 'square'),
        (3, NULL, 'square'),
        (4, 'red', 'circle'),
        (5, NULL, 'circle');

    CREATE INDEX bool_comp_repro_idx ON bool_comp_repro
    USING bm25 (id, color, shape) WITH (
        key_field = 'id',
        text_fields = '{
            "color": {"tokenizer": {"type": "keyword"}, "fast": true},
            "shape": {"tokenizer": {"type": "keyword"}, "fast": true}
        }'
    );
    "#
    .execute(&mut conn);

    let postgres_and_rows: Vec<(i32,)> = r#"
    SELECT id FROM bool_comp_repro
    WHERE NOT ((color = 'blue') AND (shape = 'square'))
    ORDER BY id;
    "#
    .fetch(&mut conn);

    let indexed_and_rows: Vec<(i32,)> = r#"
    SELECT id FROM bool_comp_repro
    WHERE NOT ((color @@@ 'blue') AND (shape @@@ 'square'))
    ORDER BY id;
    "#
    .fetch(&mut conn);

    let postgres_or_rows: Vec<(i32,)> = r#"
    SELECT id FROM bool_comp_repro
    WHERE NOT ((color = 'blue') OR (shape = 'square'))
    ORDER BY id;
    "#
    .fetch(&mut conn);

    let indexed_or_rows: Vec<(i32,)> = r#"
    SELECT id FROM bool_comp_repro
    WHERE NOT ((color @@@ 'blue') OR (shape @@@ 'square'))
    ORDER BY id;
    "#
    .fetch(&mut conn);

    assert_eq!(indexed_and_rows, postgres_and_rows);
    assert_eq!(indexed_or_rows, postgres_or_rows);
}

#[rstest]
fn bitmap_index_scan_preserves_null_semantics_issue_5264(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS bitmap_repro;
    CREATE TABLE bitmap_repro (
        id SERIAL8 PRIMARY KEY,
        quantity INTEGER
    );

    INSERT INTO bitmap_repro (quantity) VALUES
        (7),
        (8),
        (NULL);

    CREATE INDEX bitmap_repro_idx ON bitmap_repro
    USING bm25 (id, quantity) WITH (
        key_field = 'id',
        numeric_fields = '{"quantity": {"fast": true}}'
    );

    SET paradedb.enable_aggregate_custom_scan TO off;
    SET paradedb.enable_custom_scan TO off;
    SET paradedb.enable_custom_scan_without_operator TO off;
    SET paradedb.enable_filter_pushdown TO off;
    SET paradedb.enable_join_custom_scan TO off;
    SET enable_seqscan TO off;
    SET enable_indexscan TO off;
    SET max_parallel_workers TO 0;
    SET parallel_leader_participation TO off;
    "#
    .execute(&mut conn);

    let (postgres_count,) = r#"
    SELECT COUNT(*) FROM bitmap_repro WHERE NOT (quantity = 7);
    "#
    .fetch_one::<(i64,)>(&mut conn);

    let (bm25_count,) = r#"
    SELECT COUNT(*) FROM bitmap_repro WHERE NOT (quantity @@@ '7');
    "#
    .fetch_one::<(i64,)>(&mut conn);

    assert_eq!(postgres_count, 1);
    assert_eq!(bm25_count, postgres_count);
}

#[rstest]
fn negated_exists_returns_missing_rows_issue_5264(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS exists_repro;
    CREATE TABLE exists_repro (
        id INTEGER PRIMARY KEY,
        color TEXT
    );

    INSERT INTO exists_repro (id, color) VALUES
        (1, 'blue'),
        (2, NULL),
        (3, 'red'),
        (4, NULL);

    CREATE INDEX exists_repro_idx ON exists_repro
    USING bm25 (id, color) WITH (
        key_field = 'id',
        text_fields = '{"color": {"tokenizer": {"type": "keyword"}, "fast": true}}'
    );
    "#
    .execute(&mut conn);

    // Postgres truth: `NOT (color IS NOT NULL)` is the rows where color is missing.
    let missing_rows: Vec<(i32,)> = r#"
    SELECT id FROM exists_repro WHERE color IS NULL ORDER BY id;
    "#
    .fetch(&mut conn);
    assert_eq!(missing_rows, vec![(2,), (4,)]);

    // `exists` itself returns the rows where the field is present.
    let present_rows: Vec<(i32,)> = r#"
    SELECT id FROM exists_repro WHERE id @@@ paradedb.exists('color') ORDER BY id;
    "#
    .fetch(&mut conn);
    assert_eq!(present_rows, vec![(1,), (3,)]);

    // Negating `exists` must return the rows where the field is missing, not an
    // unsatisfiable `exists AND NOT exists`. Custom scan path:
    let negated_custom_scan: Vec<(i32,)> = r#"
    SELECT id FROM exists_repro WHERE NOT (id @@@ paradedb.exists('color')) ORDER BY id;
    "#
    .fetch(&mut conn);
    assert_eq!(negated_custom_scan, missing_rows);

    // Same query, but forcing the direct `search_with_query_input` operator path.
    r#"
    SET paradedb.enable_aggregate_custom_scan TO off;
    SET paradedb.enable_custom_scan TO off;
    SET paradedb.enable_custom_scan_without_operator TO off;
    SET paradedb.enable_filter_pushdown TO off;
    SET max_parallel_workers TO 0;
    SET parallel_leader_participation TO off;
    "#
    .execute(&mut conn);

    let negated_operator_path: Vec<(i32,)> = r#"
    SELECT id FROM exists_repro WHERE NOT (id @@@ paradedb.exists('color')) ORDER BY id;
    "#
    .fetch(&mut conn);
    assert_eq!(negated_operator_path, missing_rows);

    // A `boost`/`const_score` wrapper around `exists` must still be recognized
    // as an existence predicate (the wrapper is unwrapped), so the operator
    // returns false (not NULL) for missing fields and negating it returns the
    // missing rows. Without the unwrap, missing rows would evaluate to NULL and
    // drop out of the negation.
    let negated_boosted_exists: Vec<(i32,)> = r#"
    SELECT id FROM exists_repro
    WHERE NOT (id @@@ paradedb.boost(2.0, paradedb.exists('color'))) ORDER BY id;
    "#
    .fetch(&mut conn);
    assert_eq!(negated_boosted_exists, missing_rows);

    let negated_const_score_exists: Vec<(i32,)> = r#"
    SELECT id FROM exists_repro
    WHERE NOT (id @@@ paradedb.const_score(1.0, paradedb.exists('color'))) ORDER BY id;
    "#
    .fetch(&mut conn);
    assert_eq!(negated_const_score_exists, missing_rows);
}

#[rstest]
fn negated_predicate_preserves_empty_array_not_null_issue_5264(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS array_repro;
    CREATE TABLE array_repro (
        id INTEGER PRIMARY KEY,
        tags TEXT[]
    );

    INSERT INTO array_repro (id, tags) VALUES
        (1, ARRAY['beer']),
        (2, '{}'),
        (3, NULL);

    CREATE INDEX array_repro_idx ON array_repro
    USING bm25 (id, tags) WITH (
        key_field = 'id',
        text_fields = '{"tags": {"tokenizer": {"type": "keyword"}, "fast": true}}'
    );
    "#
    .execute(&mut conn);

    // Empty array is SQL NOT NULL; the null-preserving guard must not treat it as NULL.
    let indexed_rows: Vec<(i32,)> = r#"
    SELECT id FROM array_repro WHERE NOT (tags @@@ 'beer') ORDER BY id;
    "#
    .fetch(&mut conn);

    assert!(indexed_rows.iter().any(|(id,)| *id == 2));
    assert!(!indexed_rows.iter().any(|(id,)| *id == 1));
}

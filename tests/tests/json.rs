// Copyright (c) 2023-2025 ParadeDB, Inc.
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

// In addition to checking whether all the expected types work for keys, make sure to include tests for anything that
//    is reliant on keys (e.g. stable_sort, alias)

#[rstest]
fn json_datatype(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id serial8,
        value json
    );

    INSERT INTO test_table (value) VALUES ('{"currency_code": "USD", "salary": 120000 }');
    INSERT INTO test_table (value) VALUES ('{"currency_code": "USD", "salary": 75000 }');
    INSERT INTO test_table (value) VALUES ('{"currency_code": "USD", "salary": 140000 }');
    "#
    .execute(&mut conn);

    // if we don't segfault postgres here, we're good
    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value) WITH (key_field='id', json_fields='{"value": {"indexed": true, "fast": true}}');
    "#
    .execute(&mut conn);
}

#[rstest]
fn simple_jsonb_string_array_crash(mut conn: PgConnection) {
    // ensure that we can index top-level json arrays that are strings.
    // Prior to 82fb7126ce6d2368cf19dd4dc6e28915afc5cf1e (PR #1618, <=v0.9.4) this didn't work

    r#"    
    CREATE TABLE crash
    (
        id serial8,
        j  jsonb
    );
    
    INSERT INTO crash (j) SELECT '["one-element-string-array"]' FROM generate_series(1, 10000);
    
    CREATE INDEX crash_idx ON crash
    USING bm25 (id, j) WITH (key_field='id', json_fields='{"j": {"indexed": true, "fast": true}}');
    "#
    .execute(&mut conn);
}

// ============================================================================
// Field-agnostic JSON search tests (Issue #2769)
// ============================================================================

#[rstest]
fn field_agnostic_term_finds_json_string(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        data JSONB
    );

    INSERT INTO test_table (data) VALUES ('{"name": "alice", "age": 30}');
    INSERT INTO test_table (data) VALUES ('{"name": "bob", "age": 25}');
    INSERT INTO test_table (data) VALUES ('{"nested": {"value": "alice"}}');

    CREATE INDEX test_idx ON test_table
    USING bm25 (id, data) WITH (key_field='id', json_fields='{"data": {}}');
    "#
    .execute(&mut conn);

    // Field-agnostic term should find "alice" in JSON
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_table WHERE test_table.id @@@ paradedb.term(value => 'alice')
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, 1);
    assert_eq!(rows[1].0, 3);
}

#[rstest]
fn field_agnostic_term_finds_json_numeric(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        data JSONB
    );

    INSERT INTO test_table (data) VALUES ('{"count": 42}');
    INSERT INTO test_table (data) VALUES ('{"count": 100}');
    INSERT INTO test_table (data) VALUES ('{"nested": {"count": 42}}');

    CREATE INDEX test_idx ON test_table
    USING bm25 (id, data) WITH (key_field='id', json_fields='{"data": {}}');
    "#
    .execute(&mut conn);

    // Field-agnostic term should find numeric 42 in JSON
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_table WHERE test_table.id @@@ paradedb.term(value => 42)
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, 1);
    assert_eq!(rows[1].0, 3);
}

#[rstest]
fn field_agnostic_match_tokenizes_json_strings(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        data JSONB
    );

    INSERT INTO test_table (data) VALUES ('{"description": "hello world"}');
    INSERT INTO test_table (data) VALUES ('{"description": "goodbye world"}');
    INSERT INTO test_table (data) VALUES ('{"title": "hello there"}');

    CREATE INDEX test_idx ON test_table
    USING bm25 (id, data) WITH (key_field='id', json_fields='{"data": {}}');
    "#
    .execute(&mut conn);

    // Field-agnostic match should tokenize and find "hello"
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_table WHERE test_table.id @@@ paradedb.match(value => 'hello')
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, 1);
    assert_eq!(rows[1].0, 3);
}

#[rstest]
fn lenient_parse_finds_json_values(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        data JSONB
    );

    INSERT INTO test_table (data) VALUES ('{"status": "active"}');
    INSERT INTO test_table (data) VALUES ('{"status": "inactive"}');

    CREATE INDEX test_idx ON test_table
    USING bm25 (id, data) WITH (key_field='id', json_fields='{"data": {}}');
    "#
    .execute(&mut conn);

    // Lenient parse should find values in JSON
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_table WHERE test_table.id @@@ paradedb.parse('active', lenient => true)
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, 1);
}

#[rstest]
fn mixed_schema_text_and_json(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        title TEXT,
        metadata JSONB
    );

    INSERT INTO test_table (title, metadata) VALUES ('hello', '{"tag": "world"}');
    INSERT INTO test_table (title, metadata) VALUES ('world', '{"tag": "hello"}');
    INSERT INTO test_table (title, metadata) VALUES ('other', '{"tag": "other"}');

    CREATE INDEX test_idx ON test_table
    USING bm25 (id, title, metadata) WITH (
        key_field='id',
        text_fields='{"title": {}}',
        json_fields='{"metadata": {}}'
    );
    "#
    .execute(&mut conn);

    // Should find "hello" in both TEXT and JSON fields
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_table WHERE test_table.id @@@ paradedb.term(value => 'hello')
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, 1); // title = 'hello'
    assert_eq!(rows[1].0, 2); // metadata.tag = 'hello'
}

#[rstest]
fn field_agnostic_no_match_returns_empty(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        data JSONB
    );

    INSERT INTO test_table (data) VALUES ('{"name": "alice"}');

    CREATE INDEX test_idx ON test_table
    USING bm25 (id, data) WITH (key_field='id', json_fields='{"data": {}}');
    "#
    .execute(&mut conn);

    // Should return no results for non-existent value
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_table WHERE test_table.id @@@ paradedb.term(value => 'nonexistent')
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 0);
}

#[rstest]
fn field_agnostic_term_finds_json_bool(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        data JSONB
    );

    INSERT INTO test_table (data) VALUES ('{"active": true}');
    INSERT INTO test_table (data) VALUES ('{"active": false}');
    INSERT INTO test_table (data) VALUES ('{"nested": {"active": true}}');

    CREATE INDEX test_idx ON test_table
    USING bm25 (id, data) WITH (key_field='id', json_fields='{"data": {}}');
    "#
    .execute(&mut conn);

    // Field-agnostic term should find boolean true in JSON
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_table WHERE test_table.id @@@ paradedb.term(value => true)
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, 1);
    assert_eq!(rows[1].0, 3);
}

#[rstest]
fn field_agnostic_fuzzy_match_finds_json_string(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        data JSONB
    );

    INSERT INTO test_table (data) VALUES ('{"name": "alice"}');
    INSERT INTO test_table (data) VALUES ('{"name": "bob"}');
    INSERT INTO test_table (data) VALUES ('{"name": "alise"}');

    CREATE INDEX test_idx ON test_table
    USING bm25 (id, data) WITH (key_field='id', json_fields='{"data": {}}');
    "#
    .execute(&mut conn);

    // Fuzzy match with distance=1 should find both "alice" and "alise" (1 char difference)
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_table WHERE test_table.id @@@ paradedb.match(value => 'alice', distance => 1)
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, 1); // "alice" exact match
    assert_eq!(rows[1].0, 3); // "alise" is 1 edit away
}

#[rstest]
fn field_agnostic_prefix_match_finds_json_string(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        data JSONB
    );

    INSERT INTO test_table (data) VALUES ('{"name": "alexander"}');
    INSERT INTO test_table (data) VALUES ('{"name": "bob"}');
    INSERT INTO test_table (data) VALUES ('{"name": "alexis"}');

    CREATE INDEX test_idx ON test_table
    USING bm25 (id, data) WITH (key_field='id', json_fields='{"data": {}}');
    "#
    .execute(&mut conn);

    // Prefix match with distance=1 should match words starting with "alx" (typo for "alex")
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_table WHERE test_table.id @@@ paradedb.match(value => 'alx', distance => 1, prefix => true)
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, 1); // "alexander" starts with "alex" (distance 1 from "alx")
    assert_eq!(rows[1].0, 3); // "alexis" starts with "alex" (distance 1 from "alx")
}

#[rstest]
fn field_agnostic_transposition_cost_one_works(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        data JSONB
    );

    INSERT INTO test_table (data) VALUES ('{"word": "the"}');
    INSERT INTO test_table (data) VALUES ('{"word": "cat"}');

    CREATE INDEX test_idx ON test_table
    USING bm25 (id, data) WITH (key_field='id', json_fields='{"data": {}}');
    "#
    .execute(&mut conn);

    // With transposition_cost_one=true (default), 'teh' -> 'the' is 1 edit (swap e and h)
    let rows_true: Vec<(i32,)> = r#"
        SELECT id FROM test_table WHERE test_table.id @@@ paradedb.match(value => 'teh', distance => 1, transposition_cost_one => true)
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows_true.len(), 1);
    assert_eq!(rows_true[0].0, 1); // "the" found with transposition

    // With transposition_cost_one=false, 'teh' -> 'the' is 2 edits, so distance=1 won't match
    let rows_false: Vec<(i32,)> = r#"
        SELECT id FROM test_table WHERE test_table.id @@@ paradedb.match(value => 'teh', distance => 1, transposition_cost_one => false)
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows_false.len(), 0); // No match when transposition counts as 2
}

#[rstest]
fn field_agnostic_match_with_expand_dots_false(mut conn: PgConnection) {
    // Test that literal dot keys work correctly when expand_dots=false
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        data JSONB
    );

    -- JSON with a literal dot in the key name (not nested)
    INSERT INTO test_table (data) VALUES ('{"user.name": "alice"}');
    -- JSON with actual nested structure
    INSERT INTO test_table (data) VALUES ('{"user": {"name": "bob"}}');

    -- Create index with expand_dots=false so "user.name" stays as a literal key
    CREATE INDEX test_idx ON test_table
    USING bm25 (id, data) WITH (key_field='id', json_fields='{"data": {"expand_dots": false}}');
    "#
    .execute(&mut conn);

    // Field-agnostic match should find "alice" in the literal "user.name" key
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_table WHERE test_table.id @@@ paradedb.match(value => 'alice')
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, 1); // Found in literal "user.name" key

    // Should also find "bob" in the nested structure
    let rows2: Vec<(i32,)> = r#"
        SELECT id FROM test_table WHERE test_table.id @@@ paradedb.match(value => 'bob')
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows2.len(), 1);
    assert_eq!(rows2[0].0, 2); // Found in nested user.name
}

#[rstest]
fn field_agnostic_match_with_expand_dots_true(mut conn: PgConnection) {
    // Test that nested paths work correctly when expand_dots=true (default)
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        data JSONB
    );

    -- JSON with nested structure
    INSERT INTO test_table (data) VALUES ('{"user": {"profile": {"name": "alice"}}}');
    INSERT INTO test_table (data) VALUES ('{"user": {"profile": {"name": "bob"}}}');

    -- Create index with expand_dots=true (default)
    CREATE INDEX test_idx ON test_table
    USING bm25 (id, data) WITH (key_field='id', json_fields='{"data": {"expand_dots": true}}');
    "#
    .execute(&mut conn);

    // Field-agnostic match should find "alice" in deeply nested path
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_table WHERE test_table.id @@@ paradedb.match(value => 'alice')
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, 1);

    // Should find "bob" too
    let rows2: Vec<(i32,)> = r#"
        SELECT id FROM test_table WHERE test_table.id @@@ paradedb.match(value => 'bob')
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows2.len(), 1);
    assert_eq!(rows2[0].0, 2);
}

// ============================================================================
// Field-agnostic search tests for TEXT fields
// These tests verify that field-agnostic search works on plain text columns
// ============================================================================

#[rstest]
fn field_agnostic_term_on_text_field(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_text (
        id SERIAL PRIMARY KEY,
        title TEXT,
        description TEXT
    );

    INSERT INTO test_text (title, description) VALUES ('hello', 'world');
    INSERT INTO test_text (title, description) VALUES ('goodbye', 'universe');
    INSERT INTO test_text (title, description) VALUES ('hello', 'there');

    CREATE INDEX test_text_idx ON test_text
    USING bm25 (id, title, description) WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Field-agnostic term should find "hello" in title field
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_text WHERE test_text.id @@@ paradedb.term(value => 'hello')
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, 1);
    assert_eq!(rows[1].0, 3);

    // Field-agnostic term should find "world" in description field
    let rows2: Vec<(i32,)> = r#"
        SELECT id FROM test_text WHERE test_text.id @@@ paradedb.term(value => 'world')
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows2.len(), 1);
    assert_eq!(rows2[0].0, 1);
}

#[rstest]
fn field_agnostic_match_on_text_field(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_text_match (
        id SERIAL PRIMARY KEY,
        title TEXT,
        content TEXT
    );

    INSERT INTO test_text_match (title, content) VALUES ('quick brown fox', 'jumps over lazy dog');
    INSERT INTO test_text_match (title, content) VALUES ('slow white cat', 'sleeps on warm bed');
    INSERT INTO test_text_match (title, content) VALUES ('fast red bird', 'flies through blue sky');

    CREATE INDEX test_text_match_idx ON test_text_match
    USING bm25 (id, title, content) WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Field-agnostic match should find "quick" in title
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_text_match WHERE test_text_match.id @@@ paradedb.match(value => 'quick')
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, 1);

    // Field-agnostic match should find "lazy" in content
    let rows2: Vec<(i32,)> = r#"
        SELECT id FROM test_text_match WHERE test_text_match.id @@@ paradedb.match(value => 'lazy')
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows2.len(), 1);
    assert_eq!(rows2[0].0, 1);
}

#[rstest]
fn field_agnostic_fuzzy_match_on_text_field(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_text_fuzzy (
        id SERIAL PRIMARY KEY,
        name TEXT
    );

    INSERT INTO test_text_fuzzy (name) VALUES ('restaurant');
    INSERT INTO test_text_fuzzy (name) VALUES ('pharmacy');
    INSERT INTO test_text_fuzzy (name) VALUES ('hospital');

    CREATE INDEX test_text_fuzzy_idx ON test_text_fuzzy
    USING bm25 (id, name) WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Fuzzy match with distance=2 should find "restaurant" even with typo "restarant"
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_text_fuzzy WHERE test_text_fuzzy.id @@@ paradedb.match(value => 'restarant', distance => 2)
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, 1);
}

#[rstest]
fn field_agnostic_search_mixed_text_and_json(mut conn: PgConnection) {
    // Test that field-agnostic search works when both text and JSON fields are indexed
    r#"
    CREATE TABLE test_mixed (
        id SERIAL PRIMARY KEY,
        title TEXT,
        metadata JSONB
    );

    INSERT INTO test_mixed (title, metadata) VALUES ('hello world', '{"tag": "greeting"}');
    INSERT INTO test_mixed (title, metadata) VALUES ('goodbye world', '{"tag": "farewell"}');
    INSERT INTO test_mixed (title, metadata) VALUES ('test document', '{"tag": "hello"}');

    CREATE INDEX test_mixed_idx ON test_mixed
    USING bm25 (id, title, metadata) WITH (key_field='id', json_fields='{"metadata": {}}');
    "#
    .execute(&mut conn);

    // Should find "hello" in both text title (row 1) and JSON metadata (row 3)
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_mixed WHERE test_mixed.id @@@ paradedb.match(value => 'hello')
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, 1);
    assert_eq!(rows[1].0, 3);
}

#[rstest]
fn field_agnostic_match_single_positional_arg(mut conn: PgConnection) {
    // Test the exact syntax requested in issue #2769: paradedb.match('search term')
    r#"
    CREATE TABLE test_single_arg (
        id SERIAL PRIMARY KEY,
        title TEXT,
        content TEXT
    );

    INSERT INTO test_single_arg (title, content) VALUES ('running shoes', 'for athletes');
    INSERT INTO test_single_arg (title, content) VALUES ('walking boots', 'for hikers');
    INSERT INTO test_single_arg (title, content) VALUES ('athletic gear', 'running equipment');

    CREATE INDEX test_single_arg_idx ON test_single_arg
    USING bm25 (id, title, content) WITH (key_field='id');
    "#
    .execute(&mut conn);

    // Use the exact syntax: paradedb.match('running') - single positional argument
    let rows: Vec<(i32,)> = r#"
        SELECT id FROM test_single_arg WHERE test_single_arg.id @@@ paradedb.match('running')
        ORDER BY id
    "#
    .fetch(&mut conn);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, 1);
    assert_eq!(rows[1].0, 3);
}

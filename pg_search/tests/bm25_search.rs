// Copyright (c) 2023-2024 Retake, Inc.
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

use anyhow::Result;
use approx::assert_relative_eq;
use core::panic;
use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use shared::fixtures::utils::pg_search_index_directory_path;
use sqlx::PgConnection;
use tantivy::Index;

#[rstest]
async fn basic_search_query(mut conn: PgConnection) -> Result<(), sqlx::Error> {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('description:keyboard OR category:electronics', stable_sort => true)"
            .fetch_collect(&mut conn);

    assert_eq!(
        columns.description,
        concat!(
            "Plastic Keyboard,Ergonomic metal keyboard,Innovative wireless earbuds,",
            "Fast charging power bank,Bluetooth-enabled speaker"
        )
        .split(',')
        .collect::<Vec<_>>()
    );

    assert_eq!(
        columns.category,
        "Electronics,Electronics,Electronics,Electronics,Electronics"
            .split(',')
            .collect::<Vec<_>>()
    );

    Ok(())
}

#[rstest]
async fn basic_search_ids(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('description:keyboard OR category:electronics', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2, 1, 12, 22, 32]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('description:keyboard', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2, 1]);
}

#[rstest]
fn json_search(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('metadata.color:white', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![4, 15, 25]);
}

#[rstest]
fn date_search(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('last_updated_date:[2023-04-15T00:00:00Z TO 2023-04-18T00:00:00Z]', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2, 23, 41]);
}

#[rstest]
fn timestamp_search(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('created_at:[2023-04-15T00:00:00Z TO 2023-04-18T00:00:00Z]', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2, 22, 23, 41]);
}

#[rstest]
fn real_time_search(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "INSERT INTO paradedb.bm25_search (description, rating, category, in_stock, metadata, created_at, last_updated_date, latest_available_time)
        VALUES ('New keyboard', 5, 'Electronics', true, '{}', TIMESTAMP '2023-05-04 11:09:12', DATE '2023-05-06', TIME '10:07:10')"
        .execute(&mut conn);
    "DELETE FROM paradedb.bm25_search WHERE id = 1".execute(&mut conn);
    "UPDATE paradedb.bm25_search SET description = 'PVC Keyboard' WHERE id = 2".execute(&mut conn);

    let columns: SimpleProductsTableVec = "SELECT * FROM bm25_search.search('description:keyboard OR category:electronics', stable_sort => true) ORDER BY id"
        .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2, 12, 22, 32, 42]);
}

#[rstest]
fn sequential_scan_syntax(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let index_oid =
        "SELECT oid::int4 FROM pg_class WHERE relname = 'bm25_search_bm25_index' AND relkind = 'i'"
            .fetch_one::<(i32,)>(&mut conn)
            .0;

    let table_oid =
        "SELECT oid::int4 FROM pg_class WHERE relname = 'bm25_search' AND relkind = 'r'"
            .fetch_one::<(i32,)>(&mut conn)
            .0;

    let database_oid = "SELECT oid::int4 FROM pg_database WHERE datname = current_database();"
        .fetch_one::<(i32,)>(&mut conn)
        .0;

    let columns: SimpleProductsTableVec = format!(
        "SELECT * FROM paradedb.bm25_search
        WHERE paradedb.search_with_search_config(
            id,
            jsonb_build_object(
                'index_name', 'bm25_search_bm25_index',
                'index_oid', {index_oid},
                'table_oid', {table_oid},
                'database_oid', {database_oid},
                'table_name', 'bm25_search',
                'schema_name', 'paradedb',
                'key_field', 'id',
                'query', paradedb.parse('category:electronics')::text::jsonb,
                'uuid', '6817e8d2-0076-4b62-8b50-62869cc033fe'
            )
        ) ORDER BY id"
    )
    .fetch_collect(&mut conn);

    assert_eq!(columns.id, vec![1, 2, 12, 22, 32]);
}

#[rstest]
fn quoted_table_name(mut conn: PgConnection) {
    r#"CREATE TABLE "Activity" (key SERIAL, name TEXT, age INTEGER);
    INSERT INTO "Activity" (name, age) VALUES ('Alice', 29);
    INSERT INTO "Activity" (name, age) VALUES ('Bob', 34);
    INSERT INTO "Activity" (name, age) VALUES ('Charlie', 45);
    INSERT INTO "Activity" (name, age) VALUES ('Diana', 27);
    INSERT INTO "Activity" (name, age) VALUES ('Fiona', 38);
    INSERT INTO "Activity" (name, age) VALUES ('George', 41);
    INSERT INTO "Activity" (name, age) VALUES ('Hannah', 22);
    INSERT INTO "Activity" (name, age) VALUES ('Ivan', 30);
    INSERT INTO "Activity" (name, age) VALUES ('Julia', 25);
    CALL paradedb.create_bm25(
    	index_name => 'activity',
    	table_name => 'Activity',
    	key_field => 'key',
    	text_fields => paradedb.field('name')
    )"#
    .execute(&mut conn);
    let row: (i32, String, i32) =
        "SELECT * FROM activity.search('name:alice', stable_sort => true)".fetch_one(&mut conn);

    assert_eq!(row, (1, "Alice".into(), 29));
}

#[rstest]
fn text_arrays(mut conn: PgConnection) {
    r#"CREATE TABLE example_table (
        id SERIAL PRIMARY KEY,
        text_array TEXT[],
        varchar_array VARCHAR[]
    );
    INSERT INTO example_table (text_array, varchar_array) VALUES 
    ('{"text1", "text2", "text3"}', '{"vtext1", "vtext2"}'),
    ('{"another", "array", "of", "texts"}', '{"vtext3", "vtext4", "vtext5"}'),
    ('{"single element"}', '{"single varchar element"}');
    CALL paradedb.create_bm25(
    	index_name => 'example_table',
    	table_name => 'example_table',
    	key_field => 'id',
    	text_fields => paradedb.field('text_array') || paradedb.field('varchar_array')
    )"#
    .execute(&mut conn);
    let row: (i32,) =
        r#"SELECT * FROM example_table.search('text_array:text1')"#.fetch_one(&mut conn);

    assert_eq!(row, (1,));

    let row: (i32,) =
        r#"SELECT * FROM example_table.search('text_array:"single element"')"#.fetch_one(&mut conn);

    assert_eq!(row, (3,));

    let rows: Vec<(i32,)> =
        r#"SELECT * FROM example_table.search('varchar_array:varchar OR text_array:array')"#
            .fetch(&mut conn);

    assert_eq!(rows[0], (3,));
    assert_eq!(rows[1], (2,));
}

#[rstest]
fn int_arrays(mut conn: PgConnection) {
    r#"CREATE TABLE example_table (
        id SERIAL PRIMARY KEY,
        int_array INT[],
        bigint_array BIGINT[]
    );
    INSERT INTO example_table (int_array, bigint_array) VALUES 
    ('{1, 2, 3}', '{100, 200}'),
    ('{4, 5, 6}', '{300, 400, 500}'),
    ('{7, 8, 9}', '{600, 700, 800, 900}');
    CALL paradedb.create_bm25(
        index_name => 'example_table',
        table_name => 'example_table',
        key_field => 'id',
        numeric_fields => paradedb.field('int_array') || paradedb.field('bigint_array')
    )"#
    .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM example_table.search('int_array:1', stable_sort => true)".fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], (1,));

    let rows: Vec<(i32,)> =
        "SELECT id FROM example_table.search('bigint_array:500', stable_sort => true)"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], (2,));
}

#[rstest]
fn boolean_arrays(mut conn: PgConnection) {
    r#"CREATE TABLE example_table (
        id SERIAL PRIMARY KEY,
        bool_array BOOLEAN[]
    );
    INSERT INTO example_table (bool_array) VALUES 
    ('{true, true, true}'),
    ('{false, false, false}'),
    ('{true, true, false}');
    CALL paradedb.create_bm25(
        index_name => 'example_table',
        table_name => 'example_table',
        key_field => 'id',
        boolean_fields => paradedb.field('bool_array')
    )"#
    .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM example_table.search('bool_array:true', stable_sort => true)"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], (1,));
    assert_eq!(rows[1], (3,));

    let rows: Vec<(i32,)> =
        "SELECT id FROM example_table.search('bool_array:false', stable_sort => true)"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], (2,));
    assert_eq!(rows[1], (3,));
}

#[rstest]
fn datetime_arrays(mut conn: PgConnection) {
    r#"CREATE TABLE example_table (
        id SERIAL PRIMARY KEY,
        date_array DATE[],
        timestamp_array TIMESTAMP[]
    );
    INSERT INTO example_table (date_array, timestamp_array) VALUES 
    (ARRAY['2023-01-01'::DATE, '2023-02-01'::DATE], ARRAY['2023-02-01 12:00:00'::TIMESTAMP, '2023-02-01 13:00:00'::TIMESTAMP]),
    (ARRAY['2023-03-01'::DATE, '2023-04-01'::DATE], ARRAY['2023-04-01 14:00:00'::TIMESTAMP, '2023-04-01 15:00:00'::TIMESTAMP]),
    (ARRAY['2023-05-01'::DATE, '2023-06-01'::DATE], ARRAY['2023-06-01 16:00:00'::TIMESTAMP, '2023-06-01 17:00:00'::TIMESTAMP]);
    CALL paradedb.create_bm25(
        index_name => 'example_table',
        table_name => 'example_table',
        key_field => 'id',
        datetime_fields => paradedb.field('date_array') || paradedb.field('timestamp_array')
    )
    "#.execute(&mut conn);

    let rows: Vec<(i32,)> =
        r#"SELECT id FROM example_table.search('date_array:"2023-02-01T00:00:00Z"', stable_sort => true)"#
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], (1,));

    let rows: Vec<(i32,)> =
        r#"SELECT id FROM example_table.search('timestamp_array:"2023-04-01T15:00:00Z"', stable_sort => true)"#
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], (2,));
}

#[rstest]
fn json_arrays(mut conn: PgConnection) {
    r#"CREATE TABLE example_table (
        id SERIAL PRIMARY KEY,
        json_array JSONB[]
    );
    INSERT INTO example_table (json_array) VALUES 
    (ARRAY['{"name": "John", "age": 30}'::JSONB, '{"name": "Jane", "age": 25}'::JSONB]),
    (ARRAY['{"name": "Bob", "age": 40}'::JSONB, '{"name": "Alice", "age": 35}'::JSONB]),
    (ARRAY['{"name": "Mike", "age": 50}'::JSONB, '{"name": "Lisa", "age": 45}'::JSONB]);"#
        .execute(&mut conn);

    match "CALL paradedb.create_bm25(
        index_name => 'example_table',
        table_name => 'example_table',
        key_field => 'id',
        json_fields => paradedb.field('json_array')
    )"
    .execute_result(&mut conn)
    {
        Ok(_) => panic!("json arrays should not yet be supported"),
        Err(err) => assert!(err.to_string().contains("not yet supported")),
    }
}

#[rstest]
fn uuid(mut conn: PgConnection) {
    r#"
    CREATE TABLE uuid_table (
        id SERIAL PRIMARY KEY,
        random_uuid UUID,
        some_text text
    );

    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('f159c89e-2162-48cd-85e3-e42b71d2ecd0', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('38bf27a0-1aa8-42cd-9cb0-993025e0b8d0', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('b5faacc0-9eba-441a-81f8-820b46a3b57e', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('eb833eb6-c598-4042-b84a-0045828fceea', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('ea1181a0-5d3e-4f5f-a6ab-b1354ffc91ad', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('28b6374a-67d3-41c8-93af-490712f9923e', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('f6e85626-298e-4112-9abb-3856f8aa046a', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('88345d21-7b89-4fd6-87e4-83a4f68dbc3c', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('40bc9216-66d0-4ae8-87ee-ddb02e3e1b33', 'some text');
    INSERT INTO uuid_table (random_uuid, some_text) VALUES ('02f9789d-4963-47d5-a189-d9c114f5cba4', 'some text');
    
    -- Ensure that indexing works with UUID present on table.
    CALL paradedb.create_bm25(
    	index_name => 'uuid_table',
        table_name => 'uuid_table',
        key_field => 'id',
        text_fields => paradedb.field('some_text')
    );
    
    CALL paradedb.drop_bm25('uuid_table');"#
        .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        index_name => 'uuid_table',
        table_name => 'uuid_table',
        key_field => 'id',
        text_fields => paradedb.field('some_text') || paradedb.field('random_uuid')
    )"#
    .execute(&mut conn);

    let rows: Vec<(i32,)> = r#"SELECT * FROM uuid_table.search('some_text:some')"#.fetch(&mut conn);

    assert_eq!(rows.len(), 10);
}

#[rstest]
fn multi_tree(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
	    query => paradedb.boolean(
		    should => ARRAY[
			    paradedb.parse('description:shoes'),
			    paradedb.phrase_prefix(field => 'description', phrases => ARRAY['book']),
			    paradedb.term(field => 'description', value => 'speaker'),
			    paradedb.fuzzy_term(field => 'description', value => 'wolo', transposition_cost_one => false, distance => 1, prefix => true)
		    ]
	    ),
	    stable_sort => true
	);
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![32, 5, 3, 4, 7, 34, 37, 10, 33, 39, 41]);
}

#[rstest]
fn snippet(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let row: (i32, String, f32) = "
        SELECT *
        FROM bm25_search.snippet('description:shoes', highlight_field => 'description', stable_sort => true)"
        .fetch_one(&mut conn);

    assert_eq!(row.0, 5);
    assert_eq!(row.1, "Generic <b>shoes</b>");
    assert_relative_eq!(row.2, 2.8772602, epsilon = 1e-6);

    let row: (i32, String, f32) = "
        SELECT *
        FROM bm25_search.snippet('description:shoes', highlight_field => 'description', stable_sort => true, prefix => '<h1>', postfix => '</h1>')"
        .fetch_one(&mut conn);

    assert_eq!(row.0, 5);
    assert_eq!(row.1, "Generic <h1>shoes</h1>");
    assert_relative_eq!(row.2, 2.8772602, epsilon = 1e-6);
}

#[rstest]
fn hybrid_with_complex_key_field_name(mut conn: PgConnection) {
    // Create a test table.
    "CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');"
        .execute(&mut conn);

    // Add the custom key field column.
    "ALTER TABLE paradedb.bm25_search ADD COLUMN custom_key_field SERIAL".execute(&mut conn);

    // The test table will normally be created here, but it'll be skipped because we did it above.
    SimpleProductsTable::setup_with_key_field("custom_key_field").execute(&mut conn);

    r#"
    CREATE EXTENSION vector;
    ALTER TABLE paradedb.bm25_search ADD COLUMN embedding vector(3);

    UPDATE paradedb.bm25_search m
    SET embedding = ('[' ||
    ((m.id + 1) % 10 + 1)::integer || ',' ||
    ((m.id + 2) % 10 + 1)::integer || ',' ||
    ((m.id + 3) % 10 + 1)::integer || ']')::vector;

    CREATE INDEX on paradedb.bm25_search
    USING hnsw (embedding vector_l2_ops)"#
        .execute(&mut conn);

    let columns: SimpleProductsTableVec = r#"
    SELECT m.*, s.score_hybrid
    FROM paradedb.bm25_search m
    LEFT JOIN (
        SELECT * FROM bm25_search.score_hybrid(
            bm25_query => paradedb.parse('description:keyboard OR category:electronics'),
            similarity_query => '''[1,2,3]'' <-> embedding',
            bm25_weight => 0.9,
            similarity_weight => 0.1
        )
    ) s ON m.custom_key_field = s.custom_key_field
    LIMIT 5"#
        .fetch_collect(&mut conn);

    assert_eq!(columns.id, vec![2, 1, 29, 39, 9]);
}

#[rstest]
fn hybrid_with_single_result(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Here, we'll delete all rows in the table but the first.
    // This previously triggered a "division by zero" error when there was
    // only one result in the similarity query. This test ensures that we
    // check for that condition.
    "DELETE FROM paradedb.bm25_search WHERE id != 1".execute(&mut conn);

    r#"
    CREATE EXTENSION vector;
    ALTER TABLE paradedb.bm25_search ADD COLUMN embedding vector(3);

    UPDATE paradedb.bm25_search m
    SET embedding = ('[' ||
    ((m.id + 1) % 10 + 1)::integer || ',' ||
    ((m.id + 2) % 10 + 1)::integer || ',' ||
    ((m.id + 3) % 10 + 1)::integer || ']')::vector;

    CREATE INDEX on paradedb.bm25_search
    USING hnsw (embedding vector_l2_ops)"#
        .execute(&mut conn);

    let columns: SimpleProductsTableVec = r#"
    SELECT m.*, s.score_hybrid
    FROM paradedb.bm25_search m
    LEFT JOIN (
        SELECT * FROM bm25_search.score_hybrid(
            bm25_query => paradedb.parse('description:keyboard OR category:electronics'),
            similarity_query => '''[1,2,3]'' <-> embedding',
            bm25_weight => 0.9,
            similarity_weight => 0.1
        )
    ) s ON m.id = s.id
    LIMIT 5"#
        .fetch_collect(&mut conn);

    assert_eq!(columns.id, vec![1]);
}

#[rstest]
fn explain(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let plan: Vec<(String,)> =
        "SELECT * FROM bm25_search.explain('description:keyboard OR category:electronics', stable_sort => true)".fetch(&mut conn);

    assert!(plan[0].0.contains("Index Scan"));

    "CALL paradedb.create_bm25_test_table(table_name => 'mock_items', schema_name => 'public');"
        .execute(&mut conn);
    "CALL paradedb.create_bm25(
            index_name => 'search_idx',
            schema_name => 'public',
            table_name => 'mock_items',
            key_field => 'id',
            text_fields => paradedb.field('description', tokenizer => paradedb.tokenizer('en_stem')) || paradedb.field('category')
    )"
    .execute(&mut conn);

    let plan: Vec<(String,)> =
        "SELECT * FROM search_idx.explain('description:keyboard OR category:electronics', stable_sort => true)".fetch(&mut conn);

    assert!(plan[0].0.contains("Index Scan"));
}

#[rstest]
fn update_non_indexed_column(mut conn: PgConnection) -> Result<()> {
    // Create the test table and index.
    "CALL paradedb.create_bm25_test_table(table_name => 'mock_items', schema_name => 'public');"
        .execute(&mut conn);

    // For this test, we'll turn off autovacuum, as we'll be measuring the size of the index.
    // We don't want a vacuum to happen and unexpectedly change the size.
    "ALTER TABLE mock_items SET (autovacuum_enabled = false)"
        .to_string()
        .execute(&mut conn);

    "CALL paradedb.create_bm25(
            index_name => 'search_idx',
            schema_name => 'public',
            table_name => 'mock_items',
            key_field => 'id',
            text_fields => paradedb.field('description', tokenizer => paradedb.tokenizer('en_stem'))
    )"
    .execute(&mut conn);

    let index_dir_path = pg_search_index_directory_path(&mut conn, "search_idx_bm25_index");
    assert!(index_dir_path.exists());

    // Get the index metadata
    let index = Index::open_in_dir(index_dir_path.join("tantivy"))?;
    let reader = index.reader()?;
    let searcher = reader.searcher();
    let total_docs = searcher
        .segment_readers()
        .iter()
        .map(|segment_reader| segment_reader.num_docs())
        .reduce(|acc, count| acc + count)
        .unwrap_or(0);

    assert_eq!(total_docs, 41);

    // Update an indexed column.
    "UPDATE mock_items set description = 'Organic blue tea' WHERE description = 'Organic green tea'"
        .execute(&mut conn);

    reader.reload()?;

    let searcher = reader.searcher();
    let total_docs = searcher
        .segment_readers()
        .iter()
        .map(|segment_reader| segment_reader.num_docs())
        .reduce(|acc, count| acc + count)
        .unwrap_or(0);

    // The total document should be higher, as a new document was created for the updated row.
    assert_eq!(total_docs, 42);

    // Update a non-indexed column.
    "UPDATE mock_items set category = 'Books' WHERE description = 'Sleek running shoes'"
        .execute(&mut conn);

    let searcher = reader.searcher();
    let total_docs = searcher
        .segment_readers()
        .iter()
        .map(|segment_reader| segment_reader.num_docs())
        .reduce(|acc, count| acc + count)
        .unwrap_or(0);

    // The total document count should not have changed when updating a non-indexed column.
    assert_eq!(total_docs, 42);

    Ok(())
}

#[rstest]
async fn json_array_flattening(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Insert a JSON array into the metadata field
    "INSERT INTO paradedb.bm25_search (description, category, rating, in_stock, metadata, created_at, last_updated_date) VALUES 
    ('Product with array', 'Electronics', 4, true, '{\"colors\": [\"red\", \"green\", \"blue\"]}', now(), current_date)"
        .execute(&mut conn);

    // Search for individual elements in the JSON array
    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('metadata.colors:red', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('metadata.colors:green', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('metadata.colors:blue', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);
}

#[rstest]
async fn json_array_multiple_documents(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Insert multiple documents with JSON arrays
    "INSERT INTO paradedb.bm25_search (description, category, rating, in_stock, metadata, created_at, last_updated_date) VALUES 
    ('Product 1', 'Electronics', 5, true, '{\"colors\": [\"red\", \"green\"]}', now(), current_date),
    ('Product 2', 'Electronics', 3, false, '{\"colors\": [\"blue\", \"yellow\"]}', now(), current_date),
    ('Product 3', 'Electronics', 4, true, '{\"colors\": [\"green\", \"blue\"]}', now(), current_date)"
        .execute(&mut conn);

    // Search for individual elements and verify the correct documents are returned
    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('metadata.colors:red', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('metadata.colors:green', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42, 44]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('metadata.colors:blue', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![43, 44]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('metadata.colors:yellow', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![43]);
}

#[rstest]
async fn json_array_mixed_data(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Insert documents with mixed data types in JSON arrays
    "INSERT INTO paradedb.bm25_search (description, category, rating, in_stock, metadata, created_at, last_updated_date) VALUES 
    ('Product with mixed array', 'Electronics', 5, true, '{\"attributes\": [\"fast\", 4, true]}', now(), current_date)"
        .execute(&mut conn);

    // Search for each data type element in the JSON array
    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('metadata.attributes:fast', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('metadata.attributes:4', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('metadata.attributes:true', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);
}

#[rstest]
async fn json_nested_arrays(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Insert a document with nested JSON arrays into the metadata field
    "INSERT INTO paradedb.bm25_search (description, category, rating, in_stock, metadata, created_at, last_updated_date) VALUES 
    ('Product with nested array', 'Electronics', 4, true, '{\"specs\": {\"dimensions\": [\"width\", [\"height\", \"depth\"]]}}', now(), current_date)"
        .execute(&mut conn);

    // Search for elements in the nested JSON arrays
    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('metadata.specs.dimensions:width', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('metadata.specs.dimensions:height', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);

    let columns: SimpleProductsTableVec =
        "SELECT * FROM bm25_search.search('metadata.specs.dimensions:depth', stable_sort => true)"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![42]);
}

#[rstest]
fn bm25_partial_index_search(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "CALL paradedb.create_bm25_test_table(table_name => 'test_partial_index', schema_name => 'paradedb');".execute(&mut conn);

    let ret = "CALL paradedb.create_bm25(
        index_name => 'partial_idx',
        schema_name => 'paradedb',
        table_name => 'test_partial_index',
        key_field => 'id',
        text_fields => paradedb.field('description', tokenizer => paradedb.tokenizer('en_stem')) || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        predicates => 'category = ''Electronics'''
    );"
    .execute_result(&mut conn);
    assert!(ret.is_ok());

    // Ensure returned rows match the predicate
    let columns: SimpleProductsTableVec =
        "SELECT * FROM partial_idx.search( 'rating:>1', limit_rows => 20) ORDER BY rating"
            .fetch_collect(&mut conn);
    assert_eq!(columns.category.len(), 5);
    assert_eq!(
        columns.category,
        "Electronics,Electronics,Electronics,Electronics,Electronics"
            .split(',')
            .collect::<Vec<_>>()
    );
    assert_eq!(columns.rating, vec![3, 4, 4, 4, 5]);

    // Ensure no mismatch rows returned
    let rows: Vec<(String, String)> = "SELECT description, category FROM partial_idx.search( '(description:jeans OR category:Footwear) AND rating:>1', limit_rows => 20) ORDER BY rating".fetch(&mut conn);
    assert_eq!(rows.len(), 0);

    // Insert multiple tuples only 1 matches predicate and query
    "INSERT INTO paradedb.test_partial_index (description, category, rating, in_stock) VALUES 
    ('Product 1', 'Electronics', 2, true),
    ('Product 2', 'Electronics', 1, false),
    ('Product 3', 'Footwear', 2, true)"
        .execute(&mut conn);

    let rows: Vec<(String, i32, String)> = "SELECT description, rating, category FROM partial_idx.search( 'rating:>1', limit_rows => 20) ORDER BY rating".fetch(&mut conn);
    assert_eq!(rows.len(), 6);

    let (desc, rating, category) = rows[0].clone();
    assert_eq!(desc, "Product 1");
    assert_eq!(rating, 2);
    assert_eq!(category, "Electronics");

    // Update one tuple to make it no longer match the predicate
    "UPDATE paradedb.test_partial_index SET category = 'Footwear' WHERE description = 'Product 1'"
        .execute(&mut conn);

    let rows: Vec<(String, i32, String)> = "SELECT description, rating, category FROM partial_idx.search( 'rating:>1', limit_rows => 20) ORDER BY rating".fetch(&mut conn);
    assert_eq!(rows.len(), 5);
    let (desc, ..) = rows[0].clone();
    assert_ne!(desc, "Product 1");

    // Update one tuple to make it match the predicate
    "UPDATE paradedb.test_partial_index SET category = 'Electronics' WHERE description = 'Product 3'"
        .execute(&mut conn);

    let rows: Vec<(String, i32, String)> = "SELECT description, rating, category FROM partial_idx.search( 'rating:>1', limit_rows => 20) ORDER BY rating".fetch(&mut conn);
    assert_eq!(rows.len(), 6);

    let (desc, rating, category) = rows[0].clone();
    assert_eq!(desc, "Product 3");
    assert_eq!(rating, 2);
    assert_eq!(category, "Electronics");

    // Insert one row without specifying the column referenced by the predicate.
    let rows: Vec<(String, i32, String)> = "SELECT description, rating, category FROM partial_idx.search( 'rating:>1', limit_rows => 20) ORDER BY rating".fetch(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn bm25_partial_index_explain(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "CALL paradedb.create_bm25_test_table(table_name => 'test_partial_explain', schema_name => 'paradedb');".execute(&mut conn);

    let ret = "CALL paradedb.create_bm25(
        index_name => 'partial_explain',
        schema_name => 'paradedb',
        table_name => 'test_partial_explain',
        key_field => 'id',
        text_fields => paradedb.field('description', tokenizer => paradedb.tokenizer('en_stem')) || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        predicates => 'category = ''Electronics'''
    );"
    .execute_result(&mut conn);
    assert!(ret.is_ok());

    let plan: Vec<(String,)> =
        "SELECT * FROM partial_explain.explain('rating:>3', stable_sort => true)".fetch(&mut conn);
    assert!(plan[0].0.contains("Index Scan"));

    // Ensure the query plan still includes an Index Scan when the query contains the column referenced by the predicates.
    let plan: Vec<(String,)> =
        "SELECT * FROM partial_explain.explain('rating:>3 AND category:Footwear', stable_sort => true)"
            .fetch(&mut conn);
    assert!(plan[0].0.contains("Index Scan"));
}

#[rstest]
fn bm25_partial_index_hybrid(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "CALL paradedb.create_bm25_test_table(table_name => 'test_partial_hybrid', schema_name => 'paradedb');".execute(&mut conn);

    let ret = "CALL paradedb.create_bm25(
        index_name => 'partial_idx',
        schema_name => 'paradedb',
        table_name => 'test_partial_hybrid',
        key_field => 'id',
        text_fields => paradedb.field('description', tokenizer => paradedb.tokenizer('en_stem')) || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        predicates => 'category = ''Electronics'''
    );"
    .execute_result(&mut conn);
    assert!(ret.is_ok());

    r#"
    CREATE EXTENSION vector;
    ALTER TABLE paradedb.test_partial_hybrid ADD COLUMN embedding vector(3);

    UPDATE paradedb.test_partial_hybrid m
    SET embedding = ('[' ||
    ((m.id + 1) % 10 + 1)::integer || ',' ||
    ((m.id + 2) % 10 + 1)::integer || ',' ||
    ((m.id + 3) % 10 + 1)::integer || ']')::vector;

    CREATE INDEX on paradedb.test_partial_hybrid
    USING hnsw (embedding vector_l2_ops)"#
        .execute(&mut conn);

    // Ensure all of them match the predicate
    let columns: SimpleProductsTableVec = r#"
    SELECT t.*, s.score_hybrid
    FROM paradedb.test_partial_hybrid t
    RIGHT JOIN (
        SELECT * FROM partial_idx.score_hybrid(
            bm25_query => paradedb.parse('rating:>1'),
            similarity_query => '''[1,2,3]'' <-> embedding',
            bm25_weight => 0.9,
            similarity_weight => 0.1
        )
    ) s ON t.id = s.id"#
        .fetch_collect(&mut conn);

    assert_eq!(columns.category.len(), 5);
    assert_eq!(
        columns.category,
        "Electronics,Electronics,Electronics,Electronics,Electronics"
            .split(',')
            .collect::<Vec<_>>()
    );

    "INSERT INTO paradedb.test_partial_hybrid (description, category, rating, in_stock) VALUES
    ('Product 1', 'Electronics', 2, true),
    ('Product 2', 'Electronics', 1, false),
    ('Product 3', 'Footwear', 2, true);

    UPDATE paradedb.test_partial_hybrid m
    SET embedding = ('[' ||
    ((m.id + 1) % 10 + 1)::integer || ',' ||
    ((m.id + 2) % 10 + 1)::integer || ',' ||
    ((m.id + 3) % 10 + 1)::integer || ']')::vector;"
        .execute(&mut conn);

    let rows: Vec<(String,)> = r#"
    SELECT t.category
    FROM paradedb.test_partial_hybrid t
    RIGHT JOIN (
        SELECT * FROM partial_idx.score_hybrid(
            bm25_query => paradedb.parse('rating:>1'),
            similarity_query => '''[1,2,3]'' <-> embedding',
            bm25_weight => 0.9,
            similarity_weight => 0.1
        )
    ) s ON t.id = s.id"#
        .fetch(&mut conn);

    assert_eq!(rows.len(), 7);
    assert_eq!(
        rows.into_iter().map(|(s,)| s).collect::<Vec<_>>(),
        "Electronics,Electronics,Electronics,Electronics,Electronics,Electronics,Electronics"
            .split(',')
            .collect::<Vec<_>>()
    );
}

#[rstest]
fn bm25_partial_index_invalid_statement(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "CALL paradedb.create_bm25_test_table(table_name => 'test_partial_index', schema_name => 'paradedb');".execute(&mut conn);

    // Ensure report error when predicate is invalid
    // unknown column
    let ret = "CALL paradedb.create_bm25(
        index_name => 'partial_idx',
        schema_name => 'paradedb',
        table_name => 'test_partial_index',
        key_field => 'id',
        text_fields => paradedb.field('description', tokenizer => paradedb.tokenizer('en_stem')) || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        predicates => 'city = ''Electronics'''
    );"
    .execute_result(&mut conn);
    assert!(ret.is_err());

    // mismatch type
    let ret = "CALL paradedb.create_bm25(
        index_name => 'partial_idx',
        schema_name => 'paradedb',
        table_name => 'test_partial_index',
        key_field => 'id',
        text_fields => paradedb.field('description', tokenizer => paradedb.tokenizer('en_stem')) || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        predicates => 'category = ''123''::INTEGER'
    );"
    .execute_result(&mut conn);
    assert!(ret.is_err());

    // mismatch schema
    let ret = "CALL paradedb.create_bm25(
        index_name => 'public',
        schema_name => 'paradedb',
        table_name => 'test_partial_index',
        key_field => 'id',
        text_fields => paradedb.field('description', tokenizer => paradedb.tokenizer('en_stem')) || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        predicates => 'category = ''Electronics''' 
    );"
    .execute_result(&mut conn);
    assert!(ret.is_err());

    let ret = "CALL paradedb.create_bm25(
        index_name => 'partial_idx',
        schema_name => 'paradedb',
        table_name => 'test_partial_index',
        key_field => 'id',
        text_fields => paradedb.field('description', tokenizer => paradedb.tokenizer('en_stem')) || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        predicates => 'category = ''Electronics'''
    );"
    .execute_result(&mut conn);
    assert!(ret.is_ok());
}

#[rstest]
fn bm25_partial_index_alter_and_drop(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "CALL paradedb.create_bm25_test_table(table_name => 'test_partial_index', schema_name => 'paradedb');".execute(&mut conn);

    "CALL paradedb.create_bm25(
        index_name => 'partial_idx',
        schema_name => 'paradedb',
        table_name => 'test_partial_index',
        key_field => 'id',
        text_fields => paradedb.field('description', tokenizer => paradedb.tokenizer('en_stem')) || paradedb.field('category'),
        numeric_fields => paradedb.field('rating'),
        predicates => 'category = ''Electronics'''
    );"
    .execute(&mut conn);
    let rows: Vec<(String,)> =
        "SELECT relname FROM pg_class WHERE relname = 'partial_idx_bm25_index';".fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    let rows: Vec<(String,)> =
        "SELECT nspname FROM pg_namespace WHERE nspname = 'partial_idx';".fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // Drop a column that is not referenced in the partial index.
    "ALTER TABLE paradedb.test_partial_index DROP COLUMN metadata;".execute(&mut conn);
    let rows: Vec<(String,)> =
        "SELECT relname FROM pg_class WHERE relname = 'partial_idx_bm25_index';".fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    let rows: Vec<(String,)> =
        "SELECT nspname FROM pg_namespace WHERE nspname = 'partial_idx';".fetch(&mut conn);
    assert_eq!(rows.len(), 1);

    // When the predicate column is dropped with CASCADE, the index and the corresponding
    // schema are both dropped.
    "ALTER TABLE paradedb.test_partial_index DROP COLUMN category CASCADE;".execute(&mut conn);
    let rows: Vec<(String,)> =
        "SELECT relname FROM pg_class WHERE relname = 'partial_idx_bm25_index';".fetch(&mut conn);
    assert_eq!(rows.len(), 0);

    // We need to comment this test out for now, because we've had to change the implementation
    // of paradedb.drop_bm25 to rely on the index OID, which is used to determine the file path
    // for the physical index stored on disk.
    // Unfortunately, we can no longer look up the OID of the index in this situation, because a
    // DROP COLUMN on a partial index deletes the index relation. So the `drop_bm25` call below
    // will panic when no index with the name 'partial_idx_bm25_index' can be found.
    //
    // CALL drop_bm25 could clean it.
    // "CALL paradedb.drop_bm25('partial_idx');".execute(&mut conn);
    // let rows: Vec<(String,)> =
    //     "SELECT relname FROM pg_class WHERE relname = 'partial_idx_bm25_index';".fetch(&mut conn);
    // assert_eq!(rows.len(), 0);
    // let rows: Vec<(String,)> =
    //     "SELECT nspname FROM pg_namespace WHERE nspname = 'partial_idx';".fetch(&mut conn);
    // assert_eq!(rows.len(), 0);
}

#[rstest]
fn high_limit_rows(mut conn: PgConnection) {
    "CREATE TABLE large_series (id SERIAL PRIMARY KEY, description TEXT);".execute(&mut conn);
    "INSERT INTO large_series (description) SELECT 'Product ' || i FROM generate_series(1, 200000) i;"
        .execute(&mut conn);

    "CALL paradedb.create_bm25(
        table_name => 'large_series', 
        schema_name => 'public', 
        index_name => 'large_series', 
        key_field => 'id',
        text_fields => paradedb.field('description')
    );"
    .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id FROM large_series.search('description:Product', limit_rows => 200000)"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 200000);
}

#[rstest]
fn index_size(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let index_dir = pg_search_index_directory_path(&mut conn, "bm25_search_bm25_index");
    assert!(
        index_dir.exists(),
        "expected index directory to exist at: {:?}",
        index_dir
    );

    // Calculate the index size using the new method
    let size: i64 = "SELECT bm25_search.index_size()"
        .fetch_one::<(i64,)>(&mut conn)
        .0;

    // Ensure the size is greater than zero, meaning the index has been created
    assert!(size > 0);
}

#[rstest]
fn multiple_tokenizers_with_alias(mut conn: PgConnection) {
    // Create the table
    "CREATE TABLE products (
        id SERIAL PRIMARY KEY,
        name TEXT,
        description TEXT
    );"
    .execute(&mut conn);

    // Insert mock data
    "INSERT INTO products (name, description) VALUES
    ('Mechanical Keyboard', 'RGB backlit keyboard with Cherry MX switches'),
    ('Wireless Mouse', 'Ergonomic mouse with long battery life'),
    ('4K Monitor', 'Ultra-wide curved display with HDR'),
    ('Gaming Laptop', 'Powerful laptop with dedicated GPU'),
    ('Ergonomic Chair', 'Adjustable office chair with lumbar support'),
    ('Standing Desk', 'Electric height-adjustable desk'),
    ('Noise-Cancelling Headphones', 'Over-ear headphones with active noise cancellation'),
    ('Mechanical Pencil', 'Precision drafting tool with 0.5mm lead'),
    ('Wireless Keyboard', 'Slim keyboard with multi-device support'),
    ('Graphic Tablet', 'Digital drawing pad with pressure sensitivity'),
    ('Curved Monitor', 'Immersive gaming display with high refresh rate'),
    ('Ergonomic Keyboard', 'Split design keyboard for comfortable typing'),
    ('Vertical Mouse', 'Upright mouse design to reduce wrist strain'),
    ('Ultrabook Laptop', 'Thin and light laptop with all-day battery'),
    ('LED Desk Lamp', 'Adjustable lighting with multiple color temperatures');"
        .execute(&mut conn);

    // Create the BM25 index
    "CALL paradedb.create_bm25(
        index_name => 'products_index',
        table_name => 'products',
        key_field => 'id',
        text_fields => 
            paradedb.field('name', tokenizer => paradedb.tokenizer('default')) ||
            paradedb.field('name', alias => 'name_stem', tokenizer => paradedb.tokenizer('default', stemmer => 'English')) ||
            paradedb.field('description', tokenizer => paradedb.tokenizer('default')) ||
            paradedb.field('description', alias => 'description_stem', tokenizer => paradedb.tokenizer('default', stemmer => 'English'))
    );"
    .execute(&mut conn);

    // Test querying with default tokenizer
    let rows: Vec<(i32, String)> =
        "SELECT id, name FROM products_index.search('name:Keyboard')".fetch(&mut conn);
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().any(|(_, name)| name == "Mechanical Keyboard"));
    assert!(rows.iter().any(|(_, name)| name == "Wireless Keyboard"));
    assert!(rows.iter().any(|(_, name)| name == "Ergonomic Keyboard"));

    // Ensure that the default tokenizer doesn't return for stemmed queries
    let rows: Vec<(i32, String)> =
        "SELECT id, name FROM products_index.search('name:Keyboards')".fetch(&mut conn);
    assert_eq!(rows.len(), 0);

    // Test querying with stemmed alias
    let rows: Vec<(i32, String)> =
        "SELECT id, name FROM products_index.search('name_stem:Keyboards')".fetch(&mut conn);
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().any(|(_, name)| name == "Mechanical Keyboard"));
    assert!(rows.iter().any(|(_, name)| name == "Wireless Keyboard"));
    assert!(rows.iter().any(|(_, name)| name == "Ergonomic Keyboard"));

    // Test querying description with default tokenizer
    let rows: Vec<(i32, String)> =
        "SELECT id, name FROM products_index.search('description:battery')".fetch(&mut conn);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|(_, name)| name == "Wireless Mouse"));
    assert!(rows.iter().any(|(_, name)| name == "Ultrabook Laptop"));

    // Ensure that the default tokenizer doesn't return for stemmed queries
    let rows: Vec<(i32, String)> =
        "SELECT id, name FROM products_index.search('description:displaying')".fetch(&mut conn);
    assert_eq!(rows.len(), 0);

    // Test querying description with stemmed alias
    let rows: Vec<(i32, String)> =
        "SELECT id, name FROM products_index.search('description_stem:displaying')"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|(_, name)| name == "4K Monitor"));
    assert!(rows.iter().any(|(_, name)| name == "Curved Monitor"));

    // Test querying with both default and stemmed fields
    let rows: Vec<(i32, String)> =
        "SELECT id, name FROM products_index.search('name:Mouse OR description_stem:mouses')"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|(_, name)| name == "Wireless Mouse"));
    assert!(rows.iter().any(|(_, name)| name == "Vertical Mouse"));
}

#[rstest]
fn alias_cannot_be_key_field(mut conn: PgConnection) {
    // Create the table
    "CREATE TABLE products (
        id SERIAL PRIMARY KEY,
        name TEXT,
        description TEXT
    );"
    .execute(&mut conn);

    // Test alias cannot be the same as key_field
    let result = "CALL paradedb.create_bm25(
        index_name => 'products_index',
        table_name => 'products',
        key_field => 'id',
        text_fields => 
            paradedb.field('name', tokenizer => paradedb.tokenizer('default')) ||
            paradedb.field('description', alias => 'id', tokenizer => paradedb.tokenizer('default', stemmer => 'English'))
    );"
    .execute_result(&mut conn);

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("key_field id cannot be included"));

    // Test valid configuration where alias is different from key_field
    let result = "CALL paradedb.create_bm25(
        index_name => 'products_index',
        table_name => 'products',
        key_field => 'id',
        text_fields => 
            paradedb.field('name', tokenizer => paradedb.tokenizer('default')) ||
            paradedb.field('description', alias => 'desc_stem', tokenizer => paradedb.tokenizer('default', stemmer => 'English'))
    );"
    .execute_result(&mut conn);

    assert!(result.is_ok());
}

#[rstest]
fn multiple_tokenizers_same_field_in_query(mut conn: PgConnection) {
    // Create the table
    "CREATE TABLE product_reviews (
        id SERIAL PRIMARY KEY,
        product_name TEXT,
        review_text TEXT
    );"
    .execute(&mut conn);

    // Insert mock data
    "INSERT INTO product_reviews (product_name, review_text) VALUES
    ('SmartPhone X', 'This smartphone is incredible! The camera quality is amazing.'),
    ('Laptop Pro', 'Great laptop for programming. The keyboard is comfortable.'),
    ('Wireless Earbuds', 'These earbuds have excellent sound quality. Battery life could be better.'),
    ('Gaming Mouse', 'Responsive and comfortable. Perfect for long gaming sessions.'),
    ('4K TV', 'The picture quality is breathtaking. Smart features work seamlessly.'),
    ('Fitness Tracker', 'Accurate step counting and heart rate monitoring. The app is user-friendly.'),
    ('Smartwatch', 'This watch is smart indeed! Great for notifications and fitness tracking.'),
    ('Bluetooth Speaker', 'Impressive sound for its size. Waterproof feature is a plus.'),
    ('Mechanical Keyboard', 'Satisfying key presses. RGB lighting is customizable.'),
    ('External SSD', 'Super fast read/write speeds. Compact and portable design.');"
    .execute(&mut conn);

    // Create the BM25 index with multiple tokenizers
    "CALL paradedb.create_bm25(
        index_name => 'product_reviews_index',
        table_name => 'product_reviews',
        key_field => 'id',
        text_fields => 
            paradedb.field('product_name', tokenizer => paradedb.tokenizer('default')) ||
            paradedb.field('product_name', alias => 'product_name_ngram', tokenizer => paradedb.tokenizer('ngram', min_gram => 3, max_gram => 3, prefix_only => false)) ||
            paradedb.field('review_text', tokenizer => paradedb.tokenizer('default')) ||
            paradedb.field('review_text', alias => 'review_text_stem', tokenizer => paradedb.tokenizer('default', stemmer => 'English'))
    );"
    .execute(&mut conn);

    //  Exact match using default tokenizer
    let rows: Vec<(i32, String)> = "SELECT id, product_name FROM product_reviews_index.search('product_name:\"Wireless Earbuds\"')"
        .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "Wireless Earbuds");

    // Partial match using ngram tokenizer
    let rows: Vec<(i32, String)> =
        "SELECT id, product_name FROM product_reviews_index.search('product_name_ngram:phon')"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "SmartPhone X");

    // Stemmed search using English stemmer tokenizer
    let rows: Vec<(i32, String)> =
        "SELECT id, product_name FROM product_reviews_index.search('review_text_stem:gaming')"
            .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert!(rows.iter().any(|(_, name)| name == "Gaming Mouse"));

    // Using default tokenizer and stem on same field
    let rows: Vec<(i32, String)> = "SELECT id, product_name FROM product_reviews_index.search('review_text:monitoring OR review_text_stem:mon')"
        .fetch(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "Fitness Tracker");
}

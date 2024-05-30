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

use approx::assert_relative_eq;
use core::panic;
use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

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
fn with_bm25_scoring(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let rows: Vec<(i32, f32)> = "SELECT id, paradedb.rank_bm25(id) FROM bm25_search.search('category:electronics OR description:keyboard', stable_sort => true)"
        .fetch(&mut conn);

    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    let expected = [2, 1, 12, 22, 32];
    assert_eq!(ids, expected);

    let ranks: Vec<_> = rows.iter().map(|r| r.1).collect();
    let expected = [5.3764954, 4.931014, 2.1096356, 2.1096356, 2.1096356];
    assert_eq!(ranks, expected);
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

    let columns: SimpleProductsTableVec = "SELECT * FROM paradedb.bm25_search
        WHERE paradedb.search_tantivy(
            id,
            jsonb_build_object(
                'index_name', 'bm25_search_bm25_index',
                'table_name', 'bm25_test_table',
                'schema_name', 'paradedb',
                'key_field', 'id',
                'query', paradedb.parse('category:electronics')::text::jsonb
            )
        ) ORDER BY id"
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
    	text_fields => '{"name": {}}'
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
    	text_fields => '{text_array: {}, varchar_array: {}}'
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
        text_fields => '{"some_text": {}}'
    );
    
    CALL paradedb.drop_bm25('uuid_table');"#
        .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        index_name => 'uuid_table',
        table_name => 'uuid_table',
        key_field => 'id',
        text_fields => '{"some_text": {}, "random_uuid": {}}'
    )"#
    .execute(&mut conn);

    let rows: Vec<(i32,)> =
        r#"SELECT * FROM uuid_table.search('some_text:some')"#
            .fetch(&mut conn);

    assert_eq!(rows.len(), 10);
}

#[rstest]
fn hybrid(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
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

    // Test with query object.
    let columns: SimpleProductsTableVec = r#"
    SELECT m.*, s.rank_hybrid
    FROM paradedb.bm25_search m
    LEFT JOIN (
        SELECT * FROM bm25_search.rank_hybrid(
            bm25_query => paradedb.parse('description:keyboard OR category:electronics'),
            similarity_query => '''[1,2,3]'' <-> embedding',
            bm25_weight => 0.9,
            similarity_weight => 0.1
        )
    ) s ON m.id = s.id
    LIMIT 5"#
        .fetch_collect(&mut conn);

    assert_eq!(columns.id, vec![2, 1, 29, 39, 9]);

    // Test with string query.
    let columns: SimpleProductsTableVec = r#"
    SELECT m.*, s.rank_hybrid
    FROM paradedb.bm25_search m
    LEFT JOIN (
        SELECT * FROM bm25_search.rank_hybrid(
            bm25_query => 'description:keyboard OR category:electronics',
            similarity_query => '''[1,2,3]'' <-> embedding',
            bm25_weight => 0.9,
            similarity_weight => 0.1
        )
    ) s ON m.id = s.id
    LIMIT 5"#
        .fetch_collect(&mut conn);

    assert_eq!(columns.id, vec![2, 1, 29, 39, 9]);
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
			    paradedb.fuzzy_term(field => 'description', value => 'wolo')
		    ]
	    ),
	    stable_sort => true
	);
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![32, 5, 3, 4, 7, 34, 37, 10, 33, 39, 41]);
}

#[rstest]
fn highlight(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let row: (String,) = "
        SELECT paradedb.highlight(id, 'description')
        FROM bm25_search.search('description:shoes', stable_sort => true)"
        .fetch_one(&mut conn);
    assert_eq!(row.0, "Generic <b>shoes</b>");

    let row: (String,) = "
        SELECT paradedb.highlight(id, 'description', prefix => '<h1>', postfix => '</h1>')
        FROM bm25_search.search('description:shoes', stable_sort => true)"
        .fetch_one(&mut conn);
    assert_eq!(row.0, "Generic <h1>shoes</h1>")
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

    // Test with query object.
    let columns: SimpleProductsTableVec = r#"
    SELECT m.*, s.rank_hybrid
    FROM paradedb.bm25_search m
    LEFT JOIN (
        SELECT * FROM bm25_search.rank_hybrid(
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
    // This previsouly triggered a "division by zero" error when there was
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

    // Test with query object.
    let columns: SimpleProductsTableVec = r#"
    SELECT m.*, s.rank_hybrid
    FROM paradedb.bm25_search m
    LEFT JOIN (
        SELECT * FROM bm25_search.rank_hybrid(
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
fn alias(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let rows = "
        SELECT id, paradedb.highlight(id, field => 'description') FROM bm25_search.search('description:shoes')
        UNION
        SELECT id, paradedb.highlight(id, field => 'description')
        FROM bm25_search.search('description:speaker')
        ORDER BY id"
        .fetch_result::<()>(&mut conn);

    match rows {
        Ok(_) => panic!("an alias should be required for multiple search calls"),
        Err(err) => assert!(err
            .to_string()
            .contains("could not store search state in manager: AliasRequired")),
    }

    let rows: Vec<(i32, String)> = "
        SELECT id, paradedb.highlight(id, field => 'description') FROM bm25_search.search('description:shoes')
        UNION
        SELECT id, paradedb.highlight(id, field => 'description', alias => 'speaker')
        FROM bm25_search.search('description:speaker', alias => 'speaker')
        ORDER BY id"
        .fetch(&mut conn);

    assert_eq!(rows[0].0, 3);
    assert_eq!(rows[1].0, 4);
    assert_eq!(rows[2].0, 5);
    assert_eq!(rows[3].0, 32);
    assert_eq!(rows[0].1, "Sleek running <b>shoes</b>");
    assert_eq!(rows[1].1, "White jogging <b>shoes</b>");
    assert_eq!(rows[2].1, "Generic <b>shoes</b>");
    assert_eq!(rows[3].1, "Bluetooth-enabled <b>speaker</b>");

    let rows: Vec<(i32, f32)> = "
        SELECT id, paradedb.rank_bm25(id) FROM bm25_search.search('description:shoes')
        UNION
        SELECT id, paradedb.rank_bm25(id, alias => 'speaker')
        FROM bm25_search.search('description:speaker', alias => 'speaker')
        ORDER BY id"
        .fetch(&mut conn);

    assert_eq!(rows[0].0, 3);
    assert_eq!(rows[1].0, 4);
    assert_eq!(rows[2].0, 5);
    assert_eq!(rows[3].0, 32);
    assert_relative_eq!(rows[0].1, 2.4849067, epsilon = 1e-6);
    assert_relative_eq!(rows[1].1, 2.4849067, epsilon = 1e-6);
    assert_relative_eq!(rows[2].1, 2.8772602, epsilon = 1e-6);
    assert_relative_eq!(rows[3].1, 3.3322046, epsilon = 1e-6);
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
            text_fields => '{description: {tokenizer: {type: \"en_stem\"}}, category: {}}'
    )"
    .execute(&mut conn);

    let plan: Vec<(String,)> =
        "SELECT * FROM search_idx.explain('description:keyboard OR category:electronics', stable_sort => true)".fetch(&mut conn);

    assert!(plan[0].0.contains("Index Scan"));
}

#[rstest]
fn update_time(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'mock_items', schema_name => 'public');"
        .execute(&mut conn);
    "CALL paradedb.create_bm25(
            index_name => 'search_idx',
            schema_name => 'public',
            table_name => 'mock_items',
            key_field => 'id',
            text_fields => '{description: {tokenizer: {type: \"en_stem\"}}}'
    )"
    .execute(&mut conn);

    let start_time = std::time::Instant::now();
    "UPDATE mock_items set category = 'Keyboards' WHERE description = 'Plastic Keyboard'"
        .execute(&mut conn);
    let elapsed_with_index = start_time.elapsed().as_millis() as i64;

    "CALL paradedb.drop_bm25('search_idx')".execute(&mut conn);
    let start_time = std::time::Instant::now();
    "UPDATE mock_items set category = 'Instruments' WHERE description = 'Plastic Keyboard'"
        .execute(&mut conn);
    let elapsed_without_index = start_time.elapsed().as_millis() as i64;

    // There should be a negligible difference in time between the two updates
    assert!((elapsed_without_index - elapsed_with_index).abs() < 3);
}

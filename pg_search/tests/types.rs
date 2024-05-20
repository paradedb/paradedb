mod fixtures;

use core::panic;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn boolean_search(mut conn: PgConnection) {
	r#"
    CREATE TABLE test_table (
    	id SERIAL PRIMARY KEY,
    	value BOOLEAN
    );

    INSERT INTO test_table (value) VALUES (true), (false), (false), (true);
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
    	table_name => 'test_table',
    	index_name => 'test_index',
    	key_field => 'id',
    	boolean_fields => '{"value": {}}'
    );
    "#
    .execute(&mut conn);


    let rows: Vec<(i32, bool)> = r#"
    SELECT * FROM test_index.search(
    	query => paradedb.term(field => 'value', value => true),
    	stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
    	rows,
    	vec![(1, true), (4, true)]
    );
}

#[rstest]
fn integer_search(mut conn: PgConnection) {
	r#"
    CREATE TABLE test_table (
    	id SERIAL PRIMARY KEY,
    	value_int2 SMALLINT,
    	value_int4 INTEGER,
    	value_int8 BIGINT,

    );

    INSERT INTO test_table (value_int2, value_int4, value_int8) VALUES 
    	(11, 1111, 11111111), 
    	(22, 2222, 22222222), 
    	(33, 3333, 33333333), 
    	(44, 4444, 44444444);
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
    	table_name => 'test_table',
    	index_name => 'test_index',
    	key_field => 'id',
    	numeric_fields => '{"value_int2": {}, "value_int4": {}, "value_int8": {}}'
    );
    "#
    .execute(&mut conn);


    // INT2
    let rows: Vec<(i32, i16)> = r#"
    SELECT id, value_int2 FROM test_index.search(
    	query => paradedb.term(field => 'value_int2', value => 11),
    	stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
    	rows,
    	vec![(1, 11)]
    );

    // INT4
    let rows: Vec<(i32, i32)> = r#"
    SELECT id, value_int4 FROM test_index.search(
    	query => paradedb.term(field => 'value_int4', value => 2222),
    	stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
    	rows,
    	vec![(2, 2222)]
    );

    // INT8
    let rows: Vec<(i32, i64)> = r#"
    SELECT id, value_int8 FROM test_index.search(
    	query => paradedb.term(field => 'value_int8', value => 33333333),
    	stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
    	rows,
    	vec![(3, 33333333)]
    );
}

#[rstest]
fn float_search(mut conn: PgConnection) {
	r#"
    CREATE TABLE test_table (
    	id SERIAL PRIMARY KEY,
    	value_float4 FLOAT4,
    	value_float8 FLOAT8
    );

    INSERT INTO test_table (value_float4, value_float8) VALUES 
    	(1.1, 1111.1111), 
    	(2.2, 2222.2222), 
    	(3.3, 3333.3333), 
    	(4.4, 4444.4444);
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
    	table_name => 'test_table',
    	index_name => 'test_index',
    	key_field => 'id',
    	numeric_fields => '{"value_float4": {}, "value_float8": {}}'
    );
    "#
    .execute(&mut conn);


    // FLOAT4
    let rows: Vec<(i32, f32)> = r#"
    SELECT id, value_float4 FROM test_index.search(
    	query => paradedb.term(field => 'value_float4', value => 1.1::float4),
    	stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
    	rows,
    	vec![(1, 1.1)]
    );

    // FLOAT8
    let rows: Vec<(i32, f64)> = r#"
    SELECT id, value_float8 FROM test_index.search(
    	query => paradedb.term(field => 'value_float8', value => 4444.4444::float8),
    	stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
    	rows,
    	vec![(4, 4444.4444)]
    );
}

#[rstest]
fn datetime_search(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let mut columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.boolean(
            should => ARRAY[
                paradedb.parse('description:shoes'),
                paradedb.phrase_prefix(field => 'description', phrases => ARRAY['book']),
                paradedb.term(field => 'description', value => 'speaker'),
                paradedb.fuzzy_term(field => 'description', value => 'wolo'),
                paradedb.term(field => 'last_updated_date', value => '2023-05-03'::date)
            ]
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        columns.id,
        vec![5, 32, 1, 3, 4, 28, 7, 34, 37, 10, 33, 39, 41]
    );

    columns = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.term(field => 'latest_available_time', value => '10:55:43'::time)
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3])
}
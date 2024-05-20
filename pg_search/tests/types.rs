mod fixtures;

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
fn text_search(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_text TEXT,
        value_varchar VARCHAR(64)
    );

    INSERT INTO test_table (value_text, value_varchar) VALUES 
        ('abc', 'var abc'), 
        ('def', 'var def'), 
        ('ghi', 'var ghi'), 
        ('jkl', 'var jkl');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_table',
        index_name => 'test_index',
        key_field => 'id',
        text_fields => '{"value_text": {}, "value_varchar": {}}'
    );
    "#
    .execute(&mut conn);

    // TEXT
    let rows: Vec<(i32, String)> = r#"
    SELECT id, value_text FROM test_index.search(
        query => paradedb.term(field => 'value_text', value => 'abc'),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![(1, "abc".into())]
    );

    // VARCHAR
    let rows: Vec<(i32, String)> = r#"
    SELECT id, value_varchar FROM test_index.search(
        query => paradedb.term(field => 'value_varchar', value => 'ghi'),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![(3, "var ghi".into())]
    );
}

#[rstest]
fn json_search(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_json JSON,
        value_jsonb JSONB
    );

    INSERT INTO test_table (value_json, value_jsonb) VALUES 
        ('{"color": "Silver", "location": "United States"}'::JSON, '{"color": "Silver", "location": "United States"}'::JSONB),
        ('{"color": "Black", "location": "Canada"}'::JSON, '{"color": "Black", "location": "Canada"}'::JSONB);
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_table',
        index_name => 'test_index',
        key_field => 'id',
        json_fields => '{"value_json": {}, "value_jsonb": {}}'
    );
    "#
    .execute(&mut conn);

    // JSON
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_index.search(
        query => paradedb.term(field => 'value_json', value => '{"color": "Silver", "location": "United States"}'::JSON),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![(1,)]
    );

    // JSONB
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_index.search(
        query => paradedb.term(field => 'value_jsonb', value => '{"color": "Black", "location": "Canada"}'::JSONB),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![(2,)]
    );

    // Search JSONB using JSON
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_index.search(
        query => paradedb.term(field => 'value_jsonb', value => '{"color": "Black", "location": "Canada"}'::JSON),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![(2,)]
    );

    // Search JSON using JSONB
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_index.search(
        query => paradedb.term(field => 'value_json', value => '{"color": "Silver", "location": "United States"}'::JSONB),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![(1,)]
    );
}

#[rstest]
fn datetime_search(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_date DATE,
        value_timestamp TIMESTAMP,
        value_timestamptz TIMESTAMP WITH TIME ZONE,
        value_time TIME,
        value_timetz TIME WITH TIME ZONE
    );

    INSERT INTO test_table (value_date, value_timestamp, value_timestamptz, value_time, value_timetz) VALUES 
        (DATE '2023-05-03', TIMESTAMP '2023-04-15 13:27:09', TIMESTAMP WITH TIME ZONE '2023-04-15 13:27:09 PST', TIME '08:09:10', TIME WITH TIME ZONE '08:09:10 PST'),
        (DATE '2021-06-28', TIMESTAMP '2019-08-02 07:52:43', TIMESTAMP WITH TIME ZONE '2019-08-02 07:52:43 EST', TIME '11:43:21', TIME WITH TIME ZONE '11:43:21 EST');
    "#
    .execute(&mut conn);

    r#"
    CALL paradedb.create_bm25(
        table_name => 'test_table',
        index_name => 'test_index',
        key_field => 'id',
        datetime_fields => '{"value_date": {}, "value_timestamp": {}, "value_timestamptz": {}, "value_time": {}, "value_timetz": {}}'
    );
    "#
    .execute(&mut conn);

    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_index.search(
        query => paradedb.term(field => 'value_date', value => DATE '2023-05-03')
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_index.search(
        query => paradedb.term(field => 'value_timestamp', value => TIMESTAMP '2019-08-02 07:52:43')
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);

    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_index.search(
        query => paradedb.term(field => 'value_timestamptz', value => TIMESTAMP WITH TIME ZONE '2023-04-15 13:27:09 PST')
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    // Change time zone in query
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_index.search(
        query => paradedb.term(field => 'value_timestamptz', value => TIMESTAMP WITH TIME ZONE '2023-04-15 16:27:09 EST')
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_index.search(
        query => paradedb.term(field => 'value_time', value => TIME '11:43:21')
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);

    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_index.search(
        query => paradedb.term(field => 'value_timetz', value => TIME WITH TIME ZONE '11:43:21 EST')
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);

    // Change time zone in query
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_index.search(
        query => paradedb.term(field => 'value_timetz', value => TIME WITH TIME ZONE '08:43:21 PST')
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);

    // Query no time zone with time zone
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_index.search(
        query => paradedb.term(field => 'value_timestamp', value => TIMESTAMP WITH TIME ZONE '2023-04-15 13:27:09 GMT')
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    // Query time zone with no time zone (GMT = EST + 5)
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_index.search(
        query => paradedb.term(field => 'value_timestamptz', value => TIMESTAMP '2019-08-02 12:52:43')
    );
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);
}

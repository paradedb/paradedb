mod fixtures;

use fixtures::*;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

#[rstest]
fn self_referencing_var(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS test;
    CREATE TABLE test (
        id bigint NOT NULL PRIMARY KEY,
        value text
    );

    INSERT INTO test (id, value) SELECT x, md5(x::text) FROM generate_series(1, 100) x;
    UPDATE test SET value = 'value contains id = ' || id WHERE id BETWEEN 10 and 20;

    CREATE INDEX idxtest ON test USING bm25 (id, value) WITH (key_field='id');
    "#
    .execute(&mut conn);

    let results =
        "SELECT id FROM test WHERE value @@@ paradedb.with_index('idxtest', paradedb.term('value', id::text)) ORDER BY id;".fetch::<(i64,)>(&mut conn);
    assert_eq!(
        results,
        vec![
            (10,),
            (11,),
            (12,),
            (13,),
            (14,),
            (15,),
            (16,),
            (17,),
            (18,),
            (19,),
            (20,),
        ]
    );
}

#[rstest]
fn parallel_with_subselect(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS test;
    CREATE TABLE test (
        id bigint NOT NULL PRIMARY KEY,
        value text
    );

    INSERT INTO test (id, value) SELECT x, md5(x::text) FROM generate_series(1, 100) x;
    UPDATE test SET value = 'value contains id = ' || id WHERE id BETWEEN 10 and 20;

    CREATE INDEX idxtest ON test USING bm25 (id, value) WITH (key_field='id');
    "#
    .execute(&mut conn);

    if pg_major_version(&mut conn) >= 16 {
        "SET debug_parallel_query TO on".execute(&mut conn);
    }

    "PREPARE foo AS SELECT count(*) FROM test WHERE value @@@ (select $1);".execute(&mut conn);
    let (count,) = "EXECUTE foo('contains')".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 11);

    // next 4 executions use one plan, and the 5th shouldn't change
    for _ in 0..5 {
        let (plan,) = "EXPLAIN (ANALYZE, FORMAT JSON) EXECUTE foo('contains');"
            .fetch_one::<(Value,)>(&mut conn);
        eprintln!("{plan:#?}");
        let plan = plan
            .pointer("/0/Plan/Plans/1/Plans/0")
            .unwrap()
            .as_object()
            .unwrap();
        pretty_assertions::assert_eq!(
            plan.get("Custom Plan Provider"),
            Some(&Value::String(String::from("ParadeDB Scan")))
        );
    }
}

#[rstest]
fn parallel_function_with_agg_subselect(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS test;
    CREATE TABLE test (
        id bigint NOT NULL PRIMARY KEY,
        value text
    );

    INSERT INTO test (id, value) SELECT x, md5(x::text) FROM generate_series(1, 100) x;
    UPDATE test SET value = 'value contains id = ' || id WHERE id BETWEEN 10 and 20;

    CREATE INDEX idxtest ON test USING bm25 (id, value) WITH (key_field='id');
    "#
    .execute(&mut conn);

    if pg_major_version(&mut conn) >= 16 {
        "SET debug_parallel_query TO on".execute(&mut conn);
    }

    "PREPARE foo AS SELECT id FROM test WHERE id @@@ paradedb.term_set((select array_agg(paradedb.term('value', token)) from paradedb.tokenize(paradedb.tokenizer('default'), $1))) ORDER BY id;".execute(&mut conn);

    let results = "EXECUTE foo('no matches')".fetch::<(i64,)>(&mut conn);
    assert_eq!(results.len(), 0);

    let results = "EXECUTE foo('value contains id')".fetch::<(i64,)>(&mut conn);
    assert_eq!(
        results,
        vec![
            (10,),
            (11,),
            (12,),
            (13,),
            (14,),
            (15,),
            (16,),
            (17,),
            (18,),
            (19,),
            (20,),
        ]
    );
}

#[rstest]
fn test_issue2061(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    )
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata, weight_range)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let results = r#"
    SELECT id, description, paradedb.score(id)
    FROM mock_items
    WHERE id @@@ paradedb.match('description', (SELECT description FROM mock_items WHERE id = 1))
    ORDER BY paradedb.score(id) DESC;    
    "#
    .fetch::<(i32, String, f32)>(&mut conn);

    assert_eq!(
        results,
        vec![
            (1, "Ergonomic metal keyboard".into(), 9.485788),
            (2, "Plastic Keyboard".into(), 3.2668595),
        ]
    )
}

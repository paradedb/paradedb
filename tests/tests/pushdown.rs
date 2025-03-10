mod fixtures;

use fixtures::*;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

#[rstest]
fn pushdown(mut conn: PgConnection) {
    const OPERATORS: [&str; 6] = ["=", ">", "<", ">=", "<=", "<>"];
    const TYPES: &[[&str; 2]] = &[
        ["int2", "0"],
        ["int4", "0"],
        ["int8", "0"],
        ["float4", "0"],
        ["float8", "0"],
        ["date", "now()"],
        ["time", "now()"],
        ["timetz", "now()"],
        ["timestamp", "now()"],
        ["timestamptz", "now()"],
        ["text", "'foo'"],
        ["uuid", "gen_random_uuid()"],
    ];

    let sqlname = |sqltype: &str| -> String { String::from("col_") + &sqltype.replace('"', "") };

    let mut sql = String::new();
    sql += "CREATE TABLE test (id SERIAL8 NOT NULL PRIMARY KEY, col_boolean boolean DEFAULT false";
    for [sqltype, default] in TYPES {
        sql += &format!(
            ", {} {sqltype} NOT NULL DEFAULT {default}",
            sqlname(sqltype)
        );
    }
    sql += ");";

    eprintln!("{sql}");
    sql.execute(&mut conn);

    let sql = format!(
        r#"CREATE INDEX idxtest ON test USING bm25 (id, col_boolean, {}) WITH (key_field='id', text_fields = '{{"col_text": {{"tokenizer": {{"type":"raw"}} }} }}');"#,
        TYPES
            .iter()
            .map(|t| sqlname(t[0]))
            .collect::<Vec<_>>()
            .join(", ")
    );
    eprintln!("{sql}");
    sql.execute(&mut conn);

    "INSERT INTO test (id) VALUES (1);".execute(&mut conn); // insert all default values

    "SET enable_indexscan TO off;".execute(&mut conn);
    "SET enable_bitmapscan TO off;".execute(&mut conn);
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    for operator in OPERATORS {
        for [sqltype, default] in TYPES {
            let sqlname = sqlname(sqltype);
            let sql = format!(
                r#"
                EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
                SELECT count(*)
                FROM test
                WHERE {sqlname} {operator} {default}::{sqltype}
                  AND id @@@ '1';
            "#
            );

            eprintln!("/----------/");
            eprintln!("{sql}");

            let (plan,) = sql.fetch_one::<(Value,)>(&mut conn);
            eprintln!("{plan:#?}");

            let plan = plan
                .pointer("/0/Plan/Plans/0")
                .unwrap()
                .as_object()
                .unwrap();
            pretty_assertions::assert_eq!(
                plan.get("Node Type"),
                Some(&Value::String(String::from("Custom Scan")))
            );
        }
    }

    // boolean is a bit of a separate beast, so test it directly
    {
        let sqltype = "boolean";
        let sqlname = sqlname(sqltype);
        let sql = format!(
            r#"
                EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
                SELECT count(*)
                FROM test
                WHERE {sqlname} = true
                  AND id @@@ '1';
            "#
        );

        eprintln!("/----------/");
        eprintln!("{sql}");

        let (plan,) = sql.fetch_one::<(Value,)>(&mut conn);
        eprintln!("{plan:#?}");

        let plan = plan
            .pointer("/0/Plan/Plans/0")
            .unwrap()
            .as_object()
            .unwrap();
        pretty_assertions::assert_eq!(
            plan.get("Node Type"),
            Some(&Value::String(String::from("Custom Scan")))
        );
    }
    {
        let sqltype = "boolean";
        let sqlname = sqlname(sqltype);
        let sql = format!(
            r#"
                EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
                SELECT count(*)
                FROM test
                WHERE {sqlname} = false
                  AND id @@@ '1';
            "#
        );

        eprintln!("/----------/");
        eprintln!("{sql}");

        let (plan,) = sql.fetch_one::<(Value,)>(&mut conn);
        eprintln!("{plan:#?}");

        let plan = plan
            .pointer("/0/Plan/Plans/0")
            .unwrap()
            .as_object()
            .unwrap();
        pretty_assertions::assert_eq!(
            plan.get("Node Type"),
            Some(&Value::String(String::from("Custom Scan")))
        );
    }
}

#[rstest]
fn pushdown_is_not_null(mut conn: PgConnection) {
    let sql = r#"
        CREATE TABLE test (
            id SERIAL8 NOT NULL PRIMARY KEY,
            col_boolean boolean DEFAULT false,
            col_text text,
            col_int8 int8
        );
    "#;
    sql.execute(&mut conn);

    let sql = r#"
        CREATE INDEX idxtest ON test USING bm25 (id, col_boolean, col_text, col_int8)
        WITH (key_field='id', text_fields = '{"col_text": {"fast": true, "tokenizer": {"type":"raw"}}}');
    "#;
    sql.execute(&mut conn);

    "INSERT INTO test (id, col_text) VALUES (1, NULL);".execute(&mut conn);
    "INSERT INTO test (id, col_text) VALUES (2, 'foo');".execute(&mut conn);
    "INSERT INTO test (id, col_text, col_int8) VALUES (3, 'bar', 333);".execute(&mut conn);
    "INSERT INTO test (id, col_int8) VALUES (4, 444);".execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);
    "SET enable_bitmapscan TO off;".execute(&mut conn);
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    let sql = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT count(*)
        FROM test
        WHERE col_text IS NOT NULL
        AND id @@@ '1';
    "#;

    eprintln!("/----------/");
    eprintln!("{sql}");

    let (plan,) = sql.fetch_one::<(Value,)>(&mut conn);
    eprintln!("{plan:#?}");

    // Verify that the custom scan is used
    let plan = plan
        .pointer("/0/Plan/Plans/0")
        .unwrap()
        .as_object()
        .unwrap();
    pretty_assertions::assert_eq!(
        plan.get("Node Type"),
        Some(&Value::String(String::from("Custom Scan")))
    );

    // Verify that count is correct
    let count = r#"
        SELECT count(*)
        FROM test
        WHERE col_text IS NOT NULL
        AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range);
    "#
    .fetch::<(i64,)>(&mut conn);
    assert_eq!(count, vec![(2,)]);

    let count = r#"
        SELECT count(*)
        FROM test
        WHERE col_int8 IS NOT NULL
        AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range);
    "#
    .fetch::<(i64,)>(&mut conn);
    assert_eq!(count, vec![(2,)]);

    let count = r#"
        SELECT count(*)
        FROM test
        WHERE col_int8 IS NOT NULL
        AND col_text IS NOT NULL
        AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range);
    "#
    .fetch::<(i64,)>(&mut conn);
    assert_eq!(count, vec![(1,)]);

    // Verify that IS NOT NULL works with other predicates
    let count = r#"
        SELECT count(*)
        FROM test
        WHERE col_text IS NOT NULL
        AND id @@@ '>2';
    "#
    .fetch::<(i64,)>(&mut conn);
    assert_eq!(count, vec![(1,)]);

    // Verify that results are correct and ordered
    let result = r#"
        SELECT id
        FROM test
        WHERE col_text IS NOT NULL
        AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
        ORDER BY id DESC;
    "#
    .fetch::<(i64,)>(&mut conn);
    assert_eq!(result, vec![(3,), (2,)]);

    // Verify that GROUP BY works
    let result = r#"
        SELECT col_text, count(*)
        FROM test
        WHERE col_text IS NOT NULL
        and id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
        GROUP BY col_text
        ORDER BY col_text;
    "#
    .fetch::<(String, i64)>(&mut conn);
    assert_eq!(
        result,
        vec![(String::from("bar"), 1), (String::from("foo"), 1)]
    );

    // Verify that DISTIINCT works
    let result = r#"
        SELECT COUNT(DISTINCT col_text)
        FROM test
        WHERE col_text IS NOT NULL
        and id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range);
    "#
    .fetch::<(i64,)>(&mut conn);
    assert_eq!(result, vec![(2,)]);

    // Verify that JOIN works
    "CREATE TABLE test2 (id SERIAL8 NOT NULL PRIMARY KEY, ref_id int8, ref_text text);"
        .execute(&mut conn);
    let sql = r#"
        CREATE INDEX idxtest2 ON test2 USING bm25 (id, ref_id, ref_text)
        WITH (key_field='id', text_fields = '{"ref_text": {"fast": true, "tokenizer": {"type":"raw"}}}');
    "#;
    sql.execute(&mut conn);

    "INSERT INTO test2 (ref_id, ref_text) VALUES (1, 'qux');".execute(&mut conn);
    "INSERT INTO test2 (ref_id, ref_text) VALUES (3, 'foo');".execute(&mut conn);

    let join = r#"
        SELECT test.id, test.col_text, test2.ref_text
        FROM test
        INNER JOIN test2 ON test.id = test2.ref_id
        WHERE test.col_text IS NOT NULL
        AND test.id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
        ORDER BY test.id;
    "#
    .fetch_one::<(i64, String, String)>(&mut conn);
    assert_eq!(join, (3, String::from("bar"), String::from("foo")));

    // Verify that NULL is not counted after update
    "UPDATE test SET col_text = NULL".execute(&mut conn);
    let count = r#"
        SELECT count(*)
        FROM test
        WHERE col_text IS NOT NULL
        AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range);
    "#
    .fetch::<(i64,)>(&mut conn);
    assert_eq!(count, vec![(0,)]);
}

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

#[fixture]
fn setup_test_table(mut conn: PgConnection) -> PgConnection {
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
    conn
}

mod pushdown_is_not_null {
    use super::*;

    #[rstest]
    fn custom_scan(#[from(setup_test_table)] mut conn: PgConnection) {
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
    }

    #[rstest]
    fn with_count(#[from(setup_test_table)] mut conn: PgConnection) {
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
    }

    #[rstest]
    fn with_return_values(#[from(setup_test_table)] mut conn: PgConnection) {
        let res = r#"
            SELECT *
            FROM test
            WHERE col_text IS NOT NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            ORDER BY id;
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(
            res,
            vec![
                (2, false, Some(String::from("foo")), None),
                (3, false, Some(String::from("bar")), Some(333))
            ]
        );

        let res = r#"
            SELECT *
            FROM test
            WHERE col_int8 IS NOT NULL
            AND col_text IS NOT NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range);
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![(3, false, Some(String::from("bar")), Some(333))]);
    }

    #[rstest]
    fn with_multiple_predicates(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that IS NOT NULL works with other predicates
        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_text IS NOT NULL
            AND id @@@ '>2';
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(1,)]);

        let res = r#"
            SELECT *
            FROM test
            WHERE col_text IS NOT NULL
            AND id @@@ '>2';
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![(3, false, Some(String::from("bar")), Some(333))]);
    }

    #[rstest]
    fn with_ordering(#[from(setup_test_table)] mut conn: PgConnection) {
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
    }

    #[rstest]
    fn with_aggregation(#[from(setup_test_table)] mut conn: PgConnection) {
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
    }

    #[rstest]
    fn with_distinct(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that DISTIINCT works
        let count = r#"
            SELECT COUNT(DISTINCT col_text)
            FROM test
            WHERE col_text IS NOT NULL
            and id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(2,)]);

        let res = r#"
            SELECT DISTINCT col_text
            FROM test
            WHERE col_text IS NOT NULL
            and id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
            ORDER BY col_text;
        "#
        .fetch::<(Option<String>,)>(&mut conn);
        assert_eq!(
            res,
            vec![(Some(String::from("bar")),), (Some(String::from("foo")),)]
        );
    }

    #[rstest]
    fn with_join(#[from(setup_test_table)] mut conn: PgConnection) {
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
    }

    #[rstest]
    fn post_update(#[from(setup_test_table)] mut conn: PgConnection) {
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

        let res = r#"
            SELECT *
            FROM test
            WHERE col_text IS NOT NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range);
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![]);
    }
}

mod pushdown_is_null {
    use super::*;

    #[rstest]
    fn custom_scan(#[from(setup_test_table)] mut conn: PgConnection) {
        let sql = r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            SELECT count(*)
            FROM test
            WHERE col_text IS NULL
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
    }

    #[rstest]
    fn with_count(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that count is correct
        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_text IS NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(2,)]);

        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_int8 IS NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(2,)]);

        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_int8 IS NULL
            AND col_text IS NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(1,)]);
    }

    #[rstest]
    fn with_return_values(#[from(setup_test_table)] mut conn: PgConnection) {
        let res = r#"
            SELECT id, col_boolean, col_int8
            FROM test
            WHERE col_text IS NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            ORDER BY id;
        "#
        .fetch::<(i64, bool, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![(1, false, None), (4, false, Some(444))]);

        let res = r#"
            SELECT *
            FROM test
            WHERE col_int8 IS NULL
            AND col_text IS NULL
            AND id @@@ '1' OR id @@@ '2' OR id @@@ '3' OR id @@@ '4'
            ORDER BY id;
        "#
        .fetch::<(i64, bool, Option<String>, Option<i64>)>(&mut conn);
        assert_eq!(
            res,
            vec![
                (2, false, Some(String::from("foo")), None),
                (3, false, Some(String::from("bar")), Some(333))
            ]
        );
    }

    #[rstest]
    fn with_multiple_predicates(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that IS NULL works with other predicates
        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_text IS NULL
            AND id @@@ '>2';
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(1,)]);

        let res = r#"
            SELECT id, col_boolean, col_int8
            FROM test
            WHERE col_text IS NULL
            AND id @@@ '>2';
        "#
        .fetch::<(i64, bool, Option<i64>)>(&mut conn);
        assert_eq!(res, vec![(4, false, Some(444))]);
    }

    #[rstest]
    fn with_ordering(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that results are correct and ordered
        let result = r#"
            SELECT id
            FROM test
            WHERE col_text IS NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
            ORDER BY id DESC;
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(result, vec![(4,), (1,)]);
    }

    #[rstest]
    fn with_aggregation(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that GROUP BY works
        let result = r#"
            SELECT col_int8, count(*)
            FROM test
            WHERE col_text IS NULL
            and id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
            GROUP BY col_int8
            ORDER BY col_int8;
        "#
        .fetch::<(Option<i64>, i64)>(&mut conn);
        assert_eq!(result, vec![(Some(444), 1), (None, 1)]);
    }

    #[rstest]
    fn with_distinct(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that DISTIINCT works
        let result = r#"
            SELECT COUNT(DISTINCT col_int8)
            FROM test
            WHERE col_text IS NULL
            and id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(result, vec![(1,)]);
    }

    #[rstest]
    fn with_join(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that JOIN works
        "CREATE TABLE test2 (id SERIAL8 NOT NULL PRIMARY KEY, ref_id int8, ref_text text);"
            .execute(&mut conn);
        let sql = r#"
            CREATE INDEX idxtest2 ON test2 USING bm25 (id, ref_id, ref_text)
            WITH (key_field='id', text_fields = '{"ref_text": {"fast": true, "tokenizer": {"type":"raw"}}}');
        "#;
        sql.execute(&mut conn);

        "INSERT INTO test2 (ref_id, ref_text) VALUES (2, 'qux');".execute(&mut conn);
        "INSERT INTO test2 (ref_id, ref_text) VALUES (4, 'foo');".execute(&mut conn);

        let join = r#"
            SELECT test.id, test.col_text, test2.ref_text
            FROM test
            INNER JOIN test2 ON test.id = test2.ref_id
            WHERE test.col_int8 IS NULL
            AND test.id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
            ORDER BY test.id;
        "#
        .fetch_one::<(i64, String, String)>(&mut conn);
        assert_eq!(join, (2, String::from("foo"), String::from("qux")));
    }

    #[rstest]
    fn post_update(#[from(setup_test_table)] mut conn: PgConnection) {
        // Verify that NULL is not counted after update
        "UPDATE test SET col_text = NULL".execute(&mut conn);
        let count = r#"
            SELECT count(*)
            FROM test
            WHERE col_text IS NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range);
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(4,)]);

        let res = r#"
            SELECT id, col_int8, col_boolean
            FROM test
            WHERE col_text IS NULL
            AND id @@@ paradedb.range(field=> 'id', range=> '[1, 5)'::int8range)
            ORDER BY id;
        "#
        .fetch::<(i64, Option<i64>, bool)>(&mut conn);
        assert_eq!(
            res,
            vec![
                (1, None, false),
                (2, None, false),
                (3, Some(333), false),
                (4, Some(444), false)
            ]
        )
    }
}

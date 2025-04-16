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

    // test IS TRUE operator
    {
        let sqltype = "boolean";
        let sqlname = sqlname(sqltype);
        let sql = format!(
            r#"
                EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
                SELECT count(*)
                FROM test
                WHERE {sqlname} IS true
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

    // test IS FALSE operator
    {
        let sqltype = "boolean";
        let sqlname = sqlname(sqltype);
        let sql = format!(
            r#"
                EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
                SELECT count(*)
                FROM test
                WHERE {sqlname} IS false
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
fn issue2301_is_null_with_joins(mut conn: PgConnection) {
    r#"
        CREATE TABLE mcp_server (
            id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
            name text NOT NULL,
            description text NOT NULL,
            created_at timestamp with time zone NOT NULL DEFAULT now(),
            attributes jsonb NOT NULL DEFAULT '[]'::jsonb,
            updated_at timestamp with time zone NOT NULL DEFAULT now(),
            synced_at timestamp with time zone,
            removed_at timestamp with time zone
        );
        CREATE INDEX mcp_server_search_idx ON mcp_server
        USING bm25 (id, name, description)
        WITH (key_field='id');
    "#
    .execute(&mut conn);

    let (plan, ) = r#"
        EXPLAIN (VERBOSE, FORMAT JSON) SELECT ms1.id, ms1.name, paradedb.score (ms1.id)
        FROM mcp_server ms1
        WHERE
          ms1.synced_at IS NOT NULL
          AND ms1.removed_at IS NULL
          AND ms1.id @@@ '{
              "boolean": {
                "should": [
                  {"boost": {"factor": 2, "query": {"fuzzy_term": {"field": "name", "value": "cloudflare"}}}},
                  {"boost": {"factor": 1, "query": {"fuzzy_term": {"field": "description", "value": "cloudflare"}}}}
                ]
              }
            }'::jsonb
        ORDER BY paradedb.score (ms1.id) DESC;
    "#.fetch_one::<(Value, )>(&mut conn);

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
                (1, false, None, None),
                (2, false, Some(String::from("foo")), None),
                (3, false, Some(String::from("bar")), Some(333)),
                (4, false, None, Some(444))
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

/// Tests for boolean IS TRUE/FALSE operators
mod pushdown_is_bool_operator {
    use super::*;

    // Helper function to verify a query uses custom scan and returns expected results
    fn verify_boolean_is_operator(
        conn: &mut PgConnection,
        condition: &str,
        expected_id: i64,
        expected_bool_value: bool,
    ) {
        // Check execution plan uses custom scan
        let sql = format!(
            r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            SELECT *, paradedb.score(id) FROM is_true
            WHERE bool_field {condition} AND message @@@ 'beer';
            "#
        );

        eprintln!("{sql}");
        let (plan,) = sql.fetch_one::<(Value,)>(conn);
        eprintln!("{plan:#?}");

        // Verify custom scan is used (handling both direct Custom Scan and Gather with Custom Scan child)
        let plan_node = plan
            .pointer("/0/Plan/Plans/0")
            .unwrap()
            .as_object()
            .unwrap();
        let node_type = plan_node.get("Node Type").unwrap().as_str().unwrap();

        if node_type == "Custom Scan" {
            assert_eq!("Custom Scan", node_type);
        } else {
            assert_eq!("Gather", node_type);
            let child_node = plan_node
                .get("Plans")
                .unwrap()
                .as_array()
                .unwrap()
                .get(0)
                .unwrap()
                .as_object()
                .unwrap();
            assert_eq!(
                "Custom Scan",
                child_node.get("Node Type").unwrap().as_str().unwrap()
            );
        }

        // Verify query results
        let results: Vec<(i64, bool, String, f32)> = format!(
            r#"
            SELECT id, bool_field, message, paradedb.score(id)
            FROM is_true
            WHERE bool_field {condition} AND message @@@ 'beer'
            ORDER BY id;
            "#
        )
        .fetch(conn);

        assert_eq!(1, results.len());
        assert_eq!(expected_id, results[0].0); // id
        assert_eq!(expected_bool_value, results[0].1); // bool_field
        assert_eq!("beer", results[0].2); // message
    }

    // Helper for complex boolean expression tests
    fn verify_complex_boolean_expr(
        conn: &mut PgConnection,
        condition: &str,
        expected_id: i64,
        expected_bool_value: bool,
    ) {
        let sql = format!(
            r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            SELECT *, paradedb.score(id) FROM is_true
            WHERE {condition} AND message @@@ 'beer';
            "#
        );

        eprintln!("{sql}");
        let (plan,) = sql.fetch_one::<(Value,)>(conn);
        eprintln!("{plan:#?}");

        // For complex expressions we don't verify the plan type
        // since it may not use Custom Scan directly

        // Just verify the query results
        let results: Vec<(i64, bool, String, f32)> = format!(
            r#"
            SELECT id, bool_field, message, paradedb.score(id)
            FROM is_true
            WHERE {condition} AND message @@@ 'beer'
            ORDER BY id;
            "#
        )
        .fetch(conn);

        assert_eq!(1, results.len());
        assert_eq!(expected_id, results[0].0); // id
        assert_eq!(expected_bool_value, results[0].1); // bool_field
        assert_eq!("beer", results[0].2); // message
    }

    // Helper for testing NULL value handling
    fn verify_null_handling(
        conn: &mut PgConnection,
        condition: &str,
        expected_count: usize,
        should_have_false: bool,
        should_have_null: bool,
    ) {
        let results: Vec<(i64, Option<bool>, String)> = format!(
            r#"
            SELECT id, bool_field, message
            FROM bool_test_nulls
            WHERE {condition} AND message @@@ 'beer'
            ORDER BY id;
            "#
        )
        .fetch(conn);

        assert_eq!(
            expected_count,
            results.len(),
            "Expected {expected_count} rows for condition: {condition}"
        );

        if should_have_false {
            let has_false = results.iter().any(|(_, b, _)| *b == Some(false));
            assert!(
                has_false,
                "Results should include a row with bool_field = false"
            );
        }

        if should_have_null {
            let has_null = results.iter().any(|(_, b, _)| b.is_none());
            assert!(
                has_null,
                "Results should include a row with bool_field = NULL"
            );
        }
    }

    /// Test for issue #2433: Pushdown `bool_field IS true|false`
    /// Verifies that the SQL IS operator for boolean fields is properly
    /// pushed down to the ParadeDB scan operator.
    #[rstest]
    fn test_bool_is_operator_pushdown(mut conn: PgConnection) {
        r#"
    DROP TABLE IF EXISTS is_true;
    CREATE TABLE is_true (
        id serial8 not null primary key,
        bool_field boolean,
        message text
    );

    CREATE INDEX idxis_true ON is_true USING bm25 (id, bool_field, message) WITH (key_field = 'id');

    INSERT INTO is_true (bool_field, message) VALUES (true, 'beer');
    INSERT INTO is_true (bool_field, message) VALUES (false, 'beer');
    "#
        .execute(&mut conn);

        // Test all boolean IS operators using the helper function
        verify_boolean_is_operator(&mut conn, "IS true", 1, true);
        verify_boolean_is_operator(&mut conn, "IS false", 2, false);
        verify_boolean_is_operator(&mut conn, "IS NOT true", 2, false);
        verify_boolean_is_operator(&mut conn, "IS NOT false", 1, true);
    }

    /// Test for issue #2433: Complex boolean expressions with IS TRUE/FALSE operators
    /// This test checks the behavior of complex expressions (not just simple field references)
    /// with IS TRUE/FALSE operators.
    ///
    /// Note: Currently, complex expressions won't be pushed down to the ParadeDB scan operator.
    /// PostgreSQL will handle the evaluation of these expressions after the scan.
    /// We're marking this test as ignored until we implement full support for complex expressions.
    #[rstest]
    #[ignore]
    fn test_complex_bool_expressions_with_is_operator(mut conn: PgConnection) {
        r#"
    DROP TABLE IF EXISTS is_true;
    CREATE TABLE is_true (
        id serial8 not null primary key,
        bool_field boolean,
        message text
    );

    CREATE INDEX idxis_true ON is_true USING bm25 (id, bool_field, message) WITH (key_field = 'id');

    INSERT INTO is_true (bool_field, message) VALUES (true, 'beer');
    INSERT INTO is_true (bool_field, message) VALUES (false, 'beer');
    
    CREATE OR REPLACE FUNCTION is_true_test(b boolean) RETURNS boolean AS $$
    BEGIN
        RETURN b;
    END;
    $$ LANGUAGE plpgsql;
    "#
        .execute(&mut conn);

        // Test with expression IS TRUE
        verify_complex_boolean_expr(&mut conn, "(bool_field = true) IS true", 1, true);

        // Test with function call IS TRUE
        {
            let sql = r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            SELECT *, paradedb.score(id) FROM is_true
            WHERE is_true_test(bool_field) IS true AND message @@@ 'beer';
            "#;

            eprintln!("{sql}");
            let (plan,) = sql.fetch_one::<(Value,)>(&mut conn);
            eprintln!("{plan:#?}");

            // Query with function call IS TRUE
            let results: Vec<(i64, bool, String, Option<f32>)> = r#"
            SELECT id, bool_field, message, paradedb.score(id)
            FROM is_true
            WHERE is_true_test(bool_field) IS true AND message @@@ 'beer'
            ORDER BY id;
            "#
            .fetch(&mut conn);

            assert_eq!(1, results.len());
            assert_eq!(1, results[0].0); // id
            assert_eq!(true, results[0].1); // bool_field
            assert_eq!("beer", results[0].2); // message
        }

        // Test with complex expression IS FALSE
        verify_complex_boolean_expr(&mut conn, "(bool_field <> true) IS true", 2, false);
    }

    /// Test the difference between IS TRUE and = TRUE operators with NULL values
    /// IS TRUE will always return a boolean value, even if the argument is null.
    /// = TRUE will return null if the argument is null.
    #[rstest]
    #[ignore]
    fn test_bool_is_vs_equals_with_nulls(mut conn: PgConnection) {
        r#"
        DROP TABLE IF EXISTS bool_test_nulls;
        CREATE TABLE bool_test_nulls (
            id serial8 not null primary key,
            bool_field boolean,
            message text
        );

        CREATE INDEX idx_bool_test_nulls ON bool_test_nulls USING bm25 (id, bool_field, message) WITH (key_field = 'id');

        -- Insert values: true, false, and NULL
        INSERT INTO bool_test_nulls (bool_field, message) VALUES (true, 'beer');
        INSERT INTO bool_test_nulls (bool_field, message) VALUES (false, 'beer');
        INSERT INTO bool_test_nulls (bool_field, message) VALUES (NULL, 'beer');
        "#
        .execute(&mut conn);

        // Test with IS TRUE - should return only the row with true
        verify_null_handling(&mut conn, "bool_field IS TRUE", 1, false, false);

        // Test with = TRUE - should also only return the row with true
        verify_null_handling(&mut conn, "bool_field = TRUE", 1, false, false);

        // Now check IS NOT TRUE - should return rows with false and NULL
        verify_null_handling(&mut conn, "bool_field IS NOT TRUE", 2, true, true);

        // Test NOT (field = TRUE) - should only return FALSE (no NULL)
        {
            let results: Vec<(i64, Option<bool>, String, f32)> = r#"
                SELECT id, bool_field, message, paradedb.score(id)
                FROM bool_test_nulls
                WHERE NOT (bool_field = TRUE) AND message @@@ 'beer'
                ORDER BY id;
            "#
            .fetch(&mut conn);

            // According to SQL standard, NOT (field = TRUE) should return only FALSE rows (not NULL)
            if results.len() > 1 {
                // Implementation is returning both FALSE and NULL rows
                let has_false = results.iter().any(|(_, b, _, _)| *b == Some(false));
                assert!(has_false, "Results should include a FALSE row");

                let has_null = results.iter().any(|(_, b, _, _)| b.is_none());
                if has_null {
                    eprintln!("FAIL: Current implementation incorrectly includes NULL rows for NOT (field = TRUE)");
                    eprintln!("According to SQL standard, NOT (field = TRUE) should only return rows where value is FALSE");
                }

                // Explicitly fail the test if implementation is wrong
                assert_eq!(
                    1,
                    results.len(),
                    "SQL standard: NOT (field = TRUE) should only return FALSE rows, not NULL rows"
                );
            } else {
                // Implementation is correct
                assert_eq!(1, results.len(), "Should only return FALSE row");
                assert_eq!(
                    Some(false),
                    results[0].1,
                    "The row should have bool_field = false"
                );
            }
        }
    }

    /// Test specifically the custom scan behavior with NULL values for boolean IS operators
    #[rstest]
    #[ignore]
    fn test_bool_is_null_pushdown(mut conn: PgConnection) {
        r#"
        DROP TABLE IF EXISTS bool_null_test;
        CREATE TABLE bool_null_test (
            id serial8 not null primary key,
            bool_field boolean,
            message text
        );

        CREATE INDEX idx_bool_null_test ON bool_null_test USING bm25 (id, bool_field, message) WITH (key_field = 'id');

        -- Insert values: true, false, and NULL
        INSERT INTO bool_null_test (bool_field, message) VALUES (true, 'beer');
        INSERT INTO bool_null_test (bool_field, message) VALUES (false, 'beer');
        INSERT INTO bool_null_test (bool_field, message) VALUES (NULL, 'beer');
        "#
        .execute(&mut conn);

        // Helper function for testing boolean conditions with expected row count and value checks
        fn test_boolean_condition(
            conn: &mut PgConnection,
            condition: &str,
            expected_count: usize,
            expected_values: &[Option<bool>],
            description: &str,
        ) {
            // Check query plan
            let sql = format!(
                r#"
                EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
                SELECT *, paradedb.score(id) FROM bool_null_test
                WHERE {condition} AND message @@@ 'beer';
                "#
            );

            eprintln!("{sql}");
            let (plan,) = sql.fetch_one::<(Value,)>(conn);
            eprintln!("{plan:#?}");

            // Get actual results
            let results: Vec<(i64, Option<bool>, String, f32)> = format!(
                r#"
                SELECT id, bool_field, message, paradedb.score(id)
                FROM bool_null_test
                WHERE {condition} AND message @@@ 'beer'
                ORDER BY id;
                "#
            )
            .fetch(conn);

            // Check result count
            if results.len() != expected_count {
                eprintln!(
                    "FAIL: '{condition}' should return {expected_count} rows, got {}",
                    results.len()
                );
                assert_eq!(expected_count, results.len(), "SQL standard: {description}");
            }

            // Check expected values if provided
            for expected_value in expected_values {
                match expected_value {
                    Some(value) => {
                        let has_value = results.iter().any(|(_, b, _, _)| *b == Some(*value));
                        assert!(
                            has_value,
                            "Results should include a row with bool_field = {value}"
                        );
                    }
                    None => {
                        let has_null = results.iter().any(|(_, b, _, _)| b.is_none());
                        assert!(
                            has_null,
                            "Results should include a row with bool_field = NULL"
                        );
                    }
                }
            }
        }

        // Test with IS FALSE - should only return the FALSE row (not NULL)
        test_boolean_condition(
            &mut conn,
            "bool_field IS FALSE",
            1,
            &[Some(false)],
            "IS FALSE should only return FALSE rows, not NULL rows",
        );

        // Test with = FALSE - should only return the FALSE row (not NULLs)
        test_boolean_condition(
            &mut conn,
            "bool_field = FALSE",
            1,
            &[Some(false)],
            "= FALSE should only return FALSE rows, not NULL rows",
        );

        // Test with IS NOT TRUE - should return FALSE and NULL rows
        test_boolean_condition(
            &mut conn,
            "bool_field IS NOT TRUE",
            2,
            &[Some(false), None],
            "IS NOT TRUE should return both FALSE and NULL rows",
        );

        // Test NOT (field = TRUE) - should only return FALSE (no NULL)
        test_boolean_condition(
            &mut conn,
            "NOT (bool_field = TRUE)",
            1,
            &[Some(false)],
            "NOT (field = TRUE) should only return FALSE rows, not NULL rows",
        );
    }
}

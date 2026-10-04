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

// Tests for ParadeDB's Aggregate Custom Scan implementation

use futures::executor::block_on;
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;
use tests::fixtures::*;

fn assert_uses_datafusion_aggregate_scan(conn: &mut PgConnection, query: impl AsRef<str>) {
    let (plan,) = format!("EXPLAIN (FORMAT JSON) {}", query.as_ref()).fetch_one::<(Value,)>(conn);

    let plan = plan.to_string();
    assert!(
        plan.contains("ParadeDB Aggregate Scan"),
        "expected ParadeDB Aggregate Scan:\n{plan}"
    );
    assert!(
        plan.contains("DataFusion Physical Plan"),
        "expected DataFusion backend:\n{plan}"
    );
}

fn assert_uses_custom_scan(conn: &mut PgConnection, enabled: bool, query: impl AsRef<str>) {
    let (plan,) = format!(" EXPLAIN (FORMAT JSON) {}", query.as_ref()).fetch_one::<(Value,)>(conn);
    eprintln!("{plan:#?}");
    assert_eq!(
        enabled,
        plan.to_string().contains("ParadeDB Aggregate Scan")
    );
}

#[rstest]
fn test_count(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Use the aggregate custom scan only if it is enabled.
    for enabled in [true, false] {
        format!("SET paradedb.enable_aggregate_custom_scan TO {enabled};").execute(&mut conn);

        let query = "SELECT COUNT(*) FROM paradedb.bm25_search WHERE description @@@ 'keyboard'";

        assert_uses_custom_scan(&mut conn, enabled, query);

        let (count,) = query.fetch_one::<(i64,)>(&mut conn);
        assert_eq!(count, 2, "With custom scan: {enabled}");
    }
}

#[rstest]
#[case(0, true)]
#[case(9_007_199_254_740_992, true)]
#[case(9_007_199_254_740_993, false)]
#[case(-9_007_199_254_740_993, false)]
#[case(i64::MAX, false)]
#[case(i64::MIN, true)]
fn test_coalesce_default_precision(
    mut conn: PgConnection,
    #[case] default: i64,
    #[case] pushdown: bool,
    #[values("value", "(metadata->>'value')::bigint")] field: &str,
) {
    r#"
        SET paradedb.enable_aggregate_custom_scan TO on;
        SET max_parallel_workers_per_gather TO 0;
        CREATE TABLE coalesce_defaults (id bigint PRIMARY KEY, value bigint, metadata jsonb);
        INSERT INTO coalesce_defaults VALUES
            (1, 1, '{"value": 1}'),
            (2, NULL, '{"value": null}'),
            (3, NULL, '{}'),
            (4, NULL, NULL);
        CREATE INDEX ON coalesce_defaults USING paradedb (id, value, metadata)
            WITH (key_field = 'id', json_fields = '{"metadata": {"fast": true}}');
    "#
    .execute(&mut conn);

    let argument = format!("COALESCE({field}, '{default}'::bigint)");
    let query = format!(
        "SELECT COUNT({argument}), MIN({argument}), MAX({argument})
         FROM coalesce_defaults WHERE id @@@ pdb.all()"
    );
    // The planner can't know a JSON path's column type in each segment, and a segment without the
    // path reads it as unsigned, so a negative default doesn't push down.
    let pushdown = pushdown && (field == "value" || default >= 0);
    assert_uses_custom_scan(&mut conn, pushdown, &query);
    assert_eq!(
        query.fetch_one::<(i64, i64, i64)>(&mut conn),
        (4, default.min(1), default.max(1))
    );
}

#[rstest]
fn test_count_with_group_by(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);
    "SET client_min_messages TO warning;".execute(&mut conn);

    // First test simple COUNT(*) without GROUP BY
    let simple_count = "SELECT COUNT(*) FROM paradedb.bm25_search";
    eprintln!("Testing simple COUNT(*)");
    let (plan,) = format!("EXPLAIN (FORMAT JSON) {simple_count}").fetch_one::<(Value,)>(&mut conn);
    eprintln!("Simple COUNT(*) plan: {plan:#?}");
    eprintln!(
        "Uses aggregate scan: {}",
        plan.to_string().contains("ParadeDB Aggregate Scan")
    );

    // Test COUNT(*) with WHERE clause (like the working test)
    let count_with_where =
        "SELECT COUNT(*) FROM paradedb.bm25_search WHERE description @@@ 'keyboard'";
    eprintln!("\nTesting COUNT(*) with WHERE clause");
    let (plan,) =
        format!("EXPLAIN (FORMAT JSON) {count_with_where}").fetch_one::<(Value,)>(&mut conn);
    eprintln!(
        "COUNT(*) with WHERE plan uses aggregate scan: {}",
        plan.to_string().contains("ParadeDB Aggregate Scan")
    );

    // Then test WITHOUT WHERE clause but WITH GROUP BY
    let query_no_where = r#"
        SELECT rating, COUNT(*) 
        FROM paradedb.bm25_search 
        GROUP BY rating 
        ORDER BY rating
    "#;

    eprintln!("Testing query without WHERE clause");
    let (plan,) =
        format!("EXPLAIN (FORMAT JSON) {query_no_where}").fetch_one::<(Value,)>(&mut conn);
    eprintln!("Plan without WHERE: {plan:#?}");
    eprintln!(
        "Uses aggregate scan: {}",
        plan.to_string().contains("ParadeDB Aggregate Scan")
    );

    // Then test WITH WHERE clause
    let query = r#"
        SELECT rating, COUNT(*) 
        FROM paradedb.bm25_search 
        WHERE description @@@ 'shoes' 
        GROUP BY rating 
        ORDER BY rating
    "#;

    // Verify it uses the aggregate custom scan
    assert_uses_custom_scan(&mut conn, true, query);

    // Execute and verify results
    let results: Vec<(i32, i64)> = query.fetch(&mut conn);
    assert_eq!(results.len(), 3); // We should have 3 distinct ratings for shoes
    assert_eq!(results[0], (3, 1)); // rating 3, count 1
    assert_eq!(results[1], (4, 1)); // rating 4, count 1
    assert_eq!(results[2], (5, 1)); // rating 5, count 1
}

// PostgreSQL caches the plan of a prepared statement and runs it again. The
// scan must leave the plan in a state that the next run can use.
#[rstest]
fn test_prepared_tantivy_groupby_survives_reuse(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    let query = r#"
        SELECT rating, COUNT(*)
        FROM paradedb.bm25_search
        WHERE description @@@ 'shoes'
        GROUP BY rating
        ORDER BY rating
    "#;

    assert_uses_custom_scan(&mut conn, true, query);
    let (plan,) = format!("EXPLAIN (FORMAT JSON) {query}").fetch_one::<(Value,)>(&mut conn);
    let plan = plan.to_string();
    assert!(
        !plan.contains("DataFusion Physical Plan"),
        "expected the Tantivy aggregate backend:\n{plan}"
    );

    let expected: Vec<(i32, i64)> = query.fetch(&mut conn);
    assert_eq!(expected, vec![(3, 1), (4, 1), (5, 1)]);

    format!("PREPARE group_by_rating AS {query}").execute(&mut conn);

    for _ in 0..8 {
        let actual: Vec<(i32, i64)> = "EXECUTE group_by_rating".fetch(&mut conn);
        assert_eq!(actual, expected);
    }
}

// A driver sends the parameters apart from the statement. PostgreSQL makes
// custom plans for the first runs, and then it can change to a cached generic
// plan.
#[rstest]
fn test_bound_parameters_tantivy_groupby_survives_reuse(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    // The comment gives each mode its own statement.
    const AUTO: &str = r#"
        /* auto */
        SELECT rating, COUNT(*)
        FROM paradedb.bm25_search
        WHERE rating >= $1 AND description @@@ 'shoes'
        GROUP BY rating
        ORDER BY rating
    "#;
    const FORCE_GENERIC_PLAN: &str = r#"
        /* force_generic_plan */
        SELECT rating, COUNT(*)
        FROM paradedb.bm25_search
        WHERE rating >= $1 AND description @@@ 'shoes'
        GROUP BY rating
        ORDER BY rating
    "#;

    fn run(conn: &mut PgConnection, query: &'static str, min_rating: i32) -> Vec<(i32, i64)> {
        block_on(
            sqlx::query_as::<_, (i32, i64)>(query)
                .bind(min_rating)
                .fetch_all(conn),
        )
        .expect("the prepared aggregate should run")
    }

    for (plan_cache_mode, query) in [("auto", AUTO), ("force_generic_plan", FORCE_GENERIC_PLAN)] {
        format!("SET plan_cache_mode = {plan_cache_mode};").execute(&mut conn);
        for _ in 0..4 {
            assert_eq!(run(&mut conn, query, 3), vec![(3, 1), (4, 1), (5, 1)]);
            assert_eq!(run(&mut conn, query, 4), vec![(4, 1), (5, 1)]);
            assert_eq!(run(&mut conn, query, 6), vec![]);
        }
    }
}

#[rstest]
fn test_group_by(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    // Supports GROUP BY with aggregate scan
    assert_uses_custom_scan(
        &mut conn,
        true,
        r#"
        SELECT rating, COUNT(*)
        FROM paradedb.bm25_search WHERE
        description @@@ 'keyboard'
        GROUP BY rating
        ORDER BY rating
        "#,
    );
}

// PostgreSQL does not group on a key that the WHERE clause pins to one value,
// and reads the key from one row of the group. The scan must do the same.
#[rstest]
fn test_group_by_key_pinned_to_constant(mut conn: PgConnection) {
    r#"
    CREATE TABLE pinned_keys (
        id SERIAL PRIMARY KEY,
        account_id BIGINT,
        region SMALLINT,
        kind TEXT,
        price FLOAT8
    );
    INSERT INTO pinned_keys (account_id, region, kind, price)
    SELECT (g % 3) + 1, (g % 3) + 1, (ARRAY['a', 'b', 'c', 'd'])[(g % 4) + 1], g % 5
    FROM generate_series(1, 120) g;
    CREATE INDEX pinned_keys_idx ON pinned_keys
    USING paradedb (id, account_id, region, (kind::pdb.literal), price)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let queries = [
        // The only key is pinned, with and without a matching row.
        "SELECT account_id, COUNT(*) FROM pinned_keys
         WHERE account_id = 1 AND id @@@ paradedb.all() GROUP BY account_id",
        "SELECT account_id, COUNT(*) FROM pinned_keys
         WHERE account_id = 99 AND id @@@ paradedb.all() GROUP BY account_id",
        "SELECT COUNT(*), SUM(price) FROM pinned_keys
         WHERE account_id = 99 AND id @@@ paradedb.all() GROUP BY account_id",
        "SELECT account_id, COUNT(*) FILTER (WHERE kind = 'none') FROM pinned_keys
         WHERE account_id = 1 AND id @@@ paradedb.all() GROUP BY account_id",
        // A pinned key next to a key that is not pinned.
        "SELECT account_id, kind, COUNT(*), SUM(price) FROM pinned_keys
         WHERE account_id = 2 AND id @@@ paradedb.all() GROUP BY account_id, kind",
        // The constant has a different type than the key.
        "SELECT region, kind, COUNT(*) FROM pinned_keys
         WHERE region = 2::bigint AND id @@@ paradedb.all() GROUP BY region, kind",
        "SELECT region, COUNT(*) FROM pinned_keys
         WHERE region = 100000 AND id @@@ paradedb.all() GROUP BY region",
        // A node above the scan reads the aggregate.
        "SELECT DISTINCT account_id, COUNT(*) FROM pinned_keys
         WHERE account_id = 1 AND id @@@ paradedb.all() GROUP BY account_id",
        "SELECT account_id, COUNT(*), SUM(COUNT(*)) OVER () FROM pinned_keys
         WHERE account_id = 1 AND id @@@ paradedb.all() GROUP BY account_id",
        // An aggregate with its own ORDER BY.
        "SELECT account_id, COUNT(id ORDER BY kind) FROM pinned_keys
         WHERE account_id = 1 AND id @@@ paradedb.all() GROUP BY account_id",
        // Equal `float8` values are not always identical (`-0` and `0`), and the
        // key comes from a row.
        "SELECT price, kind, COUNT(*) FROM pinned_keys
         WHERE price = 1 AND id @@@ paradedb.all() GROUP BY price, kind",
    ];

    for query in queries {
        "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);
        assert_uses_datafusion_aggregate_scan(&mut conn, query);

        // The comment gives each setting its own statement, and with it its own plan.
        let [pushed_down, expected] = ["on", "off"].map(|enabled| {
            format!("SET paradedb.enable_aggregate_custom_scan TO {enabled};").execute(&mut conn);
            let (rows,) = format!(
                "/* aggregate scan {enabled} */
                 SELECT COALESCE(jsonb_agg(to_jsonb(q) ORDER BY to_jsonb(q)::text), '[]')::text
                 FROM ({query}) q"
            )
            .fetch_one::<(String,)>(&mut conn);
            rows
        });

        assert_eq!(pushed_down, expected, "{query}");
    }

    // A parameter in a reused plan must give the value of each execution.
    "SET plan_cache_mode = force_generic_plan;".execute(&mut conn);
    r#"
    PREPARE pinned_keys_count(bigint) AS
    SELECT account_id, COUNT(*) FROM pinned_keys
    WHERE account_id = $1 AND id @@@ paradedb.all() GROUP BY account_id;
    PREPARE pinned_keys_count_by_kind(bigint) AS
    SELECT account_id, kind, COUNT(*) FROM pinned_keys
    WHERE account_id = $1 AND id @@@ paradedb.all() GROUP BY account_id, kind ORDER BY kind;
    "#
    .execute(&mut conn);
    for account_id in [1_i64, 2, 3] {
        let rows =
            format!("EXECUTE pinned_keys_count({account_id})").fetch::<(i64, i64)>(&mut conn);
        assert_eq!(rows, vec![(account_id, 40)]);

        let rows = format!("EXECUTE pinned_keys_count_by_kind({account_id})")
            .fetch::<(i64, String, i64)>(&mut conn);
        let expected = ["a", "b", "c", "d"].map(|kind| (account_id, kind.to_string(), 10));
        assert_eq!(rows, expected);
    }
    let rows = "EXECUTE pinned_keys_count(99)".fetch::<(i64, i64)>(&mut conn);
    assert!(rows.is_empty());
}

// PostgreSQL takes a column that is not a GROUP BY key when the keys have the
// primary key of its table. The scan must return the value of such a column.
#[rstest]
fn test_group_by_column_that_depends_on_keys(mut conn: PgConnection) {
    r#"
    CREATE TABLE dependent_cols (
        id SERIAL PRIMARY KEY,
        account_id BIGINT,
        region INT,
        kind TEXT,
        price FLOAT8,
        created DATE
    );
    INSERT INTO dependent_cols (account_id, region, kind, price, created)
    SELECT
        CASE WHEN g % 7 = 0 THEN NULL ELSE (g % 3) + 1 END,
        (g % 5) + 1,
        CASE WHEN g % 5 = 0 THEN NULL ELSE (ARRAY['a', 'b', 'c', 'd'])[(g % 4) + 1] END,
        CASE WHEN g % 6 = 0 THEN NULL ELSE g % 5 END,
        CASE WHEN g % 4 = 0 THEN NULL ELSE DATE '2024-01-01' + g END
    FROM generate_series(1, 40) g;
    UPDATE dependent_cols SET account_id = 9223372036854775807 WHERE id = 1;
    UPDATE dependent_cols SET account_id = -9223372036854775808 WHERE id = 2;
    CREATE INDEX dependent_cols_idx ON dependent_cols
    USING paradedb (id, account_id, region, (kind::pdb.literal), price, created)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let queries = [
        // The column is in the GROUP BY, and PostgreSQL drops it from the keys.
        "SELECT id, kind, COUNT(*) FROM dependent_cols
         WHERE id @@@ paradedb.all() GROUP BY id, kind",
        // The column is not in the GROUP BY.
        "SELECT id, kind, price, account_id, created, COUNT(*), SUM(price) FROM dependent_cols
         WHERE id @@@ paradedb.all() GROUP BY id",
        // A cast of the column next to the column.
        "SELECT id, created, created::text, COUNT(*) FROM dependent_cols
         WHERE id @@@ paradedb.all() GROUP BY id",
        "SELECT id, kind, COUNT(*) FROM dependent_cols
         WHERE id @@@ paradedb.all() GROUP BY id ORDER BY kind NULLS FIRST, id LIMIT 7",
        // The primary key is pinned to one value.
        "SELECT id, kind, COUNT(*) FROM dependent_cols
         WHERE id = 7 AND id @@@ paradedb.all() GROUP BY id",
        "SELECT id, kind, COUNT(*) FROM dependent_cols
         WHERE id = 999 AND id @@@ paradedb.all() GROUP BY id",
        // One key is pinned, and the other key still decides the groups.
        "SELECT region, kind, COUNT(*) FROM dependent_cols
         WHERE region = 2 AND id @@@ paradedb.all() GROUP BY region, kind",
        "SELECT price, kind, COUNT(*) FROM dependent_cols
         WHERE price = 2 AND id @@@ paradedb.all() GROUP BY price, kind",
    ];

    // Two keys, and no key decides the other: the scan groups on both, on the
    // Tantivy backend.
    let plain_keys = "SELECT price, kind, COUNT(*) FROM dependent_cols
         WHERE id @@@ paradedb.all() GROUP BY price, kind";
    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);
    let (plan,) = format!("EXPLAIN (FORMAT JSON) {plain_keys}").fetch_one::<(Value,)>(&mut conn);
    let plan = plan.to_string();
    assert!(plan.contains("Tantivy Query"), "{plan}");

    for query in queries.into_iter().chain([plain_keys]) {
        "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);
        if query != plain_keys {
            assert_uses_datafusion_aggregate_scan(&mut conn, query);
        }

        // The comment keeps the two settings from using one cached plan.
        let [pushed_down, expected] = ["on", "off"].map(|enabled| {
            format!("SET paradedb.enable_aggregate_custom_scan TO {enabled};").execute(&mut conn);
            let (rows,) = format!(
                "/* aggregate scan {enabled} */
                 SELECT COALESCE(jsonb_agg(to_jsonb(q) ORDER BY to_jsonb(q)::text), '[]')::text
                 FROM ({query}) q"
            )
            .fetch_one::<(String,)>(&mut conn);
            rows
        });

        assert_eq!(pushed_down, expected, "{query}");
    }
}

#[rstest]
fn test_group_by_null_bucket(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    assert_uses_custom_scan(
        &mut conn,
        true,
        r#"
        SELECT rating, COUNT(*)
        FROM paradedb.bm25_search
        WHERE description @@@ 'keyboard'
        GROUP BY rating
        ORDER BY rating NULLS FIRST
    "#,
    );
}

// On PG16 and later, PostgreSQL puts the sort keys of an ordered aggregate
// after the GROUP BY keys in `group_pathkeys`. The scan must not group on them.
// PG15 has no such keys, so this test does not fail there without the fix.
#[rstest]
fn test_ordered_aggregate_is_not_a_group_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE ordered_aggs (
        id SERIAL PRIMARY KEY,
        account_id BIGINT,
        kind TEXT,
        price FLOAT8
    );
    INSERT INTO ordered_aggs (account_id, kind, price)
    SELECT (g % 3) + 1, (ARRAY['a', 'b', 'c', 'd'])[(g % 4) + 1], g % 5
    FROM generate_series(1, 120) g;
    CREATE INDEX ordered_aggs_idx ON ordered_aggs
    USING paradedb (id, account_id, (kind::pdb.literal), price)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let queries = [
        "SELECT COUNT(id ORDER BY kind) FROM ordered_aggs WHERE id @@@ paradedb.all()",
        "SELECT account_id, COUNT(id ORDER BY kind), SUM(price ORDER BY kind) FROM ordered_aggs
         WHERE id @@@ paradedb.all() GROUP BY account_id",
        "SELECT kind, MAX(price ORDER BY kind, account_id) FROM ordered_aggs
         WHERE id @@@ paradedb.all() GROUP BY kind",
    ];

    for query in queries {
        "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);
        assert_uses_custom_scan(&mut conn, true, query);

        // The comment gives each setting its own statement, and with it its own plan.
        let [pushed_down, expected] = ["on", "off"].map(|enabled| {
            format!("SET paradedb.enable_aggregate_custom_scan TO {enabled};").execute(&mut conn);
            let (rows,) = format!(
                "/* aggregate scan {enabled} */
                 SELECT COALESCE(jsonb_agg(to_jsonb(q) ORDER BY to_jsonb(q)::text), '[]')::text
                 FROM ({query}) q"
            )
            .fetch_one::<(String,)>(&mut conn);
            rows
        });

        assert_eq!(pushed_down, expected, "{query}");
    }
}

#[rstest]
fn test_no_bm25_index(mut conn: PgConnection) {
    "CALL paradedb.create_bm25_test_table(table_name => 'no_bm25', schema_name => 'paradedb');"
        .execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    // Do not use the aggregate custom scan on non-bm25 indexed tables.
    assert_uses_custom_scan(&mut conn, false, "SELECT COUNT(*) FROM paradedb.no_bm25");
}

#[rstest]
fn test_other_aggregates(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);

    for aggregate_func in ["SUM(rating)", "AVG(rating)", "MIN(rating)", "MAX(rating)"] {
        assert_uses_custom_scan(
            &mut conn,
            true,
            format!(
                r#"
                SELECT {aggregate_func}
                FROM paradedb.bm25_search WHERE
                description @@@ 'keyboard'
                "#
            ),
        );
    }
}

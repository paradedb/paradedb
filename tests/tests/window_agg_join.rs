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

//! Global window aggregates (empty `OVER ()`) over a join with fast-field join
//! keys: the join itself is JoinScan-compatible, so the window aggregate must
//! not be a reason for the custom scans to decline (issue #5637). The plan
//! assertions expect the desired post-#5637 behavior and FAIL until it is
//! implemented; the value assertions must hold no matter which plan executes.
//! The pg_regress twin is Test 27b in
//! `pg_search/tests/pg_regress/sql/topk-agg-facet.sql`.

use rstest::*;
use sqlx::PgConnection;
use tests::fixtures::*;

const JOIN_SCAN: &str = "Custom Scan (ParadeDB Join Scan)";

// products: ids 1..1000, odd ids are 'laptop' (500 matches). reviews: exactly
// two per product, score = review id. The laptop join therefore matches the
// 1000 odd-id reviews, with scores the odd numbers 1..1999:
//   count = 1000, sum = 1000^2 = 1_000_000, avg = 1000, min = 1, max = 1999.
fn setup(conn: &mut PgConnection) {
    r#"
    SET paradedb.enable_custom_scan = on;
    SET paradedb.enable_join_custom_scan = on;
    SET paradedb.enable_aggregate_custom_scan = on;
    SET max_parallel_workers_per_gather = 0;

    DROP TABLE IF EXISTS wj_products;
    DROP TABLE IF EXISTS wj_reviews;
    CREATE TABLE wj_products (id int PRIMARY KEY, description text);
    CREATE TABLE wj_reviews (id bigint PRIMARY KEY, product_id bigint, score int);

    INSERT INTO wj_products
    SELECT g, CASE WHEN g % 2 = 1 THEN 'sturdy laptop' ELSE 'flimsy tablet' END
    FROM generate_series(1, 1000) g;
    INSERT INTO wj_reviews
    SELECT g, ((g - 1) % 1000) + 1, g FROM generate_series(1, 2000) g;

    CREATE INDEX wj_products_bm25 ON wj_products
    USING paradedb (id, (description::pdb.unicode_words)) WITH (key_field = 'id');
    CREATE INDEX wj_reviews_bm25 ON wj_reviews
    USING paradedb (id, product_id, score) WITH (key_field = 'id');
    ANALYZE wj_products;
    ANALYZE wj_reviews;
    "#
    .execute(conn);
}

fn explain(conn: &mut PgConnection, query: &str) -> String {
    let lines: Vec<String> = format!("EXPLAIN (COSTS OFF) {query}").fetch_scalar(conn);
    lines.join("\n")
}

#[rstest]
fn global_window_aggregates_over_join(mut conn: PgConnection) -> Result<(), sqlx::Error> {
    setup(&mut conn);

    // Every SQL-native aggregate that window_agg.rs can convert, as one column
    // each (one WindowFunc per target entry is the supported shape).
    let query = r#"
        SELECT p.id,
               r.score,
               COUNT(*) OVER () AS total_count,
               SUM(r.score) OVER () AS total_score,
               AVG(r.score) OVER ()::float8 AS avg_score,
               MIN(r.score) OVER () AS min_score,
               MAX(r.score) OVER () AS max_score
        FROM wj_products p
        JOIN wj_reviews r ON p.id = r.product_id
        WHERE p.description @@@ 'laptop'
        ORDER BY r.score DESC
        LIMIT 3
    "#;

    // Desired #5637 behavior: the custom scans absorb the global window
    // aggregates, so the JoinScan engages and no WindowAgg node remains,
    // while a WindowAggExec now exists
    let plan = explain(&mut conn, query);
    assert!(plan.contains(JOIN_SCAN), "{plan}");
    assert!(!plan.contains("WindowAgg "), "{plan}");
    assert!(plan.contains("WindowAggExec"), "{plan}");

    let rows = query.fetch_result::<(i32, i32, i64, i64, f64, i32, i32)>(&mut conn)?;
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows.iter().map(|r| (r.0, r.1)).collect::<Vec<_>>(),
        vec![(999, 1999), (997, 1997), (995, 1995)]
    );
    for (_, _, total_count, total_score, avg_score, min_score, max_score) in &rows {
        assert_eq!(*total_count, 1000);
        assert_eq!(*total_score, 1_000_000);
        assert_eq!(*avg_score, 1000.0);
        assert_eq!(*min_score, 1);
        assert_eq!(*max_score, 1999);
    }

    Ok(())
}

// Same join shape, but the aggregated column is NUMERIC(10, 2) (Numeric64
// storage): exercises the scaled-int64 window UDAFs, the scale literal, the
// decimal-bytes SUM/MIN/MAX conversions, and the AVG count+sum blob decode.
// price = g * 0.25, so the 1000 matched odd-id reviews carry prices
// 0.25 .. 499.75 (step 0.50):
//   count = 1000, sum = 250000.00, avg = 250, min = 0.25, max = 499.75.
fn setup_numeric(conn: &mut PgConnection) {
    r#"
    SET paradedb.enable_custom_scan = on;
    SET paradedb.enable_join_custom_scan = on;
    SET paradedb.enable_aggregate_custom_scan = on;
    SET max_parallel_workers_per_gather = 0;

    DROP TABLE IF EXISTS wjn_products;
    DROP TABLE IF EXISTS wjn_reviews;
    CREATE TABLE wjn_products (id int PRIMARY KEY, description text);
    CREATE TABLE wjn_reviews (id bigint PRIMARY KEY, product_id bigint, price numeric(10, 2));

    INSERT INTO wjn_products
    SELECT g, CASE WHEN g % 2 = 1 THEN 'sturdy laptop' ELSE 'flimsy tablet' END
    FROM generate_series(1, 1000) g;
    INSERT INTO wjn_reviews
    SELECT g, ((g - 1) % 1000) + 1, (g * 0.25)::numeric(10, 2)
    FROM generate_series(1, 2000) g;

    CREATE INDEX wjn_products_bm25 ON wjn_products
    USING paradedb (id, (description::pdb.unicode_words)) WITH (key_field = 'id');
    CREATE INDEX wjn_reviews_bm25 ON wjn_reviews
    USING paradedb (id, product_id, price) WITH (key_field = 'id');
    ANALYZE wjn_products;
    ANALYZE wjn_reviews;
    "#
    .execute(conn);
}

#[rstest]
fn global_window_aggregates_over_join_numeric(mut conn: PgConnection) -> Result<(), sqlx::Error> {
    setup_numeric(&mut conn);

    // The float8 casts wrap the placeholders, guarding the wintype (NUMERIC)
    // contract end to end. NOTE: an I/O-coercion cast like `::text` must not
    // be used here — replace_in_node only recurses through FuncExpr, so a
    // CoerceViaIO-wrapped WindowFunc is found by extraction but never
    // replaced, leaving a mixed tree that errors at execution (pre-existing
    // hook bug, single-table included).
    let query = r#"
        SELECT p.id,
               r.price::float8,
               COUNT(*) OVER () AS total_count,
               SUM(r.price) OVER ()::float8 AS total_price,
               AVG(r.price) OVER ()::float8 AS avg_price,
               MIN(r.price) OVER ()::float8 AS min_price,
               MAX(r.price) OVER ()::float8 AS max_price
        FROM wjn_products p
        JOIN wjn_reviews r ON p.id = r.product_id
        WHERE p.description @@@ 'laptop'
        ORDER BY r.id DESC
        LIMIT 3
    "#;

    let plan = explain(&mut conn, query);
    assert!(plan.contains(JOIN_SCAN), "{plan}");
    assert!(!plan.contains("WindowAgg "), "{plan}");
    assert!(plan.contains("WindowAggExec"), "{plan}");

    let rows = query.fetch_result::<(i32, f64, i64, f64, f64, f64, f64)>(&mut conn)?;
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows.iter().map(|r| (r.0, r.1)).collect::<Vec<_>>(),
        vec![(999, 499.75), (997, 499.25), (995, 498.75)]
    );
    for (_, _, total_count, total_price, avg_price, min_price, max_price) in &rows {
        assert_eq!(*total_count, 1000);
        assert_eq!(*total_price, 250_000.0);
        assert_eq!(*avg_price, 250.0);
        assert_eq!(*min_price, 0.25);
        assert_eq!(*max_price, 499.75);
    }

    Ok(())
}

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
//! assertions verify the JoinScan absorbs the window aggregates into its Top-K
//! aggregate node; the value assertions must hold no matter which plan
//! executes. The pg_regress twins are Tests 27b/27c in
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
    USING paradedb (id, (description::pdb.unicode_words));
    CREATE INDEX wj_reviews_bm25 ON wj_reviews
    USING paradedb (id, product_id, score);
    ANALYZE wj_products;
    ANALYZE wj_reviews;
    "#
    .execute(conn);
}

fn explain(conn: &mut PgConnection, query: &str) -> String {
    let lines: Vec<String> = format!("EXPLAIN (COSTS OFF, VERBOSE) {query}").fetch_scalar(conn);
    lines.join("\n")
}

/// The JoinScan computes the window aggregates in its Top-K aggregate node,
/// beside `topk_as_agg`; no window operator exists anywhere in the plan.
fn assert_windows_in_topk_agg(plan: &str) {
    assert!(plan.contains(JOIN_SCAN), "{plan}");
    assert!(!plan.contains("WindowAgg "), "{plan}");
    assert!(!plan.contains("WindowAggExec"), "{plan}");
    let aggregate = plan
        .lines()
        .find(|line| line.contains("AggregateExec"))
        .unwrap_or_else(|| panic!("no AggregateExec in plan:\n{plan}"));
    assert!(
        aggregate.contains("topk_as_agg(") && aggregate.contains("as window_agg_"),
        "{aggregate}"
    );
}

#[derive(Debug, Clone, Copy)]
enum WindowJoinCase {
    /// Every SQL-native aggregate that window_func.rs can convert, as one
    /// bare column each (one WindowFunc per target entry).
    Bare,
    /// Window aggregates embedded in target list expressions: a
    /// constant-arithmetic wrapper (native DataFusion path), a function+cast
    /// wrapper (PgExprUdf path with a window input), a source column mixed
    /// with a window value in one expression, and two window functions in a
    /// single entry.
    InExpressions,
}

#[rstest]
#[case::bare(WindowJoinCase::Bare)]
#[case::in_expressions(WindowJoinCase::InExpressions)]
fn global_window_aggregates_over_join(
    mut conn: PgConnection,
    #[case] case: WindowJoinCase,
) -> Result<(), sqlx::Error> {
    setup(&mut conn);

    match case {
        WindowJoinCase::Bare => {
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
                WHERE p.description ||| 'laptop'
                ORDER BY r.score DESC
                LIMIT 3
            "#;

            // The custom scans absorb the global window aggregates (#5637),
            // so the JoinScan engages and no WindowAgg node remains.
            assert_windows_in_topk_agg(&explain(&mut conn, query));

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
        }
        WindowJoinCase::InExpressions => {
            let query = r#"
                SELECT p.id,
                       r.score,
                       COUNT(*) OVER () + 1 AS count_plus_one,
                       r.score + COUNT(*) OVER () AS score_plus_count,
                       round(AVG(r.score) OVER (), 2)::float8 AS avg_rounded,
                       COUNT(*) OVER () + SUM(r.score) OVER () AS count_plus_sum
                FROM wj_products p
                JOIN wj_reviews r ON p.id = r.product_id
                WHERE p.description ||| 'laptop'
                ORDER BY r.score DESC
                LIMIT 3
            "#;

            assert_windows_in_topk_agg(&explain(&mut conn, query));

            let rows = query.fetch_result::<(i32, i32, i64, i64, f64, i64)>(&mut conn)?;
            assert_eq!(rows.len(), 3);
            assert_eq!(
                rows.iter().map(|r| (r.0, r.1)).collect::<Vec<_>>(),
                vec![(999, 1999), (997, 1997), (995, 1995)]
            );
            for (_, score, count_plus_one, score_plus_count, avg_rounded, count_plus_sum) in &rows {
                assert_eq!(*count_plus_one, 1001);
                assert_eq!(*score_plus_count, (*score as i64) + 1000);
                assert_eq!(*avg_rounded, 1000.0);
                assert_eq!(*count_plus_sum, 1_001_000);
            }
        }
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
    USING paradedb (id, (description::pdb.unicode_words));
    CREATE INDEX wjn_reviews_bm25 ON wjn_reviews
    USING paradedb (id, product_id, price);
    ANALYZE wjn_products;
    ANALYZE wjn_reviews;
    "#
    .execute(conn);
}

#[derive(Debug, Clone, Copy)]
enum NumericWindowJoinCase {
    /// The float8 casts wrap the WindowFuncs, so every aggregate exercises
    /// the expression path: sentinel rewrite, PgExprUdf evaluation, and the
    /// storage-encoded input decode (scaled i64 for SUM/MIN/MAX, the
    /// count+sum blob for AVG) — while keeping the value assertions exact
    /// f64 comparisons.
    Wrapped,
    /// An expression whose result type is NUMERIC is not Arrow-convertible,
    /// so the entry cannot be evaluated by the scan: JoinScan must decline
    /// (falling back to PostgreSQL's WindowAgg) rather than erroring at
    /// execution.
    NumericResultDeclines,
}

#[rstest]
#[case::wrapped(NumericWindowJoinCase::Wrapped)]
#[case::numeric_result_declines(NumericWindowJoinCase::NumericResultDeclines)]
fn global_window_aggregates_over_join_numeric(
    mut conn: PgConnection,
    #[case] case: NumericWindowJoinCase,
) -> Result<(), sqlx::Error> {
    setup_numeric(&mut conn);

    match case {
        NumericWindowJoinCase::Wrapped => {
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
                WHERE p.description ||| 'laptop'
                ORDER BY r.id DESC
                LIMIT 3
            "#;

            assert_windows_in_topk_agg(&explain(&mut conn, query));

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
        }
        NumericWindowJoinCase::NumericResultDeclines => {
            let query = r#"
                SELECT p.id, SUM(r.price) OVER () * 2 AS doubled_total
                FROM wjn_products p
                JOIN wjn_reviews r ON p.id = r.product_id
                WHERE p.description ||| 'laptop'
                ORDER BY r.id DESC
                LIMIT 3
            "#;

            let plan = explain(&mut conn, query);
            assert!(!plan.contains(JOIN_SCAN), "{plan}");
            assert!(plan.contains("WindowAgg"), "{plan}");

            let rows = query.fetch_result::<(i32, bigdecimal::BigDecimal)>(&mut conn)?;
            assert_eq!(rows.len(), 3);
            assert_eq!(
                rows.iter().map(|r| r.0).collect::<Vec<_>>(),
                vec![999, 997, 995]
            );
            let expected: bigdecimal::BigDecimal = "500000.00".parse().unwrap();
            for (_, doubled_total) in &rows {
                assert_eq!(*doubled_total, expected);
            }
        }
    }

    Ok(())
}

// PostgreSQL converts a LEFT JOIN whose inner side is filtered with IS NULL
// on a strictly-joined column into an Anti Join, pruning the inner relation
// from the join output: its columns are identically NULL in every surviving
// row. Window aggregates over such pruned arguments are plan-time constants
// (0 for COUNT(col), NULL otherwise), mirroring the
// `resolve_var_or_pruned_null` semantics every other pruned-column consumer
// applies — while COUNT(*) still counts the surviving rows for real.
fn setup_anti_join(conn: &mut PgConnection) {
    r#"
    SET paradedb.enable_custom_scan = on;
    SET paradedb.enable_join_custom_scan = on;
    SET paradedb.enable_aggregate_custom_scan = on;
    SET max_parallel_workers_per_gather = 0;

    DROP TABLE IF EXISTS wja_products;
    DROP TABLE IF EXISTS wja_orders;
    CREATE TABLE wja_products (id bigint PRIMARY KEY, age int, description text);
    CREATE TABLE wja_orders (id bigint PRIMARY KEY, age int, price numeric(10, 2));

    INSERT INTO wja_products SELECT g, g, 'sturdy laptop' FROM generate_series(1, 5) g;
    -- Ages 1 and 2 match, anti-filtering products 1 and 2; 3..5 survive.
    INSERT INTO wja_orders VALUES (1, 1, 11.50), (2, 2, 22.50);

    CREATE INDEX wja_products_bm25 ON wja_products
    USING paradedb (id, age, (description::pdb.unicode_words));
    CREATE INDEX wja_orders_bm25 ON wja_orders
    USING paradedb (id, age, price);
    ANALYZE wja_products;
    ANALYZE wja_orders;
    "#
    .execute(conn);
}

#[rstest]
fn global_window_aggregates_over_pruned_anti_join(
    mut conn: PgConnection,
) -> Result<(), sqlx::Error> {
    setup_anti_join(&mut conn);

    let query = r#"
        SELECT p.id,
               (AVG(o.price) OVER ())::float8 AS avg_price,
               COUNT(o.age) OVER () AS matched_count,
               COUNT(*) OVER () AS total_count,
               SUM(o.price) OVER () AS total_price
        FROM wja_products p
        LEFT JOIN wja_orders o ON p.age = o.age
        WHERE p.description ||| 'laptop' AND o.age IS NULL
        ORDER BY p.id
        LIMIT 10
    "#;

    // COUNT(*) is a real aggregate over the surviving rows; the pruned-argument
    // aggregates are plan-time constants that never reach the aggregate node.
    assert_windows_in_topk_agg(&explain(&mut conn, query));

    let rows = query
        .fetch_result::<(i64, Option<f64>, i64, i64, Option<bigdecimal::BigDecimal>)>(&mut conn)?;
    assert_eq!(rows.len(), 3);
    assert_eq!(rows.iter().map(|r| r.0).collect::<Vec<_>>(), vec![3, 4, 5]);
    for (_, avg_price, matched_count, total_count, total_price) in &rows {
        assert_eq!(*avg_price, None);
        assert_eq!(*matched_count, 0);
        assert_eq!(*total_count, 3);
        assert_eq!(*total_price, None);
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum WindowShape {
    /// A LIMIT with no ORDER BY: the Top-K aggregate keeps any three rows,
    /// and the window aggregate still counts the whole join.
    NoOrderBy,
    /// The window aggregates run inside the Top-K aggregate, which needs k at
    /// planning time; a parameterized LIMIT declines and PostgreSQL computes
    /// the query.
    ParameterizedLimitDeclines,
}

#[rstest]
#[case::no_order_by(WindowShape::NoOrderBy)]
#[case::parameterized_limit_declines(WindowShape::ParameterizedLimitDeclines)]
fn global_window_aggregates_query_shapes(
    mut conn: PgConnection,
    #[case] shape: WindowShape,
) -> Result<(), sqlx::Error> {
    setup(&mut conn);

    match shape {
        WindowShape::NoOrderBy => {
            let query = r#"
                SELECT p.id, COUNT(*) OVER () AS total_count
                FROM wj_products p
                JOIN wj_reviews r ON p.id = r.product_id
                WHERE p.description ||| 'laptop'
                LIMIT 3
            "#;

            assert_windows_in_topk_agg(&explain(&mut conn, query));

            let rows = query.fetch_result::<(i32, i64)>(&mut conn)?;
            assert_eq!(rows.len(), 3);
            assert!(
                rows.iter().all(|(id, total)| id % 2 == 1 && *total == 1000),
                "{rows:?}"
            );
        }
        WindowShape::ParameterizedLimitDeclines => {
            r#"
            SET plan_cache_mode = force_generic_plan;
            PREPARE wj_page AS
                SELECT p.id, COUNT(*) OVER () AS total_count
                FROM wj_products p
                JOIN wj_reviews r ON p.id = r.product_id
                WHERE p.description ||| 'laptop'
                ORDER BY r.score DESC
                LIMIT $1;
            "#
            .execute(&mut conn);

            let plan = explain(&mut conn, "EXECUTE wj_page(3)");
            assert!(!plan.contains(JOIN_SCAN), "{plan}");

            let rows = "EXECUTE wj_page(3)".fetch_result::<(i32, i64)>(&mut conn)?;
            assert_eq!(rows, vec![(999, 1000), (997, 1000), (995, 1000)]);

            "DEALLOCATE wj_page".execute(&mut conn);
        }
    }

    Ok(())
}

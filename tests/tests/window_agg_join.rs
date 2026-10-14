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
    CREATE TABLE wja_orders (id bigint PRIMARY KEY, age int, price numeric(10, 2), tags text[]);

    INSERT INTO wja_products SELECT g, g, 'sturdy laptop' FROM generate_series(1, 5) g;
    -- Ages 1 and 2 match, anti-filtering products 1 and 2; 3..5 survive.
    INSERT INTO wja_orders VALUES (1, 1, 11.50, ARRAY['bulk']), (2, 2, 22.50, ARRAY['gift', 'rush']);

    CREATE INDEX wja_products_bm25 ON wja_products
    USING paradedb (id, age, (description::pdb.unicode_words));
    CREATE INDEX wja_orders_bm25 ON wja_orders
    USING paradedb (id, age, price, (tags::pdb.literal));
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

/// A LATERAL unnest of the pruned side yields no rows, so the optimizer folds the
/// join to an empty relation under the Top-K aggregate. The serialized plan loses
/// that relation's schema; the aggregate has to empty out with it instead of
/// resolving its columns against the empty schema at execution.
#[rstest]
fn global_window_aggregates_over_pruned_anti_join_unnest(
    mut conn: PgConnection,
) -> Result<(), sqlx::Error> {
    setup_anti_join(&mut conn);

    let query = r#"
        SELECT p.id, tag, COUNT(*) OVER () AS total_count
        FROM wja_products p
        LEFT JOIN wja_orders o ON p.age = o.age
        CROSS JOIN LATERAL unnest(o.tags) AS tag
        WHERE p.description ||| 'laptop' AND o.age IS NULL
        ORDER BY p.id, tag
        LIMIT 10
    "#;

    let plan = explain(&mut conn, query);
    assert!(plan.contains(JOIN_SCAN), "{plan}");
    let rows = query.fetch_result::<(i64, String, i64)>(&mut conn)?;
    assert!(rows.is_empty(), "{rows:?}");

    // The same shape with no window, forced onto the Top-K path.
    "SET paradedb.joinscan_force_topk_as_agg = on".execute(&mut conn);
    let query = r#"
        SELECT p.id, tag
        FROM wja_products p
        LEFT JOIN wja_orders o ON p.age = o.age
        CROSS JOIN LATERAL unnest(o.tags) AS tag
        WHERE p.description ||| 'laptop' AND o.age IS NULL
        ORDER BY p.id, tag
        LIMIT 10
    "#;

    let plan = explain(&mut conn, query);
    assert!(plan.contains(JOIN_SCAN), "{plan}");
    let rows = query.fetch_result::<(i64, String)>(&mut conn)?;
    assert!(rows.is_empty(), "{rows:?}");

    Ok(())
}

// `pdb.agg() OVER ()` over a join. products carry a category (three values and
// NULL) indexed under its own name and under an alias, a NUMERIC price, a
// timestamp, an array (NULL or empty for some) and a JSON document; reviews a
// reviewer, a boolean and an integer score. The aggregate scan computes the same document over the same
// join without the LIMIT, so it is the oracle.
fn setup_pdb_agg(conn: &mut PgConnection) {
    r#"
    SET paradedb.enable_custom_scan = on;
    SET paradedb.enable_join_custom_scan = on;
    SET paradedb.enable_aggregate_custom_scan = on;
    SET max_parallel_workers_per_gather = 0;

    DROP TABLE IF EXISTS wjp_products;
    DROP TABLE IF EXISTS wjp_reviews;
    CREATE TABLE wjp_products (
        id int PRIMARY KEY,
        description text,
        category text,
        price numeric(10, 2),
        created_at timestamp,
        tags text[],
        metadata jsonb
    );
    CREATE TABLE wjp_reviews (
        id bigint PRIMARY KEY,
        product_id bigint,
        score int,
        reviewer text,
        verified boolean
    );

    INSERT INTO wjp_products
    SELECT g,
           CASE WHEN g % 2 = 1 THEN 'sturdy laptop' ELSE 'flimsy tablet' END,
           (ARRAY['office', 'gaming', 'travel', NULL])[1 + (g % 4)],
           (g * 0.25)::numeric(10, 2),
           '2026-01-01'::timestamp + (g % 3) * interval '1 day',
           CASE g % 7
               WHEN 0 THEN NULL
               WHEN 1 THEN ARRAY[]::text[]
               ELSE ARRAY['tag_' || (g % 2), 'tag_' || (g % 3)]
           END,
           jsonb_build_object('color', (ARRAY['red', 'blue', 'green'])[1 + (g % 3)])
    FROM generate_series(1, 1000) g;
    INSERT INTO wjp_reviews
    SELECT g, ((g - 1) % 1000) + 1, g % 5, 'reviewer_' || (g % 7), g % 3 = 0
    FROM generate_series(1, 2000) g;

    CREATE INDEX wjp_products_bm25 ON wjp_products
    USING paradedb (
        id,
        (description::pdb.unicode_words),
        (category::pdb.literal),
        (category::pdb.literal('alias=category_exact')),
        price,
        created_at,
        (tags::pdb.literal),
        (metadata::pdb.literal)
    );
    CREATE INDEX wjp_reviews_bm25 ON wjp_reviews
    USING paradedb (id, product_id, score, (reviewer::pdb.literal), verified);
    ANALYZE wjp_products;
    ANALYZE wjp_reviews;
    "#
    .execute(conn);
}

const PDB_AGG_JOIN: &str = r#"
    FROM wjp_products p
    JOIN wjp_reviews r ON p.id = r.product_id
    WHERE p.description ||| 'laptop'
"#;

/// The document the aggregate scan computes for `spec` over the join.
fn aggregate_scan_document(conn: &mut PgConnection, spec: &str) -> serde_json::Value {
    let (document,) =
        format!("SELECT pdb.agg('{spec}') {PDB_AGG_JOIN}").fetch_one::<(serde_json::Value,)>(conn);
    document
}

/// One `pdb_agg(` call per distinct spec, in the JoinScan's Top-K aggregate
/// node beside `topk_as_agg`.
fn assert_pdb_aggs_in_topk_agg(plan: &str, distinct_specs: usize) {
    assert!(plan.contains(JOIN_SCAN), "{plan}");
    assert!(!plan.contains("WindowAgg "), "{plan}");
    let aggregate = plan
        .lines()
        .find(|line| line.contains("AggregateExec"))
        .unwrap_or_else(|| panic!("no AggregateExec in plan:\n{plan}"));
    assert!(
        aggregate.contains("topk_as_agg(") && aggregate.contains("as window_agg_"),
        "{aggregate}"
    );
    assert_eq!(plan.matches("pdb_agg(").count(), distinct_specs, "{plan}");
}

#[rstest]
#[case::avg(r#"{"avg": {"field": "score"}}"#)]
#[case::avg_with_missing(r#"{"avg": {"field": "score", "missing": 10}}"#)]
#[case::numeric_sum(r#"{"sum": {"field": "price"}}"#)]
#[case::datetime_max(r#"{"max": {"field": "created_at"}}"#)]
#[case::cardinality(r#"{"cardinality": {"field": "reviewer"}}"#)]
#[case::terms_with_null_bucket(r#"{"terms": {"field": "category"}}"#)]
#[case::terms_with_missing(r#"{"terms": {"field": "category", "missing": "none"}}"#)]
#[case::terms_on_bool(r#"{"terms": {"field": "verified"}}"#)]
#[case::terms_on_datetime(r#"{"terms": {"field": "created_at"}}"#)]
#[case::terms_on_numeric(r#"{"terms": {"field": "price", "size": 3}}"#)]
#[case::terms_on_aliased_field(r#"{"terms": {"field": "category_exact"}}"#)]
#[case::terms_on_json_sub_field(r#"{"terms": {"field": "metadata.color"}}"#)]
#[case::terms_ordered_by_metric(
    r#"{"terms": {"field": "reviewer", "size": 3, "order": {"top": "desc"}},
        "aggs": {"top": {"max": {"field": "price"}}}}"#
)]
#[case::nested_terms(
    r#"{"terms": {"field": "category"},
        "aggs": {
            "avg_score": {"avg": {"field": "score"}},
            "reviewers": {
                "terms": {"field": "reviewer", "size": 2},
                "aggs": {
                    "scores": {"cardinality": {"field": "score"}},
                    "revenue": {"sum": {"field": "price"}}
                }
            }
        }}"#
)]
// An array key: a row is in the bucket of each of its elements, and a NULL or
// empty array puts it in the NULL bucket, or the `missing` one.
#[case::terms_on_array(r#"{"terms": {"field": "tags"}}"#)]
#[case::terms_on_array_with_missing(r#"{"terms": {"field": "tags", "missing": "untagged"}}"#)]
// The root and the scalar level see each row once while the array level sees
// it per element, in both nestings.
#[case::array_terms_under_scalar_terms(
    r#"{"terms": {"field": "category"},
        "aggs": {
            "tags": {
                "terms": {"field": "tags"},
                "aggs": {"reviewers": {"cardinality": {"field": "reviewer"}}}
            }
        }}"#
)]
#[case::scalar_terms_under_array_terms(
    r#"{"terms": {"field": "tags"},
        "aggs": {
            "avg_score": {"avg": {"field": "score"}},
            "categories": {"terms": {"field": "category"}}
        }}"#
)]
// A key repeats across sibling nodes rather than on one path: both read the
// same level.
#[case::sibling_terms_on_one_field(
    r#"{"terms": {"field": "category"},
        "aggs": {
            "top_reviewers": {"terms": {"field": "reviewer", "size": 2}},
            "all_reviewers": {"terms": {"field": "reviewer", "order": {"_key": "asc"}}}
        }}"#
)]
// The same field on one path under another `missing` is another key, so each
// outer bucket holds one inner bucket and the NULL rows take both literals.
#[case::repeated_field_with_different_missing(
    r#"{"terms": {"field": "category", "missing": "outer"},
        "aggs": {"again": {"terms": {"field": "category", "missing": "inner"}}}}"#
)]
fn pdb_agg_window_matches_aggregate_scan(mut conn: PgConnection, #[case] spec: &str) {
    setup_pdb_agg(&mut conn);

    let query =
        format!("SELECT r.id, pdb.agg('{spec}') OVER () {PDB_AGG_JOIN} ORDER BY r.id DESC LIMIT 3");
    assert_pdb_aggs_in_topk_agg(&explain(&mut conn, &query), 1);

    let expected = aggregate_scan_document(&mut conn, spec);
    let rows: Vec<(i64, serde_json::Value)> = query.fetch(&mut conn);
    assert_eq!(
        rows,
        vec![
            (1999, expected.clone()),
            (1997, expected.clone()),
            (1995, expected)
        ]
    );
}

#[derive(Debug, Clone, Copy)]
enum PdbAggShape {
    /// Several entries in one target list: each spec is one aggregate, a spec
    /// written twice is computed once, and a SQL window aggregate sits beside
    /// them in the target list.
    SeveralEntries,
    /// The document as an input of a target list expression.
    InExpressions,
    /// The aggregate covers the whole join, not the rows the OFFSET and LIMIT
    /// keep.
    Offset,
    /// No row survives the join, so none carries a document.
    NoRows,
    /// DISTINCT with the document inside an expression. Its value is the same
    /// on every row, so the entry stays out of the DISTINCT key and is computed
    /// after the Top-K aggregate. Each product has two reviews, so DISTINCT
    /// halves the join while the aggregate still covers all of it.
    Distinct,
    /// A DISTINCT target list of document expressions alone: every row is one
    /// group.
    DistinctDocumentsOnly,
}

#[rstest]
#[case::several_entries(PdbAggShape::SeveralEntries)]
#[case::in_expressions(PdbAggShape::InExpressions)]
#[case::offset(PdbAggShape::Offset)]
#[case::no_rows(PdbAggShape::NoRows)]
#[case::distinct(PdbAggShape::Distinct)]
#[case::distinct_documents_only(PdbAggShape::DistinctDocumentsOnly)]
fn pdb_agg_window_query_shapes(mut conn: PgConnection, #[case] shape: PdbAggShape) {
    setup_pdb_agg(&mut conn);

    const AVG: &str = r#"{"avg": {"field": "score"}}"#;
    const TERMS: &str = r#"{"terms": {"field": "category"}}"#;

    match shape {
        PdbAggShape::SeveralEntries => {
            let query = format!(
                r#"
                SELECT r.id,
                       pdb.agg('{AVG}') OVER () AS avg_score,
                       pdb.agg('{TERMS}') OVER () AS categories,
                       pdb.agg('{AVG}') OVER () AS avg_score_again,
                       COUNT(*) OVER () AS total_count
                {PDB_AGG_JOIN}
                ORDER BY r.id DESC
                LIMIT 2
                "#
            );
            assert_pdb_aggs_in_topk_agg(&explain(&mut conn, &query), 2);

            let avg = aggregate_scan_document(&mut conn, AVG);
            let terms = aggregate_scan_document(&mut conn, TERMS);
            type Row = (
                i64,
                serde_json::Value,
                serde_json::Value,
                serde_json::Value,
                i64,
            );
            let rows: Vec<Row> = query.fetch(&mut conn);
            assert_eq!(
                rows,
                vec![
                    (1999, avg.clone(), terms.clone(), avg.clone(), 1000),
                    (1997, avg.clone(), terms, avg, 1000)
                ]
            );
        }
        PdbAggShape::InExpressions => {
            let query = format!(
                r#"
                SELECT r.id,
                       pdb.agg('{AVG}') OVER () ->> 'value' AS avg_text,
                       (pdb.agg('{AVG}') OVER () -> 'value')::float8 + r.score AS avg_plus_score,
                       jsonb_array_length(pdb.agg('{TERMS}') OVER () -> 'buckets') AS buckets
                {PDB_AGG_JOIN}
                ORDER BY r.id DESC
                LIMIT 2
                "#
            );
            assert_pdb_aggs_in_topk_agg(&explain(&mut conn, &query), 2);

            // Matched reviews have the odd ids, whose scores cycle 1, 3, 0, 2, 4.
            let rows: Vec<(i64, String, f64, i32)> = query.fetch(&mut conn);
            assert_eq!(
                rows,
                vec![
                    (1999, "2.0".to_string(), 6.0, 2),
                    (1997, "2.0".to_string(), 4.0, 2)
                ]
            );
        }
        PdbAggShape::Offset => {
            let spec = r#"{"value_count": {"field": "score"}}"#;
            let query = format!(
                "SELECT r.id, pdb.agg('{spec}') OVER () {PDB_AGG_JOIN}
                 ORDER BY r.id DESC OFFSET 998 LIMIT 5"
            );
            assert_pdb_aggs_in_topk_agg(&explain(&mut conn, &query), 1);

            let expected = serde_json::json!({"value": 1000.0});
            let rows: Vec<(i64, serde_json::Value)> = query.fetch(&mut conn);
            assert_eq!(rows, vec![(3, expected.clone()), (1, expected)]);
        }
        PdbAggShape::NoRows => {
            let query = format!(
                r#"
                SELECT r.id, pdb.agg('{TERMS}') OVER ()
                FROM wjp_products p
                JOIN wjp_reviews r ON p.id = r.product_id
                WHERE p.description ||| 'typewriter'
                ORDER BY r.id DESC
                LIMIT 2
                "#
            );
            assert_pdb_aggs_in_topk_agg(&explain(&mut conn, &query), 1);

            let rows: Vec<(i64, serde_json::Value)> = query.fetch(&mut conn);
            assert_eq!(rows, vec![]);
        }
        PdbAggShape::Distinct => {
            let spec = r#"{"value_count": {"field": "score"}}"#;
            let query = format!(
                "SELECT DISTINCT p.id, pdb.agg('{spec}') OVER () ->> 'value' AS reviews
                 {PDB_AGG_JOIN}
                 ORDER BY p.id DESC LIMIT 2"
            );
            assert_pdb_aggs_in_topk_agg(&explain(&mut conn, &query), 1);

            let rows: Vec<(i32, String)> = query.fetch(&mut conn);
            assert_eq!(
                rows,
                vec![(999, "1000.0".to_string()), (997, "1000.0".to_string())]
            );
        }
        PdbAggShape::DistinctDocumentsOnly => {
            let spec = r#"{"value_count": {"field": "score"}}"#;
            let query = format!(
                "SELECT DISTINCT pdb.agg('{spec}') OVER () ->> 'value' AS reviews
                 {PDB_AGG_JOIN}
                 LIMIT 3"
            );
            assert_pdb_aggs_in_topk_agg(&explain(&mut conn, &query), 1);

            let rows: Vec<(String,)> = query.fetch(&mut conn);
            assert_eq!(rows, vec![("1000.0".to_string(),)]);
        }
    }
}

/// What the aggregate scan turns down in a spec, JoinScan turns down too, along
/// with a visibility it cannot honor. Nothing else computes a `pdb.agg()`, so
/// the query fails.
#[rstest]
#[case::unsupported_aggregation(r#"pdb.agg('{"histogram": {"field": "score", "interval": 2}}')"#)]
#[case::terms_min_doc_count_zero(r#"pdb.agg('{"terms": {"field": "score", "min_doc_count": 0}}')"#)]
#[case::terms_field_repeats_on_path(
    r#"pdb.agg('{"terms": {"field": "category"}, "aggs": {"x": {"terms": {"field": "category"}}}}')"#
)]
#[case::terms_field_repeats_below_another(
    r#"pdb.agg('{"terms": {"field": "category"},
                "aggs": {"x": {"terms": {"field": "reviewer"},
                               "aggs": {"y": {"terms": {"field": "category"}}}}}}')"#
)]
#[case::terms_field_repeats_qualified(
    r#"pdb.agg('{"terms": {"field": "category"}, "aggs": {"x": {"terms": {"field": "p.category"}}}}')"#
)]
#[case::ambiguous_field(r#"pdb.agg('{"max": {"field": "id"}}')"#)]
#[case::visibility(r#"pdb.agg('{"avg": {"field": "score"}}', 'raw')"#)]
fn pdb_agg_window_declines(mut conn: PgConnection, #[case] call: &str) {
    setup_pdb_agg(&mut conn);

    let query = format!("SELECT r.id, {call} OVER () {PDB_AGG_JOIN} ORDER BY r.id DESC LIMIT 3");
    let error = query
        .fetch_result::<(i64, serde_json::Value)>(&mut conn)
        .expect_err("the query must not run");
    assert!(
        error
            .to_string()
            .contains("pdb.agg() must be handled by ParadeDB's custom scan"),
        "{error}"
    );
}

#[derive(Debug, Clone, Copy)]
enum WindowShape {
    /// A LIMIT with no ORDER BY: the Top-K aggregate keeps any three rows,
    /// and the window aggregate still counts the whole join.
    NoOrderBy,
    /// DISTINCT with window aggregates, bare and inside expressions. Their
    /// value is the same on every row, so they stay out of the DISTINCT key
    /// and are computed after the Top-K aggregate; PostgreSQL evaluates
    /// windows before DISTINCT, so the count covers the join, not the
    /// distinct rows.
    Distinct,
    /// A target list of window aggregates alone: every row is one group.
    DistinctWindowsOnly,
    /// A parameterized LIMIT under a generic plan: the Top-K aggregate's k is
    /// bound at execution, so the window aggregates are still computed in it.
    ParameterizedLimit,
}

#[rstest]
#[case::no_order_by(WindowShape::NoOrderBy)]
#[case::distinct(WindowShape::Distinct)]
#[case::distinct_windows_only(WindowShape::DistinctWindowsOnly)]
#[case::parameterized_limit(WindowShape::ParameterizedLimit)]
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
        WindowShape::Distinct => {
            // Each laptop product has two reviews, so DISTINCT halves the join.
            let query = r#"
                SELECT DISTINCT p.id,
                       COUNT(*) OVER () AS total_count,
                       (COUNT(*) OVER ())::float8 AS total_count_f8,
                       COUNT(*) OVER () + 1 AS total_plus_one
                FROM wj_products p
                JOIN wj_reviews r ON p.id = r.product_id
                WHERE p.description ||| 'laptop'
                ORDER BY p.id DESC
                LIMIT 3
            "#;

            let plan = explain(&mut conn, query);
            assert_windows_in_topk_agg(&plan);
            assert!(plan.contains("topk_as_agg(DISTINCT "), "{plan}");

            let rows = query.fetch_result::<(i32, i64, f64, i64)>(&mut conn)?;
            assert_eq!(
                rows,
                vec![
                    (999, 1000, 1000.0, 1001),
                    (997, 1000, 1000.0, 1001),
                    (995, 1000, 1000.0, 1001)
                ]
            );
        }
        WindowShape::DistinctWindowsOnly => {
            let query = r#"
                SELECT DISTINCT COUNT(*) OVER () AS total_count
                FROM wj_products p
                JOIN wj_reviews r ON p.id = r.product_id
                WHERE p.description ||| 'laptop'
                LIMIT 3
            "#;

            let plan = explain(&mut conn, query);
            assert_windows_in_topk_agg(&plan);
            assert!(plan.contains("topk_as_agg(DISTINCT "), "{plan}");

            let rows = query.fetch_result::<(i64,)>(&mut conn)?;
            assert_eq!(rows, vec![(1000,)]);
        }
        WindowShape::ParameterizedLimit => {
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
            assert_windows_in_topk_agg(&plan);

            let rows = "EXECUTE wj_page(3)".fetch_result::<(i32, i64)>(&mut conn)?;
            assert_eq!(rows, vec![(999, 1000), (997, 1000), (995, 1000)]);

            "DEALLOCATE wj_page".execute(&mut conn);
        }
    }

    Ok(())
}

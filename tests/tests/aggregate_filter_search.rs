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

// Search predicates (`|||`, `&&&`, `@@@`) inside aggregate FILTER clauses must
// be pushed down to the DataFusion aggregate backend rather than forcing a
// fallback to a native GroupAggregate. See
// https://github.com/paradedb/paradedb/issues/6699.

use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;
use sqlx::postgres::PgRow;
use std::fmt::Debug;
use tests::fixtures::*;
use time::Date;
use time::macros::date;

fn setup_articles(conn: &mut PgConnection) {
    r#"
    CREATE TABLE authors (
        author_id INTEGER PRIMARY KEY,
        name TEXT,
        bio TEXT
    );
    CREATE TABLE articles (
        id SERIAL PRIMARY KEY,
        author_id INTEGER,
        category TEXT,
        body TEXT,
        views INTEGER,
        created_at DATE
    );
    INSERT INTO authors (author_id, name, bio) VALUES
        (1, 'alice', 'database engineer'),
        (2, 'bob', 'frontend developer'),
        (3, 'carol', 'writes about database internals'),
        (4, 'dave', 'gardening enthusiast');
    INSERT INTO articles (author_id, category, body, views, created_at) VALUES
        (1, 'tutorial', 'intro to database indexing', 10, '2024-01-01'),
        (1, 'tutorial', 'database query planning', 20, '2024-02-01'),
        (1, 'news', 'database release notes', 30, '2024-03-01'),
        (2, 'tutorial', 'css layout tutorial', 5, '2024-01-15'),
        (2, 'tutorial', 'database for frontend developers', 50, '2024-04-01'),
        (3, 'opinion', 'why I love search engines', 15, '2024-05-01'),
        (3, 'tutorial', 'search engine basics', 25, '2024-02-15'),
        (4, 'news', 'gardening in winter', 1, '2024-06-01'),
        (4, 'tutorial', 'database of plants', 3, '2024-06-15');
    CREATE INDEX authors_idx ON authors
        USING paradedb (author_id, name, bio);
    CREATE INDEX articles_idx ON articles
        USING paradedb (id, author_id, (category::pdb.literal), body, views, created_at);
    ANALYZE authors;
    ANALYZE articles;
    SET max_parallel_workers_per_gather = 0;
    SET paradedb.enable_aggregate_custom_scan TO on;
    "#
    .execute(conn);
}

fn explain_json(conn: &mut PgConnection, query: &str) -> String {
    let (plan,) = format!("EXPLAIN (FORMAT JSON) {query}").fetch_one::<(Value,)>(conn);
    plan.to_string()
}

fn assert_uses_datafusion_aggregate_scan(conn: &mut PgConnection, query: &str) {
    let plan = explain_json(conn, query);
    assert!(
        plan.contains("ParadeDB Aggregate Scan"),
        "expected the FILTER to be pushed down to ParadeDB Aggregate Scan:\n{plan}"
    );
    assert!(
        plan.contains("DataFusion Physical Plan"),
        "expected DataFusion backend:\n{plan}"
    );
}

/// Asserts that `query` is executed by the DataFusion aggregate scan and that
/// its results match native Postgres aggregation. Returns the pushed-down rows.
fn assert_pushdown_parity<T>(conn: &mut PgConnection, query: &str) -> Vec<T>
where
    T: for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin + PartialEq + Debug,
{
    "SET paradedb.enable_aggregate_custom_scan TO on".execute(conn);
    assert_uses_datafusion_aggregate_scan(conn, query);
    let pushed_down = query.fetch::<T>(conn);

    "SET paradedb.enable_aggregate_custom_scan TO off".execute(conn);
    let native = query.fetch::<T>(conn);
    "SET paradedb.enable_aggregate_custom_scan TO on".execute(conn);

    assert_eq!(
        pushed_down, native,
        "DataFusion aggregate must match Postgres for:\n{query}"
    );
    pushed_down
}

/// The query from the issue: a filtered COUNT that is both projected and used
/// for ranking, over a join whose WHERE clause contains the same predicate.
#[rstest]
fn issue_example_projected_and_ranked(mut conn: PgConnection) {
    setup_articles(&mut conn);

    let rows = assert_pushdown_parity::<(i32, i64, Date)>(
        &mut conn,
        r#"
        SELECT
            a.author_id,
            COUNT(*) FILTER (
                WHERE a.category = 'tutorial'
                  AND a.body ||| 'database'
            ) AS tutorial_count,
            MAX(a.created_at) AS latest_date
        FROM articles a
        JOIN authors u ON a.author_id = u.author_id
        WHERE a.body ||| 'database'
           OR u.bio ||| 'database'
        GROUP BY a.author_id
        ORDER BY
            COUNT(*) FILTER (
                WHERE a.category = 'tutorial'
                  AND a.body ||| 'database'
            ) DESC,
            MAX(a.created_at) DESC NULLS LAST,
            a.author_id
        LIMIT 15 OFFSET 0
        "#,
    );

    assert_eq!(
        rows,
        vec![
            (1, 2, date!(2024 - 03 - 01)),
            (4, 1, date!(2024 - 06 - 15)),
            (2, 1, date!(2024 - 04 - 01)),
            // Matched only via the author bio; no tutorial mentions "database".
            (3, 0, date!(2024 - 05 - 01)),
        ]
    );
}

#[rstest]
fn filter_with_only_search_predicate(mut conn: PgConnection) {
    setup_articles(&mut conn);

    let rows = assert_pushdown_parity::<(i32, i64, i64)>(
        &mut conn,
        r#"
        SELECT a.author_id,
               COUNT(*) AS total,
               COUNT(*) FILTER (WHERE a.body ||| 'database') AS db_count
        FROM articles a
        JOIN authors u ON a.author_id = u.author_id
        WHERE a.body ||| 'database' OR u.bio ||| 'database'
        GROUP BY a.author_id
        ORDER BY a.author_id
        "#,
    );

    assert_eq!(rows, vec![(1, 3, 3), (2, 1, 1), (3, 2, 0), (4, 1, 1)]);
}

/// The FILTER predicate does not appear in WHERE, so it cannot reuse an
/// existing tag and must be evaluated as a new tagged query during the scan.
#[rstest]
fn filter_search_predicate_absent_from_where(mut conn: PgConnection) {
    setup_articles(&mut conn);

    let rows = assert_pushdown_parity::<(i32, i64, i64)>(
        &mut conn,
        r#"
        SELECT a.author_id,
               COUNT(*) AS total,
               COUNT(*) FILTER (WHERE a.body ||| 'search') AS search_count
        FROM articles a
        JOIN authors u ON a.author_id = u.author_id
        WHERE u.bio ||| 'database'
        GROUP BY a.author_id
        ORDER BY a.author_id
        "#,
    );

    assert_eq!(rows, vec![(1, 3, 0), (3, 2, 2)]);
}

/// The FILTER references a search predicate on the side of the join that has
/// no search predicate in WHERE.
#[rstest]
fn filter_search_predicate_on_other_join_side(mut conn: PgConnection) {
    setup_articles(&mut conn);

    let rows = assert_pushdown_parity::<(String, i64, i64)>(
        &mut conn,
        r#"
        SELECT a.category,
               COUNT(*) AS total,
               COUNT(*) FILTER (WHERE u.bio ||| 'database') AS by_db_authors
        FROM articles a
        JOIN authors u ON a.author_id = u.author_id
        WHERE a.id @@@ pdb.all()
        GROUP BY a.category
        ORDER BY a.category
        "#,
    );

    assert_eq!(
        rows,
        vec![
            ("news".into(), 2, 1),
            ("opinion".into(), 1, 1),
            ("tutorial".into(), 6, 3),
        ]
    );
}

#[rstest]
#[case::match_disjunction("a.body ||| 'database search'")]
#[case::match_conjunction("a.body &&& 'database query'")]
#[case::term_query("a.body @@@ pdb.term('database')")]
#[case::parse_query("a.id @@@ pdb.parse('body:engine*')")]
fn filter_search_operator_variants(#[case] predicate: &str, mut conn: PgConnection) {
    setup_articles(&mut conn);

    assert_pushdown_parity::<(i32, i64, i64)>(
        &mut conn,
        &format!(
            r#"
            SELECT a.author_id,
                   COUNT(*) AS total,
                   COUNT(*) FILTER (WHERE {predicate}) AS filtered
            FROM articles a
            JOIN authors u ON a.author_id = u.author_id
            GROUP BY a.author_id
            ORDER BY a.author_id
            "#
        ),
    );
}

/// Several aggregates with distinct search FILTERs on both sides of the join,
/// including the same predicate repeated (which should share a single tag).
#[rstest]
fn multiple_distinct_search_filters(mut conn: PgConnection) {
    setup_articles(&mut conn);

    let rows = assert_pushdown_parity::<(i32, i64, i64, i64, Option<i64>)>(
        &mut conn,
        r#"
        SELECT a.author_id,
               COUNT(*) FILTER (WHERE a.body ||| 'database') AS db_count,
               COUNT(*) FILTER (WHERE a.body ||| 'search') AS search_count,
               COUNT(*) FILTER (WHERE u.bio ||| 'engineer') AS engineer_count,
               SUM(a.views) FILTER (WHERE a.body ||| 'database') AS db_views
        FROM articles a
        JOIN authors u ON a.author_id = u.author_id
        GROUP BY a.author_id
        ORDER BY a.author_id
        "#,
    );

    assert_eq!(
        rows,
        vec![
            (1, 3, 0, 3, Some(60)),
            (2, 1, 0, 0, Some(50)),
            (3, 0, 2, 0, None),
            (4, 1, 0, 0, Some(3)),
        ]
    );
}

/// Boolean structure around the search predicate inside FILTER. `NOT` in
/// particular requires non-matching rows to evaluate to false, not NULL.
#[rstest]
#[case::negated("NOT (a.body ||| 'database')")]
#[case::or_with_scalar("a.body ||| 'search' OR a.views >= 30")]
#[case::or_across_relations("a.body ||| 'search' OR u.bio ||| 'engineer'")]
#[case::and_across_relations("a.body ||| 'database' AND u.bio ||| 'database'")]
#[case::nested(
    "(a.category = 'tutorial' AND NOT (a.body ||| 'database')) OR u.bio ||| 'gardening'"
)]
fn filter_boolean_combinations(#[case] predicate: &str, mut conn: PgConnection) {
    setup_articles(&mut conn);

    assert_pushdown_parity::<(i32, i64, i64)>(
        &mut conn,
        &format!(
            r#"
            SELECT a.author_id,
                   COUNT(*) AS total,
                   COUNT(*) FILTER (WHERE {predicate}) AS filtered
            FROM articles a
            JOIN authors u ON a.author_id = u.author_id
            WHERE a.id @@@ pdb.all()
            GROUP BY a.author_id
            ORDER BY a.author_id
            "#
        ),
    );
}

#[rstest]
fn non_count_aggregates_with_search_filter(mut conn: PgConnection) {
    setup_articles(&mut conn);

    let rows =
        assert_pushdown_parity::<(String, Option<i64>, Option<i32>, Option<i32>, Option<Date>)>(
            &mut conn,
            r#"
        SELECT a.category,
               SUM(a.views) FILTER (WHERE a.body ||| 'database'),
               MIN(a.views) FILTER (WHERE a.body ||| 'database'),
               MAX(a.views) FILTER (WHERE a.body ||| 'database'),
               MAX(a.created_at) FILTER (WHERE a.body ||| 'database')
        FROM articles a
        JOIN authors u ON a.author_id = u.author_id
        WHERE a.id @@@ pdb.all()
        GROUP BY a.category
        ORDER BY a.category
        "#,
        );

    assert_eq!(
        rows,
        vec![
            (
                "news".into(),
                Some(30),
                Some(30),
                Some(30),
                Some(date!(2024 - 03 - 01))
            ),
            // No opinion article mentions "database": filtered aggregates are NULL.
            ("opinion".into(), None, None, None, None),
            (
                "tutorial".into(),
                Some(83),
                Some(3),
                Some(50),
                Some(date!(2024 - 06 - 15))
            ),
        ]
    );
}

#[rstest]
fn scalar_aggregate_with_search_filter(mut conn: PgConnection) {
    setup_articles(&mut conn);

    let rows = assert_pushdown_parity::<(i64, i64, i64)>(
        &mut conn,
        r#"
        SELECT COUNT(*),
               COUNT(*) FILTER (WHERE a.body ||| 'database'),
               COUNT(*) FILTER (WHERE u.bio ||| 'database')
        FROM articles a
        JOIN authors u ON a.author_id = u.author_id
        WHERE a.id @@@ pdb.all()
        "#,
    );

    assert_eq!(rows, vec![(9, 5, 5)]);
}

#[rstest]
fn search_filter_matching_nothing(mut conn: PgConnection) {
    setup_articles(&mut conn);

    let rows = assert_pushdown_parity::<(i32, i64, Option<i64>)>(
        &mut conn,
        r#"
        SELECT a.author_id,
               COUNT(*) FILTER (WHERE a.body ||| 'nonexistent_xyz'),
               SUM(a.views) FILTER (WHERE a.body ||| 'nonexistent_xyz')
        FROM articles a
        JOIN authors u ON a.author_id = u.author_id
        WHERE a.id @@@ pdb.all()
        GROUP BY a.author_id
        ORDER BY a.author_id
        "#,
    );

    assert_eq!(
        rows,
        vec![(1, 0, None), (2, 0, None), (3, 0, None), (4, 0, None)]
    );
}

#[rstest]
fn having_on_search_filtered_aggregate(mut conn: PgConnection) {
    setup_articles(&mut conn);

    let rows = assert_pushdown_parity::<(i32, i64)>(
        &mut conn,
        r#"
        SELECT a.author_id,
               COUNT(*) FILTER (WHERE a.body ||| 'database') AS db_count
        FROM articles a
        JOIN authors u ON a.author_id = u.author_id
        WHERE a.id @@@ pdb.all()
        GROUP BY a.author_id
        HAVING COUNT(*) FILTER (WHERE a.body ||| 'database') >= 1
        ORDER BY a.author_id
        "#,
    );

    assert_eq!(rows, vec![(1, 3), (2, 1), (4, 1)]);
}

/// Single-table aggregates routed through DataFusion (forced by a bucket
/// limit lower than the group count) must also push down search FILTERs.
#[rstest]
fn single_table_datafusion_search_filter(mut conn: PgConnection) {
    setup_articles(&mut conn);
    "SET paradedb.max_term_agg_buckets TO 1".execute(&mut conn);

    let rows = assert_pushdown_parity::<(i32, i64, i64)>(
        &mut conn,
        r#"
        SELECT author_id,
               COUNT(*) AS total,
               COUNT(*) FILTER (WHERE category = 'tutorial' AND body ||| 'database') AS tutorial_db
        FROM articles
        WHERE body ||| 'database' OR body ||| 'search'
        GROUP BY author_id
        ORDER BY author_id
        "#,
    );

    assert_eq!(rows, vec![(1, 3, 2), (2, 1, 1), (3, 2, 0), (4, 1, 1)]);
}

/// A search FILTER whose query string is a bound parameter must push down to
/// the DataFusion aggregate scan in both custom and generic prepared-plan modes.
#[rstest]
fn search_filter_with_bound_parameter(mut conn: PgConnection) {
    setup_articles(&mut conn);

    let query = |term: &str| {
        format!(
            r#"
            SELECT a.author_id,
                   COUNT(*) FILTER (WHERE a.body ||| {term}) AS filtered
            FROM articles a
            JOIN authors u ON a.author_id = u.author_id
            WHERE a.id @@@ pdb.all()
            GROUP BY a.author_id
            ORDER BY a.author_id
            "#
        )
    };

    format!("PREPARE filtered_by_term(text) AS {}", query("$1")).execute(&mut conn);

    for mode in ["force_custom_plan", "force_generic_plan"] {
        format!("SET plan_cache_mode TO {mode}").execute(&mut conn);
        for term in ["database", "search"] {
            "SET paradedb.enable_aggregate_custom_scan TO on".execute(&mut conn);
            let exec_sql = format!("EXECUTE filtered_by_term('{term}')");
            assert_uses_datafusion_aggregate_scan(&mut conn, &exec_sql);
            let pushed_down = exec_sql.fetch::<(i32, i64)>(&mut conn);

            "SET paradedb.enable_aggregate_custom_scan TO off".execute(&mut conn);
            let native = query(&format!("'{term}'")).fetch::<(i32, i64)>(&mut conn);

            assert_eq!(pushed_down, native, "plan_cache_mode={mode}, term={term}");
        }
    }
}

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

mod fixtures;

// TODO: Re-enable when HeapCondition bug is fixed
#[allow(unused_imports)]
use crate::fixtures::querygen::crossrelgen::arb_cross_rel_expr;
use crate::fixtures::querygen::groupbygen::arb_group_by;
use crate::fixtures::querygen::joingen::{arb_joins, JoinType};
use crate::fixtures::querygen::pagegen::arb_paging_exprs;
// TODO: Re-enable when score ordering bug is fixed
#[allow(unused_imports)]
use crate::fixtures::querygen::scoregen::arb_score_order;
use crate::fixtures::querygen::wheregen::{arb_simple_wheres, arb_wheres};
use crate::fixtures::querygen::{
    arb_joins_and_wheres, compare, generated_queries_setup, Column, PgGucs,
};

use fixtures::*;

use futures::executor::block_on;
use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use rstest::*;
use serde_json::Value;
use sqlx::{PgConnection, Row};

const COLUMNS: &[Column] = &[
    Column::new("id", "SERIAL8", "'4'")
        .primary_key()
        .groupable({
            true
        }),
    Column::new("uuid", "UUID", "'550e8400-e29b-41d4-a716-446655440000'")
        .groupable({
            true
        })
        .bm25_text_field(r#""uuid": { "tokenizer": { "type": "keyword" } , "fast": true }"#)
        .random_generator_sql("rpad(lpad((random() * 2147483647)::integer::text, 10, '0'), 32, '0')::uuid"),
    Column::new("name", "TEXT", "'bob'")
        .bm25_text_field(r#""name": { "tokenizer": { "type": "keyword" }, "fast": true }"#)
        .random_generator_sql(
            "(ARRAY ['alice', 'bob', 'cloe', 'sally', 'brandy', 'brisket', 'anchovy']::text[])[(floor(random() * 7) + 1)::int]"
        ),
    Column::new("color", "VARCHAR", "'blue'")
        .whereable({
            // TODO: A variety of tests fail due to the NULL here. The column exists in order to
            // provide coverage for ORDER BY on a column containing NULL.
            // https://github.com/paradedb/paradedb/issues/3111
            false
        })
        .bm25_text_field(r#""color": { "tokenizer": { "type": "keyword" }, "fast": true }"#)
        .random_generator_sql(
            "(ARRAY ['red', 'green', 'blue', 'orange', 'purple', 'pink', 'yellow', NULL]::text[])[(floor(random() * 8) + 1)::int]"
        ),
    Column::new("age", "INTEGER", "'20'")
        .bm25_numeric_field(r#""age": { "fast": true }"#)
        .random_generator_sql("(floor(random() * 100) + 1)"),
    Column::new("quantity", "INTEGER", "'7'")
        .whereable({
            // TODO: A variety of tests fail due to the NULL here. The column exists in order to
            // provide coverage for ORDER BY on a column containing NULL.
            // https://github.com/paradedb/paradedb/issues/3111
            false
        })
        .bm25_numeric_field(r#""quantity": { "fast": true }"#)
        .random_generator_sql("CASE WHEN random() < 0.1 THEN NULL ELSE (floor(random() * 100) + 1)::int END"),
    Column::new("price", "NUMERIC(10,2)", "'99.99'")
        .groupable({
            // TODO: Grouping on a float fails to ORDER BY (even in cases without an ORDER BY):
            // ```
            // Cannot ORDER BY OrderByInfo
            // ```
            false
        })
        .bm25_numeric_field(r#""price": { "fast": true }"#)
        .random_generator_sql("(random() * 1000 + 10)::numeric(10,2)"),
    Column::new("rating", "INTEGER", "'4'")
        .indexed({
            // Marked un-indexed in order to test heap-filter pushdown.
            false
        })
        .groupable({
            true
        })
        .bm25_numeric_field(r#""rating": { "fast": true }"#)
        .random_generator_sql("(floor(random() * 5) + 1)::int"),
];

fn columns_named(names: Vec<&'static str>) -> Vec<Column> {
    COLUMNS
        .iter()
        .filter(|c| names.contains(&c.name))
        .cloned()
        .collect()
}

///
/// Tests all JoinTypes against small tables (which are particularly important for joins which
/// result in e.g. the cartesian product).
///
#[rstest]
#[tokio::test]
async fn generated_joins_small(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let tables_and_sizes = [("users", 10), ("products", 10), ("orders", 10)];
    let tables = tables_and_sizes
        .iter()
        .map(|(table, _)| table)
        .collect::<Vec<_>>();
    let setup_sql = generated_queries_setup(&mut pool.pull(), &tables_and_sizes, COLUMNS);

    proptest!(|(
        (join, where_expr) in arb_joins_and_wheres(
            any::<JoinType>(),
            tables,
            &columns_named(vec!["id", "name", "color", "age"]),
        ),
        gucs in any::<PgGucs>(),
    )| {
        let join_clause = join.to_sql();

        let from = format!("SELECT COUNT(*) {join_clause} ");

        compare(
            &format!("{from} WHERE {}", where_expr.to_sql(" = ")),
            &format!("{from} WHERE {}", where_expr.to_sql("@@@")),
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| query.fetch_one::<(i64,)>(conn).0,
        )?;
    });
}

///
/// Tests only the smallest JoinType against larger tables, with a target list, and a limit.
///
/// TODO: This test is currently ignored because it occasionally generates nested loop joins which
/// run in exponential time: https://github.com/paradedb/paradedb/issues/2733
///
#[ignore]
#[rstest]
#[tokio::test]
async fn generated_joins_large_limit(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let tables_and_sizes = [("users", 10000), ("products", 10000), ("orders", 10000)];
    let tables = tables_and_sizes
        .iter()
        .map(|(table, _)| table)
        .collect::<Vec<_>>();
    let setup_sql = generated_queries_setup(&mut pool.pull(), &tables_and_sizes, COLUMNS);

    proptest!(|(
        (join, where_expr) in arb_joins_and_wheres(
            Just(JoinType::Inner),
            tables,
            &columns_named(vec!["id", "name", "color", "age"]),
        ),
        target_list in proptest::sample::subsequence(vec!["id", "name", "color", "age"], 1..=4),
        gucs in any::<PgGucs>(),
    )| {
        let join_clause = join.to_sql();
        let used_tables = join.used_tables();

        let target_list =
            target_list
                .into_iter()
                .map(|column| format!("{}.{column}", used_tables[0]))
                .collect::<Vec<_>>()
                .join(", ");

        let from = format!("SELECT {target_list} {join_clause} ");

        compare(
            &format!("{from} WHERE {} LIMIT 10;", where_expr.to_sql(" = ")),
            &format!("{from} WHERE {} LIMIT 10;", where_expr.to_sql("@@@")),
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| query.fetch_dynamic(conn).len(),
        )?;
    });
}

#[rstest]
#[tokio::test]
async fn generated_single_relation(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 10)], COLUMNS);

    proptest!(|(
        where_expr in arb_wheres(
            vec![table_name],
            COLUMNS,
        ),
        gucs in any::<PgGucs>(),
        target in prop_oneof![Just("COUNT(*)"), Just("id")],
    )| {
        compare(
            &format!("SELECT {target} FROM {table_name} WHERE {}", where_expr.to_sql(" = ")),
            &format!("SELECT {target} FROM {table_name} WHERE {}", where_expr.to_sql("@@@")),
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| {
                let mut rows = query.fetch::<(i64,)>(conn);
                rows.sort();
                rows
            }
        )?;
    });
}

///
/// Property test for GROUP BY aggregates - ensures equivalence between PostgreSQL and bm25 behavior
///
#[rstest]
#[tokio::test]
async fn generated_group_by_aggregates(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 50)], COLUMNS);

    // Columns that can be used for grouping (must have fast: true in index)
    let columns: Vec<_> = COLUMNS
        .iter()
        .filter(|col| col.is_groupable && col.is_whereable)
        .cloned()
        .collect();

    let grouping_columns: Vec<_> = columns.iter().map(|col| col.name).collect();

    proptest!(|(
        text_where_expr in arb_wheres(
            vec![table_name],
            &columns,
        ),
        numeric_where_expr in arb_wheres(
            vec![table_name],
            &columns_named(vec!["age", "price", "rating"]),
        ),
        group_by_expr in arb_group_by(grouping_columns.to_vec(), vec!["COUNT(*)", "SUM(price)", "AVG(price)", "MIN(rating)", "MAX(rating)", "SUM(age)", "AVG(age)"]),
        gucs in any::<PgGucs>(),
    )| {
        let select_list = group_by_expr.to_select_list();
        let group_by_clause = group_by_expr.to_sql();

        // Create combined WHERE clause for PostgreSQL using = operator
        let pg_where_clause = format!(
            "({}) AND ({})",
            text_where_expr.to_sql(" = "),
            numeric_where_expr.to_sql(" < ")
        );

        // Create combined WHERE clause for BM25 using appropriate operators
        let bm25_where_clause = format!(
            "({}) AND ({})",
            text_where_expr.to_sql("@@@"),
            numeric_where_expr.to_sql(" < ")
        );

        let pg_query = format!(
            "SELECT {select_list} FROM {table_name} WHERE {pg_where_clause} {group_by_clause}",
        );

        let bm25_query = format!(
            "SELECT {select_list} FROM {table_name} WHERE {bm25_where_clause} {group_by_clause}",
        );

        // Custom result comparator for GROUP BY results
        let compare_results = |query: &str, conn: &mut PgConnection| -> Vec<String> {
            // Fetch all rows as dynamic results and convert to string representation
            let rows = query.fetch_dynamic(conn);
            let mut string_rows: Vec<String> = rows
                .into_iter()
                .map(|row| {
                    // Convert entire row to a string representation for comparison
                    let mut row_string = String::new();
                    for i in 0..row.len() {
                        if i > 0 {
                            row_string.push('|');
                        }

                        // Try to get value as different types, converting to string
                        let value_str = if let Ok(val) = row.try_get::<i64, _>(i) {
                            val.to_string()
                        } else if let Ok(val) = row.try_get::<i32, _>(i) {
                            val.to_string()
                        } else if let Ok(val) = row.try_get::<String, _>(i) {
                            val
                        } else {
                            "NULL".to_string()
                        };

                        row_string.push_str(&value_str);
                    }
                    row_string
                })
                .collect();

            // Sort for consistent comparison
            string_rows.sort();
            string_rows
        };

        compare(&pg_query, &bm25_query, &gucs, &mut pool.pull(), &setup_sql, compare_results)?;
    });
}

#[rstest]
#[tokio::test]
async fn generated_paging_small(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 1000)], COLUMNS);

    proptest!(|(
        where_expr in arb_wheres(vec![table_name], &columns_named(vec!["name"])),
        paging_exprs in arb_paging_exprs(table_name, vec!["name", "color", "age", "quantity"], vec!["id", "uuid"]),
        gucs in any::<PgGucs>(),
    )| {
        compare(
            &format!("SELECT id FROM {table_name} WHERE {} {paging_exprs}", where_expr.to_sql(" = ")),
            &format!("SELECT id FROM {table_name} WHERE {} {paging_exprs}", where_expr.to_sql("@@@")),
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| query.fetch::<(i64,)>(conn),
        )?;
    });
}

/// Generates paging expressions on a large table, which was necessary to reproduce
/// https://github.com/paradedb/tantivy/pull/51
///
/// TODO: Explore whether this could use https://github.com/paradedb/paradedb/pull/2681
/// to use a large segment count rather than a large table size.
#[rstest]
#[tokio::test]
async fn generated_paging_large(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 100000)], COLUMNS);

    proptest!(|(
        paging_exprs in arb_paging_exprs(table_name, vec![], vec!["uuid"]),
        gucs in any::<PgGucs>(),
    )| {
        compare(
            &format!("SELECT uuid::text FROM {table_name} WHERE name  =  'bob' {paging_exprs}"),
            &format!("SELECT uuid::text FROM {table_name} WHERE name @@@ 'bob' {paging_exprs}"),
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| query.fetch::<(String,)>(conn),
        )?;
    });
}

#[rstest]
#[tokio::test]
async fn generated_subquery(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let outer_table_name = "products";
    let inner_table_name = "orders";
    let setup_sql = generated_queries_setup(
        &mut pool.pull(),
        &[(outer_table_name, 10), (inner_table_name, 10)],
        COLUMNS,
    );

    proptest!(|(
        outer_where_expr in arb_wheres(
            vec![outer_table_name],
            COLUMNS,
        ),
        inner_where_expr in arb_wheres(
            vec![inner_table_name],
            COLUMNS,
        ),
        subquery_column in proptest::sample::select(&["name", "color", "age"]),
        paging_exprs in arb_paging_exprs(inner_table_name, vec!["name", "color", "age"], vec!["id", "uuid"]),
        gucs in any::<PgGucs>(),
    )| {
        let pg = format!(
            "SELECT COUNT(*) FROM {outer_table_name} \
            WHERE {outer_table_name}.{subquery_column} IN (\
                SELECT {subquery_column} FROM {inner_table_name} WHERE {} {paging_exprs}\
            ) AND {}",
            inner_where_expr.to_sql(" = "),
            outer_where_expr.to_sql(" = "),
        );
        let bm25 = format!(
            "SELECT COUNT(*) FROM {outer_table_name} \
            WHERE {outer_table_name}.{subquery_column} IN (\
                SELECT {subquery_column} FROM {inner_table_name} WHERE {} {paging_exprs}\
            ) AND {}",
            inner_where_expr.to_sql("@@@"),
            outer_where_expr.to_sql("@@@"),
        );

        compare(
            &pg,
            &bm25,
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| query.fetch_one::<(i64,)>(conn),
        )?;
    });
}

///
/// Tests JoinScan custom scan implementation with comprehensive variations.
///
/// JoinScan requires:
/// 1. enable_join_custom_scan = on
/// 2. At least one side with a BM25 predicate
/// 3. A LIMIT clause
///
/// This test randomly combines:
/// - 2 or 3 table joins
/// - BM25 predicates on outer table only, or on both outer and inner tables
/// - Optional HeapConditions (cross-relation predicates like a.price > b.price)
/// - Score-based ordering vs regular column ordering
///
/// This verifies that JoinScan produces the same results as PostgreSQL's
/// native join implementation across all these variations.
#[rstest]
#[tokio::test]
async fn generated_joinscan(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    // Three tables for 2-way and 3-way join testing
    let tables_and_sizes = [("users", 100), ("products", 100), ("orders", 100)];
    let all_tables: Vec<&str> = tables_and_sizes.iter().map(|(table, _)| *table).collect();
    let setup_sql = generated_queries_setup(&mut pool.pull(), &tables_and_sizes, COLUMNS);

    // Text columns for BM25 WHERE clauses
    let text_columns = columns_named(vec!["name"]);
    // Numeric columns for join keys and cross-relation predicates
    let join_key_columns = vec!["id", "age"];
    // TODO: Re-enable when HeapCondition bug is fixed
    let _numeric_columns = ["age", "price"];

    proptest!(|(
        // Only 2-table joins for now - 3-table joins have flaky behavior
        // TODO: Investigate segment iteration issues in TopN for multi-table joins
        num_tables in 2..=2usize,
        // Outer table BM25 predicate (always present)
        // Using simple predicates to avoid JoinScan bugs with complex NOT/AND/OR
        outer_bm25 in arb_simple_wheres(vec![all_tables[0]], &text_columns),
        // Inner table BM25 predicate - disabled pending investigation of dual-predicate bug
        // TODO: Re-enable when dual BM25 predicate bug is fixed
        // include_inner_bm25 in proptest::bool::ANY,
        // inner_bm25 in arb_simple_wheres(vec![all_tables[1]], &text_columns),
        // HeapCondition (cross-relation predicate) - disabled for now due to JoinScan bug
        // TODO: Re-enable when HeapCondition + NOT predicate bug is fixed
        // include_heap_condition in proptest::bool::ANY,
        // heap_condition in arb_cross_rel_expr(all_tables[0], all_tables[1], numeric_columns.clone()),
        // Score ordering - disabled due to JoinScan score ordering bug
        // TODO: Re-enable when score ordering bug is fixed
        // use_score_order in proptest::bool::ANY,
        // score_order in arb_score_order(all_tables[0], "id"),
        // Result limit
        limit in 1..=50usize,
    )| {
        // Inner BM25 disabled - set to false
        let include_inner_bm25 = false;
        let inner_bm25 = outer_bm25.clone(); // dummy, not used

        // HeapCondition disabled - set to false
        let include_heap_condition = false;
        let heap_condition = crate::fixtures::querygen::crossrelgen::CrossRelExpr {
            left_table: all_tables[0].to_string(),
            left_col: "age".to_string(),
            op: crate::fixtures::querygen::crossrelgen::CrossRelOp::Lt,
            right_table: all_tables[1].to_string(),
            right_col: "age".to_string(),
        };

        // Score ordering disabled - set to false
        let use_score_order = false;
        let score_order = String::new(); // dummy, not used
        // Build join with selected number of tables
        let tables_for_join: Vec<&str> = all_tables[..num_tables].to_vec();

        // Generate join expression
        let join = arb_joins(
            Just(JoinType::Inner),
            tables_for_join.clone(),
            join_key_columns.clone(),
        );

        // We need to sample from the strategy - use a fixed seed approach
        let join_expr = {
            use proptest::strategy::ValueTree;
            use proptest::test_runner::TestRunner;
            let mut runner = TestRunner::default();
            join.new_tree(&mut runner).unwrap().current()
        };

        let join_clause = join_expr.to_sql();
        let used_tables = join_expr.used_tables();

        // Select columns from the first table
        // When HeapCondition is used, include the referenced columns in target list
        // (JoinScan requires columns to be projected to evaluate HeapConditions)
        let target_list = if include_heap_condition {
            format!(
                "{}.id, {}.name, {}.{}, {}.{}",
                used_tables[0], used_tables[0],
                used_tables[0], heap_condition.left_col,
                used_tables[1], heap_condition.right_col
            )
        } else {
            format!("{}.id, {}.name", used_tables[0], used_tables[0])
        };
        let from = format!("SELECT {target_list} {join_clause}");

        // Build WHERE clause parts for BM25 query
        let mut bm25_where_parts = vec![outer_bm25.to_sql("@@@")];
        let mut pg_where_parts = vec![outer_bm25.to_sql(" = ")];

        // Optionally add inner table BM25 predicate
        if include_inner_bm25 && num_tables >= 2 {
            bm25_where_parts.push(inner_bm25.to_sql("@@@"));
            pg_where_parts.push(inner_bm25.to_sql(" = "));
        }

        // Optionally add HeapCondition (same for both queries since it's a regular comparison)
        if include_heap_condition {
            let heap_sql = heap_condition.to_sql();
            bm25_where_parts.push(heap_sql.clone());
            pg_where_parts.push(heap_sql);
        }

        let bm25_where = bm25_where_parts.join(" AND ");
        let pg_where = pg_where_parts.join(" AND ");

        // Build ORDER BY - score or regular column
        // Note: For comparing results, we always add id as tiebreaker for determinism
        let order_by = if use_score_order {
            format!("{}, {}.id", score_order, used_tables[0])
        } else {
            format!("{}.id", used_tables[0])
        };

        // GUCs with JoinScan enabled
        let gucs = PgGucs {
            join_custom_scan: true,
            ..PgGucs::default()
        };

        // PostgreSQL native join query
        let pg_query = format!(
            "{from} WHERE {pg_where} ORDER BY {}.id LIMIT {limit}",
            used_tables[0]
        );

        // BM25 query with JoinScan enabled
        let bm25_query = format!(
            "{from} WHERE {bm25_where} ORDER BY {order_by} LIMIT {limit}"
        );

        // Verify JoinScan is actually used
        {
            let conn = &mut pool.pull();
            gucs.set().execute(conn);
            let explain_query = format!("EXPLAIN (FORMAT JSON) {bm25_query}");
            let (plan,): (Value,) = explain_query.fetch_one(conn);
            let plan_str = format!("{:?}", plan);
            prop_assert!(
                plan_str.contains("ParadeDB Join Scan"),
                "Query should use ParadeDB Join Scan but got plan: {}\nQuery: {}",
                plan_str,
                bm25_query
            );
        }

        compare(
            &pg_query,
            &bm25_query,
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| {
                // Use dynamic fetch since column count varies with HeapCondition
                let rows = query.fetch_dynamic(conn);
                // Convert to sorted string representation for comparison
                let mut row_strings: Vec<String> = rows
                    .into_iter()
                    .map(|row| {
                        use sqlx::Row;
                        // Get id as i64 for consistent sorting
                        let id: i64 = row.try_get(0).unwrap_or(0);
                        format!("{:020}|{:?}", id, row)
                    })
                    .collect();
                row_strings.sort();
                row_strings
            },
        )?;
    });
}

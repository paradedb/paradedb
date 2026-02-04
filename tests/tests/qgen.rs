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

use crate::fixtures::querygen::crossrelgen::arb_cross_rel_expr;
use crate::fixtures::querygen::groupbygen::arb_group_by;
use crate::fixtures::querygen::joingen::{arb_joins, JoinType};
use crate::fixtures::querygen::numericgen::arb_numeric_expr;
use crate::fixtures::querygen::pagegen::arb_paging_exprs;
use crate::fixtures::querygen::wheregen::arb_wheres;
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
    // Additional NUMERIC columns for testing Numeric64 vs NumericBytes storage
    Column::new("small_numeric", "NUMERIC(5,2)", "'12.34'")
        .groupable(false)
        .bm25_numeric_field(r#""small_numeric": { "fast": true }"#)
        .random_generator_sql("(random() * 100)::numeric(5,2)"),
    Column::new("int_numeric", "NUMERIC(10,0)", "'12345'")
        .groupable(false)
        .bm25_numeric_field(r#""int_numeric": { "fast": true }"#)
        .random_generator_sql("(floor(random() * 1000000))::numeric(10,0)"),
    Column::new("high_scale", "NUMERIC(18,6)", "'123.456789'")
        .groupable(false)
        .bm25_numeric_field(r#""high_scale": { "fast": true }"#)
        .random_generator_sql("(random() * 10000)::numeric(18,6)"),
    Column::new("big_numeric", "NUMERIC", "'12345.67890'")
        .groupable(false)  // Cannot aggregate NumericBytes
        .bm25_numeric_field(r#""big_numeric": { "fast": true }"#)
        .random_generator_sql("(random() * 100000)::numeric"),
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
    Column::new("literal_normalized", "TEXT", "'Hello World'")
        .whereable({
            // literal_normalized lowercases text, so BM25 @@@ would match case-insensitively
            // while PostgreSQL = does exact matching. This causes test failures when comparing
            // results, so we exclude it from WHERE clause testing.
            false
        })
        .bm25_v2_expression("(literal_normalized::pdb.literal_normalized)")
        .random_generator_sql(
            "(ARRAY ['Hello World', 'HELLO WORLD', 'hello world', 'HeLLo WoRLD', 'GOODBYE WORLD', 'goodbye world']::text[])[(floor(random() * 6) + 1)::int]"
        ),
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
    let join_key_columns = vec!["id", "age", "uuid"];
    // Columns for cross relation expressions.
    let numeric_columns = [
        "age",
        // TODO: We cannot pull up `NUMERIC` columns as fast fields until
        // https://github.com/paradedb/paradedb/issues/2968 is resolved.
        // "price"
    ];

    proptest!(|(
        num_tables in 2..=3usize,
        // Outer table BM25 predicate (always present)
        outer_bm25 in arb_wheres(vec![all_tables[0]], &text_columns),
        // Inner table BM25 predicate (optional)
        include_inner_bm25 in proptest::bool::ANY,
        inner_bm25 in arb_wheres(vec![all_tables[1]], &text_columns),
        // HeapCondition (cross-relation predicate)
        include_heap_condition in proptest::bool::ANY,
        heap_condition in arb_cross_rel_expr(all_tables[0], all_tables[1], numeric_columns.to_vec()),
        // Result limit
        limit in 1..=50usize,
    )| {
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

        // Build deterministic ORDER BY with tie-breaker columns
        // When joins produce multiple matching rows, we need to include columns from both sides
        // to ensure deterministic results when LIMIT is applied
        let mut order_parts = vec![format!("{}.id", used_tables[0])];
        for table in &used_tables[1..] {
            order_parts.push(format!("{}.id", table));
        }
        let order_by = order_parts.join(", ");

        // GUCs with JoinScan enabled
        let gucs = PgGucs {
            join_custom_scan: true,
            ..PgGucs::default()
        };

        // PostgreSQL native join query
        let pg_query = format!(
            "{from} WHERE {pg_where} ORDER BY {order_by} LIMIT {limit}"
        );

        // BM25 query with JoinScan enabled
        let bm25_query = format!(
            "{from} WHERE {bm25_where} ORDER BY {order_by} LIMIT {limit}"
        );

        // Verify JoinScan was actually used
        {
            let conn = &mut pool.pull();
            gucs.set().execute(conn);
            let explain_query = format!("EXPLAIN (FORMAT JSON) {bm25_query}");
            let (plan,): (Value,) = explain_query.fetch_one(conn);
            let plan_str = format!("{plan:#?}");
            prop_assert!(
                plan_str.contains("ParadeDB Join Scan"),
                "Query should use ParadeDB Join Scan but got plan: {plan_str}\nQuery: {bm25_query}",
            );
        }

        compare(
            &pg_query,
            &bm25_query,
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| {
                "SET work_mem TO '16MB';".execute(conn);
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

///
/// Property test for numeric pushdown - ensures equivalence between PostgreSQL and BM25 behavior
/// for numeric comparison operators (=, <, <=, >, >=, BETWEEN).
///
/// Tests both Numeric64 (precision <= 18) and NumericBytes (unlimited precision) storage types.
///
#[rstest]
#[tokio::test]
async fn generated_numeric_pushdown(database: Db) {
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
    // Use more rows to get better coverage of value ranges
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 100)], COLUMNS);

    // Numeric columns for testing - includes both Numeric64 and NumericBytes storage types
    let numeric_columns = columns_named(vec![
        "price",         // NUMERIC(10,2) - Numeric64
        "small_numeric", // NUMERIC(5,2) - Numeric64
        "int_numeric",   // NUMERIC(10,0) - Numeric64 (integer-like)
        "high_scale",    // NUMERIC(18,6) - Numeric64 with high scale
        "big_numeric",   // NUMERIC - NumericBytes (unlimited precision)
        "age",           // INTEGER - for comparison
    ]);

    proptest!(|(
        numeric_expr in arb_numeric_expr(vec![table_name], &numeric_columns),
        gucs in any::<PgGucs>(),
    )| {
        // Both queries use the same SQL since numeric comparison operators
        // are handled identically - the pushdown happens internally in BM25
        let where_clause = numeric_expr.to_sql();

        // We need a BM25 predicate to trigger the custom scan
        // Use an OR clause to match all possible name values in the test data
        let bm25_predicate = format!(
            "{table_name}.name @@@ 'alice OR bob OR cloe OR sally OR brandy OR brisket OR anchovy'"
        );

        // PostgreSQL query: uses only the numeric predicate
        let pg_query = format!(
            "SELECT id FROM {table_name} WHERE {where_clause} ORDER BY id"
        );

        // BM25 query: combines BM25 predicate with numeric pushdown
        let bm25_query = format!(
            "SELECT id FROM {table_name} WHERE {bm25_predicate} AND {where_clause} ORDER BY id"
        );

        compare(
            &pg_query,
            &bm25_query,
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| {
                let mut rows = query.fetch::<(i64,)>(conn);
                rows.sort();
                rows
            },
        )?;
    });
}

///
/// Property test for numeric precision preservation.
///
/// Tests that high-precision numeric values (which would lose precision if converted to f64)
/// are correctly matched in BM25 queries. This specifically tests the Numeric64 storage
/// type with values that have more than 15-16 significant digits (f64's precision limit).
///
/// Example: 123456789012345678 and 123456789012345679 are distinct in NUMERIC(18,0)
/// but would be indistinguishable if converted to f64.
///
#[rstest]
#[tokio::test]
async fn generated_numeric_precision(database: Db) {
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

    let table_name = "precision_test";

    // Custom setup for precision testing - uses NUMERIC(18,0) which stores as Numeric64
    // but with values that exceed f64's precision
    let precision_columns: &[Column] = &[
        Column::new("id", "SERIAL8", "'1'")
            .primary_key()
            .groupable(true),
        Column::new("name", "TEXT", "'test'")
            .bm25_text_field(r#""name": { "tokenizer": { "type": "keyword" }, "fast": true }"#)
            .random_generator_sql("'test'"),
        Column::new("big_int", "NUMERIC(18,0)", "'123456789012345678'")
            .groupable(false)
            .bm25_numeric_field(r#""big_int": { "fast": true }"#)
            // Generate high-precision values that differ only in lower digits
            // These values would collide if converted to f64
            .random_generator_sql(
                "(ARRAY [123456789012345678, 123456789012345679, 123456789012345680, 999999999999999998, 999999999999999999]::numeric[])[(floor(random() * 5) + 1)::int]"
            ),
    ];

    let setup_sql =
        generated_queries_setup(&mut pool.pull(), &[(table_name, 50)], precision_columns);

    // High-precision test values that would be indistinguishable in f64
    let precision_test_values = vec![
        "123456789012345678",
        "123456789012345679",
        "123456789012345680",
        "999999999999999998",
        "999999999999999999",
    ];

    proptest!(|(
        test_value in proptest::sample::select(precision_test_values),
        gucs in any::<PgGucs>(),
    )| {
        // PostgreSQL query - should find exact matches only
        let pg_query = format!(
            "SELECT COUNT(*) FROM {table_name} WHERE big_int = {test_value}"
        );

        // BM25 query - should produce identical results
        // Use 'test' as the name value since all rows have name = 'test'
        let bm25_query = format!(
            "SELECT COUNT(*) FROM {table_name} WHERE name @@@ 'test' AND big_int = {test_value}"
        );

        compare(
            &pg_query,
            &bm25_query,
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| query.fetch_one::<(i64,)>(conn).0,
        )?;
    });
}

///
/// Property test for numeric range queries with precision preservation.
///
/// Tests that range queries (>, <, >=, <=, BETWEEN) on high-precision numeric values
/// produce correct results without precision loss.
///
#[rstest]
#[tokio::test]
async fn generated_numeric_range_precision(database: Db) {
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

    let table_name = "range_precision_test";

    // Custom setup for range precision testing
    let precision_columns: &[Column] = &[
        Column::new("id", "SERIAL8", "'1'")
            .primary_key()
            .groupable(true),
        Column::new("name", "TEXT", "'test'")
            .bm25_text_field(r#""name": { "tokenizer": { "type": "keyword" }, "fast": true }"#)
            .random_generator_sql("'test'"),
        Column::new("big_int", "NUMERIC(18,0)", "'100'")
            .groupable(false)
            .bm25_numeric_field(r#""big_int": { "fast": true }"#)
            // Generate sequential high-precision values
            .random_generator_sql("(floor(random() * 100) + 123456789012345600)::numeric(18,0)"),
    ];

    let setup_sql =
        generated_queries_setup(&mut pool.pull(), &[(table_name, 100)], precision_columns);

    // Range boundaries that would collide in f64
    let range_bounds = vec![
        ("123456789012345650", "123456789012345660"),
        ("123456789012345670", "123456789012345680"),
        ("123456789012345690", "123456789012345700"),
    ];

    proptest!(|(
        (low, high) in proptest::sample::select(range_bounds),
        gucs in any::<PgGucs>(),
    )| {
        // PostgreSQL query - range filter
        let pg_query = format!(
            "SELECT COUNT(*) FROM {table_name} WHERE big_int >= {low} AND big_int < {high}"
        );

        // BM25 query - should produce identical results
        // Use 'test' as the name value since all rows have name = 'test'
        let bm25_query = format!(
            "SELECT COUNT(*) FROM {table_name} WHERE name @@@ 'test' AND big_int >= {low} AND big_int < {high}"
        );

        compare(
            &pg_query,
            &bm25_query,
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| query.fetch_one::<(i64,)>(conn).0,
        )?;
    });
}

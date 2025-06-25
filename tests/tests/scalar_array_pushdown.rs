// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use fixtures::*;
use futures::executor::block_on;
use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use rstest::*;
use sqlx::PgConnection;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum Operator {
    Eq, // =
    Ne, // <>
    Lt, // <
    Le, // <=
    Gt, // >
    Ge, // >=
}

impl Operator {
    fn to_sql(&self) -> &'static str {
        match self {
            Operator::Eq => "=",
            Operator::Ne => "<>",
            Operator::Lt => "<",
            Operator::Le => "<=",
            Operator::Gt => ">",
            Operator::Ge => ">=",
        }
    }
}

#[derive(Debug, Clone)]
pub enum ArrayQuantifier {
    Any,
    All,
}

impl ArrayQuantifier {
    fn to_sql(&self) -> &'static str {
        match self {
            ArrayQuantifier::Any => "ANY",
            ArrayQuantifier::All => "ALL",
        }
    }
}

#[derive(Debug, Clone)]
pub enum TokenizerType {
    Default,
    Keyword,
}

impl TokenizerType {
    fn to_index_config(&self) -> &'static str {
        match self {
            TokenizerType::Default => r#""tokenizer": {"type": "default"}"#,
            TokenizerType::Keyword => r#""tokenizer": {"type": "keyword"}"#,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ColumnType {
    Text,
    Integer,
    Boolean,
    Timestamp,
    Uuid,
}

impl ColumnType {
    fn column_name(&self) -> &'static str {
        match self {
            ColumnType::Text => "text_col",
            ColumnType::Integer => "int_col",
            ColumnType::Boolean => "bool_col",
            ColumnType::Timestamp => "ts_col",
            ColumnType::Uuid => "uuid_col",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScalarArrayExpr {
    column_type: ColumnType,
    operator: Operator,
    quantifier: ArrayQuantifier,
    tokenizer: TokenizerType,
    use_in_syntax: bool, // Whether to use IN syntax instead of = ANY
}

impl ScalarArrayExpr {
    fn sample_values(&self) -> Vec<String> {
        match self.column_type {
            ColumnType::Text => vec![
                "'apple'".to_string(),
                "'banana'".to_string(),
                "'cherry'".to_string(),
                "'date'".to_string(),
                "NULL".to_string(),
            ],
            ColumnType::Integer => vec![
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "42".to_string(),
                "100".to_string(),
                "NULL".to_string(),
            ],
            ColumnType::Boolean => {
                vec!["true".to_string(), "false".to_string(), "NULL".to_string()]
            }
            ColumnType::Timestamp => vec![
                "'2023-01-01 00:00:00'::timestamp".to_string(),
                "'2023-06-15 12:30:00'::timestamp".to_string(),
                "'2024-01-01 00:00:00'::timestamp".to_string(),
                "NULL".to_string(),
            ],
            ColumnType::Uuid => vec![
                "'550e8400-e29b-41d4-a716-446655440000'::uuid".to_string(),
                "'6ba7b810-9dad-11d1-80b4-00c04fd430c8'::uuid".to_string(),
                "'6ba7b811-9dad-11d1-80b4-00c04fd430c8'::uuid".to_string(),
                "'12345678-1234-5678-9abc-123456789abc'::uuid".to_string(),
                "NULL".to_string(),
            ],
        }
    }

    fn to_sql(&self, values: &[String]) -> String {
        let column = self.column_type.column_name();
        let op = self.operator.to_sql();
        let quantifier = self.quantifier.to_sql();

        // Special case for IN syntax (only works with = ANY)
        if self.use_in_syntax
            && matches!(self.operator, Operator::Eq)
            && matches!(self.quantifier, ArrayQuantifier::Any)
        {
            format!("{} IN ({})", column, values.join(", "))
        } else {
            let array_literal = format!("ARRAY[{}]", values.join(", "));
            format!("{} {} {}({})", column, op, quantifier, array_literal)
        }
    }
}

fn scalar_array_setup(conn: &mut PgConnection, tokenizer: TokenizerType) -> String {
    "CREATE EXTENSION IF NOT EXISTS pg_search;".execute(conn);
    "SET log_error_verbosity TO VERBOSE;".execute(conn);
    "SET log_min_duration_statement TO 1000;".execute(conn);

    let setup_sql = format!(
        r#"
DROP TABLE IF EXISTS scalar_array_test;
CREATE TABLE scalar_array_test (
    id SERIAL8 NOT NULL PRIMARY KEY,
    text_col TEXT,
    int_col INTEGER,
    bool_col BOOLEAN,
    ts_col TIMESTAMP,
    uuid_col UUID
);

-- Insert test data
INSERT INTO scalar_array_test (text_col, int_col, bool_col, ts_col, uuid_col) VALUES
    ('apple', 1, true, '2023-01-01 00:00:00', '550e8400-e29b-41d4-a716-446655440000'),
    ('banana', 2, false, '2023-06-15 12:30:00', '6ba7b810-9dad-11d1-80b4-00c04fd430c8'),
    ('cherry', 3, true, '2024-01-01 00:00:00', '6ba7b811-9dad-11d1-80b4-00c04fd430c8'),
    ('date', 42, false, '2023-12-25 18:00:00', '12345678-1234-5678-9abc-123456789abc'),
    ('elderberry', 100, true, '2024-06-01 09:15:00', '550e8400-e29b-41d4-a716-446655440001'),
    ('fig', 1, false, '2023-03-15 14:20:00', '6ba7b810-9dad-11d1-80b4-00c04fd430c9'),
    ('grape', 2, true, '2023-09-30 20:45:00', '6ba7b811-9dad-11d1-80b4-00c04fd430c9'),
    ('honeydew', 3, false, '2024-02-14 11:30:00', '12345678-1234-5678-9abc-123456789abd'),
    -- Rows with NULL values
    (NULL, 4, true, '2024-03-01 10:00:00', '550e8400-e29b-41d4-a716-446655440002'),
    ('kiwi', NULL, false, '2024-04-01 11:00:00', '6ba7b810-9dad-11d1-80b4-00c04fd430ca'),
    ('lemon', 5, NULL, '2024-05-01 12:00:00', '6ba7b811-9dad-11d1-80b4-00c04fd430ca'),
    ('mango', 6, true, NULL, '12345678-1234-5678-9abc-123456789abe'),
    ('orange', 7, false, '2024-07-01 14:00:00', NULL);

-- Create BM25 index with configurable tokenizer
CREATE INDEX idx_scalar_array_test ON scalar_array_test
USING bm25 (id, text_col, int_col, bool_col, ts_col, uuid_col)
WITH (
    key_field = 'id',
    text_fields = '{{
        "text_col": {{ {} }},
        "uuid_col": {{ {} }}
    }}'
);
"#,
        tokenizer.to_index_config(),
        tokenizer.to_index_config()
    );

    setup_sql.clone().execute(conn);
    setup_sql
}

/// Compare results with custom scan enabled vs disabled
fn compare_scalar_array<R, F>(
    pg_query: String,
    bm25_query: String,
    conn: &mut PgConnection,
    run_query: F,
) -> Result<(), TestCaseError>
where
    R: Eq + Debug,
    F: Fn(&str, &mut PgConnection) -> R,
{
    // Run with custom scan disabled (standard Postgres behavior)
    r#"
        SET max_parallel_workers TO 8;
        SET enable_seqscan TO ON;
        SET enable_indexscan TO ON;
        SET paradedb.enable_custom_scan TO OFF;
    "#
    .execute(conn);

    let pg_result = run_query(&pg_query, conn);

    // Run with custom scan enabled and various scan types disabled
    "SET paradedb.enable_custom_scan TO ON;".execute(conn);
    for scan_type in [
        "SET enable_seqscan TO OFF",
        "SET enable_indexscan TO OFF",
        "SET max_parallel_workers TO 0",
    ] {
        scan_type.execute(conn);

        let bm25_result = run_query(&bm25_query, conn);

        prop_assert_eq!(
            &pg_result,
            &bm25_result,
            "\nscan_type={}\npg:\n  {}\nbm25:\n  {}\nexplain:\n{}\n",
            scan_type,
            pg_query,
            bm25_query,
            format!("EXPLAIN (ANALYZE, VERBOSE) {}", bm25_query)
                .fetch::<(String,)>(conn)
                .into_iter()
                .map(|(s,)| s)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    Ok(())
}

// Strategy for generating operators
fn arb_operator() -> impl Strategy<Value = Operator> {
    prop_oneof![
        Just(Operator::Eq),
        Just(Operator::Ne),
        Just(Operator::Lt),
        Just(Operator::Le),
        Just(Operator::Gt),
        Just(Operator::Ge),
    ]
}

// Strategy for generating array quantifiers
fn arb_quantifier() -> impl Strategy<Value = ArrayQuantifier> {
    prop_oneof![Just(ArrayQuantifier::Any), Just(ArrayQuantifier::All)]
}

// Strategy for generating tokenizer types
fn arb_tokenizer() -> impl Strategy<Value = TokenizerType> {
    prop_oneof![Just(TokenizerType::Default), Just(TokenizerType::Keyword)]
}

// Strategy for generating column types
fn arb_column_type() -> impl Strategy<Value = ColumnType> {
    prop_oneof![
        Just(ColumnType::Text),
        Just(ColumnType::Integer),
        Just(ColumnType::Boolean),
        Just(ColumnType::Timestamp),
        Just(ColumnType::Uuid),
    ]
}

// Strategy for generating scalar array expressions
fn arb_scalar_array_expr() -> impl Strategy<Value = ScalarArrayExpr> {
    (
        arb_column_type(),
        arb_operator(),
        arb_quantifier(),
        arb_tokenizer(),
        any::<bool>(), // use_in_syntax
    )
        .prop_map(
            |(column_type, operator, quantifier, tokenizer, use_in_syntax)| ScalarArrayExpr {
                column_type,
                operator,
                quantifier,
                tokenizer,
                use_in_syntax,
            },
        )
}

#[rstest]
#[tokio::test]
async fn test_scalar_array_pushdown_basic(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    proptest!(|(
        expr in arb_scalar_array_expr(),
    )| {
        let values = expr.sample_values();
        let selected_values = values.into_iter().take(2).collect::<Vec<_>>();

        let setup_sql = scalar_array_setup(&mut pool.pull(), expr.tokenizer.clone());
        eprintln!("Setup SQL:\n{}", setup_sql);

        let array_condition = expr.to_sql(&selected_values);

        // Test result sets ordered by id
        let pg_query = format!(
            "SELECT id FROM scalar_array_test WHERE {} ORDER BY id",
            array_condition
        );
        let bm25_query = format!(
            "SELECT id FROM scalar_array_test WHERE {} ORDER BY id",
            array_condition
        );

        eprintln!("Testing array condition: {}", array_condition);
        eprintln!("PG query: {}", pg_query);
        eprintln!("BM25 query: {}", bm25_query);

        compare_scalar_array(
            pg_query,
            bm25_query,
            &mut pool.pull(),
            |query, conn| query.fetch::<(i64,)>(conn),
        )?;
    });
}

#[rstest]
#[tokio::test]
async fn test_scalar_array_pushdown_with_results(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    proptest!(|(
        expr in arb_scalar_array_expr(),
    )| {
        let values = expr.sample_values();
        let selected_values = values.into_iter().take(2).collect::<Vec<_>>();

        let setup_sql = scalar_array_setup(&mut pool.pull(), expr.tokenizer.clone());
        eprintln!("Setup SQL:\n{}", setup_sql);

        let array_condition = expr.to_sql(&selected_values);

        // Test SELECT queries with actual results
        let pg_query = format!(
            "SELECT id, text_col FROM scalar_array_test WHERE {} ORDER BY id",
            array_condition
        );
        let bm25_query = format!(
            "SELECT id, text_col FROM scalar_array_test WHERE {} ORDER BY id",
            array_condition
        );

        eprintln!("Testing array condition: {}", array_condition);
        eprintln!("PG query: {}", pg_query);
        eprintln!("BM25 query: {}", bm25_query);

        compare_scalar_array(
            pg_query,
            bm25_query,
            &mut pool.pull(),
            |query, conn| {
                let mut rows = query.fetch::<(i64, Option<String>)>(conn);
                rows.sort();
                rows
            },
        )?;
    });
}

#[rstest]
#[tokio::test]
async fn test_scalar_array_in_syntax_variations(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    // Test specific IN syntax variations that should be equivalent
    let test_cases = vec![
        (
            "text_col = ANY(ARRAY['apple', 'banana'])",
            "text_col IN ('apple', 'banana')",
        ),
        ("int_col = ANY(ARRAY[1, 2, 3])", "int_col IN (1, 2, 3)"),
        (
            "bool_col = ANY(ARRAY[true, false])",
            "bool_col IN (true, false)",
        ),
        (
            "uuid_col = ANY(ARRAY['550e8400-e29b-41d4-a716-446655440000'::uuid, '6ba7b810-9dad-11d1-80b4-00c04fd430c8'::uuid])",
            "uuid_col IN ('550e8400-e29b-41d4-a716-446655440000'::uuid, '6ba7b810-9dad-11d1-80b4-00c04fd430c8'::uuid)",
        ),
    ];

    for (array_syntax, in_syntax) in test_cases {
        let setup_sql = scalar_array_setup(&mut pool.pull(), TokenizerType::Keyword);
        eprintln!("Setup SQL:\n{}", setup_sql);

        let pg_query_array = format!(
            "SELECT COUNT(*) FROM scalar_array_test WHERE {}",
            array_syntax
        );
        let pg_query_in = format!("SELECT COUNT(*) FROM scalar_array_test WHERE {}", in_syntax);
        let bm25_query = format!(
            "SELECT COUNT(*) FROM scalar_array_test WHERE {}",
            array_syntax
        );

        eprintln!("Array syntax: {}", array_syntax);
        eprintln!("IN syntax: {}", in_syntax);

        // First verify that array and IN syntax give same results in plain Postgres
        "SET paradedb.enable_custom_scan TO OFF;".execute(&mut pool.pull());
        let array_result = pg_query_array
            .clone()
            .fetch_one::<(i64,)>(&mut pool.pull())
            .0;
        let in_result = pg_query_in.fetch_one::<(i64,)>(&mut pool.pull()).0;

        assert_eq!(
            array_result, in_result,
            "Array syntax and IN syntax should give same results: {} vs {}",
            array_syntax, in_syntax
        );

        // Then test pushdown
        compare_scalar_array(
            pg_query_array,
            bm25_query,
            &mut pool.pull(),
            |query, conn| query.fetch_one::<(i64,)>(conn).0,
        )
        .unwrap();
    }
}

#[rstest]
#[tokio::test]
async fn test_scalar_array_edge_cases(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let setup_sql = scalar_array_setup(&mut pool.pull(), TokenizerType::Default);
    eprintln!("Setup SQL:\n{}", setup_sql);

    // Test edge cases that might cause issues
    let edge_cases = vec![
        "int_col = ANY(ARRAY[]::integer[])",
        "text_col = ANY(ARRAY['apple'])",
        "int_col >= ALL(ARRAY[1, 2])",
        "bool_col <> ANY(ARRAY[false])",
        "ts_col <= ALL(ARRAY['2024-01-01'::timestamp, '2024-12-31'::timestamp])",
        "text_col = ANY(ARRAY['apple', NULL])",
        "int_col = ANY(ARRAY[1, 2, NULL])",
        "bool_col = ANY(ARRAY[true, NULL])",
        "uuid_col = ANY(ARRAY['550e8400-e29b-41d4-a716-446655440000'::uuid, NULL])",
        "text_col IS NULL AND 'apple' = ANY(ARRAY['apple', 'banana'])",
        "int_col IS NOT NULL AND int_col = ANY(ARRAY[1, 2, NULL])",
    ];

    for edge_case in edge_cases {
        let pg_query = format!("SELECT COUNT(*) FROM scalar_array_test WHERE {}", edge_case);
        let bm25_query = format!("SELECT COUNT(*) FROM scalar_array_test WHERE {}", edge_case);

        eprintln!("Testing edge case: {}", edge_case);

        compare_scalar_array(pg_query, bm25_query, &mut pool.pull(), |query, conn| {
            query.fetch_one::<(i64,)>(conn).0
        })
        .unwrap();
    }
}

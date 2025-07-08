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
use proptest_derive::Arbitrary;
use rstest::*;
use sqlx::PgConnection;
use std::fmt::Debug;

use crate::fixtures::querygen::opexprgen::{ArrayQuantifier, Operator, ScalarArrayOperator};
use crate::fixtures::querygen::{compare, PgGucs};

#[derive(Debug, Clone, Arbitrary)]
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

#[derive(Debug, Clone, Arbitrary)]
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

#[derive(Debug, Clone, Arbitrary)]
pub enum ArrayOperation {
    OperatorQuantifier {
        operator: Operator,
        quantifier: ArrayQuantifier,
    },
    ScalarArray {
        operator: ScalarArrayOperator,
    },
}

#[derive(Debug, Clone, Arbitrary)]
pub struct ScalarArrayExpr {
    column_type: ColumnType,
    operation: ArrayOperation,
    tokenizer: TokenizerType,
    include_null: bool,
}

impl ScalarArrayExpr {
    fn sample_values(&self) -> impl Strategy<Value = String> {
        let values = match self.column_type {
            ColumnType::Text => vec![
                "'apple'".to_string(),
                "'banana'".to_string(),
                "'cherry'".to_string(),
                "'date'".to_string(),
                "'elderberry'".to_string(),
            ],
            ColumnType::Integer => vec![
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "42".to_string(),
                "100".to_string(),
            ],
            ColumnType::Boolean => {
                vec!["true".to_string(), "false".to_string()]
            }
            ColumnType::Timestamp => vec![
                "'2023-01-01 00:00:00'::timestamp".to_string(),
                "'2023-06-15 12:30:00'::timestamp".to_string(),
                "'2024-01-01 00:00:00'::timestamp".to_string(),
                "'2024-06-01 09:15:00'::timestamp".to_string(),
            ],
            ColumnType::Uuid => vec![
                "'550e8400-e29b-41d4-a716-446655440000'::uuid".to_string(),
                "'6ba7b810-9dad-11d1-80b4-00c04fd430c8'::uuid".to_string(),
                "'6ba7b811-9dad-11d1-80b4-00c04fd430c8'::uuid".to_string(),
                "'12345678-1234-5678-9abc-123456789abc'::uuid".to_string(),
            ],
        };
        proptest::sample::select(values)
    }

    fn null_value(&self) -> String {
        match self.column_type {
            ColumnType::Text => "NULL::text".to_string(),
            ColumnType::Integer => "NULL::integer".to_string(),
            ColumnType::Boolean => "NULL::boolean".to_string(),
            ColumnType::Timestamp => "NULL::timestamp".to_string(),
            ColumnType::Uuid => "NULL::uuid".to_string(),
        }
    }

    fn to_sql(&self, values: &[String]) -> String {
        let column = self.column_type.column_name();

        // Add NULL to values if include_null is true
        let mut final_values = values.to_vec();
        if self.include_null {
            final_values.push(self.null_value());
        }

        match &self.operation {
            ArrayOperation::OperatorQuantifier {
                operator,
                quantifier,
            } => {
                let op = operator.to_sql();
                let quant = quantifier.to_sql();
                let array_literal = format!("ARRAY[{}]", final_values.join(", "));
                format!("{column} {op} {quant}({array_literal})")
            }
            ArrayOperation::ScalarArray { operator } => {
                let op = operator.to_sql();
                format!("{} {} ({})", column, op, final_values.join(", "))
            }
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
    ('Apple', 2, false, '2023-06-15 12:30:00', '6ba7b810-9dad-11d1-80b4-00c04fd430c8'),
    ('Apple Tree', 3, true, '2024-01-01 00:00:00', '6ba7b811-9dad-11d1-80b4-00c04fd430c8'),
    ('banana', 42, false, '2023-12-25 18:00:00', '12345678-1234-5678-9abc-123456789abc'),
    ('banana bunch', 100, true, '2024-06-01 09:15:00', '550e8400-e29b-41d4-a716-446655440001'),
    ('Ripe Banana', 1, false, '2023-03-15 14:20:00', '6ba7b810-9dad-11d1-80b4-00c04fd430c9'),
    ('banana', 2, true, '2023-09-30 20:45:00', '6ba7b811-9dad-11d1-80b4-00c04fd430c9'),
    ('banana', 3, false, '2024-02-14 11:30:00', '12345678-1234-5678-9abc-123456789abd'),
    -- Rows with NULL values
    (NULL, 4, true, '2024-03-01 10:00:00', '550e8400-e29b-41d4-a716-446655440002'),
    ('cherry', NULL, false, '2024-04-01 11:00:00', '6ba7b810-9dad-11d1-80b4-00c04fd430ca'),
    ('date', 42, NULL, '2024-05-01 12:00:00', '6ba7b811-9dad-11d1-80b4-00c04fd430ca'),
    ('elderberry', 2, true, NULL, '12345678-1234-5678-9abc-123456789abe'),
    ('cherry', 1, false, '2024-07-01 14:00:00', NULL);

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

-- help our cost estimates
ANALYZE scalar_array_test;
"#,
        tokenizer.to_index_config(),
        tokenizer.to_index_config()
    );

    setup_sql.clone().execute(conn);
    setup_sql
}

#[rstest]
#[tokio::test]
async fn scalar_array_pushdown_correctness(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    proptest!(|(
        (expr, selected_values) in any::<ScalarArrayExpr>()
            .prop_flat_map(|expr| {
                let values_strategy = proptest::collection::vec(expr.sample_values(), 1..4);
                (Just(expr), values_strategy)
            }),
        gucs in any::<PgGucs>(),
    )| {
        let setup_sql = scalar_array_setup(&mut pool.pull(), expr.tokenizer.clone());
        eprintln!("Setup SQL:\n{setup_sql}");

        let array_condition = expr.to_sql(&selected_values);

        // Test SELECT queries with actual results
        let pg_query = format!(
            "SELECT id, text_col FROM scalar_array_test WHERE {array_condition} ORDER BY id"
        );
        let bm25_query = format!(
            "SELECT id, text_col FROM scalar_array_test WHERE {array_condition} ORDER BY id"
        );

        compare(
            pg_query,
            bm25_query,
            gucs,
            &mut pool.pull(),
            |query, conn| {
                query.fetch::<(i64, Option<String>)>(conn)
            },
        )?;
    });
}

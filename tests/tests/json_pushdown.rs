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
use proptest::strategy::{BoxedStrategy, Strategy};
use proptest_derive::Arbitrary;
use rstest::*;
use sqlx::PgConnection;
use std::fmt::Debug;

use crate::fixtures::querygen::opexprgen::Operator;
use crate::fixtures::querygen::{compare, PgGucs};

#[derive(Debug, Clone, Arbitrary)]
pub enum TokenizerType {
    Default,
    Keyword,
}

impl TokenizerType {
    fn to_config(&self) -> &'static str {
        match self {
            TokenizerType::Default => r#""type": "default""#,
            TokenizerType::Keyword => r#""type": "keyword""#,
        }
    }
}

#[derive(Debug, Clone, Arbitrary)]
pub struct IndexConfig {
    tokenizer: TokenizerType,
    fast: bool,
}

impl IndexConfig {
    fn to_json_fields_config(&self) -> String {
        format!(
            r#"{{
                "metadata": {{
                    "tokenizer": {{ {} }},
                    "fast": {}
                }}
            }}"#,
            self.tokenizer.to_config(),
            self.fast
        )
    }
}

#[derive(Debug, Clone, Arbitrary)]
pub enum JsonValueType {
    Text,
    Numeric,
    Boolean,
    Null,
}

impl JsonValueType {
    fn sample_values(&self) -> BoxedStrategy<String> {
        match self {
            JsonValueType::Text => proptest::sample::select(vec![
                "'apple'".to_string(),
                "'banana'".to_string(),
                "'cherry'".to_string(),
                "'date'".to_string(),
                "'elderberry'".to_string(),
                "'test'".to_string(),
                "'value'".to_string(),
            ])
            .boxed(),
            JsonValueType::Numeric => proptest::sample::select(vec![
                "42".to_string(),
                "100".to_string(),
                "3.14".to_string(),
                "0".to_string(),
                "-1".to_string(),
                "999".to_string(),
            ])
            .boxed(),
            JsonValueType::Boolean => {
                proptest::sample::select(vec!["true".to_string(), "false".to_string()]).boxed()
            }
            JsonValueType::Null => Just("NULL".to_string()).boxed(),
        }
    }

    fn to_json_literal(&self, value: &str) -> String {
        match self {
            JsonValueType::Text => format!("'\"{}\"'", value.trim_matches('\'')),
            JsonValueType::Numeric => format!("'{value}'"),
            JsonValueType::Boolean => format!("'{value}'"),
            JsonValueType::Null => "'null'".to_string(),
        }
    }

    fn is_compatible_with_operator(&self, operator: &Operator) -> bool {
        match (self, operator) {
            // Range operators only work with numeric types
            (JsonValueType::Numeric, Operator::Lt | Operator::Le | Operator::Gt | Operator::Ge) => {
                true
            }
            // Equality operators work with all types
            (_, Operator::Eq | Operator::Ne) => true,
            // Other combinations are not compatible
            _ => false,
        }
    }

    fn is_boolean_type(&self) -> bool {
        matches!(self, JsonValueType::Boolean)
    }
}

#[derive(Debug, Clone)]
pub enum JsonPath {
    Simple(String),
    Nested(String, String),
    DeepNested(String, String, String),
}

impl Arbitrary for JsonPath {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        prop_oneof![
            proptest::sample::select(vec![
                "name", "count", "active", "tags", "user", "settings", "level1", "items", "mixed"
            ])
            .prop_map(|key| JsonPath::Simple(key.to_string())),
            (
                proptest::sample::select(vec!["user", "settings", "level1", "mixed"]),
                proptest::sample::select(vec![
                    "name",
                    "age",
                    "theme",
                    "level2",
                    "text",
                    "number",
                    "boolean",
                    "null_value"
                ])
            )
                .prop_map(|(key1, key2)| JsonPath::Nested(key1.to_string(), key2.to_string())),
            (
                proptest::sample::select(vec!["level1"]),
                proptest::sample::select(vec!["level2"]),
                proptest::sample::select(vec!["level3"])
            )
                .prop_map(|(key1, key2, key3)| JsonPath::DeepNested(
                    key1.to_string(),
                    key2.to_string(),
                    key3.to_string()
                )),
        ]
        .boxed()
    }
}

impl JsonPath {
    fn is_boolean_field(&self) -> bool {
        match self {
            JsonPath::Simple(key) => key == "active",
            JsonPath::Nested(_, key2) => key2 == "boolean",
            JsonPath::DeepNested(_, _, _) => false,
        }
    }

    fn is_numeric_field(&self) -> bool {
        match self {
            JsonPath::Simple(key) => key == "count",
            JsonPath::Nested(_, key2) => key2 == "age" || key2 == "number",
            JsonPath::DeepNested(_, _, _) => false,
        }
    }
}

impl JsonPath {
    fn to_sql(&self) -> String {
        match self {
            JsonPath::Simple(key) => format!("'{key}'"),
            JsonPath::Nested(key1, key2) => format!("'{{{key1},{key2}}}'"),
            JsonPath::DeepNested(key1, key2, key3) => format!("'{{{key1},{key2},{key3}}}'"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum JsonOperation {
    Comparison {
        operator: Operator,
        value: JsonValueType,
    },
    IsNull,
    IsNotNull,
    IsTrue,
    IsFalse,
    In {
        values: Vec<JsonValueType>,
    },
    NotIn {
        values: Vec<JsonValueType>,
    },
}

impl Arbitrary for JsonOperation {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        prop_oneof![
            // For comparison operations, ensure type compatibility
            (any::<Operator>(), any::<JsonValueType>())
                .prop_filter(
                    "operator and value type must be compatible",
                    |(operator, value)| { value.is_compatible_with_operator(operator) }
                )
                .prop_map(|(operator, value)| JsonOperation::Comparison { operator, value }),
            Just(JsonOperation::IsNull),
            Just(JsonOperation::IsNotNull),
            // Only allow IsTrue/IsFalse for boolean types
            any::<JsonValueType>()
                .prop_filter("IsTrue/IsFalse only work with boolean types", |value| value
                    .is_boolean_type())
                .prop_map(|_| JsonOperation::IsTrue),
            any::<JsonValueType>()
                .prop_filter("IsTrue/IsFalse only work with boolean types", |value| value
                    .is_boolean_type())
                .prop_map(|_| JsonOperation::IsFalse),
            // For IN operations, generate values compatible with the field type
            any::<JsonValueType>()
                .prop_flat_map(|value_type| { proptest::collection::vec(Just(value_type), 1..4) })
                .prop_map(|values| JsonOperation::In { values }),
            // For NOT IN operations, generate values compatible with the field type
            any::<JsonValueType>()
                .prop_flat_map(|value_type| { proptest::collection::vec(Just(value_type), 1..4) })
                .prop_map(|values| JsonOperation::NotIn { values }),
        ]
        .boxed()
    }
}

#[derive(Debug, Clone)]
pub struct JsonExpr {
    path: JsonPath,
    operation: JsonOperation,
}

impl Arbitrary for JsonExpr {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (any::<JsonPath>(), any::<JsonOperation>())
            .prop_filter(
                "operation must be compatible with field type",
                |(path, operation)| {
                    match operation {
                        JsonOperation::Comparison { operator, value } => {
                            // For range operators, ensure we're using numeric fields
                            match operator {
                                Operator::Lt | Operator::Le | Operator::Gt | Operator::Ge => {
                                    path.is_numeric_field()
                                        && value.is_compatible_with_operator(operator)
                                }
                                _ => {
                                    // For other operators, ensure value type matches field type
                                    match (value, path) {
                                        (JsonValueType::Numeric, path)
                                            if path.is_numeric_field() =>
                                        {
                                            true
                                        }
                                        (JsonValueType::Boolean, path)
                                            if path.is_boolean_field() =>
                                        {
                                            true
                                        }
                                        (JsonValueType::Text, path)
                                            if !path.is_numeric_field()
                                                && !path.is_boolean_field() =>
                                        {
                                            true
                                        }
                                        (JsonValueType::Null, _) => true, // NULL works with any field
                                        _ => false, // Incompatible combinations
                                    }
                                }
                            }
                        }
                        JsonOperation::IsTrue | JsonOperation::IsFalse => {
                            // Boolean operations only work on boolean fields
                            path.is_boolean_field()
                        }
                        JsonOperation::In { values } | JsonOperation::NotIn { values } => {
                            // For IN/NOT IN, ensure all values are compatible with the field type
                            values.iter().all(|value_type| {
                                match (value_type, path) {
                                    (JsonValueType::Numeric, path) if path.is_numeric_field() => {
                                        true
                                    }
                                    (JsonValueType::Boolean, path) if path.is_boolean_field() => {
                                        true
                                    }
                                    (JsonValueType::Text, path)
                                        if !path.is_numeric_field() && !path.is_boolean_field() =>
                                    {
                                        true
                                    }
                                    (JsonValueType::Null, _) => true, // NULL works with any field
                                    _ => false,                       // Incompatible combinations
                                }
                            })
                        }
                        _ => true, // Other operations work with any field type
                    }
                },
            )
            .prop_map(|(path, operation)| JsonExpr { path, operation })
            .boxed()
    }
}

impl JsonExpr {
    fn sample_values(&self) -> BoxedStrategy<Vec<String>> {
        match &self.operation {
            JsonOperation::Comparison { value, .. } => {
                let values = value.sample_values();
                proptest::collection::vec(values, 1..3).boxed()
            }
            JsonOperation::In { values: _ } | JsonOperation::NotIn { values: _ } => {
                // For IN/NOT IN operations, we'll use simple predefined values
                let predefined_values = vec![
                    vec!["'apple'".to_string()],
                    vec!["'banana'".to_string(), "'cherry'".to_string()],
                    vec!["42".to_string(), "100".to_string()],
                ];
                proptest::sample::select(predefined_values).boxed()
            }
            _ => Just(vec![]).boxed(),
        }
    }

    fn to_sql(&self, values: &[String]) -> String {
        let column = "metadata";
        let json_expr = format!("{column} ->> {}", self.path.to_sql());

        match &self.operation {
            JsonOperation::Comparison { operator, value } => {
                if values.is_empty() {
                    return format!("{} {} NULL", json_expr, operator.to_sql());
                }
                let value_literal = value.to_json_literal(&values[0]);

                // Determine the target type based on the field path and operation
                let target_type = if self.path.is_numeric_field() {
                    "numeric"
                } else if self.path.is_boolean_field() {
                    "boolean"
                } else {
                    "text"
                };

                // Add type casting based on the target type and operation
                let final_expr = match (operator, value, target_type) {
                    // Range operations on numeric fields
                    (
                        Operator::Lt | Operator::Le | Operator::Gt | Operator::Ge,
                        JsonValueType::Numeric,
                        "numeric",
                    ) => {
                        format!("({json_expr})::numeric")
                    }
                    // Boolean comparisons
                    (_, JsonValueType::Boolean, "boolean") => {
                        format!("({json_expr})::boolean")
                    }
                    // Numeric comparisons on numeric fields
                    (_, JsonValueType::Numeric, "numeric") => {
                        format!("({json_expr})::numeric")
                    }
                    // Text comparisons (no casting needed for text fields)
                    (_, JsonValueType::Text, "text") => json_expr,
                    // Don't do cross-type comparisons - they're invalid
                    _ => {
                        // For incompatible types, just return the original expression
                        // This will likely cause a runtime error, but that's better than invalid SQL
                        json_expr
                    }
                };

                format!("{} {} {}", final_expr, operator.to_sql(), value_literal)
            }
            JsonOperation::IsNull => format!("{json_expr} IS NULL"),
            JsonOperation::IsNotNull => format!("{json_expr} IS NOT NULL"),
            JsonOperation::IsTrue => {
                // Ensure boolean operations only work on boolean fields
                format!("({json_expr})::boolean IS TRUE")
            }
            JsonOperation::IsFalse => {
                // Ensure boolean operations only work on boolean fields
                format!("({json_expr})::boolean IS FALSE")
            }
            JsonOperation::In { values } => {
                if values.is_empty() {
                    return format!("{json_expr} IN ()");
                }

                // Determine the target type based on the field path
                let target_type = if self.path.is_numeric_field() {
                    "numeric"
                } else if self.path.is_boolean_field() {
                    "boolean"
                } else {
                    "text"
                };

                // Cast the JSON expression to the appropriate type
                let casted_expr = format!("({json_expr})::{target_type}");

                // Generate values of the appropriate type (only compatible combinations)
                let value_literals: Vec<String> = values
                    .iter()
                    .map(|value_type| match value_type {
                        JsonValueType::Text => "'apple'".to_string(),
                        JsonValueType::Numeric => "42".to_string(),
                        JsonValueType::Boolean => "true".to_string(),
                        JsonValueType::Null => "NULL".to_string(),
                    })
                    .collect();
                format!("{} IN ({})", casted_expr, value_literals.join(", "))
            }
            JsonOperation::NotIn { values } => {
                if values.is_empty() {
                    return format!("{json_expr} NOT IN ()");
                }

                // Determine the target type based on the field path
                let target_type = if self.path.is_numeric_field() {
                    "numeric"
                } else if self.path.is_boolean_field() {
                    "boolean"
                } else {
                    "text"
                };

                // Cast the JSON expression to the appropriate type
                let casted_expr = format!("({json_expr})::{target_type}");

                // Generate values of the appropriate type (only compatible combinations)
                let value_literals: Vec<String> = values
                    .iter()
                    .map(|value_type| match value_type {
                        JsonValueType::Text => "'banana'".to_string(),
                        JsonValueType::Numeric => "100".to_string(),
                        JsonValueType::Boolean => "false".to_string(),
                        JsonValueType::Null => "NULL".to_string(),
                    })
                    .collect();
                format!("{} NOT IN ({})", casted_expr, value_literals.join(", "))
            }
        }
    }
}

fn json_pushdown_setup(conn: &mut PgConnection, index_config: &IndexConfig) -> String {
    "CREATE EXTENSION IF NOT EXISTS pg_search;".execute(conn);
    "SET log_error_verbosity TO VERBOSE;".execute(conn);
    "SET log_min_duration_statement TO 1000;".execute(conn);

    let json_fields_config = index_config.to_json_fields_config();

    let setup_sql = format!(
        r#"
DROP TABLE IF EXISTS json_pushdown_test;
CREATE TABLE json_pushdown_test (
    id SERIAL8 NOT NULL PRIMARY KEY,
    metadata JSONB
);

-- Insert test data with various JSON structures
INSERT INTO json_pushdown_test (metadata) VALUES
    ('{{"name": "apple", "count": 42, "active": true, "tags": ["fruit", "red"]}}'),
    ('{{"name": "banana", "count": 100, "active": false, "tags": ["fruit", "yellow"]}}'),
    ('{{"name": "cherry", "count": 3.14, "active": true, "tags": ["fruit", "red"]}}'),
    ('{{"name": "date", "count": 0, "active": false, "tags": ["fruit", "brown"]}}'),
    ('{{"name": "elderberry", "count": -1, "active": true, "tags": ["fruit", "purple"]}}'),
    ('{{"name": "test", "count": 999, "active": false, "tags": ["test", "data"]}}'),
    ('{{"name": "value", "count": 1, "active": true, "tags": ["value", "test"]}}'),
    ('{{"user": {{"name": "alice", "age": 25}}, "settings": {{"theme": "dark"}}}}'),
    ('{{"user": {{"name": "bob", "age": 30}}, "settings": {{"theme": "light"}}}}'),
    ('{{"user": {{"name": "charlie", "age": 35}}, "settings": {{"theme": "dark"}}}}'),
    ('{{"level1": {{"level2": {{"level3": "deep_value"}}}}}}'),
    ('{{"level1": {{"level2": {{"level3": "another_value"}}}}}}'),
    ('{{"items": ["item1", "item2", "item3"]}}'),
    ('{{"items": ["item4", "item5", "item6"]}}'),
    ('{{"mixed": {{"text": "hello", "number": 123, "boolean": true, "null_value": null}}}}'),
    ('{{"mixed": {{"text": "world", "number": 456, "boolean": false, "null_value": null}}}}'),
    (NULL),
    ('{{}}');

-- Create BM25 index
CREATE INDEX idx_json_pushdown_test ON json_pushdown_test
USING bm25 (id, metadata)
WITH (
    key_field = 'id',
    json_fields = '{json_fields_config}'
);

-- help our cost estimates
ANALYZE json_pushdown_test;
"#
    );

    setup_sql.clone().execute(conn);
    setup_sql
}

#[rstest]
#[tokio::test]
async fn json_pushdown_correctness(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    proptest!(|(
        (expr, selected_values) in any::<JsonExpr>()
            .prop_flat_map(|expr| {
                let values_strategy = expr.sample_values();
                (Just(expr), values_strategy)
            }),
        index_config in any::<IndexConfig>(),
        gucs in any::<PgGucs>(),
    )| {
        let setup_sql = json_pushdown_setup(&mut pool.pull(), &index_config);
        eprintln!("Setup SQL:\n{setup_sql}");

        let json_condition = expr.to_sql(&selected_values);

        // Test SELECT queries with actual results
        let pg_query = format!(
            "SELECT id, metadata FROM json_pushdown_test WHERE {json_condition} ORDER BY id"
        );
        let bm25_query = format!(
            "SELECT id, metadata FROM json_pushdown_test WHERE id @@@ paradedb.all() AND {json_condition} ORDER BY id"
        );

        compare(
            pg_query,
            bm25_query,
            gucs,
            &mut pool.pull(),
            |query, conn| {
                query.fetch::<(i64, Option<serde_json::Value>)>(conn)
            },
        )?;
    });
}

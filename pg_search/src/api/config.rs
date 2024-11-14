// Copyright (c) 2023-2024 Retake, Inc.
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

use std::collections::HashSet;

use anyhow::Result;
use pgrx::*;
use serde_json::{json, Map, Value};

#[pg_extern(immutable, parallel_safe)]
#[allow(clippy::too_many_arguments)]
pub fn field(
    name: &str,
    indexed: default!(Option<bool>, "NULL"),
    stored: default!(Option<bool>, "NULL"),
    fast: default!(Option<bool>, "NULL"),
    fieldnorms: default!(Option<bool>, "NULL"),
    record: default!(Option<String>, "NULL"),
    expand_dots: default!(Option<bool>, "NULL"),
    tokenizer: default!(Option<JsonB>, "NULL"),
    normalizer: default!(Option<String>, "NULL"),
) -> JsonB {
    let mut config = Map::new();

    indexed.map(|v| config.insert("indexed".to_string(), Value::Bool(v)));
    stored.map(|v| config.insert("stored".to_string(), Value::Bool(v)));
    fast.map(|v| config.insert("fast".to_string(), Value::Bool(v)));
    fieldnorms.map(|v| config.insert("fieldnorms".to_string(), Value::Bool(v)));
    record.map(|v| config.insert("record".to_string(), Value::String(v)));
    expand_dots.map(|v| config.insert("expand_dots".to_string(), Value::Bool(v)));
    tokenizer.map(|v| config.insert("tokenizer".to_string(), v.0));
    normalizer.map(|v| config.insert("normalizer".to_string(), Value::String(v)));

    JsonB(json!({ name: config }))
}

#[pg_extern(immutable, parallel_safe)]
#[allow(clippy::too_many_arguments)]
pub fn tokenizer(
    name: &str,
    remove_long: default!(Option<i32>, "255"),
    lowercase: default!(Option<bool>, "true"),
    min_gram: default!(Option<i32>, "NULL"),
    max_gram: default!(Option<i32>, "NULL"),
    prefix_only: default!(Option<bool>, "NULL"),
    language: default!(Option<String>, "NULL"),
    pattern: default!(Option<String>, "NULL"),
    stemmer: default!(Option<String>, "NULL"),
) -> JsonB {
    let mut config = Map::new();

    config.insert("type".to_string(), Value::String(name.to_string()));

    // Options for all types
    remove_long.map(|v| config.insert("remove_long".to_string(), Value::Number(v.into())));
    lowercase.map(|v| config.insert("lowercase".to_string(), Value::Bool(v)));
    stemmer.map(|v| config.insert("stemmer".to_string(), Value::String(v)));
    // Options for type = ngram
    min_gram.map(|v| config.insert("min_gram".to_string(), Value::Number(v.into())));
    max_gram.map(|v| config.insert("max_gram".to_string(), Value::Number(v.into())));
    prefix_only.map(|v| config.insert("prefix_only".to_string(), Value::Bool(v)));
    // Options for type = stem
    language.map(|v| config.insert("language".to_string(), Value::String(v)));
    // Options for type = regex
    pattern.map(|v| config.insert("pattern".to_string(), Value::String(v)));

    JsonB(json!(config))
}

#[pg_extern(
    sql = "
CREATE OR REPLACE FUNCTION paradedb.format_create_index(
    index_name text DEFAULT '',
    table_name text DEFAULT '',
    key_field text DEFAULT '',
    schema_name text DEFAULT CURRENT_SCHEMA,
    text_fields jsonb DEFAULT '{}',
    numeric_fields jsonb DEFAULT '{}',
    boolean_fields jsonb DEFAULT '{}',
    json_fields jsonb DEFAULT '{}',
    range_fields jsonb DEFAULT '{}',
    datetime_fields jsonb DEFAULT '{}',
    predicates text DEFAULT ''
)
RETURNS text
LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
",
    name = "format_create_index"
)]
#[allow(clippy::too_many_arguments)]
fn format_create_index(
    index_name: &str,
    table_name: &str,
    key_field: &str,
    schema_name: &str,
    text_fields: JsonB,
    numeric_fields: JsonB,
    boolean_fields: JsonB,
    json_fields: JsonB,
    range_fields: JsonB,
    datetime_fields: JsonB,
    predicates: &str,
) -> Result<String> {
    let mut column_names_set = HashSet::new();
    for jsonb in [
        &text_fields,
        &numeric_fields,
        &boolean_fields,
        &json_fields,
        &range_fields,
        &datetime_fields,
    ] {
        if let Value::Object(ref map) = jsonb.0 {
            for key in map.keys() {
                column_names_set.insert(spi::quote_identifier(key.clone()));
            }
        }
    }

    let mut column_names = column_names_set.into_iter().collect::<Vec<_>>();
    column_names.sort();

    let column_names_csv = column_names
        .clone()
        .into_iter()
        .collect::<Vec<String>>()
        .join(", ");

    let predicate_where = if !predicates.is_empty() {
        format!("WHERE {}", predicates)
    } else {
        "".to_string()
    };

    Ok(format!(
        "CREATE INDEX {} ON {}.{} USING bm25 ({}, {}) WITH (key_field={}, text_fields={}, numeric_fields={}, boolean_fields={}, json_fields={}, range_fields={}, datetime_fields={}) {};",
        spi::quote_identifier(index_name),
        spi::quote_identifier(schema_name),
        spi::quote_identifier(table_name),
        spi::quote_identifier(key_field),
        column_names_csv,
        spi::quote_literal(key_field),
        spi::quote_literal(text_fields.0.to_string()),
        spi::quote_literal(numeric_fields.0.to_string()),
        spi::quote_literal(boolean_fields.0.to_string()),
        spi::quote_literal(json_fields.0.to_string()),
        spi::quote_literal(range_fields.0.to_string()),
        spi::quote_literal(datetime_fields.0.to_string()),
        predicate_where))
}

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

use anyhow::{bail, Result};
use pgrx::prelude::*;
use pgrx::{JsonB, Spi};
use serde_json::{json, Value};
use std::collections::HashSet;
use uuid::Uuid;

use super::format::format_bm25_function;
use super::format::format_empty_function;
use super::format::format_hybrid_function;

// The maximum length of an index name in Postgres is 63 characters,
// but we need to account for the trailing _bm25_index suffix
const MAX_INDEX_NAME_LENGTH: usize = 52;

#[pg_extern(
    sql = "
CREATE OR REPLACE PROCEDURE paradedb.create_bm25(
    index_name text DEFAULT '',
    table_name text DEFAULT '',
    key_field text DEFAULT '',
    schema_name text DEFAULT CURRENT_SCHEMA,
    text_fields jsonb DEFAULT '{}',
    numeric_fields jsonb DEFAULT '{}',
    boolean_fields jsonb DEFAULT '{}',
    json_fields jsonb DEFAULT '{}',
    datetime_fields jsonb DEFAULT '{}',
    predicates text DEFAULT ''
)
LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
",
    name = "create_bm25"
)]
#[allow(clippy::too_many_arguments)]
fn create_bm25_jsonb(
    index_name: &str,
    table_name: &str,
    key_field: &str,
    schema_name: &str,
    text_fields: JsonB,
    numeric_fields: JsonB,
    boolean_fields: JsonB,
    json_fields: JsonB,
    datetime_fields: JsonB,
    predicates: &str,
) -> Result<()> {
    create_bm25_impl(
        index_name,
        table_name,
        key_field,
        schema_name,
        &serde_json::to_string(&text_fields)?,
        &serde_json::to_string(&numeric_fields)?,
        &serde_json::to_string(&boolean_fields)?,
        &serde_json::to_string(&json_fields)?,
        &serde_json::to_string(&datetime_fields)?,
        predicates,
    )
}

#[inline]
#[allow(clippy::too_many_arguments)]
fn create_bm25_impl(
    index_name: &str,
    table_name: &str,
    key_field: &str,
    schema_name: &str,
    text_fields: &str,
    numeric_fields: &str,
    boolean_fields: &str,
    json_fields: &str,
    datetime_fields: &str,
    predicates: &str,
) -> Result<()> {
    let original_client_min_messages =
        Spi::get_one::<String>("SHOW client_min_messages")?.unwrap_or_default();
    Spi::run("SET client_min_messages TO WARNING")?;

    if index_name.is_empty() {
        bail!("no index_name parameter given for bm25 index");
    }

    if index_name.len() > MAX_INDEX_NAME_LENGTH {
        bail!(
            "identifier {} exceeds maximum allowed length of {} characters",
            spi::quote_identifier(index_name),
            MAX_INDEX_NAME_LENGTH
        );
    };

    if Spi::get_one::<bool>(&format!(
        "SELECT EXISTS (SELECT i.schema_name FROM information_schema.schemata i WHERE i.schema_name = {})",
        spi::quote_literal(index_name)
    ))?.unwrap_or(false) {
        bail!("Index name cannot be the same as a schema that already exists. Please choose a different index name or drop the {} schema.", index_name);
    }

    if table_name.is_empty() {
        bail!(
            "no table_name parameter given for bm25 index '{}'",
            spi::quote_literal(index_name)
        );
    }

    if key_field.is_empty() {
        bail!(
            "no key_field parameter given for bm25 index '{}'",
            spi::quote_literal(index_name)
        );
    }

    if text_fields == "{}"
        && numeric_fields == "{}"
        && boolean_fields == "{}"
        && json_fields == "{}"
        && datetime_fields == "{}"
    {
        bail!(
            "no text_fields, numeric_fields, boolean_fields, json_fields, or datetime_fields were specified for index {}",
            spi::quote_literal(index_name)
        );
    }

    Spi::run(&format!(
        "CREATE SCHEMA {}",
        spi::quote_identifier(index_name)
    ))?;

    let mut column_names = HashSet::new();
    for fields in [
        text_fields,
        numeric_fields,
        boolean_fields,
        json_fields,
        datetime_fields,
    ] {
        match json5::from_str::<Value>(fields) {
            Ok(obj) => {
                if let Value::Object(map) = obj {
                    for key in map.keys() {
                        column_names.insert(spi::quote_identifier(key.clone()));
                    }
                }
            }
            Err(err) => {
                bail!("Error parsing {}: {}", fields, err);
            }
        }
    }
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

    let index_uuid = Uuid::new_v4().to_string();
    let index_name_suffixed = format!("{}_bm25_index", index_name);

    Spi::run(&format!(
        "CREATE INDEX {} ON {}.{} USING bm25 ({}, {}) WITH (key_field={}, text_fields={}, numeric_fields={}, boolean_fields={}, json_fields={}, datetime_fields={}, uuid={}) {};",
        spi::quote_identifier(index_name_suffixed.clone()),
        spi::quote_identifier(schema_name),
        spi::quote_identifier(table_name),
        spi::quote_identifier(key_field),
        column_names_csv,
        spi::quote_literal(key_field),
        spi::quote_literal(text_fields),
        spi::quote_literal(numeric_fields),
        spi::quote_literal(boolean_fields),
        spi::quote_literal(json_fields),
        spi::quote_literal(datetime_fields),
        spi::quote_identifier(index_uuid.clone()),
        predicate_where))?;

    let predicate = if !predicates.is_empty() {
        format!("{} AND ", predicates)
    } else {
        "".to_string()
    };

    let oid_query = format!(
        "SELECT oid FROM pg_class WHERE relname = '{}' AND relkind = 'i'",
        &index_name_suffixed
    );
    let index_oid = Spi::get_one::<pg_sys::Oid>(&oid_query)
        .expect("error looking up index in create_bm25")
        .expect("no oid for index created in create_bm25")
        .as_u32();

    let index_json = json!({
        "index_name": index_name_suffixed,
        "index_oid": index_oid,
        "table_name": table_name,
        "key_field": key_field,
        "schema_name": schema_name,
        "uuid":  index_uuid
    });

    Spi::run(&format_bm25_function(
        &spi::quote_qualified_identifier(index_name, "search"),
        &format!(
            "SETOF {}.{}",
            spi::quote_identifier(schema_name),
            spi::quote_identifier(table_name)
        ),
        &format!(
            "RETURN QUERY SELECT * FROM {}.{} WHERE {} {} @@@ __paradedb_search_config__",
            spi::quote_identifier(schema_name),
            spi::quote_identifier(table_name),
            predicate,
            spi::quote_identifier(key_field)
        ),
        &index_json,
    ))?;

    Spi::run(&format_bm25_function(
        &spi::quote_qualified_identifier(index_name, "explain"),
        "TABLE(\"QUERY PLAN\" text)",
        &format!(
            "RETURN QUERY EXPLAIN SELECT * FROM {}.{} WHERE {} {} @@@ __paradedb_search_config__",
            spi::quote_identifier(schema_name),
            spi::quote_identifier(table_name),
            predicate,
            spi::quote_identifier(key_field)
        ),
        &index_json,
    ))?;

    Spi::run(&format_empty_function(
        &spi::quote_qualified_identifier(index_name, "schema"),
        "TABLE(name text, field_type text, stored bool, indexed bool, fast bool, fieldnorms bool, expand_dots bool, tokenizer text, record text, normalizer text)",
        &format!("RETURN QUERY SELECT * FROM paradedb.schema_bm25({})", spi::quote_literal(index_name))
    ))?;

    // Get the type and type oid of the key column
    let (key_oid, key_type) = match Spi::get_two::<pg_sys::Oid, String>(&format!(
        "SELECT a.atttypid AS type_oid, CAST(t.typname AS TEXT) AS type_name
            FROM pg_attribute a
            JOIN pg_type t ON a.atttypid = t.oid
            JOIN pg_class c ON a.attrelid = c.oid
            JOIN pg_namespace n ON c.relnamespace = n.oid
            WHERE c.relname = {} AND a.attname = {} AND n.nspname = {}",
        spi::quote_literal(table_name),
        spi::quote_literal(key_field),
        spi::quote_literal(schema_name)
    ))? {
        (Some(key_oid), Some(key_type)) => (key_oid, key_type),
        _ => bail!("could not select key field type and type oid"),
    };

    let predicate_where_escape = if !predicate_where.is_empty() {
        predicate_where.replace('\'', "''")
    } else {
        "".to_string()
    };

    Spi::run(&format_bm25_function(
        &spi::quote_qualified_identifier(index_name, "score_bm25"),
        &format!("TABLE({} {}, score_bm25 REAL)", spi::quote_identifier(key_field), key_type),
        &format!(
            "RETURN QUERY SELECT * FROM paradedb.score_bm25(__paradedb_search_config__, NULL::{}, {})",
            key_type,
            key_oid.as_u32()
        ),
        &index_json,
    ))?;

    Spi::run(&format_bm25_function(
        &spi::quote_qualified_identifier(index_name, "snippet"),
        &format!(
            "TABLE({} {}, snippet TEXT, score_bm25 REAL)",
            spi::quote_identifier(key_field),
            key_type
        ),
        &format!(
            "RETURN QUERY SELECT * FROM paradedb.snippet(__paradedb_search_config__, NULL::{}, {})",
            key_type,
            key_oid.as_u32()
        ),
        &index_json,
    ))?;

    Spi::run(&format_hybrid_function(
        &spi::quote_qualified_identifier(index_name, "score_hybrid"),
        &format!("TABLE({} {}, score_hybrid real)", spi::quote_identifier(key_field), key_type),
        &format!(
            "
                WITH similarity AS (
                    SELECT
                        __key_field__ as key_field,
                        CASE
                            WHEN (MAX(__similarity_query__) OVER () - MIN(__similarity_query__) OVER ()) = 0 THEN
                                0
                            ELSE
                                1 - ((__similarity_query__) - MIN(__similarity_query__) OVER ()) / 
                                (MAX(__similarity_query__) OVER () - MIN(__similarity_query__) OVER ())
                        END AS score
                    FROM {}.{}
                    ORDER BY __similarity_query__
                    LIMIT $2
                ),
                bm25 AS (
                    SELECT 
                        id as key_field,
                        CASE
                            WHEN (MAX(score_bm25) OVER () - MIN(score_bm25) OVER ()) = 0 THEN
                                0
                            ELSE
                                ((score_bm25) - MIN(score_bm25) OVER ()) / 
                                (MAX(score_bm25) OVER () - MIN(score_bm25) OVER ())
                        END AS score
                    FROM paradedb.score_bm25($1, NULL::{}, {})
                )
                SELECT
                    COALESCE(similarity.key_field, bm25.key_field) AS __key_field__,
                    (COALESCE(similarity.score, 0.0) * $3 + COALESCE(bm25.score, 0.0) * $4)::real AS score_hybrid
                FROM similarity
                FULL OUTER JOIN bm25 ON similarity.key_field = bm25.key_field
                ORDER BY score_hybrid DESC;
            ",
            spi::quote_identifier(schema_name),
            spi::quote_identifier(table_name),
            key_type,
            key_oid.as_u32()
        ),
        &index_json
    ))?;

    // This function has been deprecated in favor of `score_hybrid` as of version 0.8.5.
    Spi::run(&format_hybrid_function(
        &spi::quote_qualified_identifier(index_name, "rank_hybrid"),
        &format!("TABLE({} {}, rank_hybrid real)", spi::quote_identifier(key_field), key_type),
        &format!(
            "
                WITH similarity AS (
                    SELECT
                        __key_field__ as key_field,
                        CASE
                            WHEN (MAX(__similarity_query__) OVER () - MIN(__similarity_query__) OVER ()) = 0 THEN
                                0
                            ELSE
                                1 - ((__similarity_query__) - MIN(__similarity_query__) OVER ()) / 
                                (MAX(__similarity_query__) OVER () - MIN(__similarity_query__) OVER ())
                        END AS score
                    FROM {}.{}
                    {}
                    ORDER BY __similarity_query__
                    LIMIT $2
                ),
                bm25 AS (
                    SELECT 
                        id as key_field,
                        rank_bm25 as score 
                    FROM paradedb.minmax_bm25($1, NULL::{}, {})
                )
                SELECT
                    COALESCE(similarity.key_field, bm25.key_field) AS __key_field__,
                    (COALESCE(similarity.score, 0.0) * $3 + COALESCE(bm25.score, 0.0) * $4)::real AS score_hybrid
                FROM similarity
                FULL OUTER JOIN bm25 ON similarity.key_field = bm25.key_field
                ORDER BY score_hybrid DESC;
            ",
            spi::quote_identifier(schema_name),
            spi::quote_identifier(table_name),
            predicate_where_escape,
            key_type,
            key_oid.as_u32()
        ),
        &index_json
    ))?;

    Spi::run(&format!(
        "SET client_min_messages TO {}",
        spi::quote_literal(original_client_min_messages)
    ))?;

    Ok(())
}

#[pg_extern(sql = "
CREATE OR REPLACE PROCEDURE paradedb.drop_bm25(
    index_name text,
    schema_name text DEFAULT CURRENT_SCHEMA
)
LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
fn drop_bm25(index_name: &str, schema_name: Option<&str>) -> Result<()> {
    let schema_name = schema_name.unwrap_or("current_schema()");

    let oid_query = format!(
        "SELECT oid FROM pg_class WHERE relname = '{}' AND relkind = 'i'",
        format!("{}_bm25_index", index_name)
    );
    let index_oid = Spi::get_one::<pg_sys::Oid>(&oid_query)
        .expect("error looking up index in drop_bm25")
        .expect("no oid for index created in drop_bm25");

    Spi::run(&format!(
        r#"
        DO $$
        DECLARE 
            original_client_min_messages TEXT;
        BEGIN
            SELECT INTO original_client_min_messages current_setting('client_min_messages');
            SET client_min_messages TO WARNING;

            EXECUTE 'DROP INDEX IF EXISTS {}.{}'; 
            EXECUTE 'DROP SCHEMA IF EXISTS {} CASCADE';
            EXECUTE 'SET client_min_messages TO ' || quote_literal(original_client_min_messages);
        END;
        $$;
        "#,
        spi::quote_identifier(schema_name),
        spi::quote_identifier(format!("{}_bm25_index", index_name)),
        spi::quote_identifier(index_name),
    ))?;

    crate::api::search::drop_bm25_internal(index_oid);

    Ok(())
}

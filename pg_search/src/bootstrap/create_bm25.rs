use anyhow::{bail, Result};
use pgrx::prelude::*;
use pgrx::Spi;
use serde_json::{json, Value};
use std::collections::HashSet;

use super::format::format_bm25_function;
use super::format::format_empty_function;
use super::format::format_hybrid_function;

#[pg_extern(sql = "
CREATE OR REPLACE PROCEDURE paradedb.create_bm25(
    index_name text DEFAULT '',
    table_name text DEFAULT '',
    key_field text DEFAULT '',
    schema_name text DEFAULT CURRENT_SCHEMA,
    text_fields text DEFAULT '{}',
    numeric_fields text DEFAULT '{}',
    boolean_fields text DEFAULT '{}',
    json_fields text DEFAULT '{}',
    datetime_fields text DEFAULT '{}'
)
LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
#[allow(clippy::too_many_arguments)]
fn create_bm25(
    index_name: &str,
    table_name: &str,
    key_field: &str,
    schema_name: &str,
    text_fields: &str,
    numeric_fields: &str,
    boolean_fields: &str,
    json_fields: &str,
    datetime_fields: &str,
) -> Result<()> {
    let original_client_min_messages =
        Spi::get_one::<String>("SHOW client_min_messages")?.unwrap_or_default();
    Spi::run("SET client_min_messages TO WARNING")?;

    if index_name.is_empty() {
        bail!("no index_name parameter given for bm25 index");
    }

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

    let index_json = json!({
        "index_name": format!("{}_bm25_index", index_name),
        "table_name": table_name,
        "key_field": key_field,
        "schema_name": schema_name
    });

    Spi::run(&format!(
        "CREATE SCHEMA {}",
        spi::quote_identifier(index_name)
    ))?;

    let mut column_names = HashSet::new();
    column_names.insert(key_field.to_string());
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
                        column_names.insert(key.clone());
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

    Spi::run(&format!(
        "CREATE INDEX {} ON {}.{} USING bm25 ({}, {}) WITH (key_field={}, text_fields={}, numeric_fields={}, boolean_fields={}, json_fields={}, datetime_fields={});",
        spi::quote_identifier(format!("{}_bm25_index", index_name)),
        spi::quote_identifier(schema_name),
        spi::quote_identifier(table_name),
        spi::quote_identifier(key_field),
        column_names_csv,
        spi::quote_literal(key_field),
        spi::quote_literal(text_fields),
        spi::quote_literal(numeric_fields),
        spi::quote_literal(boolean_fields),
        spi::quote_literal(json_fields),
        spi::quote_literal(datetime_fields)
    ))?;

    Spi::run(&format_bm25_function(
        &spi::quote_qualified_identifier(index_name, "search"),
        &format!(
            "SETOF {}.{}",
            spi::quote_identifier(schema_name),
            spi::quote_identifier(table_name)
        ),
        &format!(
            "RETURN QUERY SELECT * FROM {}.{} WHERE {} @@@ __paradedb_search_config__",
            spi::quote_identifier(schema_name),
            spi::quote_identifier(table_name),
            spi::quote_identifier(key_field)
        ),
        &index_json,
    ))?;

    Spi::run(&format_bm25_function(
        &spi::quote_qualified_identifier(index_name, "explain"),
        "TABLE(plan text)",
        &format!(
            "RETURN QUERY EXPLAIN SELECT * FROM {}.{} WHERE {} @@@ __paradedb_search_config__",
            spi::quote_identifier(schema_name),
            spi::quote_identifier(table_name),
            spi::quote_identifier(key_field)
        ),
        &index_json,
    ))?;

    Spi::run(&format_empty_function(
        &spi::quote_qualified_identifier(index_name, "schema"),
        "TABLE(name text, field_type text, stored bool, indexed bool, fast bool, fieldnorms bool, expand_dots bool, tokenizer text, record text, normalizer text)",
        &format!("RETURN QUERY SELECT * FROM paradedb.schema_bm25({})", spi::quote_literal(index_name))
    ))?;

    Spi::run(&format_hybrid_function(
        &spi::quote_qualified_identifier(index_name, "rank_hybrid"),
        &format!("TABLE({} bigint, rank_hybrid real)", spi::quote_identifier(key_field)),
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
                        rank_bm25 as score 
                    FROM paradedb.minmax_bm25($1)
                )
                SELECT
                    COALESCE(similarity.key_field, bm25.key_field) AS __key_field__,
                    (COALESCE(similarity.score, 0.0) * $3 + COALESCE(bm25.score, 0.0) * $4)::real AS score_hybrid
                FROM similarity
                FULL OUTER JOIN bm25 ON similarity.key_field = bm25.key_field
                ORDER BY score_hybrid DESC;
            ",
            spi::quote_identifier(schema_name),
            spi::quote_identifier(table_name)
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

    Spi::run(&format!(
        r#"
        DO $$
        DECLARE 
            original_client_min_messages TEXT;
        BEGIN
            SELECT INTO original_client_min_messages current_setting('client_min_messages');
            SET client_min_messages TO WARNING;

            EXECUTE 'DROP INDEX IF EXISTS {}.{}_bm25_index'; 
            EXECUTE 'DROP SCHEMA IF EXISTS {} CASCADE';
            PERFORM paradedb.drop_bm25_internal({});

            EXECUTE 'SET client_min_messages TO ' || quote_literal(original_client_min_messages);
        END;
        $$;
        "#,
        spi::quote_identifier(schema_name),
        spi::quote_identifier(index_name),
        spi::quote_identifier(index_name),
        spi::quote_literal(index_name)
    ))?;

    Ok(())
}

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
use pgrx::{JsonB, PgRelation, Spi};
use serde_json::Value;
use std::collections::HashSet;
use uuid::Uuid;

use crate::index::{SearchFs, SearchIndex, WriterDirectory};
use crate::postgres::utils::{index_oid_from_index_name, relfilenode_from_index_oid};

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
    range_fields jsonb DEFAULT '{}',
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
    range_fields: JsonB,
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
        &serde_json::to_string(&range_fields)?,
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
    range_fields: &str,
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

    let is_partitioned_query = format!(
        "SELECT EXISTS (SELECT 1 FROM pg_inherits WHERE inhparent = '{}.{}'::regclass)",
        spi::quote_identifier(schema_name),
        spi::quote_identifier(table_name),
    );
    let partitioned = Spi::get_one::<bool>(&is_partitioned_query)?.ok_or_else(|| {
        anyhow::anyhow!(
            "Could not check if {}.{} is partitioned",
            schema_name,
            table_name
        )
    })?;

    if partitioned {
        bail!(
            "Creating BM25 indexes over partitioned tables is a ParadeDB enterprise feature. Contact support@paradedb.com for access."
        );
    }

    if text_fields == "{}"
        && numeric_fields == "{}"
        && boolean_fields == "{}"
        && json_fields == "{}"
        && range_fields == "{}"
        && datetime_fields == "{}"
    {
        bail!(
            "no text_fields, numeric_fields, boolean_fields, json_fields, range_fields, or datetime_fields were specified for index {}",
            spi::quote_literal(index_name)
        );
    }

    let mut column_names = HashSet::new();
    for fields in [
        text_fields,
        numeric_fields,
        boolean_fields,
        json_fields,
        range_fields,
        datetime_fields,
    ] {
        match json5::from_str::<Value>(fields) {
            Ok(obj) => {
                if let Value::Object(map) = obj {
                    for key in map.keys() {
                        if key == key_field {
                            bail!(
                                "key_field {} cannot be included in text_fields, numeric_fields, boolean_fields, json_fields, range_fields, or datetime_fields",
                                spi::quote_identifier(key.clone())
                            );
                        }

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
        "CREATE INDEX {} ON {}.{} USING bm25 ({}, {}) WITH (key_field={}, text_fields={}, numeric_fields={}, boolean_fields={}, json_fields={}, range_fields={}, datetime_fields={}, uuid={}) {};",
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
        spi::quote_literal(range_fields),
        spi::quote_literal(datetime_fields),
        spi::quote_identifier(index_uuid.clone()),
        predicate_where))?;

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

            EXECUTE 'DROP INDEX IF EXISTS {}.{} CASCADE'; 
            EXECUTE 'DROP SCHEMA IF EXISTS {} CASCADE';
            EXECUTE 'SET client_min_messages TO ' || quote_literal(original_client_min_messages);
        END;
        $$;
        "#,
        spi::quote_identifier(schema_name),
        spi::quote_identifier(format!("{}_bm25_index", index_name)),
        spi::quote_identifier(index_name),
    ))?;

    Ok(())
}

#[pg_extern(sql = "
CREATE OR REPLACE PROCEDURE paradedb.delete_bm25_index_by_oid(
    index_oid oid
)
LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
unsafe fn delete_bm25_index_by_oid(index_oid: pg_sys::Oid) -> Result<()> {
    let database_oid = crate::MyDatabaseId();
    crate::api::search::drop_bm25_internal(database_oid, index_oid.as_u32());
    Ok(())
}

#[pg_extern]
fn index_size(index_name: &str) -> Result<i64> {
    let index_oid = index_oid_from_index_name(index_name);
    let database_oid = crate::MyDatabaseId();
    let relfilenode = relfilenode_from_index_oid(index_oid.as_u32());

    // Create a WriterDirectory with the obtained index_oid
    let writer_directory =
        WriterDirectory::from_oids(database_oid, index_oid.as_u32(), relfilenode.as_u32());

    // Call the total_size method to get the size in bytes
    let total_size = writer_directory.total_size()?;

    Ok(total_size as i64)
}

#[pg_extern]
fn index_info(
    index: PgRelation,
) -> anyhow::Result<
    TableIterator<
        'static,
        (
            name!(segno, String),
            name!(byte_size, i64),
            name!(num_docs, i64),
            name!(num_deleted, i64),
        ),
    >,
> {
    let index = unsafe { PgRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _) };
    let index_oid = index.oid();
    let database_oid = crate::MyDatabaseId();
    let relfilenode = relfilenode_from_index_oid(index_oid.as_u32());

    // Create a WriterDirectory with the obtained index_oid
    let writer_directory =
        WriterDirectory::from_oids(database_oid, index_oid.as_u32(), relfilenode.as_u32());

    let index = SearchIndex::from_disk(&writer_directory)?;
    let data = index
        .underlying_index
        .searchable_segment_metas()?
        .into_iter()
        .map(|meta| {
            let segno = meta.id().short_uuid_string();
            let byte_size = meta
                .list_files()
                .into_iter()
                .map(|file| {
                    let mut full_path = writer_directory.tantivy_dir_path(false).unwrap().0;
                    full_path.push(file);

                    if full_path.exists() {
                        full_path
                            .metadata()
                            .map(|metadata| metadata.len())
                            .unwrap_or(0)
                    } else {
                        0
                    }
                })
                .sum::<u64>() as i64;
            let num_docs = meta.num_docs() as i64;
            let num_deleted = meta.num_deleted_docs() as i64;

            (segno, byte_size, num_docs, num_deleted)
        })
        .collect::<Vec<_>>();

    Ok(TableIterator::new(data))
}

extension_sql!(
    r#"
    CREATE OR REPLACE FUNCTION paradedb.drop_bm25_event_trigger()
    RETURNS event_trigger AS $$
    DECLARE
        obj RECORD;
    BEGIN
        FOR obj IN SELECT * FROM pg_event_trigger_dropped_objects() LOOP
            IF obj.object_type = 'index' THEN
                CALL paradedb.delete_bm25_index_by_oid(obj.objid);
            END IF;
        END LOOP;
    END;
    $$ LANGUAGE plpgsql;
    
    CREATE EVENT TRIGGER trigger_on_sql_index_drop
    ON sql_drop
    EXECUTE FUNCTION paradedb.drop_bm25_event_trigger();
    "#
    name = "create_drop_bm25_event_trigger",
    requires = [ delete_bm25_index_by_oid ]
);

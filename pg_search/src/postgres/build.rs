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

use crate::api::FieldName;
use crate::index::mvcc::MvccSatisfies;
use crate::postgres::build_parallel::build_index;
use crate::postgres::options::BM25IndexOptions;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::utils::{extract_field_attributes, ExtractedFieldAttribute};
use crate::schema::{SearchFieldConfig, SearchFieldType};
use anyhow::Result;
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::*;
use tantivy::schema::Schema;
use tantivy::{Index, IndexSettings, IndexSortByField, Order};
use tokenizers::SearchTokenizer;

#[pg_guard]
pub extern "C-unwind" fn ambuild(
    heaprel: pg_sys::Relation,
    indexrel: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
) -> *mut pg_sys::IndexBuildResult {
    let heap_relation = unsafe { PgSearchRelation::from_pg(heaprel) };
    let index_relation = unsafe { PgSearchRelation::from_pg(indexrel) };

    unsafe {
        build_empty(&index_relation);
    }

    // ensure we only allow one `USING bm25` index on this relation, accounting for a REINDEX
    // and accounting for CONCURRENTLY.
    unsafe {
        let index_tuple = &(*index_relation.rd_index);
        let is_reindex = !index_tuple.indisvalid;
        let is_concurrent = (*index_info).ii_Concurrent;

        if !is_reindex {
            for existing_index in heap_relation.indices(pg_sys::AccessShareLock as _) {
                if existing_index.oid() == index_relation.oid() {
                    // the index we're about to build already exists on the table.
                    continue;
                }

                if is_bm25_index(&existing_index) && !is_concurrent {
                    panic!("a relation may only have one `USING bm25` index");
                }
            }
        }
    }

    unsafe {
        let heap_tuples = build_index(
            heap_relation,
            index_relation.clone(),
            (*index_info).ii_Concurrent,
        )
        .unwrap_or_else(|e| panic!("{e}"));

        pgrx::debug1!("build_index: flushing buffers");
        pg_sys::FlushRelationBuffers(indexrel);

        let mut result = PgBox::<pg_sys::IndexBuildResult>::alloc0();
        result.heap_tuples = heap_tuples;
        result.index_tuples = heap_tuples;
        result.into_pg()
    }
}

#[pg_guard]
pub unsafe extern "C-unwind" fn ambuildempty(index_relation: pg_sys::Relation) {
    build_empty(&PgSearchRelation::from_pg(index_relation));
}

unsafe fn build_empty(index_relation: &PgSearchRelation) {
    unsafe {
        MetaPage::init(index_relation);
    }

    validate_index_config(index_relation);

    create_index(index_relation).unwrap_or_else(|e| panic!("{e}"));
}

unsafe fn validate_index_config(index_relation: &PgSearchRelation) {
    // quick check to make sure we have "WITH" options
    if index_relation.rd_options.is_null() {
        panic!("{}", BM25IndexOptions::MISSING_KEY_FIELD_CONFIG);
    }

    let options = index_relation.options();
    let key_field_name = options.key_field_name();
    let key_field_config = options.field_config_or_default(&key_field_name);

    // warn when the `raw` tokenizer is used for the key_field
    #[allow(deprecated)]
    if key_field_config
        .tokenizer()
        .map(|tokenizer| matches!(tokenizer, SearchTokenizer::Raw(_)))
        .unwrap_or(false)
    {
        ErrorReport::new(
            PgSqlErrorCode::ERRCODE_WARNING_DEPRECATED_FEATURE,
            "the `raw` tokenizer is deprecated",
            function_name!(),
        )
            .set_detail("the `raw` tokenizer is deprecated as it also lowercases and truncates the input and this is probably not what you want for you key_field")
            .set_hint("use `keyword` instead").report(PgLogLevel::WARNING);
    }

    let options = index_relation.options();
    let text_configs = options.text_config();
    for (field_name, config) in text_configs.iter().flatten() {
        validate_field_config(field_name, &key_field_name, config, options, |t| {
            matches!(t, SearchFieldType::Text(_) | SearchFieldType::Uuid(_))
        });
    }

    let inet_configs = options.inet_config();
    for (field_name, config) in inet_configs.iter().flatten() {
        validate_field_config(field_name, &key_field_name, config, options, |t| {
            matches!(t, SearchFieldType::Inet(_))
        });
    }

    let numeric_configs = options.numeric_config();
    for (field_name, config) in numeric_configs.iter().flatten() {
        validate_field_config(field_name, &key_field_name, config, options, |t| {
            matches!(
                t,
                SearchFieldType::I64(_) | SearchFieldType::U64(_) | SearchFieldType::F64(_)
            )
        });
    }

    let boolean_configs = options.boolean_config();
    for (field_name, config) in boolean_configs.iter().flatten() {
        validate_field_config(field_name, &key_field_name, config, options, |t| {
            matches!(t, SearchFieldType::Bool(_))
        });
    }

    let json_configs = options.json_config();
    for (field_name, config) in json_configs.iter().flatten() {
        validate_field_config(field_name, &key_field_name, config, options, |t| {
            matches!(t, SearchFieldType::Json(_))
        });
    }

    let range_configs = options.range_config();
    for (field_name, config) in range_configs.iter().flatten() {
        validate_field_config(field_name, &key_field_name, config, options, |t| {
            matches!(t, SearchFieldType::Range(_))
        });
    }

    let datetime_configs = options.datetime_config();
    for (field_name, config) in datetime_configs.iter().flatten() {
        validate_field_config(field_name, &key_field_name, config, options, |t| {
            matches!(t, SearchFieldType::Date(_))
        });
    }
}

fn validate_field_config(
    field_name: &FieldName,
    key_field_name: &FieldName,
    config: &SearchFieldConfig,
    options: &BM25IndexOptions,
    matches: fn(&SearchFieldType) -> bool,
) {
    if field_name.is_ctid() {
        panic!("the name `ctid` is reserved by pg_search");
    }

    if field_name.root() == key_field_name.root() {
        panic!(
            "cannot override BM25 configuration for key_field '{field_name}', you must use an aliased field name and 'column' configuration key"
        );
    }

    if let Some(alias) = config.alias() {
        if options
            .get_field_type(&FieldName::from(alias.to_string()))
            .is_none()
        {
            panic!(
                "the column `{alias}` referenced by the field configuration for '{field_name}' does not exist"
            );
        }

        let config = options.field_config_or_default(&FieldName::from(alias.to_string()));
        if config.alias().is_some() {
            panic!("the column `{alias}` cannot alias an already aliased column");
        }
    }

    let field_name = config.alias().unwrap_or(field_name);
    let field_type = options
        .get_field_type(&FieldName::from(field_name.to_string()))
        .unwrap_or_else(|| panic!("the column `{field_name}` does not exist in the table"));
    if !matches(&field_type) {
        panic!("`{field_name}` was configured with the wrong type");
    }
}

pub fn is_bm25_index(indexrel: &PgSearchRelation) -> bool {
    indexrel.rd_amhandler == bm25_amhandler_oid().unwrap_or_default()
}

fn bm25_amhandler_oid() -> Option<pg_sys::Oid> {
    unsafe {
        let name = pg_sys::Datum::from(c"bm25".as_ptr());
        let pg_am_entry = pg_sys::SearchSysCache1(pg_sys::SysCacheIdentifier::AMNAME as _, name);
        if pg_am_entry.is_null() {
            return None;
        }

        let mut is_null = false;
        let datum = pg_sys::SysCacheGetAttr(
            pg_sys::SysCacheIdentifier::AMNAME as _,
            pg_am_entry,
            pg_sys::Anum_pg_am_amhandler as _,
            &mut is_null,
        );
        let oid = pg_sys::Oid::from_datum(datum, is_null);
        pg_sys::ReleaseSysCache(pg_am_entry);
        oid
    }
}

fn create_index(index_relation: &PgSearchRelation) -> Result<()> {
    let options = index_relation.options();
    let mut builder = Schema::builder();

    for (name, ExtractedFieldAttribute { tantivy_type, .. }) in
        unsafe { extract_field_attributes(index_relation.as_ptr()) }
    {
        let config = options.field_config_or_default(&name);

        match tantivy_type {
            SearchFieldType::Text(_) => builder.add_text_field(name.as_ref(), config.clone()),
            SearchFieldType::Uuid(_) => builder.add_text_field(name.as_ref(), config.clone()),
            SearchFieldType::Inet(_) => builder.add_ip_addr_field(name.as_ref(), config.clone()),
            SearchFieldType::I64(_) => builder.add_i64_field(name.as_ref(), config.clone()),
            SearchFieldType::U64(_) => builder.add_u64_field(name.as_ref(), config.clone()),
            SearchFieldType::F64(_) => builder.add_f64_field(name.as_ref(), config.clone()),
            SearchFieldType::Bool(_) => builder.add_bool_field(name.as_ref(), config.clone()),
            SearchFieldType::Json(_) => builder.add_json_field(name.as_ref(), config.clone()),
            SearchFieldType::Range(_) => builder.add_json_field(name.as_ref(), config.clone()),
            SearchFieldType::Date(_) => builder.add_date_field(name.as_ref(), config.clone()),
        };
    }

    // Now add any aliased fields
    for (name, config) in options.aliased_text_configs() {
        builder.add_text_field(name.as_ref(), config.clone());
    }
    for (name, config) in options.aliased_json_configs() {
        builder.add_json_field(name.as_ref(), config.clone());
    }

    // Add ctid field
    builder.add_u64_field(
        "ctid",
        options.field_config_or_default(&FieldName::from("ctid")),
    );

    let schema = builder.build();
    let directory = MvccSatisfies::Snapshot.directory(index_relation);
    let settings = IndexSettings {
        docstore_compress_dedicated_thread: false,
        sort_by_field: Some(IndexSortByField {
            field: "ctid".to_string(),
            order: Order::Asc,
        }),
        ..IndexSettings::default()
    };
    let _ = Index::create(directory, schema, settings)?;
    Ok(())
}

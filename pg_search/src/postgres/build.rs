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

use crate::api::FieldName;
use crate::index::mvcc::MvccSatisfies;
use crate::postgres::build_parallel::build_index;
use crate::postgres::options::BM25IndexOptions;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::utils::{extract_field_attributes, ExtractedFieldAttribute};
use crate::schema::{SearchFieldConfig, SearchFieldType, SearchIndexSchema};
use anyhow::Result;
use pgrx::*;
use tantivy::schema::Schema;
use tantivy::{Index, IndexSettings};
use tokenizers::SearchTokenizer;

#[pg_guard]
pub extern "C-unwind" fn ambuild(
    heaprel: pg_sys::Relation,
    indexrel: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
) -> *mut pg_sys::IndexBuildResult {
    let heap_relation = unsafe { PgSearchRelation::from_pg(heaprel) };
    let mut index_relation = unsafe { PgSearchRelation::from_pg(indexrel) };
    index_relation.set_is_create_index();

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
                SearchFieldType::I64(_)
                    | SearchFieldType::U64(_)
                    | SearchFieldType::F64(_)
                    | SearchFieldType::Numeric64(_, _)
                    | SearchFieldType::NumericBytes(..)
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
        match config {
            // we allow the user to change a TEXT key_field tokenizer to "keyword"
            SearchFieldConfig::Text { tokenizer: SearchTokenizer::Keyword, .. } => {
                // noop
            }

            // but not to anything else
            _ => panic!("cannot override BM25 configuration for key_field '{field_name}', you must use an aliased field name and 'column' configuration key")
        }
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
        .unwrap_or_else(|| panic!("the column `{field_name}` does not exist in the USING clause"));
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

    for (
        name,
        ExtractedFieldAttribute {
            tantivy_type,
            normalizer,
            ..
        },
    ) in unsafe { extract_field_attributes(index_relation.as_ptr()) }
    {
        let mut config = options.field_config_or_default(&name);
        config.set_normalizer(normalizer);
        config.prepare_tokenizer(index_relation.is_create_index());

        match tantivy_type {
            SearchFieldType::Text(_) => builder.add_text_field(name.as_ref(), config.clone()),
            SearchFieldType::Tokenized(_, _, inner_typoid)
                if inner_typoid == pg_sys::JSONOID || inner_typoid == pg_sys::JSONBOID =>
            {
                builder.add_json_field(name.as_ref(), config.clone())
            }
            SearchFieldType::Tokenized(..) => builder.add_text_field(name.as_ref(), config.clone()),
            SearchFieldType::Uuid(_) => builder.add_text_field(name.as_ref(), config.clone()),
            SearchFieldType::Inet(_) => builder.add_ip_addr_field(name.as_ref(), config.clone()),
            SearchFieldType::I64(_) => builder.add_i64_field(name.as_ref(), config.clone()),
            SearchFieldType::U64(_) => builder.add_u64_field(name.as_ref(), config.clone()),
            SearchFieldType::F64(_) => builder.add_f64_field(name.as_ref(), config.clone()),
            SearchFieldType::Bool(_) => builder.add_bool_field(name.as_ref(), config.clone()),
            SearchFieldType::Json(_) => builder.add_json_field(name.as_ref(), config.clone()),
            SearchFieldType::Range(_) => builder.add_json_field(name.as_ref(), config.clone()),
            SearchFieldType::Date(_) => builder.add_date_field(name.as_ref(), config.clone()),
            // NUMERIC with precision <= 18: stored as I64 with fixed-point scaling
            SearchFieldType::Numeric64(_, _) => {
                builder.add_i64_field(name.as_ref(), config.clone())
            }
            // NUMERIC with precision > 18 or unlimited: stored as sortable bytes
            // We use bytes storage with lexicographically sortable encoding from decimal-bytes.
            SearchFieldType::NumericBytes(..) => {
                builder.add_bytes_field(name.as_ref(), config.clone())
            }
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

    // Configure sort_by for segment sorting
    let sort_by_field = SearchIndexSchema::build_sort_by_field(&options.sort_by(), &schema);

    let settings = IndexSettings {
        sort_by_field,
        docstore_compress_dedicated_thread: false,
        ..IndexSettings::default()
    };
    let _ = Index::create(directory, schema, settings)?;
    Ok(())
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::api::FieldName;
    use crate::postgres::options::{SortByDirection, SortByField};
    use pgrx::pg_test;
    use tantivy::index::Order;
    use tantivy::schema::{NumericOptions, Schema, FAST};

    #[pg_test]
    fn test_build_sort_by_field_empty() {
        let schema = Schema::builder().build();
        let result = SearchIndexSchema::build_sort_by_field(&[], &schema);
        assert!(result.is_none());
    }

    #[pg_test]
    fn test_build_sort_by_field_asc() {
        let mut builder = Schema::builder();
        builder.add_i64_field("score", FAST);
        let schema = builder.build();

        let sort_by = vec![SortByField::new(
            FieldName::from("score".to_string()),
            SortByDirection::Asc,
        )];

        let result = SearchIndexSchema::build_sort_by_field(&sort_by, &schema);
        assert!(result.is_some());
        let sort_field = result.unwrap();
        assert_eq!(sort_field.field, "score");
        assert_eq!(sort_field.order, Order::Asc);
    }

    #[pg_test]
    fn test_build_sort_by_field_desc() {
        let mut builder = Schema::builder();
        builder.add_i64_field("score", FAST);
        let schema = builder.build();

        let sort_by = vec![SortByField::new(
            FieldName::from("score".to_string()),
            SortByDirection::Desc,
        )];

        let result = SearchIndexSchema::build_sort_by_field(&sort_by, &schema);
        assert!(result.is_some());
        let sort_field = result.unwrap();
        assert_eq!(sort_field.field, "score");
        assert_eq!(sort_field.order, Order::Desc);
    }

    #[pg_test]
    #[should_panic(expected = "does not exist")]
    fn test_build_sort_by_field_nonexistent() {
        let schema = Schema::builder().build();

        let sort_by = vec![SortByField::new(
            FieldName::from("nonexistent".to_string()),
            SortByDirection::Asc,
        )];

        SearchIndexSchema::build_sort_by_field(&sort_by, &schema);
    }

    #[pg_test]
    #[should_panic(expected = "fast field")]
    fn test_build_sort_by_field_not_fast() {
        let mut builder = Schema::builder();
        // Add field without FAST flag
        builder.add_i64_field("score", NumericOptions::default());
        let schema = builder.build();

        let sort_by = vec![SortByField::new(
            FieldName::from("score".to_string()),
            SortByDirection::Asc,
        )];

        SearchIndexSchema::build_sort_by_field(&sort_by, &schema);
    }

    #[pg_test]
    fn test_build_sort_by_field_ctid_explicit() {
        let mut builder = Schema::builder();
        builder.add_u64_field("ctid", FAST);
        let schema = builder.build();

        // Explicit ctid sort_by
        let sort_by = vec![SortByField::new(
            FieldName::from("ctid".to_string()),
            SortByDirection::Asc,
        )];

        let result = SearchIndexSchema::build_sort_by_field(&sort_by, &schema);
        assert!(result.is_some());
        let sort_field = result.unwrap();
        assert_eq!(sort_field.field, "ctid");
        assert_eq!(sort_field.order, Order::Asc);
    }

    // Note: Multi-field validation test moved to options.rs (parse_sort_by_string)

    #[pg_test]
    fn test_tantivy_index_receives_sort_settings() {
        use tantivy::directory::RamDirectory;
        use tantivy::index::IndexSortByField;

        // Build schema with fast field
        let mut builder = Schema::builder();
        builder.add_i64_field("score", FAST);
        builder.add_text_field("name", tantivy::schema::TEXT);
        let schema = builder.build();

        // Create sort_by configuration
        let sort_by_field = Some(IndexSortByField {
            field: "score".to_string(),
            order: Order::Desc,
        });

        // Create index with sort settings
        let settings = IndexSettings {
            sort_by_field,
            docstore_compress_dedicated_thread: false,
            ..IndexSettings::default()
        };

        let directory = RamDirectory::create();
        let index = Index::create(directory, schema, settings).unwrap();

        // Verify settings were stored
        let stored_settings = index.settings();
        assert!(stored_settings.sort_by_field.is_some());
        let sort_field = stored_settings.sort_by_field.as_ref().unwrap();
        assert_eq!(sort_field.field, "score");
        assert_eq!(sort_field.order, Order::Desc);
    }
}

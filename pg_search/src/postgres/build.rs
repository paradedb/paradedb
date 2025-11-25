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
use crate::postgres::utils::{ExtractedFieldAttribute, extract_field_attributes};
use crate::schema::{SearchFieldConfig, SearchFieldType};
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

unsafe fn check_first_column_has_unique_index(heap_relation: &PgSearchRelation, first_field: &FieldName) -> bool {
    // Get list of indexes on this relation
    let index_list = pg_sys::RelationGetIndexList(heap_relation.as_ptr());
    if index_list.is_null() {
        return false;
    }
    
    let mut has_unique = false;
    
    // Use pgrx's PgList to iterate through indexes
    let pg_list = PgList::<pg_sys::Oid>::from_pg(index_list);
    for index_oid in pg_list.iter_oid() {
        if index_oid == pg_sys::InvalidOid {
            continue;
        }
        
        // Get pg_index row for this index
        let index_tuple = pg_sys::SearchSysCache1(pg_sys::SysCacheIdentifier::INDEXRELID as _, index_oid.into());
        if index_tuple.is_null() {
            continue;
        }
        
        let index_form = pg_sys::GETSTRUCT(index_tuple) as *const pg_sys::FormData_pg_index;
        
        // Check if index is unique and valid (including primary keys)
        if (*index_form).indisunique && (*index_form).indisvalid {
            // Check if this is a single-column index on our field
            if (*index_form).indnatts == 1 {
                let attno_ptr = (*index_form).indkey.values.as_ptr();
                let attno = *attno_ptr;
                
                // Get column name from attribute number
                let tupdesc = heap_relation.tuple_desc();
                if attno > 0 && (attno as usize) <= tupdesc.len() {
                    let column_name = tupdesc.get((attno - 1) as usize).unwrap().name();
                    if column_name == first_field.as_ref() {
                        has_unique = true;
                    }
                }
            }
        }
        
        pg_sys::ReleaseSysCache(index_tuple);
        
        if has_unique {
            break;
        }
    }
    
    has_unique
}

unsafe fn validate_index_config(index_relation: &PgSearchRelation) {
    // Check if key_field was explicitly provided (show deprecation warning)
    if !index_relation.rd_options.is_null() {
        let options = index_relation.options();
        if let Some(_) = options.options_data().key_field_name() {
            pgrx::notice!("WITH (key_field='...') is deprecated. The first column in the index definition is automatically used as the key field.");
        }
    }
    
    // Always use custom logic - check that first column has unique constraint
    let key_field_name = {
        let first_field = unsafe { 
            crate::postgres::options::get_first_index_field_from_relation(index_relation.as_ptr())
        }.unwrap_or_else(|e| panic!("{}", e));
        
        if let Some(heap_relation) = index_relation.heap_relation() {
            if check_first_column_has_unique_index(&heap_relation, &first_field) {
                first_field
            } else {
                panic!("First column '{}' in index must have a unique constraint (primary key or unique index)", 
                       first_field.as_ref());
            }
        } else {
            panic!("Could not access table information");
        }
    };

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
        match config {
            // we allow the user to change a TEXT key_field tokenizer to "keyword"
            SearchFieldConfig::Text {
                tokenizer: SearchTokenizer::Keyword,
                ..
            } => {
                // noop
            }

            // but not to anything else
            _ => panic!(
                "cannot override BM25 configuration for key_field '{field_name}', you must use an aliased field name and 'column' configuration key"
            ),
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
        ..IndexSettings::default()
    };
    let _ = Index::create(directory, schema, settings)?;
    Ok(())
}

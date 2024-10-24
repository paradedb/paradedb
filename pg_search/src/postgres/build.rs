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

use crate::index::WriterDirectory;
use crate::index::{SearchIndex, WriterResources};
use crate::postgres::index::relfilenode_from_pg_relation;
use crate::postgres::insert::init_insert_state;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::postgres::utils::row_to_search_document;
use crate::schema::{IndexRecordOption, SearchFieldConfig, SearchFieldName, SearchFieldType};
use pg_sys::BuiltinOid;
use pgrx::*;
use std::collections::HashMap;
use std::ffi::CStr;
use std::time::Instant;
use tokenizers::manager::SearchTokenizerFilters;
use tokenizers::{SearchNormalizer, SearchTokenizer};

// For now just pass the count on the build callback state
struct BuildState {
    count: usize,
    memctx: PgMemoryContexts,
    index_info: *mut pg_sys::IndexInfo,
    tupdesc: PgTupleDesc<'static>,
    start: Instant,
}

impl BuildState {
    fn new(indexrel: &PgRelation, index_info: *mut pg_sys::IndexInfo) -> Self {
        BuildState {
            count: 0,
            memctx: PgMemoryContexts::new("pg_search_index_build"),
            index_info,
            tupdesc: unsafe { PgTupleDesc::from_pg_copy(indexrel.rd_att) },
            start: Instant::now(),
        }
    }
}

#[pg_guard]
pub extern "C" fn ambuild(
    heaprel: pg_sys::Relation,
    indexrel: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
) -> *mut pg_sys::IndexBuildResult {
    let heap_relation = unsafe { PgRelation::from_pg(heaprel) };
    let index_relation = unsafe { PgRelation::from_pg(indexrel) };
    let index_oid = index_relation.oid();
    let database_oid = crate::MyDatabaseId();
    let relfilenode = relfilenode_from_pg_relation(&index_relation);

    // ensure we only allow one `USING bm25` index on this relation, accounting for a REINDEX
    for existing_index in heap_relation.indices(pg_sys::AccessShareLock as _) {
        if existing_index.oid() == index_oid {
            // the index we're about to build already exists on the table.
            // we're likely here as a result of REINDEX
            continue;
        }

        if is_bm25_index(&existing_index) {
            panic!("a relation may only have one `USING bm25` index");
        }
    }

    let rdopts: PgBox<SearchIndexCreateOptions> = if !index_relation.rd_options.is_null() {
        unsafe { PgBox::from_pg(index_relation.rd_options as *mut SearchIndexCreateOptions) }
    } else {
        let ops = unsafe { PgBox::<SearchIndexCreateOptions>::alloc0() };
        ops.into_pg_boxed()
    };

    // Create a map from column name to column type. We'll use this to verify that index
    // configurations passed by the user reference the correct types for each column.
    let name_type_map: HashMap<SearchFieldName, (SearchFieldType, PgOid)> = heap_relation
        .tuple_desc()
        .into_iter()
        .filter_map(|attribute| {
            let attname = attribute.name();
            let attribute_type_oid = attribute.type_oid();
            let array_type = unsafe { pg_sys::get_element_type(attribute_type_oid.value()) };
            let base_oid = if array_type != pg_sys::InvalidOid {
                PgOid::from(array_type)
            } else {
                attribute_type_oid
            };
            if let Ok(search_field_type) = SearchFieldType::try_from(&base_oid) {
                Some((attname.into(), (search_field_type, base_oid)))
            } else {
                None
            }
        })
        .collect();

    // Parse and validate the index configurations for each column.
    let text_fields =
        rdopts
            .get_text_fields()
            .into_iter()
            .map(|(name, config)| match name_type_map.get(&name) {
                Some((field_type @ SearchFieldType::Text, base_oid)) => {
                    (name, config, *field_type, *base_oid)
                }
                _ => panic!("'{name}' cannot be indexed as a text field"),
            });

    let numeric_fields = rdopts
        .get_numeric_fields()
        .into_iter()
        .map(|(name, config)| match name_type_map.get(&name) {
            Some((field_type @ SearchFieldType::U64, base_oid))
            | Some((field_type @ SearchFieldType::I64, base_oid))
            | Some((field_type @ SearchFieldType::F64, base_oid)) => {
                (name, config, *field_type, *base_oid)
            }
            _ => panic!("'{name}' cannot be indexed as a numeric field"),
        });

    let boolean_fields = rdopts
        .get_boolean_fields()
        .into_iter()
        .map(|(name, config)| match name_type_map.get(&name) {
            Some((field_type @ SearchFieldType::Bool, base_oid)) => {
                (name, config, *field_type, *base_oid)
            }
            _ => panic!("'{name}' cannot be indexed as a boolean field"),
        });

    let json_fields =
        rdopts
            .get_json_fields()
            .into_iter()
            .map(|(name, config)| match name_type_map.get(&name) {
                Some((field_type @ SearchFieldType::Json, base_oid)) => {
                    (name, config, *field_type, *base_oid)
                }
                _ => panic!("'{name}' cannot be indexed as a JSON field"),
            });

    let range_fields = rdopts.get_range_fields().into_iter().map(|(name, config)| {
        match name_type_map.get(&name) {
            Some((field_type @ SearchFieldType::Range, base_oid)) => {
                (name, config, *field_type, *base_oid)
            }
            _ => panic!("'{name}' cannot be indexed as a range field"),
        }
    });

    let datetime_fields = rdopts
        .get_datetime_fields()
        .into_iter()
        .map(|(name, config)| match name_type_map.get(&name) {
            Some((field_type @ SearchFieldType::Date, base_oid)) => {
                (name, config, *field_type, *base_oid)
            }
            _ => panic!("'{name}' cannot be indexed as a datetime field"),
        });

    let key_field = rdopts.get_key_field().expect("must specify key field");
    let (key_field_type, key_field_oid) = match name_type_map.get(&key_field) {
        Some(field_type) => field_type,
        None => panic!("key field does not exist"),
    };
    let key_config = match key_field_type {
        SearchFieldType::I64 | SearchFieldType::U64 | SearchFieldType::F64 => {
            SearchFieldConfig::Numeric {
                indexed: true,
                fast: true,
                stored: true,
            }
        }
        SearchFieldType::Text => SearchFieldConfig::Text {
            indexed: true,
            fast: true,
            stored: true,
            fieldnorms: false,
            tokenizer: SearchTokenizer::Raw(SearchTokenizerFilters::default()),
            record: IndexRecordOption::Basic,
            normalizer: SearchNormalizer::Raw,
        },
        SearchFieldType::Json => SearchFieldConfig::Json {
            indexed: true,
            fast: true,
            stored: true,
            expand_dots: false,
            tokenizer: SearchTokenizer::Raw(SearchTokenizerFilters::default()),
            record: IndexRecordOption::Basic,
            normalizer: SearchNormalizer::Raw,
        },
        SearchFieldType::Range => SearchFieldConfig::Range { stored: true },
        SearchFieldType::Bool => SearchFieldConfig::Boolean {
            indexed: true,
            fast: true,
            stored: true,
        },
        SearchFieldType::Date => SearchFieldConfig::Date {
            indexed: true,
            fast: true,
            stored: true,
        },
    };

    // Concatenate the separate lists of fields.
    let fields: Vec<_> = text_fields
        .chain(numeric_fields)
        .chain(boolean_fields)
        .chain(json_fields)
        .chain(range_fields)
        .chain(datetime_fields)
        .chain(std::iter::once((
            key_field.clone(),
            key_config,
            *key_field_type,
            *key_field_oid,
        )))
        // "ctid" is a reserved column name in Postgres, so we don't need to worry about
        // creating a name conflict with a user-named column.
        .chain(std::iter::once((
            "ctid".into(),
            SearchFieldConfig::Ctid,
            SearchFieldType::U64,
            PgOid::BuiltIn(BuiltinOid::TIDOID),
        )))
        .collect();

    let key_field_index = fields
        .iter()
        .position(|(name, _, _, _)| name == &key_field)
        .expect("key field not found in columns"); // key field is already validated by now.

    // If there's only two fields in the vector, then those are just the Key and Ctid fields,
    // which we added above, and the user has not specified any fields to index.
    if fields.len() == 2 {
        panic!("no fields specified")
    }

    let directory =
        WriterDirectory::from_oids(database_oid, index_oid.as_u32(), relfilenode.as_u32());

    SearchIndex::create_index(directory, fields, key_field_index)
        .expect("error creating new index instance");

    let state = do_heap_scan(index_info, &heap_relation, &index_relation);
    let mut result = unsafe { PgBox::<pg_sys::IndexBuildResult>::alloc0() };
    result.heap_tuples = state.count as f64;
    result.index_tuples = state.count as f64;

    result.into_pg()
}

#[pg_guard]
pub extern "C" fn ambuildempty(_index_relation: pg_sys::Relation) {}

fn do_heap_scan<'a>(
    index_info: *mut pg_sys::IndexInfo,
    heap_relation: &'a PgRelation,
    index_relation: &'a PgRelation,
) -> BuildState {
    let mut state = BuildState::new(index_relation, index_info);
    unsafe {
        pg_sys::IndexBuildHeapScan(
            heap_relation.as_ptr(),
            index_relation.as_ptr(),
            index_info,
            Some(build_callback),
            &mut state,
        );
    }
    state
}

#[pg_guard]
unsafe extern "C" fn build_callback(
    index: pg_sys::Relation,
    ctid: pg_sys::ItemPointer,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    _tuple_is_alive: bool,
    state: *mut std::os::raw::c_void,
) {
    build_callback_internal(*ctid, values, isnull, state, index);
}

#[inline(always)]
unsafe fn build_callback_internal(
    ctid: pg_sys::ItemPointerData,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    state: *mut std::os::raw::c_void,
    indexrel: pg_sys::Relation,
) {
    check_for_interrupts!();
    let build_state = (state as *mut BuildState)
        .as_mut()
        .expect("BuildState pointer should not be null");

    let tupdesc = &build_state.tupdesc;
    let insert_state = init_insert_state(
        indexrel,
        build_state.index_info,
        WriterResources::CreateIndex,
    );
    let search_index = &(*insert_state).index;
    let writer = &(*insert_state).writer;
    let schema = &(*insert_state).index.schema;

    // In the block below, we switch to the memory context we've defined on our build
    // state, resetting it before and after. We do this because we're looking up a
    // PgTupleDesc... which is supposed to free the corresponding Postgres memory when it
    // is dropped. However, in practice, we're not seeing the memory get freed, which is
    // causing huge memory usage when building large indexes.
    //
    // By running in our own memory context, we can force the memory to be freed with
    // the call to reset().
    unsafe {
        build_state.memctx.reset();
        build_state.memctx.switch_to(|_| {
            let search_document =
                row_to_search_document(ctid, tupdesc, values, isnull, schema).unwrap_or_else(|err| {
                    panic!(
                        "error creating index entries for index '{}': {err}",
                        CStr::from_ptr((*(*indexrel).rd_rel).relname.data.as_ptr())
                            .to_string_lossy()
                    );
                });

            search_index
                .insert(writer, search_document)
                .unwrap_or_else(|err| {
                    panic!("error inserting document during build callback.  See Postgres log for more information: {err:?}")
                });
        });
        build_state.memctx.reset();

        // important to count the number of items we've indexed for proper statistics updates,
        // especially after CREATE INDEX has finished
        build_state.count += 1;

        if crate::gucs::log_create_index_progress() && build_state.count % 100_000 == 0 {
            let secs = build_state.start.elapsed().as_secs_f64();
            let rate = build_state.count as f64 / secs;
            pgrx::log!(
                "processed {} rows in {secs:.2} seconds ({rate:.2} per second)",
                build_state.count,
            );
        }
    }
}

fn is_bm25_index(indexrel: &PgRelation) -> bool {
    unsafe {
        // SAFETY:  we ensure that `indexrel.rd_indam` is non null and can be dereferenced
        !indexrel.rd_indam.is_null() && (*indexrel.rd_indam).ambuild == Some(ambuild)
    }
}

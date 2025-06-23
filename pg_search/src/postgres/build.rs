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
use crate::index::mvcc::MVCCDirectory;
use crate::postgres::build_parallel::build_index;
use crate::postgres::options::SearchIndexOptions;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{
    SegmentMetaEntry, CLEANUP_LOCK, METADATA, SCHEMA_START, SEGMENT_METAS_START, SETTINGS_START,
};
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::storage::metadata::MetaPageMut;
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use crate::postgres::utils::extract_field_attributes;
use crate::schema::{SearchFieldType, SearchIndexSchema};
use anyhow::Result;
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::*;
use tantivy::schema::Schema;
use tantivy::{Index, IndexSettings};

#[pg_guard]
pub extern "C-unwind" fn ambuild(
    heaprel: pg_sys::Relation,
    indexrel: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
) -> *mut pg_sys::IndexBuildResult {
    let heap_relation = unsafe { PgSearchRelation::from_pg(heaprel) };
    let index_relation = unsafe { PgSearchRelation::from_pg(indexrel) };

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
        ambuildempty(indexrel);

        let heap_tuples = build_index(
            heap_relation,
            index_relation.clone(),
            (*index_info).ii_Concurrent,
        )
        .unwrap_or_else(|e| panic!("{e}"));

        record_create_index_segment_ids(&index_relation).unwrap_or_else(|e| panic!("{e}"));

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
    let index_relation = PgSearchRelation::from_pg(index_relation);

    unsafe {
        init_fixed_buffers(&index_relation);
    }

    create_index(&index_relation).unwrap_or_else(|e| panic!("{e}"));

    // warn that the `raw` tokenizer is deprecated
    let schema = SearchIndexSchema::open(&index_relation).unwrap_or_else(|e| panic!("{e}"));
    for search_field in schema.search_fields() {
        #[allow(deprecated)]
        if search_field.uses_raw_tokenizer() {
            ErrorReport::new(
                    PgSqlErrorCode::ERRCODE_WARNING_DEPRECATED_FEATURE,
                    "the `raw` tokenizer is deprecated",
                    function_name!(),
                )
                    .set_detail("the `raw` tokenizer is deprecated as it also lowercases and truncates the input and this is probably not what you want")
                    .set_hint("use `keyword` instead").report(PgLogLevel::WARNING);
        }
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

unsafe fn init_fixed_buffers(index_relation: &crate::postgres::rel::PgSearchRelation) {
    let mut bman = BufferManager::new(index_relation);

    // Init merge lock buffer
    let mut merge_lock = bman.new_buffer();
    assert_eq!(merge_lock.number(), METADATA);
    merge_lock.init_page();

    // Init cleanup lock buffer
    let mut cleanup_lock = bman.new_buffer();
    assert_eq!(cleanup_lock.number(), CLEANUP_LOCK);
    cleanup_lock.init_page();

    // initialize all the other required buffers
    let schema = LinkedBytesList::create(index_relation);
    let settings = LinkedBytesList::create(index_relation);
    let segment_metas = LinkedItemList::<SegmentMetaEntry>::create(index_relation);

    assert_eq!(schema.header_blockno, SCHEMA_START);
    assert_eq!(settings.header_blockno, SETTINGS_START);
    assert_eq!(segment_metas.header_blockno, SEGMENT_METAS_START);
}

fn create_index(index_relation: &PgSearchRelation) -> Result<()> {
    let options = unsafe { SearchIndexOptions::from_relation(index_relation) };
    let mut builder = Schema::builder();

    for (name, type_oid) in extract_field_attributes(index_relation) {
        let type_oid: PgOid = type_oid.into();
        let name = FieldName::from(name);
        let field_type: SearchFieldType = (&type_oid).try_into().unwrap_or_else(|_| {
            panic!("failed to convert attribute {} to search field type", name)
        });
        let config = options.field_config_or_default(&name);

        match field_type {
            SearchFieldType::Text(_) => builder.add_text_field(name.as_ref(), config.clone()),
            SearchFieldType::Uuid(_) => builder.add_text_field(name.as_ref(), config.clone()),
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
    let directory = MVCCDirectory::snapshot(index_relation);
    let settings = IndexSettings {
        docstore_compress_dedicated_thread: false,
        ..IndexSettings::default()
    };
    let _ = Index::create(directory, schema, settings)?;
    Ok(())
}

unsafe fn record_create_index_segment_ids(
    indexrel: &crate::postgres::rel::PgSearchRelation,
) -> anyhow::Result<()> {
    let metadata = MetaPageMut::new(indexrel);
    let directory = MVCCDirectory::snapshot(indexrel);
    let index = Index::open(directory.clone())?;
    let segment_ids = index.searchable_segment_ids()?;

    pgrx::debug1!("record_create_index_segment_ids: {:?}", segment_ids);

    metadata
        .record_create_index_segment_ids(segment_ids)
        .expect("do_heap_scan: should be able to record segment ids in merge lock");
    Ok(())
}

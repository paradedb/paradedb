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

use crate::index::IndexError;
use crate::postgres::types::TantivyValue;
use crate::schema::{SearchDocument, SearchFieldName, SearchIndexSchema};
use pgrx::itemptr::{item_pointer_get_both, item_pointer_set_all};
use pgrx::*;
use std::mem::size_of;
use std::ptr::null_mut;

const P_NEW: u32 = pg_sys::InvalidBlockNumber;
const RBM_NORMAL: u32 = pg_sys::ReadBufferMode::RBM_NORMAL;
const METADATA_BLOCKNO: pg_sys::BlockNumber = 0;
const MANAGED_BLOCKNO: pg_sys::BlockNumber = 1;

pub(crate) struct BM25SpecialData {
    next_blockno: pg_sys::BlockNumber,
    len: u32,
}

/// Finds and returns the first `USING bm25` index on the specified relation, or [`None`] if there
/// aren't any
pub fn locate_bm25_index(heaprelid: pg_sys::Oid) -> Option<PgRelation> {
    unsafe {
        let heaprel = PgRelation::open(heaprelid);
        for index in heaprel.indices(pg_sys::AccessShareLock as _) {
            if !index.rd_indam.is_null()
                && (*index.rd_indam).ambuild == Some(crate::postgres::build::ambuild)
            {
                return Some(index);
            }
        }
        None
    }
}

/// Rather than using pgrx' version of this function, we use our own, which doesn't leave 2
/// empty bytes in the middle of the 64bit representation.  A ctid being only 48bits means
/// if we leave the upper 16 bits (2 bytes) empty, tantivy will have a better chance of
/// bitpacking or compressing these values.
#[inline(always)]
pub fn item_pointer_to_u64(ctid: pg_sys::ItemPointerData) -> u64 {
    let (blockno, offno) = item_pointer_get_both(ctid);
    let blockno = blockno as u64;
    let offno = offno as u64;

    // shift the BlockNumber left 16 bits -- the length of the OffsetNumber we OR onto the end
    // pgrx's version shifts left 32, which is wasteful
    (blockno << 16) | offno
}

/// Rather than using pgrx' version of this function, we use our own, which doesn't leave 2
/// empty bytes in the middle of the 64bit representation.  A ctid being only 48bits means
/// if we leave the upper 16 bits (2 bytes) empty, tantivy will have a better chance of
/// bitpacking or compressing these values.
#[inline(always)]
pub fn u64_to_item_pointer(value: u64, tid: &mut pg_sys::ItemPointerData) {
    // shift right 16 bits to pop off the OffsetNumber, leaving only the BlockNumber
    // pgrx's version must shift right 32 bits to be in parity with `item_pointer_to_u64()`
    let blockno = (value >> 16) as pg_sys::BlockNumber;
    let offno = value as pg_sys::OffsetNumber;
    item_pointer_set_all(tid, blockno, offno);
}

pub unsafe fn row_to_search_document(
    ctid: pg_sys::ItemPointerData,
    tupdesc: &PgTupleDesc,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    schema: &SearchIndexSchema,
) -> Result<SearchDocument, IndexError> {
    let mut document = schema.new_document();

    // Create a vector of index entries from the postgres row.
    for (attno, attribute) in tupdesc.iter().enumerate() {
        let attname = attribute.name().to_string();
        let attribute_type_oid = attribute.type_oid();

        // If we can't lookup the attribute name in the field_lookup parameter,
        // it means that this field is not part of the index. We should skip it.
        let search_field =
            if let Some(index_field) = schema.get_search_field(&attname.clone().into()) {
                index_field
            } else {
                continue;
            };

        let array_type = unsafe { pg_sys::get_element_type(attribute_type_oid.value()) };
        let (base_oid, is_array) = if array_type != pg_sys::InvalidOid {
            (PgOid::from(array_type), true)
        } else {
            (attribute_type_oid, false)
        };

        let is_json = matches!(
            base_oid,
            PgOid::BuiltIn(pg_sys::BuiltinOid::JSONBOID | pg_sys::BuiltinOid::JSONOID)
        );

        let datum = *values.add(attno);
        let isnull = *isnull.add(attno);

        let SearchFieldName(key_field_name) = schema.key_field().name;
        if key_field_name == attname && isnull {
            return Err(IndexError::KeyIdNull(key_field_name));
        }

        if isnull {
            continue;
        }

        if is_array {
            for value in TantivyValue::try_from_datum_array(datum, base_oid)? {
                document.insert(search_field.id, value.tantivy_schema_value());
            }
        } else if is_json {
            for value in TantivyValue::try_from_datum_json(datum, base_oid)? {
                document.insert(search_field.id, value.tantivy_schema_value());
            }
        } else {
            document.insert(
                search_field.id,
                TantivyValue::try_from_datum(datum, base_oid)?.tantivy_schema_value(),
            );
        }
    }

    // Insert the ctid value into the entries.
    let ctid_index_value = item_pointer_to_u64(ctid);
    document.insert(schema.ctid_field().id, ctid_index_value.into());

    Ok(document)
}

pub unsafe fn bm25_create_meta(index: pg_sys::Relation, forkno: i32) {
    bm25_init_page(index, P_NEW, forkno);
}

pub unsafe fn bm25_create_managed(index: pg_sys::Relation, forkno: i32) {
    bm25_init_page(index, P_NEW, forkno);
}

pub unsafe fn bm25_write_meta(index_oid: u32, metadata: &[u8]) {
    write_to_page(index_oid, METADATA_BLOCKNO, metadata);
}

pub unsafe fn bm25_write_managed(index_oid: u32, managed: &[u8]) {
    write_to_page(index_oid, MANAGED_BLOCKNO, managed);
}

pub unsafe fn read_meta(index_oid: u32) -> Vec<u8> {
    read_page_contents(index_oid, METADATA_BLOCKNO)
}

pub unsafe fn read_managed(index_oid: u32) -> Vec<u8> {
    read_page_contents(index_oid, MANAGED_BLOCKNO)
}

pub unsafe fn read_page_contents(index_oid: u32, blockno: pg_sys::BlockNumber) -> Vec<u8> {
    let index = pg_sys::relation_open(index_oid.into(), pg_sys::AccessShareLock as i32);
    let buffer = pg_sys::ReadBufferExtended(
        index,
        pg_sys::ForkNumber::MAIN_FORKNUM,
        blockno,
        RBM_NORMAL,
        null_mut(),
    );
    pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_SHARE as i32);

    let page = pg_sys::BufferGetPage(buffer);
    let item = pg_sys::PageGetItem(page, pg_sys::PageGetItemId(page, pg_sys::FirstOffsetNumber));
    let special = bm25_get_special(page);

    pg_sys::UnlockReleaseBuffer(buffer);
    pg_sys::RelationClose(index);

    Vec::from_raw_parts(
        item as *mut u8,
        (*special).len as usize,
        (*special).len as usize,
    )
}

pub unsafe fn write_to_page(index_oid: u32, blockno: pg_sys::BlockNumber, data: &[u8]) {
    let index = pg_sys::relation_open(index_oid.into(), pg_sys::AccessShareLock as i32);
    let buffer = pg_sys::ReadBufferExtended(
        index,
        pg_sys::ForkNumber::MAIN_FORKNUM,
        blockno,
        RBM_NORMAL,
        null_mut(),
    );
    pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as i32);

    let page = pg_sys::BufferGetPage(buffer);
    let contents = pg_sys::PageGetContents(page);

    let special = bm25_get_special(page);
    (*special).len = data.len() as u32;

    pg_sys::PageAddItemExtended(
        page,
        data.as_ptr() as pg_sys::Item,
        data.len(),
        pg_sys::FirstOffsetNumber,
        pg_sys::PAI_OVERWRITE as i32,
    );
    pg_sys::MarkBufferDirty(buffer);
    pg_sys::UnlockReleaseBuffer(buffer);
    pg_sys::RelationClose(index);
}

pub unsafe fn bm25_init_page(index: pg_sys::Relation, blockno: pg_sys::BlockNumber, forkno: i32) {
    let buffer = pg_sys::ReadBufferExtended(index, forkno, blockno, RBM_NORMAL, null_mut());
    pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as i32);

    let page = pg_sys::BufferGetPage(buffer);
    pg_sys::PageInit(
        page,
        pg_sys::BufferGetPageSize(buffer),
        size_of::<BM25SpecialData>(),
    );
    let special = bm25_get_special(page);
    (*special).next_blockno = pg_sys::InvalidBlockNumber;
    (*special).len = 0;

    pg_sys::MarkBufferDirty(buffer);
    pg_sys::UnlockReleaseBuffer(buffer);
}

pub unsafe fn bm25_get_special(page: pg_sys::Page) -> *mut BM25SpecialData {
    pg_sys::PageGetSpecialPointer(page) as *mut BM25SpecialData
}

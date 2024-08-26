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

use crate::postgres::types::TantivyValue;
use crate::schema::{SearchDocument, SearchFieldName, SearchIndexSchema};
use crate::writer::IndexError;
use pgrx::itemptr::{item_pointer_get_block_number, item_pointer_get_both, item_pointer_set_all};
use pgrx::pg_sys::{Buffer, BuiltinOid, ItemPointerData};
use pgrx::*;

/// Rather than using pgrx' version of this function, we use our own, which doesn't leave 2
/// empty bytes in the middle of the 64bit representation.  A ctid being only 48bits means
/// if we leave the upper 16 bits (2 bytes) empty, tantivy will have a better chance of
/// bitpacking or compressing these values.
#[inline(always)]
pub fn item_pointer_to_u64(ctid: ItemPointerData) -> u64 {
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
    ctid: ItemPointerData,
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
            PgOid::BuiltIn(BuiltinOid::JSONBOID | BuiltinOid::JSONOID)
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
    let ctid_index_value = crate::postgres::utils::item_pointer_to_u64(ctid);
    document.insert(schema.ctid_field().id, ctid_index_value.into());

    Ok(document)
}

/// Helper to manage the information necessary to validate that a "ctid" is currently visible to
/// a snapshot
pub struct VisibilityChecker {
    relation: pg_sys::Relation,
    snapshot: pg_sys::Snapshot,
    last_blockno: pg_sys::BlockNumber,
    last_buffer: pg_sys::Buffer,
    ipd: pg_sys::ItemPointerData,
}

impl Drop for VisibilityChecker {
    fn drop(&mut self) {
        unsafe {
            if self.last_buffer != pg_sys::InvalidBuffer as pg_sys::Buffer {
                pg_sys::UnlockReleaseBuffer(self.last_buffer);
            }
            // SAFETY:  `self.relation` is always a valid, open relation, created via `pg_sys::RelationGetRelation`
            pg_sys::RelationClose(self.relation);
        }
    }
}

impl VisibilityChecker {
    /// Construct a new [`VisibilityChecker`] that can validate ctid visibility against the specified
    /// `relid` in whatever the current snapshot happens to be at the time this function is called.
    pub fn new(relid: pg_sys::Oid) -> Self {
        unsafe {
            // SAFETY:  `pg_sys::RelationIdGetRelation()` will raise an ERROR if the specified
            // relation oid is not a valid relation.
            //
            // `pg_sys::GetTransactionSnapshot()` causes no concern
            Self {
                relation: pg_sys::RelationIdGetRelation(relid),
                snapshot: pg_sys::GetTransactionSnapshot(),
                last_blockno: pg_sys::InvalidBlockNumber,
                last_buffer: pg_sys::InvalidBuffer as pg_sys::Buffer,
                ipd: pg_sys::ItemPointerData::default(),
            }
        }
    }

    /// Returns true if the specified 64bit ctid is visible by the backing snapshot in the backing
    /// relation
    pub fn ctid_satisfies_snapshot(&mut self, ctid: u64) -> bool {
        unsafe {
            // Using ctid, get itempointer => buffer => page => heaptuple
            crate::postgres::utils::u64_to_item_pointer(ctid, &mut self.ipd);

            let blockno = item_pointer_get_block_number(&self.ipd);

            if blockno == self.last_blockno {
                // this ctid is on the buffer we already have locked
                return self.check_page_vis(self.last_buffer);
            } else if self.last_buffer != pg_sys::InvalidBuffer as pg_sys::Buffer {
                // this ctid is on a different buffer, so release the one we've got locked
                pg_sys::UnlockReleaseBuffer(self.last_buffer);
            }

            self.last_blockno = blockno;
            self.last_buffer = pg_sys::ReadBuffer(self.relation, self.last_blockno);

            pg_sys::LockBuffer(self.last_buffer, pg_sys::BUFFER_LOCK_SHARE as i32);

            self.check_page_vis(self.last_buffer)
        }
    }

    unsafe fn check_page_vis(&mut self, buffer: Buffer) -> bool {
        unsafe {
            let mut heap_tuple = pg_sys::HeapTupleData::default();

            // Check if heaptuple is visible
            // In Postgres, the indexam `amgettuple` calls `heap_hot_search_buffer` for its visibility check
            pg_sys::heap_hot_search_buffer(
                &mut self.ipd,
                self.relation,
                buffer,
                self.snapshot,
                &mut heap_tuple,
                std::ptr::null_mut(),
                true,
            )
        }
    }
}

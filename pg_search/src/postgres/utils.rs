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
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::*;

/// Finds and returns the first `USING bm25` index on the specified relation, or [`None`] if there
/// aren't any
pub fn locate_bm25_index(heaprelid: pg_sys::Oid) -> Option<PgRelation> {
    unsafe {
        let heaprel = PgRelation::open(heaprelid);
        for index in heaprel.indices(pg_sys::AccessShareLock as _) {
            if !(index.rd_indam.is_null())
                && (*index.rd_indam).ambuild == Some(crate::postgres::build::ambuild)
            {
                return Some(index);
            }
        }
        None
    }
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
    let ctid_index_value = pgrx::itemptr::item_pointer_to_u64(ctid);
    document.insert(schema.ctid_field().id, ctid_index_value.into());

    Ok(document)
}

/// Helper to manage the information necessary to validate that a "ctid" is currently visible to
/// a snapshot
pub struct VisibilityChecker {
    relation: pg_sys::Relation,
    snapshot: pg_sys::Snapshot,
    last_buffer: pg_sys::Buffer,
    ipd: pg_sys::ItemPointerData,
}

impl Drop for VisibilityChecker {
    fn drop(&mut self) {
        unsafe {
            if self.last_buffer != pg_sys::InvalidBuffer as pg_sys::Buffer {
                pg_sys::ReleaseBuffer(self.last_buffer);
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
            pgrx::itemptr::u64_to_item_pointer(ctid, &mut self.ipd);

            let blockno = item_pointer_get_block_number(&self.ipd);

            self.last_buffer =
                pg_sys::ReleaseAndReadBuffer(self.last_buffer, self.relation, blockno);

            pg_sys::LockBuffer(self.last_buffer, pg_sys::BUFFER_LOCK_SHARE as _);
            let found = self.check_page_vis(self.last_buffer);
            pg_sys::LockBuffer(self.last_buffer, pg_sys::BUFFER_LOCK_UNLOCK as _);
            found
        }
    }

    #[inline(always)]
    unsafe fn check_page_vis(&mut self, buffer: pg_sys::Buffer) -> bool {
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

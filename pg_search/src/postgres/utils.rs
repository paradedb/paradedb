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
use crate::schema::{SearchConfig, SearchDocument, SearchFieldName, SearchIndexSchema};
use crate::writer::IndexError;
use pgrx::itemptr::{item_pointer_get_block_number, item_pointer_get_both, item_pointer_set_all};
use pgrx::*;

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

/// Helper to manage the information necessary to validate that a "ctid" is currently visible to
/// a snapshot
pub struct VisibilityChecker {
    relation: pg_sys::Relation,
    need_close: bool,
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

            if self.need_close {
                // SAFETY:  `self.relation` is always a valid, open relation, created via `pg_sys::RelationGetRelation`
                pg_sys::RelationClose(self.relation);
            }
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
                need_close: true,
                snapshot: pg_sys::GetTransactionSnapshot(),
                last_buffer: pg_sys::InvalidBuffer as pg_sys::Buffer,
                ipd: pg_sys::ItemPointerData::default(),
            }
        }
    }

    pub fn with_rel_and_snap(relation: pg_sys::Relation, snapshot: pg_sys::Snapshot) -> Self {
        Self {
            relation,
            need_close: false,
            snapshot,
            last_buffer: pg_sys::InvalidBuffer as pg_sys::Buffer,
            ipd: pg_sys::ItemPointerData::default(),
        }
    }

    /// Returns true if the specified 64bit ctid is visible by the backing snapshot in the backing
    /// relation
    pub fn ctid_satisfies_snapshot(&mut self, ctid: u64) -> bool {
        self.exec_if_visible(ctid, |_, _| ()).is_some()
    }

    pub fn exec_if_visible<T, F: FnMut(pg_sys::HeapTupleData, pg_sys::Buffer) -> T>(
        &mut self,
        ctid: u64,
        mut func: F,
    ) -> Option<T> {
        unsafe {
            // Using ctid, get itempointer => buffer => page => heaptuple
            u64_to_item_pointer(ctid, &mut self.ipd);

            let blockno = item_pointer_get_block_number(&self.ipd);

            self.last_buffer =
                pg_sys::ReleaseAndReadBuffer(self.last_buffer, self.relation, blockno);

            pg_sys::LockBuffer(self.last_buffer, pg_sys::BUFFER_LOCK_SHARE as _);
            let (found, htup) = self.check_page_vis(self.last_buffer);
            pg_sys::LockBuffer(self.last_buffer, pg_sys::BUFFER_LOCK_UNLOCK as _);

            let result = found.then(|| func(htup, self.last_buffer));
            result
        }
    }

    unsafe fn check_page_vis(&mut self, buffer: pg_sys::Buffer) -> (bool, pg_sys::HeapTupleData) {
        unsafe {
            let mut heap_tuple = pg_sys::HeapTupleData::default();

            // Check if heaptuple is visible
            // In Postgres, the indexam `amgettuple` calls `heap_hot_search_buffer` for its visibility check
            let found = pg_sys::heap_hot_search_buffer(
                &mut self.ipd,
                self.relation,
                buffer,
                self.snapshot,
                &mut heap_tuple,
                std::ptr::null_mut(),
                true,
            );
            (found, heap_tuple)
        }
    }
}

/// Retrieves the `relfilenode` from a `SearchConfig`, handling PostgreSQL version differences.
pub fn relfilenode_from_search_config(search_config: &SearchConfig) -> pg_sys::Oid {
    let index_oid = search_config.index_oid;
    relfilenode_from_index_oid(index_oid)
}

/// Retrieves the `relfilenode` for a given index OID, handling PostgreSQL version differences.
pub fn relfilenode_from_index_oid(index_oid: u32) -> pg_sys::Oid {
    let index_relation = unsafe { PgRelation::open(pg_sys::Oid::from(index_oid)) };
    relfilenode_from_pg_relation(&index_relation)
}

/// Retrieves the `relfilenode` from a `PgRelation`, handling PostgreSQL version differences.
pub fn relfilenode_from_pg_relation(index_relation: &PgRelation) -> pg_sys::Oid {
    #[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
    {
        index_relation.rd_node.relNode
    }
    #[cfg(feature = "pg16")]
    {
        index_relation.rd_locator.relNumber
    }
}

/// Retrieves the OID for an index from Postgres.
pub fn index_oid_from_index_name(index_name: &str) -> pg_sys::Oid {
    // TODO: Switch to the implementation below when we eventually drop the generated index schemas.
    // This implementation will require the schema name to fully qualify the index name.
    // unsafe {
    //     // SAFETY:: Safe as long as the underlying function in `direct_function_call` is safe.
    //     let cstr_name = CString::new(index_name).expect("relation name is a valid CString");
    //     let indexrelid =
    //         direct_function_call::<pg_sys::Oid>(pg_sys::regclassin, &[cstr_name.into_datum()])
    //             .expect("index name should be a valid relation");
    //     let indexrel = PgRelation::with_lock(indexrelid, pg_sys::AccessShareLock as _);
    //     assert!(indexrel.is_index());
    //     indexrel.oid()
    // }

    let oid_query = format!(
        "SELECT oid FROM pg_class WHERE relname = '{}' AND relkind = 'i'",
        index_name
    );

    match Spi::get_one::<pg_sys::Oid>(&oid_query) {
        Ok(Some(index_oid)) => index_oid,
        Ok(None) => panic!("no oid for index '{index_name}' in schema_bm25"),
        Err(err) => panic!("error looking up index '{index_name}': {err}"),
    }
}

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

use crate::globals::WriterGlobal;
use crate::index::state::SearchHitIter;
use crate::index::SearchIndex;
use crate::schema::SearchConfig;
use crate::{env::needs_commit, writer::WriterDirectory};
use pgrx::*;

#[pg_guard]
pub extern "C" fn ambeginscan(
    indexrel: pg_sys::Relation,
    nkeys: ::std::os::raw::c_int,
    norderbys: ::std::os::raw::c_int,
) -> pg_sys::IndexScanDesc {
    let scandesc: PgBox<pg_sys::IndexScanDescData> =
        unsafe { PgBox::from_pg(pg_sys::RelationGetIndexScan(indexrel, nkeys, norderbys)) };

    scandesc.into_pg()
}

// An annotation to guard the function for PostgreSQL's threading model.
#[pg_guard]
pub extern "C" fn amrescan(
    scan: pg_sys::IndexScanDesc,
    keys: pg_sys::ScanKey,
    nkeys: ::std::os::raw::c_int,
    _orderbys: pg_sys::ScanKey,
    _norderbys: ::std::os::raw::c_int,
) {
    // Ensure there's at least one key provided for the search.
    if nkeys == 0 {
        panic!("no ScanKeys provided");
    }

    // Convert the raw pointer to a safe wrapper. This action takes ownership of the object
    // pointed to by the raw pointer in a safe way.
    let mut scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(scan) };

    // Convert the raw keys into a slice for easier access.
    let nkeys = nkeys as usize;
    let keys = unsafe { std::slice::from_raw_parts(keys as *const pg_sys::ScanKeyData, nkeys) };

    // Convert the first scan key argument into a byte array. This is assumed to be the `::jsonb` search config.
    let config_jsonb = unsafe {
        JsonB::from_datum(keys[0].sk_argument, false)
            .expect("failed to convert query to tuple of strings")
    };

    let search_config =
        SearchConfig::from_jsonb(config_jsonb).expect("could not parse search config");

    // Create the index and scan state
    let index_oid = unsafe { (*scan.indexRelation).rd_id.as_u32() };
    let directory = WriterDirectory::from_index_oid(index_oid);
    let search_index = SearchIndex::from_cache(&directory, &search_config.uuid)
        .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));
    let writer_client = WriterGlobal::client();
    let state = search_index
        .search_state(&writer_client, &search_config, needs_commit(index_oid))
        .unwrap();

    let top_docs = unsafe {
        // leak the `SearchState` for the lifetime of the current memory context
        //
        // it's necessary to leak this as the iterator returned from `SearchState::search()` borrows
        // from `&self`, so it needs to be in a known memory address before we perform the search
        let state = PgMemoryContexts::CurrentMemoryContext.leak_and_drop_on_delete(state);

        // SAFETY:  we were given a valid pointer for `state`
        let state = state.as_mut().unwrap();
        state.search(SearchIndex::executor())
    };

    // Save the iterator onto the current memory context.
    // let iter:HitsIterator = Box::new(top_docs);
    scan.opaque =
        PgMemoryContexts::CurrentMemoryContext.leak_and_drop_on_delete(top_docs) as void_mut_ptr;

    // Return scan state back management to Postgres.
    scan.into_pg();
}

#[pg_guard]
pub extern "C" fn amendscan(_scan: pg_sys::IndexScanDesc) {}

#[pg_guard]
pub extern "C" fn amgettuple(
    scan: pg_sys::IndexScanDesc,
    _direction: pg_sys::ScanDirection,
) -> bool {
    let mut scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(scan) };
    let iter = unsafe {
        // SAFETY:  `amrescan()` leaked an instance of `HitsIterator` into the current Postgres MemoryContext
        //          and set `scan.opaque` to point to it.
        (scan.opaque as *mut SearchHitIter).as_mut()
    }
    .expect("no scandesc state");

    scan.xs_recheck = false;

    match iter.next() {
        Some(hit) => {
            let tid = &mut scan.xs_heaptid;
            u64_to_item_pointer(hit.ctid, tid);

            true
        }
        None => false,
    }
}

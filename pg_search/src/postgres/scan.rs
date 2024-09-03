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
use crate::index::state::SearchResults;
use crate::index::SearchIndex;
use crate::schema::SearchConfig;
use crate::{env::needs_commit, writer::WriterDirectory};
use pgrx::itemptr::u64_to_item_pointer;
use pgrx::*;

type SearchResultIter = SearchResults;

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

    let leaked_results_iter = unsafe {
        // we need to leak both the `SearchState` we're about to create and the result iterator from
        // the search function.  The result iterator needs to be leaked so we can use it across the
        // IAM API calls, and the SearchState needs to be leaked because that iterator borrows from it
        // so it needs to be in a known memory address prior to performing a search.
        let state = search_index
            .search_state(&writer_client, &search_config, needs_commit(index_oid))
            .unwrap();
        let state = PgMemoryContexts::CurrentMemoryContext.leak_and_drop_on_delete(state);

        // SAFETY:  `leak_and_drop_on_delete()` gave us a non-null, aligned pointer to the SearchState
        let results_iter: SearchResultIter = state
            .as_ref()
            .unwrap()
            .search_minimal(SearchIndex::executor());

        PgMemoryContexts::CurrentMemoryContext.leak_and_drop_on_delete(results_iter)
    };

    // Save the iterator onto the current memory context.
    scan.opaque = leaked_results_iter.cast();

    // Return scan state back management to Postgres.
    scan.into_pg();
}

#[pg_guard]
pub extern "C" fn amendscan(_scan: pg_sys::IndexScanDesc) {}

#[pg_guard]
pub extern "C" fn amgettuple(
    scan: pg_sys::IndexScanDesc,
    _direction: pg_sys::ScanDirection::Type,
) -> bool {
    let mut scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(scan) };
    let iter = unsafe {
        // SAFETY:  We set `scan.opaque` to a leaked pointer of type `SearchResultIter` above in
        // amrescan, which is always called prior to this function
        scan.opaque.cast::<SearchResultIter>().as_mut()
    }
    .expect("no scandesc state");

    scan.xs_recheck = false;

    match iter.next() {
        Some((scored, _)) => {
            let tid = &mut scan.xs_heaptid;
            u64_to_item_pointer(scored.ctid, tid);

            true
        }
        None => false,
    }
}

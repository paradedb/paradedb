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
use crate::postgres::ScanStrategy;
use crate::query::SearchQueryInput;
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
    fn key_to_config(indexrel: pg_sys::Relation, key: &pg_sys::ScanKeyData) -> SearchConfig {
        match ScanStrategy::try_from(key.sk_strategy).expect("`key.sk_strategy` is unrecognized") {
            ScanStrategy::SearchConfigJson => unsafe {
                let search_config = SearchConfig::from_jsonb(
                    JsonB::from_datum(key.sk_argument, false)
                        .expect("ScanKey.sk_argument must not be null"),
                )
                .expect("`ScanKey.sk_argument` should be a valid `SearchConfig`");

                // assert that the index from the `SearchConfig` is the **same** index as the one Postgres
                // has decided to use for this IndexScan.  If it's not, we cannot continue.
                //
                // As we disallow creating multiple `USING bm25` indexes on a given table, this should never
                // happen, but it's hard to know what the state of existing databases are out there in the wild
                let postgres_index_oid = (*indexrel).rd_id;
                let our_index_id = search_config.index_oid;
                assert_eq!(
                    postgres_index_oid,
                    pg_sys::Oid::from(our_index_id),
                    "SearchConfig jsonb index doesn't match the index in the current IndexScan"
                );
                search_config
            },

            ScanStrategy::TextQuery => unsafe {
                let query = String::from_datum(key.sk_argument, false)
                    .expect("ScanKey.sk_argument must not be null");
                let indexrel = PgRelation::from_pg(indexrel);
                SearchConfig::from((query, indexrel))
            },

            ScanStrategy::SearchQueryInput => unsafe {
                let query = SearchQueryInput::from_datum(key.sk_argument, false)
                    .expect("ScanKey.sk_argument must not be null");
                let indexrel = PgRelation::from_pg(indexrel);
                SearchConfig::from((query, indexrel))
            },
        }
    }

    let (indexrel, keys) = unsafe {
        // SAFETY:  assert the pointers we're going to use are non-null
        assert!(!scan.is_null());
        assert!(!(*scan).indexRelation.is_null());
        assert!(!keys.is_null());
        assert!(nkeys > 0); // Ensure there's at least one key provided for the search.

        let indexrel = (*scan).indexRelation;
        let keys = std::slice::from_raw_parts(keys as *const pg_sys::ScanKeyData, nkeys as usize);

        (indexrel, keys)
    };

    // build a Boolean "must" clause of all the ScanKeys
    let mut search_config = key_to_config(indexrel, &keys[0]);
    for key in &keys[1..] {
        let key = key_to_config(indexrel, key);

        search_config.query = SearchQueryInput::Boolean {
            must: vec![search_config.query, key.query],
            should: vec![],
            must_not: vec![],
        };
    }

    // Create the index and scan state
    let index_oid = search_config.index_oid;
    let relfile_oid = unsafe { (*scan.indexRelation).rd_locator.relNumber.as_u32() };
    let database_oid = crate::MyDatabaseId();
    let directory = WriterDirectory::from_oids(database_oid, index_oid, relfile_oid);
    
    let search_index = SearchIndex::from_cache(&directory, &search_config.uuid)
        .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));
    let writer_client = WriterGlobal::client();
    let state = search_index
        .search_state(&writer_client, &search_config, needs_commit(index_oid))
        .expect("SearchState should construct cleanly");

    // Save the iterator onto the current memory context.
    unsafe {
        // SAFETY:  We asserted above that `scan` is non-null
        (*scan).opaque = {
            let results_iter: SearchResultIter =
                state.search_minimal(false, SearchIndex::executor());
            PgMemoryContexts::CurrentMemoryContext
                .leak_and_drop_on_delete(results_iter)
                .cast()
        }
    }
}

#[pg_guard]
pub extern "C" fn amendscan(_scan: pg_sys::IndexScanDesc) {}

#[pg_guard]
pub extern "C" fn amgettuple(
    scan: pg_sys::IndexScanDesc,
    _direction: pg_sys::ScanDirection::Type,
) -> bool {
    let iter = unsafe {
        // SAFETY:  We set `scan.opaque` to a leaked pointer of type `SearchResultIter` above in
        // amrescan, which is always called prior to this function
        (*scan).opaque.cast::<SearchResultIter>().as_mut()
    }
    .expect("no scan.opaque state");

    unsafe {
        (*scan).xs_recheck = false;
    }

    match iter.next() {
        Some((scored, _)) => {
            let tid = unsafe { &mut (*scan).xs_heaptid };
            u64_to_item_pointer(scored.ctid, tid);

            true
        }
        None => false,
    }
}

#[pg_guard]
pub extern "C" fn amgetbitmap(scan: pg_sys::IndexScanDesc, tbm: *mut pg_sys::TIDBitmap) -> i64 {
    assert!(!tbm.is_null());
    assert!(!scan.is_null());

    let iter = unsafe {
        // SAFETY:  We set `scan.opaque` to a leaked pointer of type `SearchResultIter` above in
        // amrescan, which is always called prior to this function
        (*scan).opaque.cast::<SearchResultIter>().as_mut()
    }
    .expect("no scan.opaque state");

    let mut cnt = 0i64;
    for (scored, _) in iter {
        let mut tid = pg_sys::ItemPointerData::default();
        u64_to_item_pointer(scored.ctid, &mut tid);

        unsafe {
            // SAFETY:  `tbm` has been asserted to be non-null and our `&mut tid` has been
            // initialized as a stack-allocated ItemPointerData
            pg_sys::tbm_add_tuples(tbm, &mut tid, 1, false);
        }

        cnt += 1;
    }

    cnt
}

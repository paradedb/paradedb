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

    let search_config = match ScanStrategy::try_from(keys[0].sk_strategy).expect("invalid strategy")
    {
        // Build a Boolean "must" set of each query from the scan keys, using the first one's JSONB
        // definition as the overall SearchConfig
        ScanStrategy::SearchConfigJson => unsafe {
            let mut first = SearchConfig::from_jsonb(
                JsonB::from_datum(keys[0].sk_argument, false)
                    .expect("ScanKey.sk_argument must not be null"),
            )
            .expect("`ScanKey.sk_argument` should be a valid `SearchConfig`");

            for key in &keys[1..] {
                let next = SearchConfig::from_jsonb(
                    JsonB::from_datum(key.sk_argument, false)
                        .expect("ScanKey.sk_argument must not be null"),
                )
                .expect("`ScanKey.sk_argument` should be a valid `SearchConfig`");

                first.query = SearchQueryInput::Boolean {
                    must: vec![first.query, next.query],
                    should: Default::default(),
                    must_not: Default::default(),
                };
            }

            first
        },

        // Directly create a SearchConfig by building a query of ANDed scan keys
        ScanStrategy::TextQuery => unsafe {
            let mut query = String::new();
            for key in keys {
                if !query.is_empty() {
                    query.push_str(" AND ");
                }

                let clause = String::from_datum(key.sk_argument, false)
                    .expect("ScanKey.sk_argument must not be null");
                query.push('(');
                query.push_str(&clause);
                query.push(')');
            }

            let indexrel = PgRelation::from_pg(scan.indexRelation);
            SearchConfig::from((query, indexrel))
        },

        // Directly create a SearchConfig using the provided SearchQueryInput keys
        ScanStrategy::SearchQueryInput => unsafe {
            let mut queries = Vec::with_capacity(nkeys);
            for key in keys {
                let query = SearchQueryInput::from_datum(key.sk_argument, false)
                    .expect("ScanKey.sk_argument must not be null");
                queries.push(query);
            }

            let query = if queries.len() == 1 {
                queries.pop().unwrap()
            } else {
                SearchQueryInput::Boolean {
                    must: queries,
                    should: vec![],
                    must_not: vec![],
                }
            };

            let indexrel = PgRelation::from_pg(scan.indexRelation);
            SearchConfig::from((query, indexrel))
        },
    };

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
            pg_sys::tbm_add_tuples(tbm, &mut tid, 1, false);
        }

        cnt += 1;
    }

    cnt
}

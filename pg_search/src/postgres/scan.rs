// Copyright (c) 2023-2026 ParadeDB, Inc.
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

use crate::api::operator::searchqueryinput_typoid;
use crate::index::fast_fields_helper::{FFType, resolve_ctid};
use crate::index::mvcc::{MvccSatisfies, SegmentView};
use crate::index::reader::index::{MultiSegmentSearchResults, SearchIndexReader};
use crate::postgres::index_only::IndexOnlyScanState;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::{ParallelScanState, ScanStrategy, parallel};
use crate::query::SearchQueryInput;

use pgrx::pg_sys::IndexScanDesc;
use pgrx::*;

pub struct Bm25ScanState {
    reader: SearchIndexReader,
    results: Option<MultiSegmentSearchResults>,
    index_only: Option<IndexOnlyScanState>,
    #[allow(dead_code)]
    ambulkdelete_epoch: u32,
    /// Cached per-segment ctid fast-field reader. Avoids re-opening the column
    /// reader for every row returned from the same segment.
    ctid_cache: Option<(tantivy::SegmentOrdinal, FFType)>,
}

#[pg_guard]
pub extern "C-unwind" fn ambeginscan(
    indexrel: pg_sys::Relation,
    nkeys: ::std::os::raw::c_int,
    norderbys: ::std::os::raw::c_int,
) -> pg_sys::IndexScanDesc {
    unsafe {
        let scandesc = pg_sys::RelationGetIndexScan(indexrel, nkeys, norderbys);

        // we may or may not end up doing an Index Only Scan, but regardless we only need to do
        // this one time
        (*scandesc).xs_hitupdesc = (*indexrel).rd_att;

        scandesc
    }
}

// An annotation to guard the function for PostgreSQL's threading model.
#[pg_guard]
pub extern "C-unwind" fn amrescan(
    scan: pg_sys::IndexScanDesc,
    keys: pg_sys::ScanKey,
    nkeys: ::std::os::raw::c_int,
    _orderbys: pg_sys::ScanKey,
    _norderbys: ::std::os::raw::c_int,
) {
    fn key_to_search_query_input(key: &pg_sys::ScanKeyData) -> SearchQueryInput {
        let strategy =
            ScanStrategy::try_from(key.sk_strategy).expect("`key.sk_strategy` is unrecognized");
        let is_array = (key.sk_flags as u32 & pg_sys::SK_SEARCHARRAY) != 0;

        match strategy {
            ScanStrategy::TextQuery if key.sk_attno != 1 => {
                // Fielded searches must be rewritten before reaching the index AM.
                let query = unsafe {
                    if is_array {
                        format!(
                            "{:?}",
                            Vec::<String>::from_datum(key.sk_argument, false)
                                .expect("text array argument should not be NULL")
                        )
                    } else {
                        String::from_datum(key.sk_argument, false)
                            .expect("text argument should not be NULL")
                    }
                };
                panic!(
                    "query is incompatible with pg_search's `@@@(field, TEXT)` operator: `{query}`"
                )
            }
            ScanStrategy::TextQuery => {
                if is_array {
                    let strings = unsafe {
                        <Vec<String> as FromDatum>::from_datum(key.sk_argument, false)
                            .expect("text array argument should not be NULL")
                    };
                    let should = strings
                        .into_iter()
                        .map(|query_string| SearchQueryInput::Parse {
                            query_string,
                            lenient: None,
                            conjunction_mode: None,
                        })
                        .collect();
                    SearchQueryInput::boolean_disjunction(should)
                } else {
                    let query_string = unsafe {
                        String::from_datum(key.sk_argument, false)
                            .expect("text argument should not be NULL")
                    };
                    SearchQueryInput::Parse {
                        query_string,
                        lenient: None,
                        conjunction_mode: None,
                    }
                }
            }
            ScanStrategy::SearchQueryInput => {
                if is_array {
                    // ScalarArrayOpExpr: decode as array of SearchQueryInput
                    let should = unsafe {
                        <Vec<SearchQueryInput> as FromDatum>::from_polymorphic_datum(
                            key.sk_argument,
                            false,
                            searchqueryinput_typoid(),
                        )
                        .expect("SearchQueryInput array should not be NULL")
                    };
                    SearchQueryInput::boolean_disjunction(should)
                } else {
                    // Single SearchQueryInput value
                    unsafe {
                        SearchQueryInput::from_datum(key.sk_argument, false)
                            .expect("SearchQueryInput should not be NULL")
                    }
                }
            }
        }
    }

    let (indexrel, keys) = unsafe {
        // SAFETY:  assert the pointers we're going to use are non-null
        assert!(!scan.is_null());
        assert!(!(*scan).indexRelation.is_null());
        assert!(nkeys >= 0);

        amendscan(scan);

        let indexrel = (*scan).indexRelation;
        let keys = if nkeys == 0 {
            &[]
        } else {
            assert!(!keys.is_null());
            std::slice::from_raw_parts(keys as *const pg_sys::ScanKeyData, nkeys as usize)
        };

        ((PgSearchRelation::from_pg(indexrel)), keys)
    };

    // build a Boolean "must" clause of all the ScanKeys
    let mut search_query_input = keys
        .first()
        .map(key_to_search_query_input)
        .unwrap_or(SearchQueryInput::All);
    for key in keys.iter().skip(1) {
        let key = key_to_search_query_input(key);

        search_query_input = SearchQueryInput::Boolean {
            must: vec![search_query_input, key],
            should: vec![],
            must_not: vec![],
            minimum_should_match: None,
        };
    }

    let ambulkdelete_epoch = MetaPage::open(&indexrel).ambulkdelete_epoch();

    // Parallel scan coordination:
    // - The leader opens with Snapshot visibility to see all currently-visible segments
    // - The leader then populates shared state with its segment list
    // - Workers WAIT for the leader to initialize, then get segment IDs from shared state
    // - Workers open with ParallelWorker visibility, which restricts them to ONLY those segments
    //
    // This ensures all participants see the exact same segment list, even if segment merges
    // occur between when the leader opens and when workers open. The segment FILES remain
    // on disk (pinned by the leader), so workers can access them.
    //
    // DON'T claim segments here - claim lazily in amgettuple/amgetbitmap.
    // Reason: PostgreSQL might call amrescan for a worker but never call amgettuple/amgetbitmap,
    // which would leave claimed segments unprocessed, causing data loss.
    let search_reader = unsafe {
        let is_parallel = !(*scan).parallel_scan.is_null();
        let is_worker = pg_sys::ParallelWorkerNumber >= 0;

        if is_parallel && is_worker {
            // Workers use ParallelWorker visibility with the segment IDs from shared state.
            // This is because workers pick specific segments to query that are known to be
            // held open/pinned by the leader, but might not pass a ::Snapshot visibility
            // test due to concurrent merges/garbage collects.
            let view = wait_for_segment_view(scan);
            SearchIndexReader::open(
                &indexrel,
                search_query_input,
                false,
                MvccSatisfies::ParallelWorker(view),
            )
            .expect("amrescan: worker should be able to open a SearchIndexReader")
        } else {
            // The leader (ParallelWorkerNumber == -1) or non-parallel scans use Snapshot
            // visibility to see all currently snapshot-visible segments.
            let reader = SearchIndexReader::open(
                &indexrel,
                search_query_input,
                false,
                MvccSatisfies::Snapshot,
            )
            .expect("amrescan: should be able to open a SearchIndexReader");

            // For parallel scans, leader initializes shared state with its segment list
            if is_parallel {
                parallel::maybe_init_parallel_scan(scan, &reader);
            }

            reader
        }
    };

    unsafe {
        let results = if (*scan).parallel_scan.is_null() {
            // not a parallel scan - search all segments
            Some(search_reader.search())
        } else {
            // parallel scan: DON'T claim segments here
            // Segments will be claimed lazily in search_next_segment during amgettuple/amgetbitmap
            None
        };

        let natts = (*(*scan).xs_hitupdesc).natts as usize;
        let index_only = if (*scan).xs_want_itup {
            Some(IndexOnlyScanState::new(&search_reader, &indexrel, natts))
        } else {
            None
        };
        let scan_state = Bm25ScanState {
            reader: search_reader,
            results,
            index_only,
            ambulkdelete_epoch,
            ctid_cache: None,
        };

        (*scan).opaque = PgMemoryContexts::CurrentMemoryContext
            .leak_and_drop_on_delete(Some(scan_state))
            .cast();
    }
}

#[pg_guard]
pub extern "C-unwind" fn amendscan(scan: pg_sys::IndexScanDesc) {
    unsafe {
        // Safety check: opaque might be NULL if amrescan was never called
        // This can happen in parallel workers that are terminated early
        if scan.is_null() {
            return;
        }
        (*scan).xs_hitup = std::ptr::null_mut();
        if (*scan).opaque.is_null() {
            return;
        }
        let scan_state = (*(*scan).opaque.cast::<Option<Bm25ScanState>>()).take();
        (*scan).opaque = std::ptr::null_mut();
        if let Some(mut state) = scan_state
            && let Some(index_only) = state.index_only.take()
        {
            index_only.delete();
        }
    }
}

#[pg_guard]
pub unsafe extern "C-unwind" fn amgettuple(
    scan: pg_sys::IndexScanDesc,
    _direction: pg_sys::ScanDirection::Type,
) -> bool {
    let state = {
        // SAFETY:  We set `scan.opaque` to a leaked pointer of type `Bm25ScanState` above in
        // amrescan, which is always called prior to this function
        (*(*scan).opaque.cast::<Option<Bm25ScanState>>())
            .as_mut()
            .expect("opaque should be a Bm25ScanState")
    };

    (*scan).xs_recheck = false;
    (*scan).xs_hitup = std::ptr::null_mut();
    if let Some(index_only) = &mut state.index_only {
        index_only.reset();
    }

    loop {
        // Extract the next result first so the temporary mutable borrow on
        // `state.results` is dropped before we access `state.ctid_cache`.
        let next_result = state.results.as_mut().and_then(|r| r.next());
        match next_result {
            None => {
                state.ctid_cache = None;
                if search_next_segment(scan, state) {
                    // loop back around to start returning results from this segment
                    continue;
                }

                // we are done returning results
                return false;
            }
            Some((_scored, doc_address)) => {
                // Fetch the real ctid from the fast-field reader, caching per segment.
                let searcher = state.reader.searcher();
                let ctid = resolve_ctid(&mut state.ctid_cache, searcher, doc_address);

                let ipd = &mut (*scan).xs_heaptid;
                crate::postgres::utils::u64_to_item_pointer(ctid, ipd);

                if let Some(index_only) = &mut state.index_only {
                    (*scan).xs_hitup = index_only.form_tuple((*scan).xs_hitupdesc, doc_address);
                }

                return true;
            }
        }
    }
}

#[pg_guard]
pub unsafe extern "C-unwind" fn amgetbitmap(
    scan: pg_sys::IndexScanDesc,
    tbm: *mut pg_sys::TIDBitmap,
) -> i64 {
    assert!(!tbm.is_null());
    assert!(!scan.is_null());

    let state = {
        // SAFETY:  We set `scan.opaque` to a leaked pointer of type `Bm25ScanState` above in
        // amrescan, which is always called prior to this function
        (*(*scan).opaque.cast::<Option<Bm25ScanState>>())
            .as_mut()
            .expect("opaque should be a Bm25ScanState")
    };

    let mut cnt = 0i64;
    loop {
        // Clone the Searcher (cheap Arc clone) to avoid holding a borrow on
        // `state.results` while also needing to look up `state.ctid_cache`.
        let searcher = state.reader.searcher().clone();
        let mut ctid_cache: Option<(tantivy::SegmentOrdinal, FFType)> = None;
        if let Some(search_results) = state.results.as_mut() {
            for (_scored, doc_address) in search_results {
                let ctid = resolve_ctid(&mut ctid_cache, &searcher, doc_address);

                let mut ipd = pg_sys::ItemPointerData::default();
                crate::postgres::utils::u64_to_item_pointer(ctid, &mut ipd);

                // SAFETY:  `tbm` has been asserted to be non-null and our `&mut tid` has been
                // initialized as a stack-allocated ItemPointerData
                pg_sys::tbm_add_tuples(tbm, &mut ipd, 1, false);

                cnt += 1;
            }
        }

        // check if the bitmap scan needs to claim another individual segment
        if search_next_segment(scan, state) {
            continue;
        }

        break;
    }

    cnt
}

/// Wait for parallel scan state to be initialized by the leader, then return its segment view.
/// This ensures workers see the exact same segments as the leader, preventing race conditions
/// where workers might see different segments due to concurrent merges.
unsafe fn wait_for_segment_view(scan: IndexScanDesc) -> SegmentView {
    let state = get_parallel_scan_state(scan)
        .expect("wait_for_segment_view called but no parallel scan state");

    // segment_view() internally calls wait_for_initialization()
    state.segment_view()
}

/// Get the parallel scan state from an IndexScanDesc, if it's a parallel scan.
unsafe fn get_parallel_scan_state(scan: IndexScanDesc) -> Option<&'static mut ParallelScanState> {
    if (*scan).parallel_scan.is_null() {
        return None;
    }

    let ps = (*scan).parallel_scan;
    let offset = {
        #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
        {
            (*ps).ps_offset
        }
        #[cfg(feature = "pg18")]
        {
            (*ps).ps_offset_am
        }
    };

    ps.cast::<std::ffi::c_void>()
        .add(offset)
        .cast::<ParallelScanState>()
        .as_mut()
}

// if there's a segment to be claimed for parallel query execution, do that now
unsafe fn search_next_segment(scan: IndexScanDesc, state: &mut Bm25ScanState) -> bool {
    if let Some(segment_number) = parallel::maybe_claim_segment(scan) {
        state.results = Some(state.reader.search_segments([segment_number].into_iter()));
        return true;
    }
    false
}

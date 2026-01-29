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
use crate::api::HashSet;
use crate::index::fast_fields_helper::{FFHelper, FastFieldType};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::{Bm25Settings, MultiSegmentSearchResults, SearchIndexReader};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::{parallel, ParallelScanState, ScanStrategy};
use crate::query::SearchQueryInput;
use pgrx::pg_sys::IndexScanDesc;
use pgrx::*;
use tantivy::index::SegmentId;

pub struct Bm25ScanState {
    fast_fields: FFHelper,
    reader: SearchIndexReader,
    results: Option<MultiSegmentSearchResults>,
    itup: (Vec<pg_sys::Datum>, Vec<bool>),
    key_field_oid: PgOid,
    #[allow(dead_code)]
    ambulkdelete_epoch: u32,
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
        assert!(!keys.is_null());
        assert!(nkeys > 0); // Ensure there's at least one key provided for the search.

        // Clean up any previous scan state before creating a new one.
        // This is necessary for rescans - PostgreSQL may call amrescan multiple times
        // without calling amendscan in between.
        if !(*scan).opaque.is_null() {
            let old_state = (*(*scan).opaque.cast::<Option<Bm25ScanState>>()).take();
            drop(old_state);
            (*scan).opaque = std::ptr::null_mut();
        }

        let indexrel = (*scan).indexRelation;
        let keys = std::slice::from_raw_parts(keys as *const pg_sys::ScanKeyData, nkeys as usize);

        ((PgSearchRelation::from_pg(indexrel)), keys)
    };

    // build a Boolean "must" clause of all the ScanKeys
    let mut search_query_input = key_to_search_query_input(&keys[0]);
    for key in &keys[1..] {
        let key = key_to_search_query_input(key);

        search_query_input = SearchQueryInput::Boolean {
            must: vec![search_query_input, key],
            should: vec![],
            must_not: vec![],
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
            let segment_ids = wait_for_segment_ids(scan);
            SearchIndexReader::open(
                &indexrel,
                search_query_input,
                Bm25Settings::default(),
                MvccSatisfies::ParallelWorker(segment_ids),
            )
            .expect("amrescan: worker should be able to open a SearchIndexReader")
        } else {
            // The leader (ParallelWorkerNumber == -1) or non-parallel scans use Snapshot
            // visibility to see all currently snapshot-visible segments.
            let reader = SearchIndexReader::open(
                &indexrel,
                search_query_input,
                Bm25Settings::default(),
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
        let scan_state = if (*scan).xs_want_itup {
            let schema = indexrel.schema().expect("indexrel should have a schema");
            Bm25ScanState {
                fast_fields: FFHelper::with_fields(
                    &search_reader,
                    &[(
                        schema.key_field_name(),
                        FastFieldType::from(schema.key_field_type()),
                    )
                        .into()],
                ),
                reader: search_reader,
                results,
                itup: (vec![pg_sys::Datum::null(); natts], vec![true; natts]),
                key_field_oid: PgOid::from({
                    #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
                    {
                        (*(*scan).xs_hitupdesc).attrs.as_slice(natts)[0].atttypid
                    }
                    #[cfg(feature = "pg18")]
                    {
                        (*pg_sys::TupleDescAttr((*scan).xs_hitupdesc, 0)).atttypid
                    }
                }),
                ambulkdelete_epoch,
            }
        } else {
            Bm25ScanState {
                fast_fields: FFHelper::empty(),
                reader: search_reader,
                results,
                itup: (vec![], vec![]),
                key_field_oid: PgOid::Invalid,
                ambulkdelete_epoch,
            }
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
        if scan.is_null() || (*scan).opaque.is_null() {
            return;
        }
        let scan_state = (*(*scan).opaque.cast::<Option<Bm25ScanState>>()).take();
        drop(scan_state);
    }
}

#[pg_guard]
pub unsafe extern "C-unwind" fn amgettuple(
    scan: pg_sys::IndexScanDesc,
    _direction: pg_sys::ScanDirection::Type,
) -> bool {
    let state = {
        // SAFETY:  We set `scan.opaque` to a leaked pointer of type `PgSearchScanState` above in
        // amrescan, which is always called prior to this function
        (*(*scan).opaque.cast::<Option<Bm25ScanState>>())
            .as_mut()
            .expect("opaque should be a Bm25ScanState")
    };

    (*scan).xs_recheck = false;

    loop {
        match state.results.as_mut().and_then(|r| r.next()) {
            Some((scored, doc_address)) => {
                let ipd = &mut (*scan).xs_heaptid;
                crate::postgres::utils::u64_to_item_pointer(scored.ctid, ipd);

                if (*scan).xs_want_itup {
                    let key = state
                        .fast_fields
                        .value(0, doc_address)
                        .expect("key_field should be a fast_field");
                    match key
                        .try_into_datum(state.key_field_oid)
                        .expect("key_field value should convert to a Datum")
                    {
                        // got a valid Datum
                        Some(key_field_datum) => {
                            state.itup.0[0] = key_field_datum;
                            state.itup.1[0] = false;
                        }

                        // we got a NULL for the key_field.  Highly unlikely but definitely possible
                        None => {
                            state.itup.0[0] = pg_sys::Datum::null();
                            state.itup.1[0] = true;
                        }
                    }

                    let values = state.itup.0.as_mut_ptr();
                    let nulls = state.itup.1.as_mut_ptr();

                    if (*scan).xs_hitup.is_null() {
                        (*scan).xs_hitup =
                            pg_sys::heap_form_tuple((*scan).xs_hitupdesc, values, nulls);
                    } else {
                        pg_sys::ffi::pg_guard_ffi_boundary(|| {
                            extern "C-unwind" {
                                fn heap_compute_data_size(
                                    tupleDesc: pg_sys::TupleDesc,
                                    values: *mut pg_sys::Datum,
                                    isnull: *mut bool,
                                ) -> pg_sys::Size;
                                fn heap_fill_tuple(
                                    tupleDesc: pg_sys::TupleDesc,
                                    values: *mut pg_sys::Datum,
                                    isnull: *mut bool,
                                    data: *mut ::core::ffi::c_char,
                                    data_size: pg_sys::Size,
                                    infomask: *mut pg_sys::uint16,
                                    bit: *mut pg_sys::bits8,
                                );
                            }
                            let data_len =
                                heap_compute_data_size((*scan).xs_hitupdesc, values, nulls);
                            let td = (*(*scan).xs_hitup).t_data;

                            // TODO:  seems like this could crash with a varlena "key_field" of varrying sizes per row
                            heap_fill_tuple(
                                (*scan).xs_hitupdesc,
                                values,
                                nulls,
                                td.cast::<std::ffi::c_char>().add((*td).t_hoff as usize),
                                data_len,
                                &mut (*td).t_infomask,
                                (*td).t_bits.as_mut_ptr(),
                            );
                        });
                    }
                }

                return true;
            }
            None => {
                if search_next_segment(scan, state) {
                    // loop back around to start returning results from this segment
                    continue;
                }

                // we are done returning results
                return false;
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
        // SAFETY:  We set `scan.opaque` to a leaked pointer of type `PgSearchScanState` above in
        // amrescan, which is always called prior to this function
        (*(*scan).opaque.cast::<Option<Bm25ScanState>>())
            .as_mut()
            .expect("opaque should be a Bm25ScanState")
    };

    let mut cnt = 0i64;
    loop {
        if let Some(search_results) = state.results.as_mut() {
            for (scored, _) in search_results {
                let mut ipd = pg_sys::ItemPointerData::default();
                crate::postgres::utils::u64_to_item_pointer(scored.ctid, &mut ipd);

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

/// Wait for parallel scan state to be initialized by the leader, then return the segment IDs.
/// This ensures workers see the exact same segments as the leader, preventing race conditions
/// where workers might see different segments due to concurrent merges.
unsafe fn wait_for_segment_ids(scan: IndexScanDesc) -> HashSet<SegmentId> {
    let state = get_parallel_scan_state(scan)
        .expect("wait_for_segment_ids called but no parallel scan state");

    // segments() internally calls wait_for_initialization() and returns segment IDs
    state.segments().into_keys().collect()
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

#[pg_guard]
pub extern "C-unwind" fn amcanreturn(indexrel: pg_sys::Relation, attno: i32) -> bool {
    if attno != 1 {
        // currently, we only support returning the "key_field", which will always be the first
        // index attribute
        return false;
    }

    unsafe {
        assert!(!indexrel.is_null());
        assert!(!(*indexrel).rd_att.is_null());
        let tupdesc = PgTupleDesc::from_pg_unchecked((*indexrel).rd_att);

        let att = tupdesc
            .get((attno - 1) as usize)
            .expect("attno should exist in index tupledesc");

        // we can only return a field if it's one of the below types -- basically pass-by-value (non tokenized) data types
        [
            pg_sys::INT4OID,
            pg_sys::INT8OID,
            pg_sys::FLOAT4OID,
            pg_sys::FLOAT8OID,
            pg_sys::BOOLOID,
            // we index UUID as strings, but it's beneficial to support returning due to Parallel Index Only Scans
            pg_sys::UUIDOID,
        ]
        .contains(&att.atttypid)
    }
}

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

use crate::index::reader::SearchResults;
use crate::index::SearchIndex;
use crate::postgres::index::open_search_index;
use crate::postgres::ScanStrategy;
use crate::query::SearchQueryInput;
use pgrx::*;

struct PgSearchScanState {
    results: SearchResults,
    itup: (Vec<pg_sys::Datum>, Vec<bool>),
    key_field_oid: PgOid,
}

#[pg_guard]
pub extern "C" fn ambeginscan(
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
pub extern "C" fn amrescan(
    scan: pg_sys::IndexScanDesc,
    keys: pg_sys::ScanKey,
    nkeys: ::std::os::raw::c_int,
    _orderbys: pg_sys::ScanKey,
    _norderbys: ::std::os::raw::c_int,
) {
    fn key_to_search_query_input(key: &pg_sys::ScanKeyData) -> SearchQueryInput {
        match ScanStrategy::try_from(key.sk_strategy).expect("`key.sk_strategy` is unrecognized") {
            ScanStrategy::SearchConfigJson => unsafe {
                SearchQueryInput::from_datum(key.sk_argument, false)
                    .expect("ScanKey.sk_argument must not be null")
            },

            ScanStrategy::TextQuery => unsafe {
                let query_string = String::from_datum(key.sk_argument, false)
                    .expect("ScanKey.sk_argument must not be null");
                SearchQueryInput::Parse {
                    query_string,
                    lenient: None,
                    conjunction_mode: None,
                }
            },

            ScanStrategy::SearchQueryInput => unsafe {
                SearchQueryInput::from_datum(key.sk_argument, false)
                    .expect("ScanKey.sk_argument must not be null")
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

        (PgRelation::from_pg(indexrel), keys)
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

    // Create the index and scan state
    let search_index = open_search_index(&indexrel).expect("should be able to open search index");
    let state = search_index
        .get_reader()
        .expect("SearchState should construct cleanly");

    unsafe {
        let query = search_index.query(&search_query_input, &state);
        let results = state.search_minimal(
            (*scan).xs_want_itup,
            search_query_input.contains_more_like_this(),
            search_index.key_field_name(),
            SearchIndex::executor(),
            &query,
        );
        let natts = (*(*scan).xs_hitupdesc).natts as usize;
        let scan_state = if (*scan).xs_want_itup {
            PgSearchScanState {
                results,
                itup: (vec![pg_sys::Datum::null(); natts], vec![true; natts]),
                key_field_oid: PgOid::from(
                    (*(*scan).xs_hitupdesc).attrs.as_slice(natts)[0].atttypid,
                ),
            }
        } else {
            PgSearchScanState {
                results,
                itup: (vec![], vec![]),
                key_field_oid: PgOid::Invalid,
            }
        };

        (*scan).opaque = PgMemoryContexts::CurrentMemoryContext
            .leak_and_drop_on_delete(scan_state)
            .cast();
    }
}

#[pg_guard]
pub extern "C" fn amendscan(_scan: pg_sys::IndexScanDesc) {}

#[pg_guard]
pub extern "C" fn amgettuple(
    scan: pg_sys::IndexScanDesc,
    _direction: pg_sys::ScanDirection::Type,
) -> bool {
    let state = unsafe {
        // SAFETY:  We set `scan.opaque` to a leaked pointer of type `PgSearchScanState` above in
        // amrescan, which is always called prior to this function
        (*scan).opaque.cast::<PgSearchScanState>().as_mut()
    }
    .expect("no scan.opaque state");

    unsafe {
        (*scan).xs_recheck = false;
    }

    match state.results.next() {
        Some((scored, _)) => unsafe {
            let tid = &mut (*scan).xs_heaptid;
            crate::postgres::utils::u64_to_item_pointer(scored.ctid, tid);

            if (*scan).xs_want_itup {
                match scored
                    .key
                    .expect("should have retrieved the key_field")
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

                (*scan).xs_hitup = pg_sys::heap_form_tuple(
                    (*scan).xs_hitupdesc,
                    state.itup.0.as_mut_ptr(),
                    state.itup.1.as_mut_ptr(),
                );
            }

            true
        },
        None => false,
    }
}

#[pg_guard]
pub extern "C" fn amgetbitmap(scan: pg_sys::IndexScanDesc, tbm: *mut pg_sys::TIDBitmap) -> i64 {
    assert!(!tbm.is_null());
    assert!(!scan.is_null());

    let state = unsafe {
        // SAFETY:  We set `scan.opaque` to a leaked pointer of type `PgSearchScanState` above in
        // amrescan, which is always called prior to this function
        (*scan).opaque.cast::<PgSearchScanState>().as_mut()
    }
    .expect("no scan.opaque state");

    let mut cnt = 0i64;
    for (scored, _) in &mut state.results {
        let mut tid = pg_sys::ItemPointerData::default();
        crate::postgres::utils::u64_to_item_pointer(scored.ctid, &mut tid);

        unsafe {
            // SAFETY:  `tbm` has been asserted to be non-null and our `&mut tid` has been
            // initialized as a stack-allocated ItemPointerData
            pg_sys::tbm_add_tuples(tbm, &mut tid, 1, false);
        }

        cnt += 1;
    }

    cnt
}

#[pg_guard]
pub extern "C" fn amcanreturn(indexrel: pg_sys::Relation, attno: i32) -> bool {
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
        ]
        .contains(&att.atttypid)
    }
}

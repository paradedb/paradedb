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

//! Per-row search evaluation when a search predicate executes as a heap filter.

mod args;
mod keyset;

pub(crate) use keyset::KeySet;

use self::args::{FakeAnyElement, FakeCtid, FakeSearchQueryInput};
use crate::api::HashMap;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::planner_warnings::{warn_filter_spilled, warn_sequential_scan};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types::TantivyValue;
use crate::postgres::utils::Ctid;
use crate::query::SearchQueryInput;
use pgrx::{pg_extern, pg_func_extra, pg_getarg_datum_raw, pg_sys};

struct QueryCacheEntry {
    matches: KeySet,
    /// CTIDs for rows where the indexed field is absent (SQL NULL semantics).
    missing_values: Option<KeySet>,
}

#[derive(Default)]
struct Cache {
    by_query: HashMap<Vec<u8>, QueryCacheEntry>,
}

#[allow(unused_variables)]
#[pg_extern(immutable, parallel_safe, cost = 1000000000)]
pub fn search_with_query_input(
    element: FakeAnyElement,
    query: FakeSearchQueryInput,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<bool> {
    search_with_query_input_impl(fcinfo, None)
}

#[allow(unused_variables)]
#[pg_extern(immutable, parallel_safe, cost = 1000000000)]
pub fn search_with_query_input_ctid(
    element: Option<FakeAnyElement>,
    query: FakeSearchQueryInput,
    ctid: FakeCtid,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<bool> {
    search_with_query_input_impl(fcinfo, Some(unsafe { Ctid::from_fcinfo(fcinfo, 2) }?))
}

#[allow(unused_variables)]
#[pg_extern(immutable, parallel_safe, cost = 1000000000)]
pub fn search_with_query_input_ctid_strict(
    element: FakeAnyElement,
    query: FakeSearchQueryInput,
    ctid: FakeCtid,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<bool> {
    search_with_query_input_impl(fcinfo, Some(unsafe { Ctid::from_fcinfo(fcinfo, 2) }?))
}

fn search_with_query_input_impl(
    fcinfo: pg_sys::FunctionCallInfo,
    ctid: Option<Ctid>,
) -> Option<bool> {
    // get the Cache attached to this instance of the function
    let mut cache = unsafe { pg_func_extra(fcinfo, Cache::default) };

    // Planner-generated calls always provide a non-NULL query datum.
    let query_datum = unsafe { pg_getarg_datum_raw(fcinfo, 1) };
    let key = unsafe {
        let varlena = query_datum.cast_mut_ptr::<pg_sys::varlena>();
        pgrx::varlena_to_byte_slice(varlena).to_vec()
    };

    let mut newly_built = false;
    let query_cache = cache.by_query.entry(key).or_insert_with(|| {
        newly_built = true;
        let search_query_input = unsafe {
            SearchQueryInput::from_datum(query_datum, query_datum.is_null())
                .expect("the query argument cannot be NULL")
        };

        // `empty()` cannot match any index document, including for a partial index.
        if matches!(&search_query_input, SearchQueryInput::Empty) {
            return QueryCacheEntry {
                matches: KeySet::None,
                missing_values: None,
            };
        }

        let index_oid = search_query_input.index_oid().unwrap_or_else(|| {
            panic!("pg_search: could not determine the index to use for this query")
        });

        let index_relation =
            PgSearchRelation::with_lock(index_oid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
        let is_partial =
            unsafe { !pg_sys::RelationGetIndexPredicate(index_relation.as_ptr()).is_null() };

        // `all()` matches every document, but a partial index may not contain every table row.
        if search_query_input.is_match_all() && !is_partial {
            return QueryCacheEntry {
                matches: KeySet::All,
                missing_values: None,
            };
        }

        // Reaching here means the planner could not use the ParadeDB index to satisfy this query, so we
        // materialize the match set and apply it as a per-row filter (the slow path).

        let heap_relation = index_relation
            .heap_relation()
            .expect("a ParadeDB index must have a heap relation");
        let mut visibility = VisibilityChecker::with_rel_and_snap(&heap_relation, unsafe {
            pg_sys::GetActiveSnapshot()
        });

        let null_guard = index_relation
            .schema()
            .expect("a ParadeDB index must have a schema")
            .null_guard(&search_query_input);

        let search_reader = SearchIndexReader::open(
            &index_relation,
            search_query_input,
            false,
            MvccSatisfies::Snapshot,
        )
        .expect("search_with_query_input: should be able to open a SearchIndexReader");

        // Collect matching CTIDs into a memory-bounded set (spills to a temp file past
        // `work_mem`), reused for every row of the scan.
        let matches = search_reader.collect_ctidset(&mut visibility);

        let missing_values = if let Some(null_guard) = null_guard {
            // Collect rows where the field is absent (the complement of `exists`). Membership in
            // this set means SQL NULL for negation semantics.
            let complement_query = SearchQueryInput::WithIndex {
                oid: index_oid,
                query: Box::new(SearchQueryInput::Boolean {
                    must: vec![SearchQueryInput::All],
                    should: Default::default(),
                    must_not: vec![null_guard],
                    minimum_should_match: None,
                }),
            };

            let complement_reader = SearchIndexReader::open(
                &index_relation,
                complement_query,
                false,
                MvccSatisfies::Snapshot,
            )
            .expect(
                "search_with_query_input: should be able to open a complement SearchIndexReader",
            );

            Some(complement_reader.collect_ctidset(&mut visibility))
        } else {
            None
        };

        QueryCacheEntry {
            matches,
            missing_values,
        }
    });

    // Reaching this function at all means the search-operator predicate is being applied as a
    // per-row filter rather than an index scan, so warn whenever we evaluate a query here -- regardless of the
    // all()/empty() short-circuits -- but at most once per statement. Separately warn if the
    // materialized match set spilled past work_mem.
    let spilled = newly_built
        && (matches!(query_cache.matches, KeySet::Spilled(_))
            || matches!(&query_cache.missing_values, Some(KeySet::Spilled(_))));

    let result = match &query_cache.matches {
        KeySet::All => Some(true),
        KeySet::None => Some(false),
        _ => {
            let ctid = ctid.expect("heap-filter query should carry a CTID");
            let row_identity = TantivyValue::try_from(u64::from(ctid))
                .expect("ctid should convert to a Tantivy value");

            if query_cache.matches.contains(&row_identity) {
                Some(true)
            } else if let Some(missing_values) = &query_cache.missing_values {
                if missing_values.contains(&row_identity) {
                    None
                } else {
                    Some(false)
                }
            } else {
                Some(false)
            }
        }
    };

    if newly_built {
        warn_sequential_scan();
    }
    if spilled {
        warn_filter_spilled();
    }

    result
}

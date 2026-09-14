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
mod inline;
mod keyset;

pub(crate) use inline::MaybeInlineRow;
pub(crate) use keyset::KeySet;

use self::args::{FakeAnyElement, FakeCtid, FakeRecord, FakeRow, FakeSearchQueryInput};
use self::inline::RowMatcher;
use crate::api::HashMap;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::index::{is_partitioned_index, partition_member_index};
use crate::postgres::planner_warnings::{warn_filter_spilled, warn_sequential_scan};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types::TantivyValue;
use crate::postgres::utils::Ctid;
use crate::query::SearchQueryInput;
use pgrx::heap_tuple::PgHeapTuple;
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{
    Array, FromDatum, PgLogLevel, PgMemoryContexts, PgSqlErrorCode, default, function_name,
    pg_extern, pg_func_extra, pg_getarg_datum, pg_getarg_datum_raw, pg_sys,
};

struct QueryCacheEntry {
    matches: KeySet,
    /// CTIDs for rows where the indexed field is absent (SQL NULL semantics).
    missing_values: Option<KeySet>,
}

impl QueryCacheEntry {
    fn is_valid(&self) -> bool {
        self.matches.is_valid()
            && self
                .missing_values
                .as_ref()
                .is_none_or(|missing_values| missing_values.is_valid())
    }
}

/// A query planned above an Append carries a partitioned index, which has no storage of
/// its own (#4643). Rows from every partition then flow through one function call, so the
/// match sets are built per partition, lazily, keyed by the `tableoid` the planner ships
/// in the trailing record argument.
enum CacheEntry {
    Single(QueryCacheEntry),
    Partitioned {
        parent_index_oid: pg_sys::Oid,
        by_child: HashMap<pg_sys::Oid, QueryCacheEntry>,
    },
}

/// The same split for the inline-row fallback, whose matcher reads schema and storage
/// from one concrete leaf index.
enum InlineEntry {
    Single(Box<RowMatcher>),
    Partitioned {
        parent_index_oid: pg_sys::Oid,
        by_child: HashMap<pg_sys::Oid, RowMatcher>,
    },
}

#[derive(Default)]
struct Cache {
    by_query: HashMap<Vec<u8>, CacheEntry>,
    inline_rows: HashMap<Vec<u8>, InlineEntry>,
}

#[allow(unused_variables)]
#[pg_extern(immutable, parallel_safe, cost = 1000000000)]
pub fn search_with_query_input(
    element: FakeAnyElement,
    query: FakeSearchQueryInput,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<bool> {
    if unsafe {
        pgrx::is_a(
            (*(*fcinfo).flinfo).fn_expr,
            pg_sys::NodeTag::T_ScalarArrayOpExpr,
        )
    } {
        ErrorReport::new(
            PgSqlErrorCode::ERRCODE_FEATURE_NOT_SUPPORTED,
            "Unsupported query shape. Please report at https://github.com/paradedb/paradedb/issues/new/choose",
            function_name!(),
        )
        .report(PgLogLevel::ERROR);
    }
    search_with_query_input_impl(fcinfo, None)
}

#[allow(unused_variables)]
#[pg_extern(immutable, parallel_safe, cost = 1000000000)]
pub fn search_with_query_input_ctid(
    element: Option<FakeAnyElement>,
    query: FakeSearchQueryInput,
    ctid: FakeCtid,
    original_lhs: default!(FakeRecord, "ROW()"),
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
    original_lhs: default!(FakeRecord, "ROW()"),
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<bool> {
    search_with_query_input_impl(fcinfo, Some(unsafe { Ctid::from_fcinfo(fcinfo, 2) }?))
}

#[pg_extern(immutable, parallel_safe, cost = 1000000000)]
pub fn search_with_query_input_ctid_or_row_strict(
    element: FakeAnyElement,
    query: FakeSearchQueryInput,
    ctid: FakeCtid,
    fallback_row: FakeRow,
    original_lhs: default!(FakeRecord, "ROW()"),
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<bool> {
    search_with_query_input_ctid_or_row(
        Some(element),
        query,
        ctid,
        Some(fallback_row),
        original_lhs,
        fcinfo,
    )
}

#[allow(unused_variables)]
#[pg_extern(immutable, parallel_safe, cost = 1000000000)]
pub fn search_with_query_input_ctid_or_row(
    element: Option<FakeAnyElement>,
    query: FakeSearchQueryInput,
    ctid: FakeCtid,
    fallback_row: Option<FakeRow>,
    original_lhs: default!(FakeRecord, "ROW()"),
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<bool> {
    let ctid = unsafe { Ctid::from_fcinfo(fcinfo, 2) }?;
    fallback_row?;
    let rows = unsafe {
        pg_sys::pg_detoast_datum(pg_getarg_datum_raw(fcinfo, 3).cast_mut_ptr())
            .cast::<pg_sys::ArrayType>()
    };
    // Check the empty-array marker without unpacking or materializing a heap row.
    if unsafe { (*rows).ndim } == 0 {
        if !ctid.is_valid() {
            return None;
        }
        return search_with_query_input_impl(fcinfo, Some(ctid));
    }

    assert!(
        unsafe { pg_sys::type_is_rowtype((*rows).elemtype) },
        "inline row must be a composite value"
    );
    let rows = unsafe { Array::<pg_sys::Datum>::from_datum(pg_sys::Datum::from(rows), false) }?;
    assert_eq!(rows.len(), 1, "inline evaluation requires exactly one row");
    let row = rows.get(0)??;

    let query_datum = unsafe { pg_getarg_datum(fcinfo, 1) }?;
    let query_datum = unsafe { pg_sys::pg_detoast_datum(query_datum.cast_mut_ptr()) };

    let mut cache = unsafe { pg_func_extra(fcinfo, Cache::default) };
    let key = unsafe { pgrx::varlena_to_byte_slice(query_datum).to_vec() };

    let entry = cache.inline_rows.entry(key).or_insert_with(|| {
        let query = unsafe { deserialize_query(query_datum) };
        let index_oid = query.index_oid().unwrap_or_else(|| {
            panic!("pg_search: could not determine the index to use for this query")
        });
        if is_partitioned_index(index_oid) {
            return InlineEntry::Partitioned {
                parent_index_oid: index_oid,
                by_child: HashMap::default(),
            };
        }
        let index_relation =
            PgSearchRelation::with_lock(index_oid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
        InlineEntry::Single(Box::new(unsafe {
            build_row_matcher(fcinfo, index_relation, query)
        }))
    });

    let matcher = match entry {
        InlineEntry::Single(matcher) => matcher.as_mut(),
        InlineEntry::Partitioned {
            parent_index_oid,
            by_child,
        } => {
            partition_entry(fcinfo, *parent_index_oid, by_child, |index_relation| {
                let query = unsafe { deserialize_query(query_datum) };
                unsafe { build_row_matcher(fcinfo, index_relation, query) }
            })
            .0
        }
    };

    unsafe { matcher.matches(row) }
}

#[allow(unused_variables)]
#[pg_extern(immutable, strict, parallel_safe)]
pub fn ctid_is_valid(ctid: FakeCtid, fcinfo: pg_sys::FunctionCallInfo) -> bool {
    unsafe { Ctid::from_fcinfo(fcinfo, 0) }.is_some_and(Ctid::is_valid)
}

#[pg_extern(stable, strict, parallel_safe)]
pub fn xmin_is_visible(
    xmin: pg_sys::TransactionId,
    tableoid: pg_sys::Oid,
    ctid: pg_sys::ItemPointerData,
) -> bool {
    unsafe {
        if !pg_sys::TransactionIdIsCurrentTransactionId(xmin) {
            return !pg_sys::XidInMVCCSnapshot(xmin, pg_sys::GetActiveSnapshot());
        }
        if !Ctid::from(ctid).is_valid() {
            return false;
        }

        // The tuple visibility check distinguishes earlier commands from current-command writes.
        let heaprel = PgSearchRelation::with_lock(tableoid, pg_sys::AccessShareLock as _);
        let mut tuple = pg_sys::HeapTupleData {
            t_self: ctid,
            ..Default::default()
        };
        let mut buffer = pg_sys::InvalidBuffer as pg_sys::Buffer;
        let visible = pg_sys::heap_fetch(
            heaprel.as_ptr(),
            pg_sys::GetActiveSnapshot(),
            &mut tuple,
            &mut buffer,
            false,
        );
        if buffer != pg_sys::InvalidBuffer as pg_sys::Buffer {
            pg_sys::ReleaseBuffer(buffer);
        }
        visible
    }
}

fn search_with_query_input_impl(
    fcinfo: pg_sys::FunctionCallInfo,
    ctid: Option<Ctid>,
) -> Option<bool> {
    let query_datum = unsafe { pg_getarg_datum(fcinfo, 1) }?;
    let query_datum = unsafe { pg_sys::pg_detoast_datum(query_datum.cast_mut_ptr()) };

    // get the Cache attached to this instance of the function
    let mut cache = unsafe { pg_func_extra(fcinfo, Cache::default) };

    let key = unsafe { pgrx::varlena_to_byte_slice(query_datum).to_vec() };
    if cache.by_query.get(&key).is_some_and(|entry| match entry {
        CacheEntry::Single(entry) => !entry.is_valid(),
        CacheEntry::Partitioned { by_child, .. } => {
            by_child.values().any(|entry| !entry.is_valid())
        }
    }) {
        cache.by_query.remove(&key);
    }

    let mut newly_built = false;
    let entry = cache.by_query.entry(key).or_insert_with(|| {
        newly_built = true;
        let search_query_input = unsafe { deserialize_query(query_datum) };

        // `empty()` cannot match any index document, including for a partial index.
        if matches!(&search_query_input, SearchQueryInput::Empty) {
            return CacheEntry::Single(QueryCacheEntry {
                matches: KeySet::None,
                missing_values: None,
            });
        }

        let index_oid = search_query_input.index_oid().unwrap_or_else(|| {
            panic!("pg_search: could not determine the index to use for this query")
        });

        // The planner resolves a query above an Append to the parent partitioned index;
        // its match sets are built per partition as rows arrive (#4643).
        if is_partitioned_index(index_oid) {
            return CacheEntry::Partitioned {
                parent_index_oid: index_oid,
                by_child: HashMap::default(),
            };
        }

        let index_relation =
            PgSearchRelation::with_lock(index_oid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
        CacheEntry::Single(build_query_cache_entry(
            fcinfo,
            ctid,
            index_relation,
            search_query_input,
        ))
    });

    let query_cache = match entry {
        CacheEntry::Single(query_cache) => query_cache,
        CacheEntry::Partitioned {
            parent_index_oid,
            by_child,
        } => {
            let (query_cache, built) =
                partition_entry(fcinfo, *parent_index_oid, by_child, |index_relation| {
                    let search_query_input = unsafe { deserialize_query(query_datum) };
                    build_query_cache_entry(fcinfo, ctid, index_relation, search_query_input)
                });
            newly_built |= built;
            query_cache
        }
    };

    // Reaching this function at all means the search-operator predicate is being applied as a
    // per-row filter rather than an index scan, so warn whenever we evaluate a query here -- regardless of the
    // all()/empty() short-circuits -- but at most once per statement. Separately warn if the
    // materialized match set spilled past work_mem.
    let spilled = newly_built
        && (matches!(query_cache.matches, KeySet::Spilled(_))
            || matches!(&query_cache.missing_values, Some(KeySet::Spilled(_))));

    let result = match (&query_cache.matches, &query_cache.missing_values) {
        (KeySet::All, None) => Some(true),
        (KeySet::None, None) => Some(false),
        (matches, missing_values) => {
            let ctid = ctid.expect("heap-filter query should carry a CTID");
            let row_identity = TantivyValue::try_from(u64::from(ctid))
                .expect("ctid should convert to a Tantivy value");

            if missing_values
                .as_ref()
                .is_some_and(|missing_values| missing_values.contains(&row_identity))
            {
                None
            } else {
                Some(matches.contains(&row_identity))
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

/// Materialize the match sets for one concrete leaf index: the slow path behind the
/// scalar operator, collecting matching CTIDs into a memory-bounded set that is reused
/// for every row of the scan.
fn build_query_cache_entry(
    fcinfo: pg_sys::FunctionCallInfo,
    ctid: Option<Ctid>,
    index_relation: PgSearchRelation,
    search_query_input: SearchQueryInput,
) -> QueryCacheEntry {
    let is_partial =
        unsafe { !pg_sys::RelationGetIndexPredicate(index_relation.as_ptr()).is_null() };
    let null_guard = index_relation
        .schema()
        .expect("a ParadeDB index must have a schema")
        .null_guard(&search_query_input);
    let is_match_all = search_query_input.is_match_all() && !is_partial;

    // `all()` matches every document, but a partial index may not contain every table row.
    if is_match_all && null_guard.is_none() {
        return QueryCacheEntry {
            matches: KeySet::All,
            missing_values: None,
        };
    }

    if ctid.is_none() {
        let index_info = unsafe { &*index_relation.index_info() };
        if is_partial
            && index_info.ii_IndexAttrNumbers[..index_info.ii_NumIndexAttrs as usize]
                .iter()
                .all(|&attno| attno == 0)
        {
            ErrorReport::new(
                PgSqlErrorCode::ERRCODE_FEATURE_NOT_SUPPORTED,
                "searches on expression-only partial indexes require an index scan",
                function_name!(),
            )
            .set_hint(
                "Add a directly indexed table column to support searches without an index scan.",
            )
            .report(PgLogLevel::ERROR);
        }
        report_missing_row_identity();
    }

    // Reaching here means the planner could not use the ParadeDB index to satisfy this query, so we
    // materialize the match set and apply it as a per-row filter (the slow path).

    let heap_relation = index_relation
        .heap_relation()
        .expect("a ParadeDB index must have a heap relation");
    let mut visibility = VisibilityChecker::with_rel_and_snap(&heap_relation, unsafe {
        pg_sys::GetActiveSnapshot()
    });
    let mut cache_context = unsafe { PgMemoryContexts::For((*(*fcinfo).flinfo).fn_mcxt) };

    // Collect matching CTIDs into a memory-bounded set (spills to a temp file past
    // `work_mem`), reused for every row of the scan.
    let matches = if is_match_all {
        KeySet::All
    } else {
        let search_reader = SearchIndexReader::open(
            &index_relation,
            search_query_input,
            false,
            MvccSatisfies::Snapshot,
        )
        .expect("search_with_query_input: should be able to open a SearchIndexReader");

        unsafe { cache_context.switch_to(|_| search_reader.collect_ctidset(&mut visibility)) }
    };

    let missing_values = if let Some(null_guard) = null_guard {
        // Collect rows where the field is absent (the complement of `exists`). Membership in
        // this set means SQL NULL for negation semantics.
        let complement_query = SearchQueryInput::WithIndex {
            oid: index_relation.oid(),
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
        .expect("search_with_query_input: should be able to open a complement SearchIndexReader");

        Some(unsafe {
            cache_context.switch_to(|_| complement_reader.collect_ctidset(&mut visibility))
        })
    } else {
        None
    };

    QueryCacheEntry {
        matches,
        missing_values,
    }
}

/// The entry for the partition this row came from, built on the partition's own index the
/// first time a row from it arrives. The `bool` reports whether this call built it.
fn partition_entry<T>(
    fcinfo: pg_sys::FunctionCallInfo,
    parent_index_oid: pg_sys::Oid,
    by_child: &mut HashMap<pg_sys::Oid, T>,
    build: impl FnOnce(PgSearchRelation) -> T,
) -> (&mut T, bool) {
    let child_heap_oid =
        unsafe { record_tableoid(fcinfo) }.unwrap_or_else(|| report_missing_row_identity());
    let mut built = false;
    let entry = by_child.entry(child_heap_oid).or_insert_with(|| {
        built = true;
        build(
            partition_member_index(child_heap_oid, parent_index_oid).unwrap_or_else(|| {
                report_missing_partition_index(child_heap_oid, parent_index_oid)
            }),
        )
    });
    (entry, built)
}

/// A [`RowMatcher`] in the function's own memory context, so it outlives this call.
unsafe fn build_row_matcher(
    fcinfo: pg_sys::FunctionCallInfo,
    index_relation: PgSearchRelation,
    query: SearchQueryInput,
) -> RowMatcher {
    unsafe {
        PgMemoryContexts::For((*(*fcinfo).flinfo).fn_mcxt)
            .switch_to(|_| RowMatcher::new(index_relation, query))
    }
}

/// The detoasted query argument as a [`SearchQueryInput`].
unsafe fn deserialize_query(query_datum: *mut pg_sys::varlena) -> SearchQueryInput {
    unsafe {
        SearchQueryInput::from_datum(query_datum.into(), false)
            .expect("the query argument cannot be NULL")
    }
}

/// The `tableoid` shipped as the trailing column of the final record argument when the
/// planner resolved a partitioned index (#4643). `None` when no such column exists, e.g.
/// for a form without row identity.
unsafe fn record_tableoid(fcinfo: pg_sys::FunctionCallInfo) -> Option<pg_sys::Oid> {
    unsafe {
        // The record is the last argument of every heap-filter form.
        let nargs = (*fcinfo).nargs;
        if nargs < 4 {
            return None;
        }
        let record = pg_getarg_datum(fcinfo, nargs as usize - 1)?;
        let record = PgHeapTuple::from_composite_datum(record);
        record.get_by_name::<pg_sys::Oid>("tableoid").ok().flatten()
    }
}

/// A search evaluated without access to the identity of the row it is filtering.
fn report_missing_row_identity() -> ! {
    ErrorReport::new(
        PgSqlErrorCode::ERRCODE_FEATURE_NOT_SUPPORTED,
        "search query requires row identity that is unavailable in this context",
        function_name!(),
    )
    .set_hint("Apply the search operator in a table query. Use an ordinary SQL predicate to define a partial index.")
    .report(PgLogLevel::ERROR);
    unreachable!()
}

/// A partition with no valid member of the partitioned index the query was planned with.
fn report_missing_partition_index(child_heap_oid: pg_sys::Oid, parent_index_oid: pg_sys::Oid) -> ! {
    ErrorReport::new(
        PgSqlErrorCode::ERRCODE_UNDEFINED_OBJECT,
        format!(
            "partition \"{}\" has no valid member of the partitioned index \"{}\"",
            PgSearchRelation::open(child_heap_oid).name(),
            PgSearchRelation::open(parent_index_oid).name(),
        ),
        function_name!(),
    )
    .report(PgLogLevel::ERROR);
    unreachable!()
}

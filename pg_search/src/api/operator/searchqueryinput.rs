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
use super::keyset::KeySet;
use super::{anyelement_query_input_opoid, request_simplify};
use crate::api::operator::{estimate_selectivity, find_var_relation, ReturnedNodePointer};
use crate::api::{HashMap, HashSet};
use crate::gucs::per_tuple_cost;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::planner_warnings::{warn_filter_spilled, warn_sequential_scan};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::types::TantivyValue;
use crate::query::SearchQueryInput;
use crate::{nodecast, PARAMETERIZED_SELECTIVITY, UNKNOWN_SELECTIVITY};
use pgrx::callconv::{Arg, ArgAbi};
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, ReturnsError, ReturnsRef, SqlMappingRef, SqlTranslatable, TypeOrigin,
};
use pgrx::{
    pg_extern, pg_func_extra, pg_getarg_datum_raw, pg_getarg_type, pg_sys, Internal, PgList, PgOid,
    PgRelation,
};
use std::ptr::NonNull;

/// SQL API for allowing the user to specify the index to query.
///
/// This is useful (required, even) in cases where a query must be planned a sequential scan.
///
/// An example might be a query like this, that reads "find everything from `t` where the `body` field
/// contains a term from the `keywords` field.
///
/// ```sql
/// SELECT * FROM t WHERE key_field @@@ paradedb.term('body', keywords);
/// ```
///
/// In order for pg_search to execute this, we need to know the index to use, so it would need to be written
/// as:
///
/// ```sql
/// SELECT * FROM t WHERE key_field @@@ paradedb.with_index('bm25_idxt', paradedb.term('body', keywords));
/// ```
#[pg_extern(immutable, parallel_safe)]
pub fn with_index(index: PgRelation, query: SearchQueryInput) -> SearchQueryInput {
    SearchQueryInput::WithIndex {
        oid: index.oid(),
        query: Box::new(query),
    }
}

struct QueryCacheEntry {
    element_oid: PgOid,
    matches: KeySet,
    /// Key-field values for rows where the indexed field is absent (SQL NULL semantics).
    missing_values: Option<KeySet>,
}

#[derive(Default)]
struct Cache {
    by_query: HashMap<Vec<u8>, QueryCacheEntry>,
}

/// Allows us to have a UDF with an argument of type `anyelement` but not do any pgrx-related
/// datum conversion
pub struct FakeAnyElement;

/// Allows us to have a UDF with an argument of type `SearchQueryInput` but not do any pgrx-related
/// datum conversion
pub struct FakeSearchQueryInput;

unsafe impl<'fcx> ArgAbi<'fcx> for FakeAnyElement {
    unsafe fn unbox_arg_unchecked(_arg: Arg<'_, 'fcx>) -> Self {
        Self
    }
}

// Unlike `FakeSearchQueryInput` below, this does not borrow another type's resolution
// because `anyelement` is a Postgres pseudo-type with no graph entry to depend on.
unsafe impl SqlTranslatable for FakeAnyElement {
    const TYPE_IDENT: &'static str = pgrx::pgrx_resolved_type!(FakeAnyElement);
    const TYPE_ORIGIN: TypeOrigin = TypeOrigin::External;
    const ARGUMENT_SQL: Result<SqlMappingRef, ArgumentError> =
        Ok(SqlMappingRef::literal("anyelement"));
    const RETURN_SQL: Result<ReturnsRef, ReturnsError> = Err(ReturnsError::Datum);
}

unsafe impl<'fcx> ArgAbi<'fcx> for FakeSearchQueryInput {
    unsafe fn unbox_arg_unchecked(_arg: Arg<'_, 'fcx>) -> Self {
        Self
    }
}

unsafe impl SqlTranslatable for FakeSearchQueryInput {
    // This is intentionally borrowing `SearchQueryInput`'s `TYPE_IDENT`: current
    // pgrx tolerates two Rust types sharing that identifier, and we rely on that
    // observed behavior so the SQL graph still emits `CREATE TYPE SearchQueryInput`
    // before any function that consumes this fake wrapper.
    const TYPE_IDENT: &'static str = <SearchQueryInput as SqlTranslatable>::TYPE_IDENT;
    const TYPE_ORIGIN: TypeOrigin = <SearchQueryInput as SqlTranslatable>::TYPE_ORIGIN;
    const ARGUMENT_SQL: Result<SqlMappingRef, ArgumentError> =
        <SearchQueryInput as SqlTranslatable>::ARGUMENT_SQL;
    const RETURN_SQL: Result<ReturnsRef, ReturnsError> = Err(ReturnsError::Datum);
}

/// Whether the null-preserving existence guard is valid for this field.
///
/// The guard equates "field absent from the index" with SQL NULL, which is only
/// correct for scalar columns. Array and JSON columns can be non-NULL in SQL
/// while having no indexed values (e.g. `'{}'::text[]`, `'{}'::jsonb`).
fn field_supports_null_preserving_guard(
    schema: &crate::schema::SearchIndexSchema,
    field: &str,
) -> bool {
    let Some(search_field) = schema.search_field(field) else {
        return false;
    };
    if !search_field.is_fast() {
        return false;
    }

    let categorized = schema.categorized_fields();
    let root = crate::api::FieldName::from(field).root();
    categorized
        .iter()
        .find(|(sf, _)| sf.field_name().root() == root)
        .is_none_or(|(_, data)| !data.is_array && !data.is_json)
}

#[allow(unused_variables)]
#[pg_extern(immutable, parallel_safe, cost = 1000000000)]
pub fn search_with_query_input(
    element: FakeAnyElement,
    query: FakeSearchQueryInput,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<bool> {
    assert!(
        unsafe { (*(*fcinfo).flinfo).fn_strict },
        "paradedb.search_with_query_input must be STRICT"
    );

    // get the Cache attached to this instance of the function
    let mut cache = unsafe { pg_func_extra(fcinfo, Cache::default) };

    // get the raw query datum from fcinfo.  because this function is declared STRICT we're guaranteed
    // that it won't be SQL NULL
    let query_datum = unsafe { pg_getarg_datum_raw(fcinfo, 1) };

    // we build a cache of query results, where the key is the Vec<u8> representation of the raw query datum.
    // this form is chosen as it's the most efficient way to uniquely identify the input query with as
    // minimal overhead as possible.
    let key = unsafe {
        let varlena = query_datum.cast_mut_ptr::<pg_sys::varlena>();
        pgrx::varlena_to_byte_slice(varlena).to_vec()
    };

    let mut newly_built = false;
    let query_cache = cache.by_query.entry(key).or_insert_with(|| {
        newly_built = true;
        let element_oid = PgOid::from_untagged(unsafe { pg_getarg_type(fcinfo, 0) });
        let search_query_input = unsafe {
            SearchQueryInput::from_datum(query_datum, query_datum.is_null())
                .expect("the query argument cannot be NULL")
        };

        // Short-circuit the degenerate queries so we never materialize a key per row: `all()` (in
        // any wrapped/fielded form) matches everything, `empty()` matches nothing.
        if search_query_input.is_match_all() {
            return QueryCacheEntry {
                element_oid,
                matches: KeySet::All,
                missing_values: None,
            };
        }
        if matches!(&search_query_input, SearchQueryInput::Empty) {
            return QueryCacheEntry {
                element_oid,
                matches: KeySet::None,
                missing_values: None,
            };
        }

        // Reaching here means the planner could not use the BM25 index to satisfy this query, so we
        // materialize the match set and apply it as a per-row filter (the slow path).
        let index_oid = search_query_input.index_oid().unwrap_or_else(|| {
            panic!("pg_search: could not determine the index to use for this query")
        });

        let index_relation =
            PgSearchRelation::with_lock(index_oid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);

        let mut field_names = HashSet::default();
        search_query_input.extract_field_names(&mut field_names);

        // For an `Exists` predicate a missing field is FALSE, not NULL, so we skip the missing-values
        // computation below and let missing fields fall through to `Some(false)`.
        let is_exists_query = search_query_input.is_exists();

        let search_reader = SearchIndexReader::open(
            &index_relation,
            search_query_input,
            false,
            MvccSatisfies::Snapshot,
        )
        .expect("search_with_query_input: should be able to open a SearchIndexReader");
        let schema = search_reader.schema();

        // collect the matching key-field values into a memory-bounded set (spills to a temp file
        // past `work_mem`), reused for every row of the scan.
        let matches = search_reader.collect_keyset();

        let missing_values = if field_names.len() == 1 && !is_exists_query {
            let field = field_names
                .into_iter()
                .next()
                .expect("field_names should contain exactly one field");

            if !field_supports_null_preserving_guard(schema, &field) {
                return QueryCacheEntry {
                    element_oid,
                    matches,
                    missing_values: None,
                };
            }

            // Collect rows where the field is absent (the complement of `exists`). Membership in
            // this set means SQL NULL for negation semantics.
            let complement_query = SearchQueryInput::WithIndex {
                oid: index_oid,
                query: Box::new(SearchQueryInput::Boolean {
                    must: vec![SearchQueryInput::All],
                    should: Default::default(),
                    must_not: vec![SearchQueryInput::FieldedQuery {
                        field: field.into(),
                        query: crate::query::pdb_query::pdb::Query::Exists,
                    }],
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

            Some(complement_reader.collect_keyset())
        } else {
            None
        };

        QueryCacheEntry {
            element_oid,
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

    // see if the value on the lhs of the operator (which should always be our "key_field") is
    // contained in the matches set
    let result = unsafe {
        let element = pg_getarg_datum_raw(fcinfo, 0);
        let user_value = TantivyValue::try_from_datum(element, query_cache.element_oid)
            .expect("no value present");

        if query_cache.matches.contains(&user_value) {
            Some(true)
        } else if let Some(missing_values) = &query_cache.missing_values {
            if missing_values.contains(&user_value) {
                None
            } else {
                Some(false)
            }
        } else {
            Some(false)
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

#[pg_extern(immutable, parallel_safe)]
pub unsafe fn query_input_support(arg: Internal) -> ReturnedNodePointer {
    let datum = match arg.unwrap() {
        Some(d) => d,
        None => return ReturnedNodePointer(None),
    };

    if let Some(node) = query_input_support_request_simplify(datum) {
        return node;
    }

    if let Some(node) = search_query_input_request_cost(datum) {
        return node;
    }

    ReturnedNodePointer(None)
}

fn query_input_support_request_simplify(arg: pg_sys::Datum) -> Option<ReturnedNodePointer> {
    unsafe {
        request_simplify(
            arg.cast_mut_ptr::<pg_sys::Node>(),
            //
            // if either of these closures are called that represents a logic error
            // in `request_simplify`'s implementation
            //
            // in this case, we know that the rhs of the expression has a type of `SearchQueryInput`
            // so there's no need for additional rewriting
            //
            |_, _, _| {
                unreachable!(
                    "query_input_support_request_simplify should never be called for Const rewriting"
                )
            },
            |_, _, _| {
                unreachable!("query_input_support_request_simplify should never be called for rhs expression rewriting")
            },
        )
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn query_input_restrict(
    planner_info: Internal, // <pg_sys::PlannerInfo>,
    operator_oid: pg_sys::Oid,
    args: Internal, // <pg_sys::List>,
    _var_relid: i32,
) -> f64 {
    fn inner_query_input(
        planner_info: Internal, // <pg_sys::PlannerInfo>,
        args: Internal,         // <pg_sys::List>,
    ) -> Option<f64> {
        unsafe {
            let info = planner_info.unwrap()?.cast_mut_ptr::<pg_sys::PlannerInfo>();
            let args =
                PgList::<pg_sys::Node>::from_pg(args.unwrap()?.cast_mut_ptr::<pg_sys::List>());

            let var = nodecast!(Var, T_Var, args.get_ptr(0)?)?;
            let rhs = args.get_ptr(1)?;

            match (*rhs).type_ {
                pg_sys::NodeTag::T_Const => {
                    let const_ = rhs.cast::<pg_sys::Const>();
                    let (heaprelid, _, _) = find_var_relation(var, info);
                    let indexrel = rel_get_bm25_index(heaprelid)?.1;
                    let search_query_input =
                        SearchQueryInput::from_datum((*const_).constvalue, (*const_).constisnull)?;
                    estimate_selectivity(&indexrel, search_query_input)
                }
                pg_sys::NodeTag::T_Param => Some(PARAMETERIZED_SELECTIVITY),
                _ => None,
            }
        }
    }

    assert!(operator_oid == anyelement_query_input_opoid());

    let mut selectivity = inner_query_input(planner_info, args).unwrap_or(UNKNOWN_SELECTIVITY);
    if selectivity > 1.0 {
        selectivity = UNKNOWN_SELECTIVITY;
    }

    selectivity
}

fn search_query_input_request_cost(arg: pg_sys::Datum) -> Option<ReturnedNodePointer> {
    unsafe {
        let src = nodecast!(
            SupportRequestCost,
            T_SupportRequestCost,
            arg.cast_mut_ptr::<pg_sys::Node>()
        )?;
        // our `search_with_*` functions are *incredibly* expensive.  So much so that
        // we really don't ever want Postgres to prefer them.
        //
        // The higher the `per_tuple` cost is here, the better.
        //
        // it can cost a lot to startup the `@@@` operator outside of an IndexScan because
        // ultimately we have to hash all the resulting ctids in memory.  For lack of a better
        // value, we say it costs as much as the `GUCS.per_tuple_cost()`.  This is an arbitrary
        // number that we've documented as needing to be big.
        (*src).startup = per_tuple_cost();

        // similarly, use the same GUC here.  Postgres will then add this into its per-tuple
        // cost evaluations for whatever scan it's considering using for the `@@@` operator.
        // our IAM provides more intelligent costs for the IndexScan situation.
        (*src).per_tuple = per_tuple_cost();

        Some(ReturnedNodePointer(NonNull::new(src.cast())))
    }
}

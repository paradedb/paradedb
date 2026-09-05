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
use super::anyelement_query_input_opoid;
use crate::api::operator::{ReturnedNodePointer, estimate_selectivity, find_var_relation};
use crate::postgres::rel_get_bm25_index;
use crate::query::SearchQueryInput;
use crate::{PARAMETERIZED_SELECTIVITY, UNKNOWN_SELECTIVITY, nodecast};
use pgrx::{Internal, PgList, PgRelation, pg_extern, pg_sys};

/// SQL API for allowing the user to specify the index to query.
///
/// This is useful (required, even) in cases where a query must be planned a sequential scan.
///
/// An example might be a query like this, that reads "find everything from `t` where the `body` field
/// contains a term from the `keywords` field.
///
/// ```sql
/// SELECT * FROM t WHERE indexed_field @@@ paradedb.term('body', keywords);
/// ```
///
/// In order for pg_search to execute this, we need to know the index to use, so it would need to be written
/// as:
///
/// ```sql
/// SELECT * FROM t WHERE indexed_field @@@ paradedb.with_index('bm25_idxt', paradedb.term('body', keywords));
/// ```
#[pg_extern(immutable, parallel_safe)]
pub fn with_index(index: PgRelation, query: SearchQueryInput) -> SearchQueryInput {
    SearchQueryInput::WithIndex {
        oid: index.oid(),
        query: Box::new(query),
    }
}

/// PostgreSQL asks a function's support callback for an index condition when any one of the
/// function's arguments matches an index column; it intentionally leaves validation of the other
/// arguments to the callback. This lets the CTID-aware heap predicate trigger on its anchor while
/// `ReturnedNodePointer::for_support_index_condition` derives an exact `anchor @@@ query` qual that
/// excludes the row-varying CTID argument.
#[pg_extern(immutable, parallel_safe)]
pub unsafe fn query_input_support(arg: Internal) -> ReturnedNodePointer {
    let datum = match arg.unwrap() {
        Some(d) => d,
        None => return ReturnedNodePointer::unsupported(),
    };

    let request = datum.cast_mut_ptr::<pg_sys::Node>();
    match (*request).type_ {
        pg_sys::NodeTag::T_SupportRequestSimplify => {
            ReturnedNodePointer::for_support_simplify(request, super::SimplifyRhs::SearchQueryInput)
        }
        pg_sys::NodeTag::T_SupportRequestIndexCondition => {
            ReturnedNodePointer::for_support_index_condition(request.cast())
        }
        pg_sys::NodeTag::T_SupportRequestSelectivity => {
            ReturnedNodePointer::for_support_selectivity(request.cast())
        }
        pg_sys::NodeTag::T_SupportRequestCost => {
            ReturnedNodePointer::for_support_cost(request.cast())
        }
        _ => ReturnedNodePointer::unsupported(),
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn query_input_restrict(
    planner_info: Internal, // <pg_sys::PlannerInfo>,
    operator_oid: pg_sys::Oid,
    args: Internal, // <pg_sys::List>,
    _var_relid: i32,
) -> f64 {
    assert!(operator_oid == anyelement_query_input_opoid());
    unsafe {
        query_input_selectivity(
            planner_info
                .unwrap()
                .map_or(std::ptr::null_mut(), |datum| datum.cast_mut_ptr()),
            args.unwrap()
                .map_or(std::ptr::null_mut(), |datum| datum.cast_mut_ptr()),
        )
    }
}

pub(super) unsafe fn query_input_selectivity(
    planner_info: *mut pg_sys::PlannerInfo,
    args: *mut pg_sys::List,
) -> f64 {
    if planner_info.is_null() || args.is_null() {
        return UNKNOWN_SELECTIVITY;
    }

    let args = PgList::<pg_sys::Node>::from_pg(args);
    let Some(var) = args.get_ptr(0).and_then(|lhs| nodecast!(Var, T_Var, lhs)) else {
        return UNKNOWN_SELECTIVITY;
    };
    let Some(rhs) = args.get_ptr(1) else {
        return UNKNOWN_SELECTIVITY;
    };

    let selectivity = match (*rhs).type_ {
        pg_sys::NodeTag::T_Const => {
            let const_ = rhs.cast::<pg_sys::Const>();
            let (heaprelid, _, _) = find_var_relation(var, planner_info);
            let Some((_, indexrel)) = rel_get_bm25_index(heaprelid) else {
                return UNKNOWN_SELECTIVITY;
            };
            let Some(search_query_input) =
                SearchQueryInput::from_datum((*const_).constvalue, (*const_).constisnull)
            else {
                return UNKNOWN_SELECTIVITY;
            };
            estimate_selectivity(&indexrel, search_query_input).unwrap_or(UNKNOWN_SELECTIVITY)
        }
        pg_sys::NodeTag::T_Param => PARAMETERIZED_SELECTIVITY,
        _ => UNKNOWN_SELECTIVITY,
    };

    if selectivity <= 1.0 {
        selectivity
    } else {
        UNKNOWN_SELECTIVITY
    }
}

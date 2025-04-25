// Copyright (c) 2023-2025 ParadeDB, Inc.
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
use super::{
    anyelement_query_input_opoid, anyelement_query_input_procoid, anyelement_text_opoid,
    make_search_query_input_opexpr_node,
};
use crate::api::operator::{estimate_selectivity, find_var_relation, ReturnedNodePointer};
use crate::gucs::per_tuple_cost;
use crate::index::fast_fields_helper::FFHelper;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::types::TantivyValue;
use crate::postgres::utils::locate_bm25_index;
use crate::query::SearchQueryInput;
use crate::{nodecast, UNKNOWN_SELECTIVITY};
use parking_lot::Mutex;
use pgrx::{
    check_for_interrupts, pg_extern, pg_func_extra, pg_sys, AnyElement, FromDatum, Internal,
    PgList, PgOid, PgRelation,
};
use rustc_hash::{FxHashMap, FxHashSet};
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

#[derive(Default)]
struct Cache {
    search_readers: Mutex<FxHashMap<pg_sys::Oid, (SearchIndexReader, FFHelper)>>,
    matches: Mutex<FxHashMap<(pg_sys::Oid, String), FxHashSet<TantivyValue>>>,
}

#[pg_extern(immutable, parallel_safe, cost = 1000000000)]
pub fn search_with_query_input(
    element: AnyElement,
    query: SearchQueryInput,
    fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    let index_oid = query
        .index_oid()
        .unwrap_or_else(|| panic!("the query argument must be wrapped in a `SearchQueryInput::WithIndex` variant.  Try using `paradedb.with_index('<index name>', <original expression>)`"));

    // get the Cache attached to this instance of the function
    let cache = unsafe { pg_func_extra(fcinfo, Cache::default) };

    // and get/initialize the SearchReader and FFHelper for this index_oid
    let mut search_readers = cache.search_readers.lock();
    let (search_reader, ff_helper) = search_readers.entry(index_oid).or_insert_with(|| {
        let index_relation = unsafe {
            PgRelation::with_lock(index_oid, pg_sys::AccessShareLock as pg_sys::LOCKMODE)
        };
        let search_reader = SearchIndexReader::open(&index_relation, MvccSatisfies::Snapshot)
            .expect("search_with_query_input: should be able to open a SearchIndexReader");
        let key_field = search_reader.key_field();
        let key_field_name = key_field.name.0;
        let key_field_type = key_field.type_.into();
        let ff_helper =
            FFHelper::with_fields(&search_reader, &[(key_field_name, key_field_type).into()]);

        (search_reader, ff_helper)
    });

    // now, query the SearchReader and collect up the docs that match our query.
    // the matches are cached so that the same input query will return the same results
    // throughout the duration of the scan
    let mut matches = cache.matches.lock();
    let matches_key = (index_oid, format!("{query:?}")); // NB:  ideally, `SearchQueryInput` would `#[derive(Hash)]`, but it can't (easily)
    let matches = matches.entry(matches_key).or_insert_with(|| {
        search_reader
            .search(query.need_scores(), false, &query, None)
            .map(|(_, doc_address)| {
                check_for_interrupts!();
                ff_helper
                    .value(0, doc_address)
                    .expect("key_field value should not be null")
            })
            .collect()
    });

    // finally, see if the value on the lhs of the @@@ operator (which should always be our "key_field")
    // is contained in the matches set
    unsafe {
        let user_value =
            TantivyValue::try_from_datum(element.datum(), PgOid::from_untagged(element.oid()))
                .expect("no value present");

        matches.contains(&user_value)
    }
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

    if let Some(node) = query_input_support_request_index_condition(datum) {
        return node;
    }

    ReturnedNodePointer(None)
}

fn query_input_support_request_simplify(arg: pg_sys::Datum) -> Option<ReturnedNodePointer> {
    unsafe {
        pgrx::log!("query_input_support_request_simplify: Starting function");

        // Try to cast the input arg to a SupportRequestSimplify node
        let srs = match nodecast!(
            SupportRequestSimplify,
            T_SupportRequestSimplify,
            arg.cast_mut_ptr::<pg_sys::Node>()
        ) {
            Some(node) => {
                pgrx::log!("query_input_support_request_simplify: Successfully cast to SupportRequestSimplify");
                node
            }
            None => {
                pgrx::log!("query_input_support_request_simplify: Failed to cast to SupportRequestSimplify");
                pgrx::log!(
                    "query_input_support_request_simplify: Node type={:?}",
                    (*arg.cast_mut_ptr::<pg_sys::Node>()).type_
                );
                return None;
            }
        };

        // Log if the fcall is null
        if (*srs).fcall.is_null() {
            pgrx::log!("query_input_support_request_simplify: fcall is null");
            return None;
        }

        // Log function details
        let func_oid = (*(*srs).fcall).funcid;
        pgrx::log!(
            "query_input_support_request_simplify: Function OID={:?}",
            func_oid
        );

        if let Ok(func_name) = std::ffi::CStr::from_ptr(pg_sys::get_func_name(func_oid)).to_str() {
            pgrx::log!(
                "query_input_support_request_simplify: Function name={}",
                func_name
            );
        }

        // Rewrite this node to use the @@@(key_field, paradedb.searchqueryinput) operator.
        // This involves converting the rhs of the operator into a SearchQueryInput.
        let mut input_args = PgList::<pg_sys::Node>::from_pg((*(*srs).fcall).args);

        pgrx::log!(
            "query_input_support_request_simplify: Input args length={}",
            input_args.len()
        );

        // Log arg types
        for (i, arg) in input_args.iter_ptr().enumerate() {
            pgrx::log!(
                "query_input_support_request_simplify: Arg #{} type={:?}",
                i,
                (*arg).type_
            );
        }

        // Get the left-hand side variable
        let var = match nodecast!(Var, T_Var, input_args.get_ptr(0)?) {
            Some(v) => {
                pgrx::log!(
                    "query_input_support_request_simplify: Left arg is Var with varno={}, varattno={}, varlevelsup={}",
                    (*v).varno, (*v).varattno, (*v).varlevelsup
                );
                v
            }
            None => {
                pgrx::log!("query_input_support_request_simplify: Left arg is not a Var");
                return None;
            }
        };

        // Get the right-hand side
        let rhs = match input_args.get_ptr(1) {
            Some(node) => {
                pgrx::log!(
                    "query_input_support_request_simplify: Right arg type={:?}",
                    (*node).type_
                );
                node
            }
            None => {
                pgrx::log!("query_input_support_request_simplify: Right arg is missing");
                return None;
            }
        };

        // Try to get the query from the right-hand side
        let query = nodecast!(Const, T_Const, rhs)
            .map(|const_| {
                pgrx::log!(
                    "query_input_support_request_simplify: Right arg is Const with consttype={:?}, constisnull={}",
                    (*const_).consttype, (*const_).constisnull
                );
                SearchQueryInput::from_datum((*const_).constvalue, (*const_).constisnull)
            })
            .flatten();

        if query.is_none() {
            pgrx::log!("query_input_support_request_simplify: Failed to convert right arg to SearchQueryInput");
        } else {
            pgrx::log!("query_input_support_request_simplify: Successfully converted right arg to SearchQueryInput");
        }

        // Get operator OIDs for logging
        let op_oid = anyelement_query_input_opoid();
        let proc_oid = anyelement_query_input_procoid();
        pgrx::log!(
            "query_input_support_request_simplify: Using operator OID={:?}, proc OID={:?}",
            op_oid,
            proc_oid
        );

        // Create the node
        let result = make_search_query_input_opexpr_node(
            srs,
            &mut input_args,
            var,
            query,
            None,
            op_oid,
            proc_oid,
        );

        pgrx::log!(
            "query_input_support_request_simplify: Result node created={}",
            result.0.is_some()
        );

        Some(result)
    }
}

fn query_input_support_request_index_condition(arg: pg_sys::Datum) -> Option<ReturnedNodePointer> {
    unsafe {
        pgrx::log!("query_input_support_request_index_condition: Starting function");

        // Try to cast the input arg to a SupportRequestIndexCondition node
        let src = match nodecast!(
            SupportRequestIndexCondition,
            T_SupportRequestIndexCondition,
            arg.cast_mut_ptr::<pg_sys::Node>()
        ) {
            Some(node) => {
                pgrx::log!("query_input_support_request_index_condition: Successfully cast to SupportRequestIndexCondition");
                node
            }
            None => {
                pgrx::log!("query_input_support_request_index_condition: Failed to cast to SupportRequestIndexCondition");
                pgrx::log!(
                    "query_input_support_request_index_condition: Node type={:?}",
                    (*arg.cast_mut_ptr::<pg_sys::Node>()).type_
                );
                return None;
            }
        };

        // Log node information
        pgrx::log!("query_input_support_request_index_condition: Examining node");

        if (*src).node.is_null() {
            pgrx::log!("query_input_support_request_index_condition: node is null");
            return None;
        }

        // Check if the node is an OpExpr and uses our operator
        let node_type = (*(*src).node).type_;
        pgrx::log!(
            "query_input_support_request_index_condition: Node type={:?}",
            node_type
        );

        // Verify it's our operator
        if node_type == pg_sys::NodeTag::T_OpExpr {
            let opexpr = (*src).node.cast::<pg_sys::OpExpr>();
            let op_oid = (*opexpr).opno;
            let our_query_input_op_oid = anyelement_query_input_opoid();
            let our_text_op_oid = anyelement_text_opoid(); // Get the text version of the operator OID

            pgrx::log!(
                "query_input_support_request_index_condition: Checking if op_oid={:?} matches our BM25 operators (query_input={:?}, text={:?})",
                op_oid,
                our_query_input_op_oid,
                our_text_op_oid
            );

            // Check if it's either of our operators
            let is_our_operator = op_oid == our_query_input_op_oid || op_oid == our_text_op_oid;

            if is_our_operator {
                pgrx::log!("query_input_support_request_index_condition: Found our BM25 operator");

                // Get the index of the argument that corresponds to the index column
                let indexarg = (*src).indexarg;
                pgrx::log!(
                    "query_input_support_request_index_condition: indexarg={}",
                    indexarg
                );

                // Get all arguments
                let mut args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
                if args.len() >= 2 && indexarg >= 0 && (indexarg as usize) < args.len() {
                    pgrx::log!(
                        "query_input_support_request_index_condition: Examining operator arguments: {:?}",
                        args.len()
                    );

                    // For BM25, we typically expect the index column to be the left arg (0)
                    // and our query to be the right arg (1)
                    if let (Some(larg), Some(rarg)) = (args.get_ptr(0), args.get_ptr(1)) {
                        // Check that this looks like a proper BM25 operator usage
                        if let Some(var) = nodecast!(Var, T_Var, larg) {
                            // Check right argument - can be either a Const with text or with SearchQueryInput
                            if let Some(const_) = nodecast!(Const, T_Const, rarg) {
                                // For text operator
                                if op_oid == our_text_op_oid {
                                    // This is our text operator - mark it as not lossy
                                    (*src).lossy = false;

                                    // Create a list with the original expression
                                    let result_list = pg_sys::NIL.cast_mut();
                                    let result_list =
                                        pg_sys::lappend(result_list, (*src).node.cast());

                                    pgrx::log!("query_input_support_request_index_condition: Created result list for text operator");

                                    return Some(ReturnedNodePointer(NonNull::new(
                                        result_list.cast(),
                                    )));
                                }
                                // For SearchQueryInput operator
                                else {
                                    let query = SearchQueryInput::from_datum(
                                        (*const_).constvalue,
                                        (*const_).constisnull,
                                    );
                                    if query.is_some() {
                                        pgrx::log!("query_input_support_request_index_condition: Successfully extracted search query");

                                        // For BM25, we can handle the condition exactly as is
                                        // This is not a "lossy" index condition
                                        (*src).lossy = false;

                                        // Create a list with the original expression
                                        let result_list = pg_sys::NIL.cast_mut();
                                        let result_list =
                                            pg_sys::lappend(result_list, (*src).node.cast());

                                        pgrx::log!("query_input_support_request_index_condition: Created result list for query_input operator");

                                        return Some(ReturnedNodePointer(NonNull::new(
                                            result_list.cast(),
                                        )));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        pgrx::log!("query_input_support_request_index_condition: Could not handle index condition");
        None
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
            let const_ = nodecast!(Const, T_Const, args.get_ptr(1)?)?;

            let (heaprelid, _, _) = find_var_relation(var, info);
            let indexrel = locate_bm25_index(heaprelid)?;

            // create the search query from the rhs Const node
            let search_query_input =
                SearchQueryInput::from_datum((*const_).constvalue, (*const_).constisnull)?;

            estimate_selectivity(&indexrel, &search_query_input)
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

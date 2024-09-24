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

use crate::api::operator::{
    anyelement_query_input_opoid, estimate_selectivity, find_var_relation,
    make_search_config_opexpr_node, ReturnedNodePointer,
};
use crate::postgres::utils::{locate_bm25_index, relfilenode_from_pg_relation};
use crate::query::SearchQueryInput;
use crate::schema::SearchConfig;
use crate::UNKNOWN_SELECTIVITY;
use pgrx::{is_a, pg_extern, pg_sys, AnyElement, FromDatum, Internal, PgList};

/// This is the function behind the `@@@(anyelement, paradedb.searchqueryinput)` operator. Since we
/// transform those to use `@@@(anyelement, jsonb`), this function won't be called in normal
/// circumstances, but it could be called if the rhs of the @@@ is some kind of volatile value.
///
/// And in that case we just have to give up.
#[pg_extern(immutable, parallel_safe)]
pub fn search_with_query_input(
    _element: AnyElement,
    query: SearchQueryInput,
    _fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    panic!("query is incompatible with pg_search's `@@@(key_field, paradedb.searchqueryinput)` operator: `{query:?}`")
}

#[pg_extern(immutable, parallel_safe)]
pub fn query_input_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        let node = arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>();

        match (*node).type_ {
            // rewrite this node, which is using the @@@(key_field, paradedb.searchqueryinput) operator
            // to instead use the @@@(key_field, jsonb) operator.  This involves converting the rhs
            // of the operator into the jsonb representation of a SearchConfig, which is built
            // in `make_new_opexpr_node()`
            pg_sys::NodeTag::T_SupportRequestSimplify => {
                let srs = node.cast::<pg_sys::SupportRequestSimplify>();

                let mut input_args = PgList::<pg_sys::Node>::from_pg((*(*srs).fcall).args);
                if let (Some(lhs), Some(rhs)) = (input_args.get_ptr(0), input_args.get_ptr(1)) {
                    if is_a(lhs, pg_sys::NodeTag::T_Var) && is_a(rhs, pg_sys::NodeTag::T_Const) {
                        let var = lhs.cast::<pg_sys::Var>();
                        let const_ = rhs.cast::<pg_sys::Const>();

                        // the query comes from the rhs of the @@@ operator.  we've already proved it's a `pg_sys::Const` node
                        let query = SearchQueryInput::from_datum(
                            (*const_).constvalue,
                            (*const_).constisnull,
                        )
                        .expect("query must not be NULL");

                        return make_search_config_opexpr_node(srs, &mut input_args, var, query);
                    }
                }

                ReturnedNodePointer(None)
            }

            _ => ReturnedNodePointer(None),
        }
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

            let (lhs, rhs) = (args.get_ptr(0)?, args.get_ptr(1)?);
            if is_a(lhs, pg_sys::NodeTag::T_Var) && is_a(rhs, pg_sys::NodeTag::T_Const) {
                let var = lhs.cast::<pg_sys::Var>();
                let const_ = rhs.cast::<pg_sys::Const>();

                let (heaprelid, _) = find_var_relation(var, info);
                let indexrel = locate_bm25_index(heaprelid)?;
                let relfilenode = relfilenode_from_pg_relation(&indexrel);

                let query =
                    SearchQueryInput::from_datum((*const_).constvalue, (*const_).constisnull)?;
                let search_config = SearchConfig::from((query, indexrel));

                return estimate_selectivity(heaprelid, relfilenode, &search_config);
            }

            None
        }
    }

    assert!(operator_oid == anyelement_query_input_opoid());

    let mut selectivity = inner_query_input(planner_info, args).unwrap_or(UNKNOWN_SELECTIVITY);
    if selectivity > 1.0 {
        selectivity = UNKNOWN_SELECTIVITY;
    }

    selectivity
}

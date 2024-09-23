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
    anyelement_text_opoid, estimate_selectivity, find_var_relation, make_search_config_opexpr_node,
    ReturnedNodePointer,
};
use crate::postgres::utils::{locate_bm25_index, relfilenode_from_search_config};
use crate::query::SearchQueryInput;
use crate::schema::SearchConfig;
use crate::UNKNOWN_SELECTIVITY;
use pgrx::pg_sys::planner_rt_fetch;
use pgrx::{is_a, pg_extern, pg_sys, AnyElement, FromDatum, Internal, PgList, PgRelation};
use std::ffi::CStr;

/// This is the function behind the `@@@(anyelement, text)` operator. Since we transform those to
/// use `@@@(anyelement, jsonb`), this function won't be called in normal circumstances, but it
/// could be called if the rhs of the @@@ is some kind of volatile value.
///
/// And in that case we just have to give up.
#[pg_extern(immutable, parallel_safe)]
pub fn search_with_text(
    _element: AnyElement,
    query: &str,
    _fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    panic!("query is incompatible with pg_search's `@@@(key_field, TEXT)` operator: `{query}`")
}

#[pg_extern(immutable, parallel_safe)]
pub fn text_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        let node = arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>();

        match (*node).type_ {
            // rewrite this node, which is using the @@@(key_field, text) operator
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

                        // the field name comes from the lhs of the @@@ operator, which is a `pg_sys::Var` node
                        let rte = planner_rt_fetch((*var).varno as pg_sys::Index, (*srs).root);
                        let field = if (*rte).rtekind == pg_sys::RTEKind::RTE_SUBQUERY {
                            let subquery = (*rte).subquery;
                            let targetlist =
                                PgList::<pg_sys::TargetEntry>::from_pg((*subquery).targetList);

                            let te = targetlist
                                .get_ptr((*var).varattno as usize - 1)
                                .expect("var.varattno must exist in the subquery");
                            let resname = CStr::from_ptr((*te).resname);
                            resname
                                .to_str()
                                .expect("resname must be valid UTF8")
                                .to_string()
                        } else {
                            let relid = (*rte).relid;
                            let heaprel = PgRelation::open(relid);
                            let tupdesc = heaprel.tuple_desc();
                            let attribute = tupdesc
                                .get((*var).varattno as usize - 1)
                                .expect("Var.varattno must exist in the relation");
                            attribute.name().to_string()
                        };

                        // the query comes from the rhs of the @@@ operator.  we've already proved it's a `pg_sys::Const` node
                        let query_string =
                            String::from_datum((*const_).constvalue, (*const_).constisnull)
                                .expect("query must not be NULL");

                        let query = SearchQueryInput::ParseWithField {
                            field: field.to_string(),
                            query_string,
                        };
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
pub fn text_restrict(
    planner_info: Internal, // <pg_sys::PlannerInfo>,
    operator_oid: pg_sys::Oid,
    args: Internal, // <pg_sys::List>,
    _var_relid: i32,
) -> f64 {
    fn inner_text(
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

                let query = String::from_datum((*const_).constvalue, (*const_).constisnull)?;
                let search_config = SearchConfig::from((query, indexrel));
                let relfilenode = relfilenode_from_search_config(&search_config);

                return estimate_selectivity(heaprelid, relfilenode, &search_config);
            }

            None
        }
    }

    assert!(operator_oid == anyelement_text_opoid());

    let mut selectivity = inner_text(planner_info, args).unwrap_or(UNKNOWN_SELECTIVITY);
    if selectivity > 1.0 {
        selectivity = UNKNOWN_SELECTIVITY;
    }

    selectivity
}

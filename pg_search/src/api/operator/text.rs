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
use crate::{nodecast, UNKNOWN_SELECTIVITY};
use pgrx::{pg_extern, pg_sys, AnyElement, FromDatum, Internal, PgList, PgRelation};

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
pub unsafe fn text_support(arg: Internal) -> ReturnedNodePointer {
    text_support_request_simplify(arg).unwrap_or_else(|| ReturnedNodePointer(None))
}

fn text_support_request_simplify(arg: Internal) -> Option<ReturnedNodePointer> {
    unsafe {
        let srs = nodecast!(
            SupportRequestSimplify,
            T_SupportRequestSimplify,
            arg.unwrap()?.cast_mut_ptr::<pg_sys::Node>()
        )?;
        let mut input_args = PgList::<pg_sys::Node>::from_pg((*(*srs).fcall).args);

        let var = nodecast!(Var, T_Var, input_args.get_ptr(0)?)?;
        let const_ = nodecast!(Const, T_Const, input_args.get_ptr(1)?)?;

        // the field name comes from the lhs of the @@@ operator
        let (_, query) = make_query_from_var_and_const((*srs).root, var, const_);
        Some(make_search_config_opexpr_node(
            srs,
            &mut input_args,
            var,
            query,
        ))
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
            let var = nodecast!(Var, T_Var, args.get_ptr(0)?)?;
            let const_ = nodecast!(Const, T_Const, args.get_ptr(1)?)?;

            let (heaprelid, query) = make_query_from_var_and_const(info, var, const_);
            let indexrel = locate_bm25_index(heaprelid)?;
            let search_config = SearchConfig::from((query, indexrel));
            let relfilenode = relfilenode_from_search_config(&search_config);

            estimate_selectivity(heaprelid, relfilenode, &search_config)
        }
    }

    assert!(operator_oid == anyelement_text_opoid());

    let mut selectivity = inner_text(planner_info, args).unwrap_or(UNKNOWN_SELECTIVITY);
    if selectivity > 1.0 {
        selectivity = UNKNOWN_SELECTIVITY;
    }

    selectivity
}

unsafe fn make_query_from_var_and_const(
    root: *mut pg_sys::PlannerInfo,
    var: *mut pg_sys::Var,
    const_: *mut pg_sys::Const,
) -> (pg_sys::Oid, SearchQueryInput) {
    let (heaprelid, varattno, _) = find_var_relation(var, root);
    let heaprel = PgRelation::open(heaprelid);
    let tupdesc = heaprel.tuple_desc();
    let attribute = tupdesc
        .get(varattno as usize - 1)
        .expect("varattno must exist in the relation");
    let field = attribute.name().to_string();

    // the query comes from the rhs of the @@@ operator.  we've already proved it's a `pg_sys::Const` node
    let query_string = String::from_datum((*const_).constvalue, (*const_).constisnull)
        .expect("query must not be NULL");

    let query = SearchQueryInput::ParseWithField {
        field: field.to_string(),
        query_string,
    };
    (heaprelid, query)
}

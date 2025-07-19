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
use crate::api::operator::{
    anyelement_text_opoid, anyelement_text_procoid, attname_from_var, find_node_relation,
    make_search_query_input_opexpr_node, searchqueryinput_typoid, ReturnedNodePointer,
};
use crate::api::FieldName;
use crate::nodecast;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::locate_bm25_index_from_heaprel;
use crate::query::SearchQueryInput;
use crate::PG_SEARCH_PREFIX;
use pgrx::{pg_sys, FromDatum, Internal, PgList};

mod andandand;
mod atatat;
mod eqeqeq;
mod hashhashhash;
mod ororor;

enum RHSValue {
    Text(String),
    TextArray(Vec<String>),
    SearchQueryInput(SearchQueryInput),
}

unsafe fn request_simplify<F: FnOnce(Option<FieldName>, RHSValue) -> SearchQueryInput>(
    arg: Internal,
    ctor: F,
) -> Option<ReturnedNodePointer> {
    let srs = nodecast!(
        SupportRequestSimplify,
        T_SupportRequestSimplify,
        arg.unwrap()?.cast_mut_ptr::<pg_sys::Node>()
    )?;
    if (*srs).root.is_null() {
        return None;
    }
    let mut input_args = PgList::<pg_sys::Node>::from_pg((*(*srs).fcall).args);

    let lhs = input_args.get_ptr(0)?;
    let rhs = input_args.get_ptr(1)?;

    let (_heaprelid, field) = tantivy_field_name_from_node((*srs).root, lhs)?;
    let (query, param) = if let Some(const_) = nodecast!(Const, T_Const, rhs) {
        // the field name comes from the lhs of the @@@ operator

        let rhs_value = match (*const_).consttype {
            pg_sys::TEXTOID | pg_sys::VARCHAROID => RHSValue::Text(
                String::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs text value must not be NULL"),
            ),

            pg_sys::TEXTARRAYOID | pg_sys::VARCHARARRAYOID => RHSValue::TextArray(
                Vec::<String>::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs text array value must not be NULL"),
            ),

            pg_sys::UUIDARRAYOID => RHSValue::TextArray(
                Vec::<pgrx::datum::Uuid>::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs uuid array value must not be NULL")
                    .into_iter()
                    .map(|uuid| uuid.to_string())
                    .collect(),
            ),

            other if other == searchqueryinput_typoid() => RHSValue::SearchQueryInput(
                SearchQueryInput::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs query value must not be NULL"),
            ),

            other => panic!("unsupported type for @@@ operator oid: {other}"),
        };

        let query = ctor(field, rhs_value);
        (Some(query), None)
    } else {
        (None, Some((rhs, field)))
    };

    Some(make_search_query_input_opexpr_node(
        srs,
        &mut input_args,
        lhs,
        query,
        param,
        anyelement_text_opoid(),
        anyelement_text_procoid(),
    ))
}

/// Given a [`pg_sys::PlannerInfo`] and a [`pg_sys::Node`] from it, figure out the name of the `Node`.
/// It supports `FuncExpr` and `Var` nodes. Note that for the heap relation, the `Var` must be
/// the first argument of the `FuncExpr`.
/// This function requires the node to be related to a `bm25` index, otherwise it will panic.
///
/// Returns the heap relation [`pg_sys::Oid`] that contains the `Node` along with its name.
unsafe fn tantivy_field_name_from_node(
    root: *mut pg_sys::PlannerInfo,
    node: *mut pg_sys::Node,
) -> Option<(pg_sys::Oid, Option<FieldName>)> {
    match (*node).type_ {
        pg_sys::NodeTag::T_FuncExpr | pg_sys::NodeTag::T_OpExpr => {
            // We expect the funcexpr/opexpr to contain the var of the field name we're looking for
            let (heaprelid, _, _) = find_node_relation(node, root);
            if heaprelid == pg_sys::Oid::INVALID {
                panic!("could not find heap relation for node");
            }
            let heaprel = PgSearchRelation::open(heaprelid);
            let indexrel = locate_bm25_index_from_heaprel(&heaprel)
                .expect("could not find bm25 index for heaprelid");

            let attnum = find_expr_attnum(&indexrel, node)?;
            let expression_str = format!("{PG_SEARCH_PREFIX}{attnum}").into();
            Some((heaprelid, Some(expression_str)))
        }
        pg_sys::NodeTag::T_Var => {
            let var = nodecast!(Var, T_Var, node).expect("node is not a Var");
            let (oid, attname) = attname_from_var(root, var);
            Some((oid, attname))
        }
        _ => None,
    }
}

fn find_expr_attnum(indexrel: &PgSearchRelation, node: *mut pg_sys::Node) -> Option<i32> {
    let index_info = unsafe { *pg_sys::BuildIndexInfo(indexrel.as_ptr()) };

    let expressions = unsafe { PgList::<pg_sys::Expr>::from_pg(index_info.ii_Expressions) };
    let mut expressions_iter = expressions.iter_ptr();

    for i in 0..index_info.ii_NumIndexAttrs {
        let heap_attno = index_info.ii_IndexAttrNumbers[i as usize];
        if heap_attno == 0 {
            let Some(expression) = expressions_iter.next() else {
                panic!("Expected expression for index attribute {i}.");
            };

            if unsafe { pg_sys::equal(node.cast(), expression.cast()) } {
                return Some(i);
            }
        }
    }
    None
}

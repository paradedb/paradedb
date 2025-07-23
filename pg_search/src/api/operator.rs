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

mod andandand;
mod atatat;
mod eqeqeq;
mod hashhashhash;
mod ororor;
mod searchqueryinput;

use crate::api::FieldName;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::nodecast;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::{locate_bm25_index_from_heaprel, ToPalloc};
use crate::postgres::var::find_var_relation;
use crate::query::{PdbQuery, SearchQueryInput};
use pgrx::callconv::{BoxRet, FcInfo};
use pgrx::datum::Datum;
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
};
use pgrx::*;
use std::ptr::NonNull;

enum RHSValue {
    Text(String),
    TextArray(Vec<String>),
    FieldedQueryInput(PdbQuery),
}

#[derive(Debug)]
#[repr(transparent)]
pub struct ReturnedNodePointer(pub Option<NonNull<pg_sys::Node>>);

unsafe impl BoxRet for ReturnedNodePointer {
    unsafe fn box_into<'fcx>(self, fcinfo: &mut FcInfo<'fcx>) -> Datum<'fcx> {
        self.0
            .map(|nonnull| {
                let datum = pg_sys::Datum::from(nonnull.as_ptr());
                fcinfo.return_raw_datum(datum)
            })
            .unwrap_or_else(Datum::null)
    }
}

unsafe impl SqlTranslatable for ReturnedNodePointer {
    fn argument_sql() -> Result<SqlMapping, ArgumentError> {
        Ok(SqlMapping::As("internal".into()))
    }

    fn return_sql() -> Result<Returns, ReturnsError> {
        Ok(Returns::One(SqlMapping::As("internal".into())))
    }
}

pub fn with_index_procoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            // NB:  the SQL signature here needs to match our Rust implementation
            &[c"paradedb.with_index(regclass, paradedb.searchqueryinput)".into_datum()],
        )
        .expect(
            "the `paradedb.with_index(regclass, paradedb.searchqueryinput)` function should exist",
        )
    }
}

pub fn anyelement_query_input_procoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"paradedb.search_with_query_input(anyelement, paradedb.searchqueryinput)".into_datum()],
        )
            .expect("the `paradedb.search_with_query_input(anyelement, paradedb.searchqueryinput) function should exist")
    }
}

pub fn anyelement_query_input_opoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regoperatorin,
            &[c"@@@(anyelement, paradedb.searchqueryinput)".into_datum()],
        )
        .expect("the `@@@(anyelement, paradedb.searchqueryinput)` operator should exist")
    }
}

pub fn searchqueryinput_typoid() -> pg_sys::Oid {
    unsafe {
        let oid = direct_function_call::<pg_sys::Oid>(
            pg_sys::regtypein,
            &[c"paradedb.SearchQueryInput".into_datum()],
        )
        .expect("type `paradedb.SearchQueryInput` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `paradedb.SearchQueryInput` should exist");
        }
        oid
    }
}

pub fn fieldedqueryinput_typoid() -> pg_sys::Oid {
    unsafe {
        let oid = direct_function_call::<pg_sys::Oid>(
            pg_sys::regtypein,
            &[c"paradedb.FieldedQueryInput".into_datum()],
        )
        .expect("type `paradedb.FieldedQueryInput` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `paradedb.FieldedQueryInput` should exist");
        }
        oid
    }
}

pub(crate) fn estimate_selectivity(
    indexrel: &PgSearchRelation,
    search_query_input: SearchQueryInput,
) -> Option<f64> {
    let reltuples = indexrel
        .heap_relation()
        .expect("indexrel should be an index")
        .reltuples()
        .unwrap_or(1.0) as f64;
    if !reltuples.is_normal() || reltuples.is_sign_negative() {
        // we can't estimate against a non-normal or negative estimate of heap tuples
        return None;
    }

    let search_reader = SearchIndexReader::open(
        indexrel,
        search_query_input,
        false,
        MvccSatisfies::LargestSegment,
    )
    .expect("estimate_selectivity: should be able to open a SearchIndexReader");
    let estimate = search_reader.estimate_docs(reltuples).unwrap_or(1) as f64;
    let mut selectivity = estimate / reltuples;
    if selectivity > 1.0 {
        selectivity = 1.0;
    }

    Some(selectivity)
}

unsafe fn get_expr_result_type(expr: *mut pg_sys::Node) -> pg_sys::Oid {
    let mut typoid = pg_sys::Oid::INVALID;
    let mut tupdesc = pg_sys::TupleDesc::default();
    pg_sys::get_expr_result_type(expr, &mut typoid, &mut tupdesc);
    if !tupdesc.is_null() {
        pg_sys::FreeTupleDesc(tupdesc);
    }
    typoid
}

/// Given a [`pg_sys::PlannerInfo`] and a [`pg_sys::Node`] from it, figure out the name of the `Node`.
/// It supports `FuncExpr` and `Var` nodes. Note that for the heap relation, the `Var` must be
/// the first argument of the `FuncExpr`.
/// This function requires the node to be related to a `bm25` index, otherwise it will panic.
///
/// Returns the heap relation [`pg_sys::Oid`] that contains the `Node` along with its name.
pub unsafe fn tantivy_field_name_from_node(
    root: *mut pg_sys::PlannerInfo,
    node: *mut pg_sys::Node,
) -> Option<(pg_sys::Oid, Option<FieldName>)> {
    match (*node).type_ {
        pg_sys::NodeTag::T_FuncExpr | pg_sys::NodeTag::T_OpExpr => {
            use crate::PG_SEARCH_PREFIX;

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

unsafe fn request_simplify<ConstRewrite, ExecRewrite>(
    arg: *mut pg_sys::Node,
    const_rewrite: ConstRewrite,
    exec_rewrite: ExecRewrite,
) -> Option<ReturnedNodePointer>
where
    ConstRewrite: FnOnce(Option<FieldName>, RHSValue) -> SearchQueryInput,
    ExecRewrite: FnOnce(Option<FieldName>, *mut pg_sys::Node) -> pg_sys::FuncExpr,
{
    let srs = nodecast!(SupportRequestSimplify, T_SupportRequestSimplify, arg)?;
    if (*srs).root.is_null() {
        return None;
    }
    let search_query_input_typoid = searchqueryinput_typoid();

    let input_args = PgList::<pg_sys::Node>::from_pg((*(*srs).fcall).args);
    let lhs = input_args.get_ptr(0)?;
    let rhs = input_args.get_ptr(1)?;

    let (_heaprelid, field) = tantivy_field_name_from_node((*srs).root, lhs)?;
    let rhs = rewrite_rhs_to_search_query_input(
        const_rewrite,
        exec_rewrite,
        search_query_input_typoid,
        rhs,
        field,
    );

    Some(rewrite_to_search_query_input_opexpr(srs, lhs, rhs))
}

unsafe fn rewrite_to_search_query_input_opexpr(
    srs: *mut pg_sys::SupportRequestSimplify,
    lhs: *mut pg_sys::Node,
    rhs: *mut pg_sys::Node,
) -> ReturnedNodePointer {
    let rhs_type = get_expr_result_type(rhs);
    assert_eq!(
        rhs_type,
        searchqueryinput_typoid(),
        "rhs must represent a SearchQueryInput"
    );

    let indexrel = rewrite_lhs_to_key_field(srs, lhs);

    let rhs = wrap_with_index(indexrel, rhs);

    let mut args = PgList::<pg_sys::Node>::new();
    args.push(lhs);
    args.push(rhs);

    let mut opexpr = PgBox::<pg_sys::OpExpr>::alloc_node(pg_sys::NodeTag::T_OpExpr);
    opexpr.args = args.into_pg();
    opexpr.opno = anyelement_query_input_opoid();
    opexpr.opfuncid = anyelement_query_input_procoid();
    opexpr.opresulttype = searchqueryinput_typoid();
    opexpr.opretset = false;
    opexpr.opcollid = pg_sys::Oid::INVALID;
    opexpr.inputcollid = pg_sys::DEFAULT_COLLATION_OID;
    opexpr.location = (*(*srs).fcall).location;

    ReturnedNodePointer(NonNull::new(opexpr.into_pg().cast()))
}

unsafe fn rewrite_lhs_to_key_field(
    srs: *mut pg_sys::SupportRequestSimplify,
    lhs: *mut pg_sys::Node,
) -> PgSearchRelation {
    let (relid, _nodeattno, targetlist) = find_node_relation(lhs, (*srs).root);
    if relid == pg_sys::Oid::INVALID {
        panic!("could not determine relation for node");
    }

    // we need to use what should be the only `USING bm25` index on the table
    let heaprel = PgSearchRelation::open(relid);
    let indexrel = locate_bm25_index_from_heaprel(&heaprel).unwrap_or_else(|| {
        panic!(
            "relation `{}.{}` must have a `USING bm25` index",
            heaprel.namespace(),
            heaprel.name()
        )
    });

    let keys = &(*indexrel.rd_index).indkey;
    let keys = keys.values.as_slice(keys.dim1 as usize);
    let tupdesc = PgTupleDesc::from_pg_unchecked(indexrel.rd_att);
    let att = tupdesc
        .get(0)
        .unwrap_or_else(|| panic!("attribute `{}` not found", keys[0]));

    if (*lhs).type_ == pg_sys::NodeTag::T_Var {
        let var = nodecast!(Var, T_Var, lhs).expect("lhs is not a Var");
        if let Some(targetlist) = &targetlist {
            // if we have a targetlist, find the first field of the index definition in it -- its location
            // in the target list becomes the var's attno
            let mut found = false;
            for (i, te) in targetlist.iter_ptr().enumerate() {
                if te.is_null() {
                    continue;
                }
                if (*te).resorigcol == keys[0] {
                    (*var).varattno = (i + 1) as _;
                    (*var).varattnosyn = (*var).varattno;
                    found = true;
                    break;
                }
            }

            if !found {
                panic!("index's first column is not in the var's targetlist");
            }
        } else {
            // the Var must look like the first attribute from the index definition
            (*var).varattno = keys[0];
            (*var).varattnosyn = (*var).varattno;
        }

        // the Var must also assume the type of the first attribute from the index definition,
        // regardless of where we found the Var
        (*var).vartype = att.atttypid;
        (*var).vartypmod = att.atttypmod;
        (*var).varcollid = att.attcollation;
    }

    indexrel
}

unsafe fn wrap_with_index(indexrel: PgSearchRelation, rhs: *mut pg_sys::Node) -> *mut pg_sys::Node {
    if let Some(rhs_const) = nodecast!(Const, T_Const, rhs) {
        // Const nodes are always of type SearchQueryInput, so we can instantiate a new Const version
        let query = SearchQueryInput::from_datum((*rhs_const).constvalue, (*rhs_const).constisnull)
            .unwrap();
        let query = SearchQueryInput::WithIndex {
            oid: indexrel.oid(),
            query: Box::new(query),
        };
        let as_const: *mut pg_sys::Const = query.into();
        as_const.cast()
    } else {
        // otherwise we need to wrap the rhs in a `FuncExpr` that calls `paradedb.with_index()`
        let mut args = PgList::<pg_sys::Node>::new();
        args.push(
            pg_sys::makeConst(
                pg_sys::REGCLASSOID,
                -1,
                pg_sys::Oid::INVALID,
                size_of::<pg_sys::Oid>() as _,
                indexrel.oid().into_datum().unwrap(),
                false,
                true,
            )
            .cast(),
        );
        args.push(rhs);

        pg_sys::FuncExpr {
            xpr: pg_sys::Expr {
                type_: pg_sys::NodeTag::T_FuncExpr,
            },
            funcid: with_index_procoid(),
            funcresulttype: searchqueryinput_typoid(),
            funcretset: false,
            funcvariadic: false,
            funcformat: pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
            funccollid: pg_sys::Oid::INVALID,
            inputcollid: pg_sys::Oid::INVALID,
            args: args.into_pg(),
            location: -1,
        }
        .palloc()
        .cast()
    }
}

unsafe fn rewrite_rhs_to_search_query_input<ConstRewrite, ExecRewrite>(
    const_rewrite: ConstRewrite,
    exec_rewrite: ExecRewrite,
    search_query_input_typoid: pg_sys::Oid,
    rhs: *mut pg_sys::Node,
    field: Option<FieldName>,
) -> *mut pg_sys::Node
where
    ConstRewrite: FnOnce(Option<FieldName>, RHSValue) -> SearchQueryInput,
    ExecRewrite: FnOnce(Option<FieldName>, *mut pg_sys::Node) -> pg_sys::FuncExpr,
{
    let rhs: *mut pg_sys::Node = if get_expr_result_type(rhs) == search_query_input_typoid {
        // the rhs is already of type SearchQueryInput, so we can use it directly
        rhs
    } else if let Some(const_) = nodecast!(Const, T_Const, rhs) {
        // the rhs is a Const of some other type.  The caller gets the opportunity to rewrite the
        // user-provided Const to a SearchQueryInput.
        //
        // we currently only support rewriting Consts of type TEXT or TEXT[]
        let rhs_value = match (*const_).consttype {
            // these are used for the @@@, &&&, |||, ###, and === operators
            pg_sys::TEXTOID | pg_sys::VARCHAROID => RHSValue::Text(
                String::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs text value must not be NULL"),
            ),

            // these arrays are only supported by the === operator
            pg_sys::TEXTARRAYOID | pg_sys::VARCHARARRAYOID => RHSValue::TextArray(
                Vec::<String>::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs text array value must not be NULL"),
            ),

            // this is specifically used for the `@@@(anyelement, paradedb.fieldedqueryinput)` operator
            other if other == fieldedqueryinput_typoid() => RHSValue::FieldedQueryInput(
                PdbQuery::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs fielded query input value must not be NULL"),
            ),

            other => panic!("operator does not support rhs type {other}"),
        };

        let query: *mut pg_sys::Const = const_rewrite(field, rhs_value).into();
        query.cast()
    } else {
        // the rhs is a complex expression that needs to be evaluated at runtime
        // but its return type is not SearchQueryInput, so we need to rewrite it
        exec_rewrite(field, rhs).palloc().cast()
    };
    rhs
}

/// Given a [`pg_sys::Node`] and a [`pg_sys::PlannerInfo`], attempt to find the relation Oid that
/// is referenced by the node.
///
/// It's possible the returned Oid will be [`pg_sys::Oid::INVALID`] if the Node doesn't eventually
/// come from a relation. In case of a FuncExpr we stop on the first relation found.
///
/// The returned [`pg_sys::AttrNumber`] is the physical attribute number in the relation the Node
/// is from.
unsafe fn find_node_relation(
    node: *mut pg_sys::Node,
    root: *mut pg_sys::PlannerInfo,
) -> (
    pg_sys::Oid,
    pg_sys::AttrNumber,
    Option<PgList<pg_sys::TargetEntry>>,
) {
    // If the Node is var, immediately return it: otherwise examine the arguments of whatever type
    // it is.
    let args = match (*node).type_ {
        pg_sys::NodeTag::T_Var => return find_var_relation(node.cast(), root),
        pg_sys::NodeTag::T_FuncExpr => {
            let funcexpr: *mut pg_sys::FuncExpr = node.cast();
            PgList::<pg_sys::Node>::from_pg((*funcexpr).args)
        }
        pg_sys::NodeTag::T_OpExpr => {
            let opexpr: *mut pg_sys::OpExpr = node.cast();
            PgList::<pg_sys::Node>::from_pg((*opexpr).args)
        }
        _ => return (pg_sys::Oid::INVALID, 0, None),
    };

    args.iter_ptr()
        .filter_map(|arg| match (*arg).type_ {
            pg_sys::NodeTag::T_FuncExpr | pg_sys::NodeTag::T_OpExpr | pg_sys::NodeTag::T_Var => {
                Some(find_node_relation(arg, root))
            }
            _ => None,
        })
        .reduce(|(acc_oid, acc_attno, acc_tl), (oid, _attno, _tl)| {
            if acc_oid != oid {
                panic!("expressions cannot contain multiple relations");
            }
            (acc_oid, acc_attno, acc_tl)
        })
        .unwrap_or_else(|| (pg_sys::Oid::INVALID, 0, None))
}

/// Given a [`pg_sys::PlannerInfo`] and a [`pg_sys::Var`] from it, figure out the name of the `Var`
///
/// Returns the heap relation [`pg_sys::Oid`] that contains the `Var` along with its name.
unsafe fn attname_from_var(
    root: *mut pg_sys::PlannerInfo,
    var: *mut pg_sys::Var,
) -> (pg_sys::Oid, Option<FieldName>) {
    let (heaprelid, varattno, _) = find_var_relation(var, root);
    if (*var).varattno == 0 {
        return (heaprelid, None);
    }
    let heaprel = PgRelation::open(heaprelid);
    let tupdesc = heaprel.tuple_desc();
    let attname = if varattno == pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber {
        Some("ctid".into())
    } else {
        tupdesc
            .get(varattno as usize - 1)
            .map(|attribute| attribute.name().into())
    };
    (heaprelid, attname)
}

extension_sql!(
    r#"
ALTER FUNCTION paradedb.search_with_query_input SUPPORT paradedb.query_input_support;

CREATE OPERATOR pg_catalog.@@@ (
    PROCEDURE = search_with_query_input,
    LEFTARG = anyelement,
    RIGHTARG = paradedb.searchqueryinput,
    RESTRICT = query_input_restrict
);

CREATE OPERATOR CLASS anyelement_bm25_ops DEFAULT FOR TYPE anyelement USING bm25 AS
    OPERATOR 1 pg_catalog.@@@(anyelement, text),                         /* for querying with a tantivy-compatible text query */
    OPERATOR 2 pg_catalog.@@@(anyelement, paradedb.searchqueryinput),    /* for querying with a paradedb.searchqueryinput structure */
    STORAGE anyelement;
"#,
    name = "bm25_ops_anyelement_operator",
    requires = [
        // for using plain text on the rhs
        atatat::search_with_parse,
        // for using SearchQueryInput on the rhs
        searchqueryinput::search_with_query_input,
        searchqueryinput::query_input_restrict,
        searchqueryinput::query_input_support,
    ]
);

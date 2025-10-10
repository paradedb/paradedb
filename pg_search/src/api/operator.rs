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
mod boost;
mod eqeqeq;
mod fuzzy;
mod hashhashhash;
mod ororor;
mod proximity;
mod searchqueryinput;
mod slop;

use crate::api::operator::boost::{boost_to_boost, BoostType};
use crate::api::operator::fuzzy::{fuzzy_to_fuzzy, FuzzyType};
use crate::api::operator::slop::{slop_to_slop, SlopType};
use crate::api::tokenizers::{
    lookup_alias_typmod, lookup_generic_typmod, type_is_alias, type_is_tokenizer,
};
use crate::api::FieldName;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::nodecast;
use crate::postgres::catalog::lookup_type_name;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::{deparse_expr, locate_bm25_index_from_heaprel, ToPalloc};
use crate::postgres::var::{
    find_json_path, find_one_var, find_var_relation, find_vars, VarContext,
};
use crate::query::pdb_query::pdb;
use crate::query::proximity::ProximityClause;
use crate::query::SearchQueryInput;
use pgrx::callconv::{BoxRet, FcInfo};
use pgrx::datum::Datum;
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
};
use pgrx::*;
use std::ptr::NonNull;

enum RHSValue {
    Text(String),
    TextArray(Vec<String>),
    PdbQuery(pdb::Query),
    ProximityClause(ProximityClause),
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

pub fn pdb_query_typoid() -> pg_sys::Oid {
    unsafe {
        let oid =
            direct_function_call::<pg_sys::Oid>(pg_sys::regtypein, &[c"pdb.Query".into_datum()])
                .expect("type `pdb.Query` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `pdb.Query` should exist");
        }
        oid
    }
}

pub fn boost_typoid() -> pg_sys::Oid {
    unsafe {
        let oid =
            direct_function_call::<pg_sys::Oid>(pg_sys::regtypein, &[c"pdb.boost".into_datum()])
                .expect("type `pdb.boost` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `pdb.boost` should exist");
        }
        oid
    }
}

pub fn fuzzy_typoid() -> pg_sys::Oid {
    unsafe {
        let oid =
            direct_function_call::<pg_sys::Oid>(pg_sys::regtypein, &[c"pdb.fuzzy".into_datum()])
                .expect("type `pdb.fuzzy` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `pdb.fuzzy` should exist");
        }
        oid
    }
}

pub fn slop_typoid() -> pg_sys::Oid {
    unsafe {
        let oid =
            direct_function_call::<pg_sys::Oid>(pg_sys::regtypein, &[c"pdb.slop".into_datum()])
                .expect("type `pdb.slop` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `pdb.slop` should exist");
        }
        oid
    }
}

pub fn pdb_proximityclause_typoid() -> pg_sys::Oid {
    unsafe {
        let oid = direct_function_call::<pg_sys::Oid>(
            pg_sys::regtypein,
            &[c"pdb.ProximityClause".into_datum()],
        )
        .expect("type `pdb.ProximityClause` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `pdb.ProximityClause` should exist");
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
///
/// Returns the heap relation [`pg_sys::Oid`] that contains the `Node` along with its name.
pub unsafe fn tantivy_field_name_from_node(
    root: *mut pg_sys::PlannerInfo,
    node: *mut pg_sys::Node,
) -> Option<(PgSearchRelation, Option<FieldName>)> {
    let (heaprelid, _, _) = find_node_relation(node, root);
    if heaprelid == pg_sys::Oid::INVALID {
        return None;
    }
    let heaprel = PgSearchRelation::open(heaprelid);
    let indexrel = locate_bm25_index_from_heaprel(&heaprel)
        .unwrap_or_else(|| panic!("`{}` does not contain a `USING bm25` index", heaprel.name()));

    let field_name =
        field_name_from_node(VarContext::from_planner(root), &heaprel, &indexrel, node)?;
    Some((indexrel, Some(field_name)))
}

unsafe fn field_name_from_node(
    context: VarContext,
    heaprel: &PgSearchRelation,
    indexrel: &PgSearchRelation,
    node: *mut pg_sys::Node,
) -> Option<FieldName> {
    // just directly reach in and pluck out the alias if the type is cast to it
    if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, node) {
        if type_is_alias((*relabel).resulttype) {
            if let Ok(alias) = lookup_alias_typmod((*relabel).resulttypmod) {
                return Some(FieldName::from(alias.alias));
            }
        }
    }

    let index_info = unsafe { *pg_sys::BuildIndexInfo(indexrel.as_ptr()) };
    if let Some(var) = nodecast!(Var, T_Var, node) {
        // the expression we're looking for is just a simple Var.

        if (*var).varattno == 0 {
            // the var references the whole row -- this means the fieldname is the name of the "key_field"
            return Some(
                indexrel
                    .schema()
                    .expect("index should have a valid schema")
                    .key_field_name(),
            );
        }

        // otherwise the var might be a specific index attribute or meaning to reference an indexed expression

        let expressions = unsafe { PgList::<pg_sys::Expr>::from_pg(index_info.ii_Expressions) };
        let mut expr_no = 0;
        for i in 0..index_info.ii_NumIndexAttrs {
            let heap_attno = index_info.ii_IndexAttrNumbers[i as usize];

            if heap_attno == (*var).varattno {
                // this is a Var that directly matches an indexed attribute
                return attname_from_var(heaprel, var);
            } else if heap_attno == 0 {
                // see if the Var we're looking for matches a custom tokenizer definition
                let Some(expression) = expressions.get_ptr(expr_no) else {
                    panic!("expected expression for index attribute {expr_no}");
                };

                if type_is_tokenizer(pg_sys::exprType(expression.cast())) {
                    let vars = find_vars(expression.cast());
                    if vars.len() == 1 && pg_sys::equal(node.cast(), vars[0].cast()) {
                        // the Var is the expression that matches the Var we're looking for
                        // but lets make sure the whole expression is one without an alias
                        // we pick the first un-aliased custom tokenizer expression that uses the
                        // Var as the matching indexed expression
                        let typmod = pg_sys::exprTypmod(expression.cast());
                        let parsed = lookup_generic_typmod(typmod)
                            .unwrap_or_else(|e| panic!("failed to lookup typmod {typmod}: {e}"));
                        if parsed.alias().is_none() {
                            return attname_from_var(heaprel, var);
                        }
                    }
                    expr_no += 1;
                }
            }
        }

        return None;
    }

    //
    // we're looking for a more complex expression
    //

    let expressions = unsafe { PgList::<pg_sys::Expr>::from_pg(index_info.ii_Expressions) };
    let mut expressions_iter = expressions.iter_ptr();

    for i in 0..index_info.ii_NumIndexAttrs {
        let heap_attno = index_info.ii_IndexAttrNumbers[i as usize];
        if heap_attno == 0 {
            let Some(indexed_expression) = expressions_iter.next() else {
                panic!("Expected expression for index attribute {i}.");
            };

            let mut reduced_expression = indexed_expression;
            loop {
                let inner_expression = if let Some(coerce) =
                    nodecast!(CoerceViaIO, T_CoerceViaIO, reduced_expression)
                {
                    (*coerce).arg
                } else {
                    reduced_expression
                };

                if unsafe { pg_sys::equal(node.cast(), inner_expression.cast()) } {
                    let field_name = if type_is_tokenizer(pg_sys::exprType(
                        indexed_expression.cast(),
                    )) {
                        let typmod = pg_sys::exprTypmod(indexed_expression.cast());
                        let parsed = lookup_generic_typmod(typmod)
                            .unwrap_or_else(|e| panic!("failed to lookup typmod {typmod}: {e}"));

                        parsed.alias().map(FieldName::from).or_else(|| {
                            find_one_var(indexed_expression.cast())
                                .and_then(|var| attname_from_var(heaprel, var.cast()))
                        })
                    } else {
                        let expr_str = deparse_expr(heaprel, Some(indexed_expression));
                        panic!("indexed expression requires a tokenizer cast with an alias: {expr_str}");
                    };

                    return field_name;
                }

                if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, reduced_expression) {
                    reduced_expression = (*relabel).arg.cast();
                    continue;
                }

                break;
            }
        }
    }

    //
    // the node we're evaluating doesn't match either an index expression or a direct Var
    //

    // could it be a json(b) path reference like:  json_field->'foo'->>'bar'?
    let json_path = find_json_path(&context, node);
    if !json_path.is_empty() {
        return Some(FieldName::from(json_path.join(".")));
    }

    // whatever they're searching for, it's not something we know how to identify
    None
}

unsafe fn request_simplify<ConstRewrite, ExecRewrite>(
    arg: *mut pg_sys::Node,
    const_rewrite: ConstRewrite,
    exec_rewrite: ExecRewrite,
) -> Option<ReturnedNodePointer>
where
    ConstRewrite: FnOnce(*mut pg_sys::Node, Option<FieldName>, RHSValue) -> SearchQueryInput,
    ExecRewrite:
        FnOnce(Option<FieldName>, *mut pg_sys::Node, *mut pg_sys::Node) -> pg_sys::FuncExpr,
{
    let srs = nodecast!(SupportRequestSimplify, T_SupportRequestSimplify, arg)?;
    if (*srs).root.is_null() {
        return None;
    }
    let search_query_input_typoid = searchqueryinput_typoid();

    let input_args = PgList::<pg_sys::Node>::from_pg((*(*srs).fcall).args);
    let lhs = input_args.get_ptr(0)?;
    let rhs = input_args.get_ptr(1)?;

    let (indexrel, field) = tantivy_field_name_from_node((*srs).root, lhs)?;
    let rhs = rewrite_rhs_to_search_query_input(
        const_rewrite,
        exec_rewrite,
        search_query_input_typoid,
        lhs,
        rhs,
        field,
    );

    Some(rewrite_to_search_query_input_opexpr(
        srs, &indexrel, lhs, rhs,
    ))
}

unsafe fn rewrite_to_search_query_input_opexpr(
    srs: *mut pg_sys::SupportRequestSimplify,
    indexrel: &PgSearchRelation,
    lhs: *mut pg_sys::Node,
    rhs: *mut pg_sys::Node,
) -> ReturnedNodePointer {
    let rhs_type = get_expr_result_type(rhs);
    assert_eq!(
        rhs_type,
        searchqueryinput_typoid(),
        "rhs must represent a SearchQueryInput"
    );

    let lhs_var = make_lhs_var(indexrel, lhs);

    let rhs = wrap_with_index(indexrel, rhs);

    let mut args = PgList::<pg_sys::Node>::new();
    args.push(lhs_var.cast());
    args.push(rhs);

    let mut opexpr = PgBox::<pg_sys::OpExpr>::alloc_node(pg_sys::NodeTag::T_OpExpr);
    opexpr.args = args.into_pg();
    opexpr.opno = anyelement_query_input_opoid();
    opexpr.opfuncid = anyelement_query_input_procoid();
    opexpr.opresulttype = pg_sys::BOOLOID;
    opexpr.opretset = false;
    opexpr.opcollid = pg_sys::Oid::INVALID;
    opexpr.inputcollid = pg_sys::DEFAULT_COLLATION_OID;
    opexpr.location = (*(*srs).fcall).location;

    ReturnedNodePointer(NonNull::new(opexpr.into_pg().cast()))
}

unsafe fn make_lhs_var(indexrel: &PgSearchRelation, lhs: *mut pg_sys::Node) -> *mut pg_sys::Var {
    let index_info = unsafe { *pg_sys::BuildIndexInfo(indexrel.as_ptr()) };
    let heap_attno = index_info.ii_IndexAttrNumbers[0];

    let vars = find_vars(lhs);
    if vars.is_empty() {
        panic!("provided lhs does not contain a Var")
    }

    let tupdesc = indexrel.tuple_desc();
    let att = tupdesc
        .get(0)
        .expect("`USING bm25` index must have at least one attribute which is the 'key_field'");

    let var = pg_sys::copyObjectImpl(vars[0].cast()).cast::<pg_sys::Var>();

    // the Var must look like the first attribute from the index definition
    (*var).varattno = heap_attno;
    (*var).varattnosyn = (*var).varattno;

    // the Var must also assume the type of the first attribute from the index definition
    (*var).vartype = att.atttypid;
    (*var).vartypmod = att.atttypmod;
    (*var).varcollid = att.attcollation;

    var
}

unsafe fn wrap_with_index(
    indexrel: &PgSearchRelation,
    rhs: *mut pg_sys::Node,
) -> *mut pg_sys::Node {
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
    lhs: *mut pg_sys::Node,
    rhs: *mut pg_sys::Node,
    field: Option<FieldName>,
) -> *mut pg_sys::Node
where
    ConstRewrite: FnOnce(*mut pg_sys::Node, Option<FieldName>, RHSValue) -> SearchQueryInput,
    ExecRewrite:
        FnOnce(Option<FieldName>, *mut pg_sys::Node, *mut pg_sys::Node) -> pg_sys::FuncExpr,
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

            // this is specifically used for the `@@@(anyelement, pdb.query)` operator
            other if other == pdb_query_typoid() => RHSValue::PdbQuery(
                pdb::Query::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs fielded query input value must not be NULL"),
            ),

            other if other == boost_typoid() => {
                let boost = BoostType::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs boost value must not be NULL");
                let boost = boost_to_boost(boost, (*const_).consttypmod, true);
                RHSValue::PdbQuery(boost.into())
            }

            other if other == fuzzy_typoid() => {
                let fuzzy = FuzzyType::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs fuzzy value must not be NULL");
                let fuzzy = fuzzy_to_fuzzy(fuzzy, (*const_).consttypmod, true);
                RHSValue::PdbQuery(fuzzy.into())
            }

            other if other == slop_typoid() => {
                let slop = SlopType::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs slop value must not be NULL");
                let slop = slop_to_slop(slop, (*const_).consttypmod, true);
                RHSValue::PdbQuery(slop.into())
            }

            other if other == pdb_proximityclause_typoid() => {
                let prox = ProximityClause::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs fielded proximity clause must not be NULL");
                RHSValue::ProximityClause(prox)
            }

            other => panic!("operator does not support rhs type {other}"),
        };

        let query: *mut pg_sys::Const = const_rewrite(lhs, field, rhs_value).into();
        query.cast()
    } else {
        // the rhs is a complex expression that needs to be evaluated at runtime
        // but its return type is not SearchQueryInput, so we need to rewrite it
        exec_rewrite(field, lhs, rhs).palloc().cast()
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
    let var = find_vars(node);
    if var.is_empty() {
        panic!("cannot determine relation: node does not contain a Var");
    }

    // NB:  assumes all the found vars belong to the same relation
    //      they'd have to, right?  Right?
    find_var_relation(var[0], root)
}

/// Given a [`pg_sys::PlannerInfo`] and a [`pg_sys::Var`] from it, figure out the name of the `Var`
///
/// Returns the heap relation [`pg_sys::Oid`] that contains the `Var` along with its name.
unsafe fn attname_from_var(heaprel: &PgSearchRelation, var: *mut pg_sys::Var) -> Option<FieldName> {
    if (*var).varattno == 0 {
        return None;
    }
    let tupdesc = heaprel.tuple_desc();
    let attname = if (*var).varattno == pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber
    {
        Some("ctid".into())
    } else {
        tupdesc
            .get((*var).varattno as usize - 1)
            .map(|attribute| attribute.name().into())
    };
    attname
}

#[track_caller]
#[inline]
unsafe fn validate_lhs_type_as_text_compatible(lhs: *mut pg_sys::Node, operator_name: &str) {
    #[inline]
    pub fn type_is_text_compatible(oid: pg_sys::Oid) -> bool {
        oid == pg_sys::TEXTOID
            || oid == pg_sys::VARCHAROID
            || oid == pg_sys::TEXTARRAYOID
            || oid == pg_sys::VARCHARARRAYOID
            || oid == pg_sys::JSONOID
            || oid == pg_sys::JSONBOID
            || type_is_tokenizer(oid)
    }

    let typoid = pg_sys::exprType(lhs);
    if !type_is_text_compatible(typoid) {
        let typname = lookup_type_name(typoid).unwrap_or_else(|| String::from("<unknown type>"));
        ErrorReport::new(
            PgSqlErrorCode::ERRCODE_SYNTAX_ERROR,
            format!("type `{typname}` is not compatible with the `{operator_name}` operator"),
            function_name!(),
        )
        .report(PgLogLevel::ERROR);
    }
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

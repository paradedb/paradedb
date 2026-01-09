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

mod andandand;
mod atatat;
mod boost;
mod const_score;
mod eqeqeq;
mod fuzzy;
mod hashhashhash;
mod jsonb_values;
mod ororor;
mod proximity;
mod searchqueryinput;
mod slop;

use crate::api::operator::boost::{boost_to_boost, BoostType};
use crate::api::operator::fuzzy::{fuzzy_to_fuzzy, FuzzyType};
use crate::api::operator::slop::{slop_to_slop, SlopType};
use crate::api::tokenizers::type_can_be_tokenized;
use crate::api::tokenizers::{
    try_get_alias, type_is_alias, type_is_tokenizer, AliasTypmod, UncheckedTypmod,
};
use crate::api::FieldName;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::nodecast;
use crate::postgres::catalog::{lookup_type_name, lookup_typoid};
use crate::postgres::composite::get_composite_type_fields;
use crate::postgres::deparse::deparse_expr;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::{locate_bm25_index_from_heaprel, ToPalloc};
#[cfg(feature = "pg18")]
use crate::postgres::var::resolve_rte_group_var;
use crate::postgres::var::{
    find_json_path, find_one_var, find_var_relation, find_vars, VarContext,
};
use crate::query::pdb_query::pdb;
use crate::query::proximity::ProximityClause;
use crate::query::SearchQueryInput;
use crate::schema::{SearchFieldConfig, SearchFieldType};
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

pub struct JsonbValuesInfo {
    pub base_field: FieldName,
    pub indexrel: PgSearchRelation,
    pub sub_path: Vec<String>,
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

pub fn type_is_jsonb_values(oid: pg_sys::Oid) -> bool {
    Some(oid) == lookup_typoid(c"pdb", c"jsonb_values")
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

pub fn const_typoid() -> pg_sys::Oid {
    unsafe {
        let oid =
            direct_function_call::<pg_sys::Oid>(pg_sys::regtypein, &[c"pdb.const".into_datum()])
                .expect("type `pdb.const` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `pdb.const` should exist");
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

pub(crate) unsafe fn make_searchqueryinput_array(
    elements: Vec<*mut pg_sys::Node>,
) -> *mut pg_sys::ArrayExpr {
    let element_typoid = searchqueryinput_typoid();
    let array_typoid = pg_sys::get_array_type(element_typoid);
    assert!(
        array_typoid != pg_sys::Oid::INVALID,
        "array type for SearchQueryInput should exist"
    );

    let mut element_list = PgList::<pg_sys::Node>::new();
    for element in elements {
        element_list.push(element);
    }

    let mut array_expr = PgBox::<pg_sys::ArrayExpr>::alloc_node(pg_sys::NodeTag::T_ArrayExpr);
    array_expr.array_typeid = array_typoid;
    array_expr.array_collid = pg_sys::Oid::INVALID;
    array_expr.element_typeid = element_typoid;
    array_expr.elements = element_list.into_pg();
    array_expr.multidims = false;
    array_expr.location = -1;

    array_expr.into_pg()
}

pub(crate) unsafe fn make_boolean_should_expr(
    should_elements: Vec<*mut pg_sys::Node>,
) -> *mut pg_sys::FuncExpr {
    let mut args = PgList::<pg_sys::Node>::new();
    args.push(make_searchqueryinput_array(Vec::new()).cast());
    args.push(make_searchqueryinput_array(should_elements).cast());
    args.push(make_searchqueryinput_array(Vec::new()).cast());

    pg_sys::FuncExpr {
        xpr: pg_sys::Expr {
            type_: pg_sys::NodeTag::T_FuncExpr,
        },
        funcid: direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"paradedb.boolean(paradedb.searchqueryinput[], paradedb.searchqueryinput[], paradedb.searchqueryinput[])".into_datum()],
        )
        .expect("`paradedb.boolean(searchqueryinput[], searchqueryinput[], searchqueryinput[])` should exist"),
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
}

pub unsafe fn detect_cast_to_jsonb_values(node: *mut pg_sys::Node) -> Option<*mut pg_sys::Node> {
    if let Some(func) = nodecast!(FuncExpr, T_FuncExpr, node) {
        if type_is_jsonb_values((*func).funcresulttype) {
            let args = PgList::<pg_sys::Node>::from_pg((*func).args);
            return args.get_ptr(0);
        }
    }

    if let Some(coerce) = nodecast!(CoerceViaIO, T_CoerceViaIO, node) {
        if type_is_jsonb_values((*coerce).resulttype) {
            return Some((*coerce).arg.cast::<pg_sys::Node>());
        }
    }

    None
}

pub unsafe fn unwrap_jsonb_values_cast(
    root: *mut pg_sys::PlannerInfo,
    node: *mut pg_sys::Node,
) -> Option<JsonbValuesInfo> {
    let inner = detect_cast_to_jsonb_values(node)?;
    let inner = if let Some(coerce) = nodecast!(CoerceViaIO, T_CoerceViaIO, inner) {
        (*coerce).arg.cast::<pg_sys::Node>()
    } else {
        inner
    };

    let (indexrel, field) = tantivy_field_name_from_node(root, inner)?;
    let json_path = find_json_path(&VarContext::from_planner(root), inner);

    let (base_field, sub_path) = if !json_path.is_empty() {
        (
            FieldName::from(json_path[0].clone()),
            json_path[1..].to_vec(),
        )
    } else {
        (field?, Vec::new())
    };

    Some(JsonbValuesInfo {
        base_field,
        indexrel,
        sub_path,
    })
}

pub fn get_jsonb_values_paths_for_jsonb_values(
    jv_info: &JsonbValuesInfo,
) -> Result<Vec<String>, String> {
    let options = jv_info.indexrel.options();
    let field_type = options.get_field_type(&jv_info.base_field).ok_or_else(|| {
        format!(
            "Field '{}' is not configured in the BM25 index",
            jv_info.base_field
        )
    })?;

    if !matches!(field_type, SearchFieldType::Json(_)) {
        return Err(format!(
            "pdb.jsonb_values can only be used with JSON/JSONB fields.\n\
             HINT: Field '{}' is not a JSON field. Use regular search operators instead.",
            jv_info.base_field
        ));
    }

    let expand_dots = match options.field_config_or_default(&jv_info.base_field) {
        SearchFieldConfig::Json { expand_dots, .. } => expand_dots,
        _ => true,
    };

    if !expand_dots {
        return Err(format!(
            "jsonb_values requires expand_dots=true for field '{}'. \
             Recreate index with expand_dots=true (default) or use explicit path queries.",
            jv_info.base_field
        ));
    }

    let configured_paths = options.jsonb_values_paths();
    if configured_paths.is_empty() {
        return Err(format!(
            "jsonb_values_paths not configured for this index. \
             HINT: ALTER INDEX {} SET (jsonb_values_paths = '{{\"{}\":[...]}}')",
            jv_info.indexrel.name(),
            jv_info.base_field
        ));
    }

    let all_paths = configured_paths.get(&jv_info.base_field).ok_or_else(|| {
        format!(
            "Field '{}' is not configured in jsonb_values_paths. \
             HINT: ALTER INDEX {} SET (jsonb_values_paths = '{{\"{}\":[...]}}')",
            jv_info.base_field,
            jv_info.indexrel.name(),
            jv_info.base_field
        )
    })?;

    if all_paths.is_empty() {
        return Err(format!(
            "jsonb_values_paths is empty for JSON field '{}'. \
             HINT: Add at least one path: [\"color\", \"brand.name\"]",
            jv_info.base_field
        ));
    }

    let invalid_paths: Vec<&str> = all_paths
        .iter()
        .map(|p| p.as_str())
        .filter(|p| p.trim().is_empty() || p.split('.').any(|seg| seg.is_empty()))
        .collect();
    if !invalid_paths.is_empty() {
        return Err(format!(
            "jsonb_values_paths contains empty or invalid path(s) for JSON field '{}': {:?}",
            jv_info.base_field, invalid_paths
        ));
    }

    let paths = if jv_info.sub_path.is_empty() {
        all_paths.to_vec()
    } else {
        let prefix = jv_info.sub_path.join(".");
        let dot_prefix = format!("{}.", prefix);
        let filtered: Vec<String> = all_paths
            .iter()
            .filter(|p| p.starts_with(&dot_prefix) || p.as_str() == prefix.as_str())
            .cloned()
            .collect();

        if filtered.is_empty() {
            return Err(format!(
                "No searchable paths found under '{}.{}'. \
                 Configured paths: {:?}",
                jv_info.base_field, prefix, all_paths
            ));
        }
        filtered
    };

    let mut seen = std::collections::HashSet::new();
    let paths: Vec<String> = paths
        .into_iter()
        .filter(|p| seen.insert(p.clone()))
        .collect();

    if paths.is_empty() {
        return Err(format!(
            "jsonb_values_paths is empty for JSON field '{}'",
            jv_info.base_field
        ));
    }

    Ok(paths)
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
    let estimate = search_reader.estimate_docs(reltuples) as f64;
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

/// Checks if a support function node represents a jsonb_values cast and extracts
/// everything needed for query expansion.
///
/// Returns `(srs, jv_info, inner_lhs, rhs)` if this is a jsonb_values expansion case,
/// or `None` if not applicable.
pub(crate) unsafe fn try_jsonb_values_expansion(
    node: *mut pg_sys::Node,
) -> Option<(
    *mut pg_sys::SupportRequestSimplify,
    JsonbValuesInfo,
    *mut pg_sys::Node,
    *mut pg_sys::Node,
)> {
    let srs = nodecast!(SupportRequestSimplify, T_SupportRequestSimplify, node)?;
    if (*srs).root.is_null() {
        return None;
    }
    let input_args = PgList::<pg_sys::Node>::from_pg((*(*srs).fcall).args);
    let lhs = input_args.get_ptr(0)?;
    let rhs = input_args.get_ptr(1)?;

    let jv_info = unwrap_jsonb_values_cast((*srs).root, lhs)?;
    let inner_lhs = detect_cast_to_jsonb_values(lhs)
        .expect("jsonb_values cast should unwrap to inner expression");

    Some((srs, jv_info, inner_lhs, rhs))
}

pub(crate) unsafe fn build_jsonb_values_exec_boolean(
    jv_info: &JsonbValuesInfo,
    paths: Vec<String>,
    rhs: *mut pg_sys::Node,
    funcid: pg_sys::Oid,
    mut add_args: impl FnMut(&mut PgList<pg_sys::Node>),
) -> *mut pg_sys::Node {
    let mut per_path = Vec::with_capacity(paths.len());
    for path in paths {
        let full_path = format!("{}.{}", jv_info.base_field, path);
        let mut args = PgList::<pg_sys::Node>::new();
        args.push(FieldName::from(full_path).into_const().cast());
        args.push(pg_sys::copyObjectImpl(rhs.cast()).cast());
        add_args(&mut args);

        let func = pg_sys::FuncExpr {
            xpr: pg_sys::Expr {
                type_: pg_sys::NodeTag::T_FuncExpr,
            },
            funcid,
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
        .cast();
        per_path.push(func);
    }

    make_boolean_should_expr(per_path).cast()
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

/// Compare two Vars for equality, ignoring varno and other context-dependent fields.
///
/// In CTE contexts, several fields may legitimately differ:
/// - varno: points to different range table entries (CTE vs original table)
/// - varnosyn, varattnosyn: syntactic variants that may differ in query rewriting
/// - varlevelsup: may differ in nested subquery contexts
/// - varnullingrels: nulling relationships may vary
/// - location: source location in the query text
///
/// The essential fields that must match for the Vars to represent the same column:
/// - varattno: the actual column number in the table
/// - vartype: the column's data type
/// - vartypmod: type modifier (e.g., varchar length)
/// - varcollid: collation (important for text comparisons)
unsafe fn vars_equal_ignoring_varno(a: *const pg_sys::Var, b: *const pg_sys::Var) -> bool {
    (*a).varattno == (*b).varattno
        && (*a).vartype == (*b).vartype
        && (*a).vartypmod == (*b).vartypmod
        && (*a).varcollid == (*b).varcollid
}

unsafe fn row_expr_from_indexed_expr(mut expr: *mut pg_sys::Expr) -> Option<*mut pg_sys::RowExpr> {
    loop {
        if let Some(row_expr) = nodecast!(RowExpr, T_RowExpr, expr) {
            return Some(row_expr);
        }
        if let Some(coerce) = nodecast!(CoerceViaIO, T_CoerceViaIO, expr) {
            expr = (*coerce).arg.cast();
            continue;
        }
        if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, expr) {
            expr = (*relabel).arg.cast();
            continue;
        }
        return None;
    }
}

unsafe fn simple_var_from_expr(mut expr: *mut pg_sys::Expr) -> Option<*const pg_sys::Var> {
    loop {
        if let Some(var) = nodecast!(Var, T_Var, expr) {
            return Some(var);
        }
        if let Some(coerce) = nodecast!(CoerceViaIO, T_CoerceViaIO, expr) {
            expr = (*coerce).arg.cast();
            continue;
        }
        if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, expr) {
            expr = (*relabel).arg.cast();
            continue;
        }
        return None;
    }
}

unsafe fn var_matches_tokenizer_expr(var: *const pg_sys::Var, expr: *mut pg_sys::Expr) -> bool {
    if !type_is_tokenizer(pg_sys::exprType(expr.cast())) {
        return false;
    }
    if !type_can_be_tokenized((*var).vartype) {
        return false;
    }
    let vars = find_vars(expr.cast());
    if vars.len() != 1 {
        return false;
    }
    let expr_var = vars[0];
    if !vars_equal_ignoring_varno(expr_var, var) {
        return false;
    }
    // the Var is the expression that matches the Var we're looking for
    // but lets make sure the whole expression is one without an alias
    // we pick the first un-aliased custom tokenizer expression that uses the
    // Var as the matching indexed expression
    let typmod = pg_sys::exprTypmod(expr.cast());
    let alias = UncheckedTypmod::try_from(typmod)
        .unwrap_or_else(|e| panic!("{e}"))
        .alias();
    alias.is_none()
}

unsafe fn expr_matches_node(node: *mut pg_sys::Node, indexed_expr: *mut pg_sys::Expr) -> bool {
    let mut reduced_expression = indexed_expr;
    loop {
        let inner_expression =
            if let Some(coerce) = nodecast!(CoerceViaIO, T_CoerceViaIO, reduced_expression) {
                (*coerce).arg
            } else {
                reduced_expression
            };

        if pg_sys::equal(node.cast(), inner_expression.cast()) {
            return true;
        }

        if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, reduced_expression) {
            reduced_expression = (*relabel).arg.cast();
            continue;
        }

        // a cast to `pdb.alias` can make it a `FuncExpr` that we need to unwrap
        // Only unwrap pdb.alias casts; unwrapping other FuncExprs like abs() causes false index matches (#3760).
        if let Some(func) = nodecast!(FuncExpr, T_FuncExpr, reduced_expression) {
            if type_is_alias((*func).funcresulttype) {
                let args = PgList::<pg_sys::Node>::from_pg((*func).args);
                if args.len() == 1 {
                    if let Some(arg) = args.get_ptr(0) {
                        reduced_expression = arg.cast();
                        continue;
                    }
                }
            }
        }

        return false;
    }
}

pub unsafe fn field_name_from_node(
    context: VarContext,
    heaprel: &PgSearchRelation,
    indexrel: &PgSearchRelation,
    node: *mut pg_sys::Node,
) -> Option<FieldName> {
    // just directly reach in and pluck out the alias if the type is cast to it
    if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, node) {
        if type_is_alias((*relabel).resulttype) {
            let typmod =
                AliasTypmod::try_from((*relabel).resulttypmod).unwrap_or_else(|e| panic!("{e}"));
            if let Some(alias) = typmod.alias() {
                return Some(FieldName::from(alias));
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

                // Check if the expression is a composite type (RowExpr like ROW(a,b)::my_type)
                let expr_type = pg_sys::exprType(expression.cast());
                let is_composite = crate::postgres::composite::is_composite_type(expr_type);

                if is_composite {
                    if let Some(row_expr) = row_expr_from_indexed_expr(expression) {
                        let composite_oid = pg_sys::exprType(expression.cast());
                        let Ok(fields) = get_composite_type_fields(composite_oid) else {
                            expr_no += 1;
                            continue;
                        };

                        let row_args = PgList::<pg_sys::Node>::from_pg((*row_expr).args);

                        for (position, arg) in row_args.iter_ptr().enumerate() {
                            if position >= fields.len() || fields[position].is_dropped {
                                continue;
                            }

                            if let Some(arg_var) = simple_var_from_expr(arg.cast()) {
                                if vars_equal_ignoring_varno(arg_var, var) {
                                    return Some(FieldName::from(
                                        fields[position].field_name.clone(),
                                    ));
                                }
                                continue;
                            }

                            if var_matches_tokenizer_expr(var, arg.cast()) {
                                return Some(FieldName::from(fields[position].field_name.clone()));
                            }
                        }
                    }
                    expr_no += 1;
                } else if type_is_tokenizer(expr_type) {
                    // Early return for non-tokenizable var types (preserves main branch behavior)
                    if !type_can_be_tokenized((*var).vartype) {
                        return None;
                    }
                    if var_matches_tokenizer_expr(var, expression.cast()) {
                        return attname_from_var(heaprel, var);
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

            // Check if the expression is a composite type (RowExpr like ROW(a,b)::my_type)
            let expr_type = unsafe { pg_sys::exprType(indexed_expression.cast()) };
            let is_composite = crate::postgres::composite::is_composite_type(expr_type);

            if is_composite {
                if let Some(row_expr) = row_expr_from_indexed_expr(indexed_expression) {
                    let composite_oid = unsafe { pg_sys::exprType(indexed_expression.cast()) };
                    let Ok(fields) = get_composite_type_fields(composite_oid) else {
                        continue;
                    };

                    let row_args = unsafe { PgList::<pg_sys::Node>::from_pg((*row_expr).args) };

                    for (position, arg) in row_args.iter_ptr().enumerate() {
                        if position >= fields.len() || fields[position].is_dropped {
                            continue;
                        }

                        if expr_matches_node(node, arg.cast()) {
                            return Some(FieldName::from(fields[position].field_name.clone()));
                        }
                    }
                }
            } else if expr_matches_node(node, indexed_expression) {
                let field_name =
                    if type_is_tokenizer(unsafe { pg_sys::exprType(indexed_expression.cast()) }) {
                        let oid = unsafe { pg_sys::exprType(indexed_expression.cast()) };
                        let typmod = unsafe { pg_sys::exprTypmod(indexed_expression.cast()) };
                        try_get_alias(oid, typmod).map(FieldName::from).or_else(|| {
                            find_one_var(indexed_expression.cast())
                                .and_then(|var| attname_from_var(heaprel, var.cast()))
                        })
                    } else {
                        let expr_str = deparse_expr(None, heaprel, indexed_expression.cast());
                        panic!(
                            "indexed expression requires a tokenizer cast with an alias: {expr_str}"
                        );
                    };

                return field_name;
            }
        }
    }

    //
    // the node we're evaluating doesn't match either an index expression or a direct Var
    //

    // could it be a json(b) path reference like:  json_field->'foo'->>'bar'?
    let json_path = find_json_path(&context, node);
    if json_path.len() > 1 {
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

    let lhs_var = make_lhs_var((*srs).root, indexrel, lhs);

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

#[cfg_attr(not(feature = "pg18"), allow(unused_variables))]
unsafe fn make_lhs_var(
    root: *mut pg_sys::PlannerInfo,
    indexrel: &PgSearchRelation,
    lhs: *mut pg_sys::Node,
) -> *mut pg_sys::Var {
    let index_info = unsafe { *pg_sys::BuildIndexInfo(indexrel.as_ptr()) };
    let heap_attno = index_info.ii_IndexAttrNumbers[0];

    let vars = find_vars(lhs);
    if vars.is_empty() {
        panic!("provided lhs does not contain a Var")
    }

    let base_var = vars[0];
    #[cfg(feature = "pg18")]
    let base_var = resolve_lhs_var_for_group(root, base_var);
    let tupdesc = indexrel.tuple_desc();
    let att = tupdesc
        .get(0)
        .expect("`USING bm25` index must have at least one attribute which is the 'key_field'");

    let var = pg_sys::copyObjectImpl(base_var.cast()).cast::<pg_sys::Var>();

    // the Var must look like the first attribute from the index definition
    (*var).varattno = heap_attno;
    (*var).varattnosyn = (*var).varattno;

    // the Var must also assume the type of the first attribute from the index definition
    (*var).vartype = att.atttypid;
    (*var).vartypmod = att.atttypmod;
    (*var).varcollid = att.attcollation;

    var
}

#[cfg(feature = "pg18")]
#[inline]
unsafe fn resolve_lhs_var_for_group(
    root: *mut pg_sys::PlannerInfo,
    var: *mut pg_sys::Var,
) -> *mut pg_sys::Var {
    let varno = (*var).varno as pg_sys::Index;
    let rtable = (*(*root).parse).rtable;

    // Bounds check: varno is 1-indexed and must be within the rtable
    let rtable_size = if !rtable.is_null() {
        PgList::<pg_sys::RangeTblEntry>::from_pg(rtable).len()
    } else {
        0
    };
    if varno == 0 || varno as usize > rtable_size {
        return var;
    }

    let rte = pg_sys::rt_fetch(varno, rtable);
    if (*rte).rtekind == pg_sys::RTEKind::RTE_GROUP {
        // PG18 introduces RTE_GROUP for GROUP BY expressions. When a Var references
        // RTE_GROUP, it points to a synthetic range table entry rather than the base
        // relation. We must resolve these Vars back to their originating column so
        // that operator rewriting can correctly identify the indexed field.
        if let Some(group_var) = resolve_rte_group_var(rte, (*var).varattno) {
            return group_var;
        }
    }

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
    let typoid = pg_sys::exprType(lhs);
    if !type_can_be_tokenized(typoid) && !type_is_tokenizer(typoid) {
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

mod f16_typmod {
    // we clamp the user-provided typmod bounds to this so that we're sure they'll fit after being
    // converted to an f16 without accuracy loss on integer values
    pub(in crate::api::operator) const TYPMOD_BOUNDS: (f32, f32) = (-2048.0, 2048.0);

    /// Serialize an f32 to a non‑negative i32 by first converting to f16.
    /// Panics if val is NaN/Inf or out of f16’s representable range.
    pub fn serialize_f32_to_i32(val: f32) -> i32 {
        assert!(
            val.is_finite() && val >= TYPMOD_BOUNDS.0 && val <= TYPMOD_BOUNDS.1,
            "only 16 bit floats in the range [{}..{}] are supported",
            TYPMOD_BOUNDS.0,
            TYPMOD_BOUNDS.1
        );
        let half: half::f16 = half::f16::from_f32(val);
        let bits: u16 = half.to_bits();
        bits as i32 // in [0, 0xFFFF], always >= 0
    }

    /// Deserialize the i32 back to a f32 via f16.
    /// Panics if encoded is outside [0, 65535].
    pub fn deserialize_i32_to_f32(encoded: i32) -> f32 {
        assert!(
            (0..=u16::MAX as i32).contains(&encoded),
            "invalid typemod `{encoded}`: must be between 0 and {}",
            u16::MAX
        );

        let bits: u16 = encoded as u16;
        let half = half::f16::from_bits(bits);
        half.to_f32()
    }

    #[test]
    fn roundtrip() {
        use proptest::proptest;

        proptest!(|(typmod in TYPMOD_BOUNDS.0 as i32..TYPMOD_BOUNDS.1 as i32)| {
            let encoded = serialize_f32_to_i32(typmod as f32);
            let decoded = deserialize_i32_to_f32(encoded) as i32;
            assert!(typmod == decoded, "typmod={typmod}, decoded={decoded}");
        });
    }
}

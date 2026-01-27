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
use crate::api::builder_fns::{term_set_str, term_str};
use crate::api::operator::boost::{boost_to_boost, BoostType};
use crate::api::operator::fuzzy::{fuzzy_to_fuzzy, FuzzyType};
use crate::api::operator::{
    boost_typoid, build_jsonb_values_exec_boolean, fuzzy_typoid, get_expr_result_type,
    get_jsonb_values_paths_for_jsonb_values, pdb_query_typoid, request_simplify,
    rewrite_to_search_query_input_opexpr, searchqueryinput_typoid, try_jsonb_values_expansion,
    validate_lhs_type_as_text_compatible, RHSValue, ReturnedNodePointer,
};
use crate::api::FieldName;
use crate::nodecast;
use crate::query::pdb_query::{pdb, to_search_query_input};
use crate::query::SearchQueryInput;
use pgrx::{
    direct_function_call, extension_sql, opname, pg_extern, pg_operator, pg_sys, AnyElement,
    FromDatum, Internal, IntoDatum, PgList,
};

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.===)]
fn search_with_term(_field: AnyElement, term: &str) -> bool {
    panic!("query is incompatible with pg_search's `===(field, TEXT)` operator: `{term}`")
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.===)]
fn search_with_term_array(_field: AnyElement, terms: Vec<String>) -> bool {
    panic!("query is incompatible with pg_search's `===(field, TEXT[])` operator: `{terms:?}`")
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.===)]
fn search_with_term_pdb_query(_field: AnyElement, term: pdb::Query) -> bool {
    panic!("query is incompatible with pg_search's `===(field, pdb.query)` operator: `{term:?}`")
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.===)]
fn search_with_term_boost(_field: AnyElement, term: BoostType) -> bool {
    panic!("query is incompatible with pg_search's `===(field, boost)` operator: `{term:?}`")
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.===)]
fn search_with_term_fuzzy(_field: AnyElement, term: FuzzyType) -> bool {
    panic!("query is incompatible with pg_search's `===(field, fuzzy)` operator: `{term:?}`")
}

#[pg_extern(immutable, parallel_safe)]
fn search_with_term_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        let node = arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>();
        if let Some((srs, jv_info, inner_lhs, rhs)) = try_jsonb_values_expansion(node) {
            if nodecast!(Const, T_Const, rhs).is_some() {
                return expand_jsonb_values_term_query_const(srs, &jv_info, inner_lhs, rhs);
            }
            return expand_jsonb_values_term_query_exec(srs, &jv_info, inner_lhs, rhs);
        }

        request_simplify(node, |lhs, field, term| {
            validate_lhs_type_as_text_compatible(lhs, "===");

            let field = field
                .expect("The left hand side of the `===(field, TEXT)` operator must be a field.");

            match term {
                RHSValue::Text(term) => to_search_query_input(field, term_str(term)),
                RHSValue::TextArray(terms) => to_search_query_input(field, term_set_str(terms)),
                RHSValue::PdbQuery(pdb::Query::ScoreAdjusted { query, score }) => {
                    let mut query = *query;
                    if let pdb::Query::UnclassifiedString { string, fuzzy_data, slop_data } = query {
                        query = term_str(string);
                        query.apply_fuzzy_data(fuzzy_data);
                        query.apply_slop_data(slop_data);
                    } else if let pdb::Query::UnclassifiedArray {array, fuzzy_data, slop_data} = query {
                        query = term_set_str(array);
                        query.apply_fuzzy_data(fuzzy_data);
                        query.apply_slop_data(slop_data);
                    }
                    to_search_query_input(field, pdb::Query::ScoreAdjusted { query: Box::new(query), score })
                }
                RHSValue::PdbQuery(pdb::Query::UnclassifiedString { string, fuzzy_data, slop_data }) => {
                    let mut query = term_str(string);
                    query.apply_fuzzy_data(fuzzy_data);
                    query.apply_slop_data(slop_data);
                    to_search_query_input(field, query)
                }
                RHSValue::PdbQuery(pdb::Query::UnclassifiedArray { array, fuzzy_data, slop_data }) => {
                    let mut query = term_set_str(array);
                    query.apply_fuzzy_data(fuzzy_data);
                    query.apply_slop_data(slop_data);
                    to_search_query_input(field, query)
                }
                _ => unreachable!("The right-hand side of the `===(field, TEXT)` operator must be a text or text array value")
            }
        }, |field, lhs, rhs| {
            validate_lhs_type_as_text_compatible(lhs, "===");
            let field = field.expect("The left hand side of the `===(field, TEXT)` operator must be a field.");
            let expr_type = get_expr_result_type(rhs);
            assert!({
                        expr_type == pg_sys::TEXTOID || expr_type == pg_sys::VARCHAROID || expr_type == pg_sys::TEXTARRAYOID || expr_type == pg_sys::VARCHARARRAYOID
                    }, "The right-hand side of the `===(field, TEXT)` operator must be a text or text array value");
            let is_array = expr_type == pg_sys::TEXTARRAYOID || expr_type == pg_sys::VARCHARARRAYOID;

            let mut args = PgList::<pg_sys::Node>::new();
            args.push(field.into_const().cast());
            args.push(rhs.cast());

            pg_sys::FuncExpr {
                xpr: pg_sys::Expr { type_: pg_sys::NodeTag::T_FuncExpr },
                funcid: if is_array {
                    direct_function_call::<pg_sys::Oid>(
                        pg_sys::regprocedurein,
                        &[c"paradedb.term_set(paradedb.fieldname, text[])".into_datum()],
                    )
                        .expect("`paradedb.term_set(paradedb.fieldname, text[])` should exist")
                } else {
                    direct_function_call::<pg_sys::Oid>(
                        pg_sys::regprocedurein,
                        &[c"paradedb.term(paradedb.fieldname, text)".into_datum()],
                    )
                        .expect("`paradedb.term(paradedb.fieldname, text)` should exist")
                },
                funcresulttype: searchqueryinput_typoid(),
                funcretset: false,
                funcvariadic: false,
                funcformat: pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
                funccollid: pg_sys::Oid::INVALID,
                inputcollid: pg_sys::Oid::INVALID,
                args: args.into_pg(),
                location: -1,
            }
        })
        .unwrap_or(ReturnedNodePointer(None))
    }
}

unsafe fn expand_jsonb_values_term_query_const(
    srs: *mut pg_sys::SupportRequestSimplify,
    jv_info: &crate::api::operator::JsonbValuesInfo,
    lhs: *mut pg_sys::Node,
    rhs: *mut pg_sys::Node,
) -> ReturnedNodePointer {
    let paths = match get_jsonb_values_paths_for_jsonb_values(jv_info) {
        Ok(paths) => paths,
        Err(e) => pgrx::error!("{}", e),
    };

    let const_node = nodecast!(Const, T_Const, rhs)
        .expect("expand_jsonb_values_term_query_const requires a Const RHS");

    let normalize_query = |query: pdb::Query| -> pdb::Query {
        match query {
            pdb::Query::UnclassifiedString {
                string,
                fuzzy_data,
                slop_data,
            } => {
                let mut q = term_str(string);
                q.apply_fuzzy_data(fuzzy_data);
                q.apply_slop_data(slop_data);
                q
            }
            pdb::Query::UnclassifiedArray {
                array,
                fuzzy_data,
                slop_data,
            } => {
                let mut q = term_set_str(array);
                q.apply_fuzzy_data(fuzzy_data);
                q.apply_slop_data(slop_data);
                q
            }
            pdb::Query::ScoreAdjusted { query, score } => {
                let inner = match *query {
                    pdb::Query::UnclassifiedString {
                        string,
                        fuzzy_data,
                        slop_data,
                    } => {
                        let mut q = term_str(string);
                        q.apply_fuzzy_data(fuzzy_data);
                        q.apply_slop_data(slop_data);
                        q
                    }
                    pdb::Query::UnclassifiedArray {
                        array,
                        fuzzy_data,
                        slop_data,
                    } => {
                        let mut q = term_set_str(array);
                        q.apply_fuzzy_data(fuzzy_data);
                        q.apply_slop_data(slop_data);
                        q
                    }
                    other => other,
                };
                pdb::Query::ScoreAdjusted {
                    query: Box::new(inner),
                    score,
                }
            }
            _ => {
                pgrx::error!(
                    "jsonb_values ===: pdb.query RHS must be UnclassifiedString, UnclassifiedArray, or ScoreAdjusted"
                );
            }
        }
    };

    let base_query: pdb::Query = match (*const_node).consttype {
        pg_sys::TEXTOID | pg_sys::VARCHAROID => {
            let term = String::from_datum((*const_node).constvalue, (*const_node).constisnull)
                .expect("rhs text value must not be NULL");
            term_str(term)
        }
        pg_sys::TEXTARRAYOID | pg_sys::VARCHARARRAYOID => {
            let terms =
                Vec::<String>::from_datum((*const_node).constvalue, (*const_node).constisnull)
                    .expect("rhs text array value must not be NULL");
            term_set_str(terms)
        }
        other if other == pdb_query_typoid() => {
            let query = pdb::Query::from_datum((*const_node).constvalue, (*const_node).constisnull)
                .expect("rhs pdb query value must not be NULL");
            normalize_query(query)
        }
        other if other == boost_typoid() => {
            let boost = BoostType::from_datum((*const_node).constvalue, (*const_node).constisnull)
                .expect("rhs boost value must not be NULL");
            let query: pdb::Query = boost_to_boost(boost, (*const_node).consttypmod, true).into();
            normalize_query(query)
        }
        other if other == fuzzy_typoid() => {
            let fuzzy = FuzzyType::from_datum((*const_node).constvalue, (*const_node).constisnull)
                .expect("rhs fuzzy value must not be NULL");
            let query: pdb::Query = fuzzy_to_fuzzy(fuzzy, (*const_node).consttypmod, true).into();
            normalize_query(query)
        }
        other => {
            pgrx::error!("jsonb_values === does not support RHS type OID {}", other);
        }
    };

    let subqueries: Vec<SearchQueryInput> = paths
        .iter()
        .map(|path| {
            let full_path = format!("{}.{}", jv_info.base_field, path);
            to_search_query_input(FieldName::from(full_path), base_query.clone())
        })
        .collect();

    let expanded = SearchQueryInput::Boolean {
        should: subqueries,
        must: vec![],
        must_not: vec![],
    };
    let as_const: *mut pg_sys::Const = expanded.into();

    rewrite_to_search_query_input_opexpr(srs, &jv_info.indexrel, lhs, as_const.cast())
}

unsafe fn expand_jsonb_values_term_query_exec(
    srs: *mut pg_sys::SupportRequestSimplify,
    jv_info: &crate::api::operator::JsonbValuesInfo,
    lhs: *mut pg_sys::Node,
    rhs: *mut pg_sys::Node,
) -> ReturnedNodePointer {
    let paths = match get_jsonb_values_paths_for_jsonb_values(jv_info) {
        Ok(paths) => paths,
        Err(e) => pgrx::error!("{}", e),
    };

    let expr_type = get_expr_result_type(rhs);
    let is_array = expr_type == pg_sys::TEXTARRAYOID || expr_type == pg_sys::VARCHARARRAYOID;
    if !(expr_type == pg_sys::TEXTOID || expr_type == pg_sys::VARCHAROID || is_array) {
        pgrx::error!("jsonb_values === requires a text or text[] RHS for runtime expressions");
    }

    let funcid = if is_array {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"paradedb.term_set(paradedb.fieldname, text[])".into_datum()],
        )
        .expect("`paradedb.term_set(paradedb.fieldname, text[])` should exist")
    } else {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"paradedb.term(paradedb.fieldname, text)".into_datum()],
        )
        .expect("`paradedb.term(paradedb.fieldname, text)` should exist")
    };

    let boolean_expr = build_jsonb_values_exec_boolean(jv_info, paths, rhs, funcid, |_| {});
    rewrite_to_search_query_input_opexpr(srs, &jv_info.indexrel, lhs, boolean_expr)
}

extension_sql!(
    r#"
        ALTER FUNCTION paradedb.search_with_term SUPPORT paradedb.search_with_term_support;
        ALTER FUNCTION paradedb.search_with_term_array SUPPORT paradedb.search_with_term_support;
        ALTER FUNCTION paradedb.search_with_term_pdb_query SUPPORT paradedb.search_with_term_support;
        ALTER FUNCTION paradedb.search_with_term_boost SUPPORT paradedb.search_with_term_support;
        ALTER FUNCTION paradedb.search_with_term_fuzzy SUPPORT paradedb.search_with_term_support;
    "#,
    name = "search_with_term_support_fn",
    requires = [
        search_with_term,
        search_with_term_array,
        search_with_term_pdb_query,
        search_with_term_boost,
        search_with_term_fuzzy,
        search_with_term_support
    ]
);

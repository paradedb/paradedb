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
use crate::api::builder_fns::{phrase_array, phrase_string};
use crate::api::operator::boost::{boost_to_boost, BoostType};
use crate::api::operator::slop::{slop_to_slop, SlopType};
use crate::api::operator::{
    boost_typoid, build_jsonb_values_exec_boolean, get_expr_result_type,
    get_jsonb_values_paths_for_jsonb_values, pdb_query_typoid, request_simplify,
    rewrite_to_search_query_input_opexpr, searchqueryinput_typoid, slop_typoid,
    try_jsonb_values_expansion, validate_lhs_type_as_text_compatible, RHSValue,
    ReturnedNodePointer,
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
#[opname(pg_catalog.###)]
fn search_with_phrase(_field: AnyElement, terms_to_tokenize: &str) -> bool {
    panic!(
        "query is incompatible with pg_search's `###(field, TEXT)` operator: `{terms_to_tokenize}`"
    )
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.###)]
fn search_with_phrase_array(_field: AnyElement, tokens: Vec<String>) -> bool {
    panic!("query is incompatible with pg_search's `###(field, TEXT[])` operator: `{tokens:?}`")
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.###)]
fn search_with_phrase_pdb_query(_field: AnyElement, terms_to_tokenize: pdb::Query) -> bool {
    panic!(
        "query is incompatible with pg_search's `###(field, pdb.query)` operator: `{terms_to_tokenize:?}`"
    )
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.###)]
fn search_with_phrase_boost(_field: AnyElement, terms_to_tokenize: BoostType) -> bool {
    panic!(
        "query is incompatible with pg_search's `###(field, boost)` operator: `{terms_to_tokenize:?}`"
    )
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000, requires = ["SlopType_final"])]
#[opname(pg_catalog.###)]
fn search_with_phrase_slop(_field: AnyElement, terms_to_tokenize: SlopType) -> bool {
    panic!(
        "query is incompatible with pg_search's `###(field, slop)` operator: `{terms_to_tokenize:?}`"
    )
}

#[pg_extern(immutable, parallel_safe)]
fn search_with_phrase_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        let node = arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>();
        if let Some((srs, jv_info, inner_lhs, rhs)) = try_jsonb_values_expansion(node) {
            if nodecast!(Const, T_Const, rhs).is_some() {
                return expand_jsonb_values_phrase_query_const(srs, &jv_info, inner_lhs, rhs);
            }
            return expand_jsonb_values_phrase_query_exec(srs, &jv_info, inner_lhs, rhs);
        }

        request_simplify(node, |lhs, field, to_tokenize| {
            validate_lhs_type_as_text_compatible(lhs, "###");
            let field = field
                .expect("The left hand side of the `###(field, TEXT)` operator must be a field.");
            match to_tokenize {
                RHSValue::Text(text) => {
                    to_search_query_input(field, phrase_string(text))
                },
                RHSValue::TextArray(tokens) => {
                    to_search_query_input(field, phrase_array(tokens))
                }
                RHSValue::PdbQuery(pdb::Query::ScoreAdjusted { query, score}) => {
                    let mut query = *query;
                    if let pdb::Query::UnclassifiedString {string, slop_data, ..} = query {
                        query = phrase_string(string);
                        query.apply_slop_data(slop_data);
                    } else if let pdb::Query::UnclassifiedArray {array,  slop_data, ..} = query {
                        query = phrase_array(array);
                        query.apply_slop_data(slop_data);
                    }
                    to_search_query_input(field, pdb::Query::ScoreAdjusted { query: Box::new(query), score})
                }
                RHSValue::PdbQuery(pdb::Query::UnclassifiedString {string, slop_data, ..}) => {
                    let mut query = phrase_string(string);
                    query.apply_slop_data(slop_data);
                    to_search_query_input(field, query)
                }
                RHSValue::PdbQuery(pdb::Query::UnclassifiedArray { array, slop_data, .. }) => {
                    let mut query = phrase_array(array);
                    query.apply_slop_data(slop_data);
                    to_search_query_input(field, query)
                }
                _ => panic!("The right-hand side of the `###(field, TEXT)` operator must be a text value."),
            }
        }, |field, lhs, rhs| {
            validate_lhs_type_as_text_compatible(lhs, "###");
            let field = field.expect("The left hand side of the `###(field, TEXT)` operator must be a field.");
            assert!(get_expr_result_type(rhs) == pg_sys::TEXTOID, "The right-hand side of the `###(field, TEXT)` operator must be a text value");
            let mut args = PgList::<pg_sys::Node>::new();

            args.push(field.into_const().cast());
            args.push(rhs.cast());

            pg_sys::FuncExpr {
                xpr: pg_sys::Expr { type_: pg_sys::NodeTag::T_FuncExpr },
                funcid: direct_function_call::<pg_sys::Oid>(
                    pg_sys::regprocedurein,
                    &[c"paradedb.phrase(paradedb.fieldname, text)".into_datum()],
                )
                    .expect("`paradedb.phrase(paradedb.fieldname, text)` should exist"),
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

unsafe fn expand_jsonb_values_phrase_query_const(
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
        .expect("expand_jsonb_values_phrase_query_const requires a Const RHS");

    let normalize_query = |query: pdb::Query| -> pdb::Query {
        match query {
            pdb::Query::UnclassifiedString {
                string, slop_data, ..
            } => {
                let mut q = phrase_string(string);
                q.apply_slop_data(slop_data);
                q
            }
            pdb::Query::UnclassifiedArray {
                array, slop_data, ..
            } => {
                let mut q = phrase_array(array);
                q.apply_slop_data(slop_data);
                q
            }
            pdb::Query::ScoreAdjusted { query, score } => {
                let mut inner = *query;
                if let pdb::Query::UnclassifiedString {
                    string, slop_data, ..
                } = inner
                {
                    inner = phrase_string(string);
                    inner.apply_slop_data(slop_data);
                } else if let pdb::Query::UnclassifiedArray {
                    array, slop_data, ..
                } = inner
                {
                    inner = phrase_array(array);
                    inner.apply_slop_data(slop_data);
                }
                pdb::Query::ScoreAdjusted {
                    query: Box::new(inner),
                    score,
                }
            }
            _ => {
                pgrx::error!(
                    "jsonb_values ###: pdb.query RHS must be UnclassifiedString, UnclassifiedArray, or ScoreAdjusted"
                );
            }
        }
    };

    let base_query: pdb::Query = match (*const_node).consttype {
        pg_sys::TEXTOID | pg_sys::VARCHAROID => {
            let text = String::from_datum((*const_node).constvalue, (*const_node).constisnull)
                .expect("rhs text value must not be NULL");
            phrase_string(text)
        }
        pg_sys::TEXTARRAYOID | pg_sys::VARCHARARRAYOID => {
            let tokens =
                Vec::<String>::from_datum((*const_node).constvalue, (*const_node).constisnull)
                    .expect("rhs text array value must not be NULL");
            phrase_array(tokens)
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
        other if other == slop_typoid() => {
            let slop = SlopType::from_datum((*const_node).constvalue, (*const_node).constisnull)
                .expect("rhs slop value must not be NULL");
            let query: pdb::Query = slop_to_slop(slop, (*const_node).consttypmod, true).into();
            normalize_query(query)
        }
        other => {
            pgrx::error!("jsonb_values ### does not support RHS type OID {}", other);
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

unsafe fn expand_jsonb_values_phrase_query_exec(
    srs: *mut pg_sys::SupportRequestSimplify,
    jv_info: &crate::api::operator::JsonbValuesInfo,
    lhs: *mut pg_sys::Node,
    rhs: *mut pg_sys::Node,
) -> ReturnedNodePointer {
    let paths = match get_jsonb_values_paths_for_jsonb_values(jv_info) {
        Ok(paths) => paths,
        Err(e) => pgrx::error!("{}", e),
    };

    if get_expr_result_type(rhs) != pg_sys::TEXTOID {
        pgrx::error!("jsonb_values ### requires a text RHS for runtime expressions");
    }

    let funcid = direct_function_call::<pg_sys::Oid>(
        pg_sys::regprocedurein,
        &[c"paradedb.phrase(paradedb.fieldname, text)".into_datum()],
    )
    .expect("`paradedb.phrase(paradedb.fieldname, text)` should exist");

    let boolean_expr = build_jsonb_values_exec_boolean(jv_info, paths, rhs, funcid, |_| {});
    rewrite_to_search_query_input_opexpr(srs, &jv_info.indexrel, lhs, boolean_expr)
}

extension_sql!(
    r#"
        ALTER FUNCTION paradedb.search_with_phrase SUPPORT paradedb.search_with_phrase_support;
        ALTER FUNCTION paradedb.search_with_phrase_array SUPPORT paradedb.search_with_phrase_support;
        ALTER FUNCTION paradedb.search_with_phrase_pdb_query SUPPORT paradedb.search_with_phrase_support;
        ALTER FUNCTION paradedb.search_with_phrase_boost SUPPORT paradedb.search_with_phrase_support;
        ALTER FUNCTION paradedb.search_with_phrase_slop SUPPORT paradedb.search_with_phrase_support;
    "#,
    name = "search_with_phrase_support_fn",
    requires = [
        search_with_phrase,
        search_with_phrase_array,
        search_with_phrase_pdb_query,
        search_with_phrase_boost,
        search_with_phrase_support
    ]
);

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
use crate::api::builder_fns::{match_disjunction, match_disjunction_array, term_set_str};
use crate::api::operator::boost::BoostType;
use crate::api::operator::fuzzy::FuzzyType;
use crate::api::operator::{
    get_expr_result_type, request_simplify, searchqueryinput_typoid,
    validate_lhs_type_as_text_compatible, RHSValue, ReturnedNodePointer,
};
use crate::query::pdb_query::{pdb, to_search_query_input};
use pgrx::{
    direct_function_call, extension_sql, opname, pg_extern, pg_operator, pg_sys, AnyElement,
    Internal, IntoDatum, PgList,
};

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.|||)]
fn search_with_match_disjunction(_field: AnyElement, terms_to_tokenize: &str) -> bool {
    panic!(
        "query is incompatible with pg_search's `|||(field, TEXT)` operator: `{terms_to_tokenize}`"
    )
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.|||)]
fn search_with_match_disjunction_array(_field: AnyElement, exact_tokens: Vec<String>) -> bool {
    panic!(
        "query is incompatible with pg_search's `|||(field, TEXT[])` operator: `{exact_tokens:?}`"
    )
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.|||)]
fn search_with_match_disjunction_pdb_query(
    _field: AnyElement,
    terms_to_tokenize: pdb::Query,
) -> bool {
    panic!(
        "query is incompatible with pg_search's `|||(field, pdb.query)` operator: `{terms_to_tokenize:?}`"
    )
}
#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.|||)]
fn search_with_match_disjunction_boost(_field: AnyElement, terms_to_tokenize: BoostType) -> bool {
    panic!(
        "query is incompatible with pg_search's `|||(field, boost)` operator: `{terms_to_tokenize:?}`"
    )
}
#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.|||)]
fn search_with_match_disjunction_fuzzy(_field: AnyElement, terms_to_tokenize: FuzzyType) -> bool {
    panic!(
        "query is incompatible with pg_search's `|||(field, fuzzy)` operator: `{terms_to_tokenize:?}`"
    )
}

#[pg_extern(immutable, parallel_safe)]
fn search_with_match_disjunction_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        request_simplify(arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>(), |lhs, field, to_tokenize| {
            validate_lhs_type_as_text_compatible(lhs, "|||");
            let field = field.expect("The left hand side of the `|||(field, TEXT)` operator must be a field.");
            match to_tokenize {
                RHSValue::Text(text) => {
                    to_search_query_input(field, match_disjunction(text))
                },
                RHSValue::TextArray(tokens) => {
                    to_search_query_input(field, match_disjunction_array(tokens))
                }
                RHSValue::PdbQuery(pdb::Query::ScoreAdjusted { query, score}) => {
                    let mut query = *query;
                    if let pdb::Query::UnclassifiedString {string, fuzzy_data, slop_data} = query {
                        query = match_disjunction(string);
                        query.apply_fuzzy_data(fuzzy_data);
                        query.apply_slop_data(slop_data);
                    } else if let pdb::Query::UnclassifiedArray {array, fuzzy_data, slop_data} = query {
                        query = match_disjunction_array(array);
                        query.apply_fuzzy_data(fuzzy_data);
                        query.apply_slop_data(slop_data);
                    }
                    to_search_query_input(field, pdb::Query::ScoreAdjusted { query: Box::new(query), score})
                }
                RHSValue::PdbQuery(pdb::Query::UnclassifiedString {string, fuzzy_data, slop_data}) => {
                    let mut query = match_disjunction(string);
                    query.apply_fuzzy_data(fuzzy_data);
                    query.apply_slop_data(slop_data);
                    to_search_query_input(field, query)
                }
                RHSValue::PdbQuery(pdb::Query::UnclassifiedArray { array, fuzzy_data, slop_data }) => {
                    let mut query = term_set_str(array);
                    query.apply_fuzzy_data(fuzzy_data);
                    query.apply_slop_data(slop_data);

                    assert!(matches!(query, pdb::Query::MatchArray{..}));
                    let pdb::Query::MatchArray { conjunction_mode, .. } = &mut query else {
                        unreachable!()
                    };
                    *conjunction_mode = Some(false);

                    to_search_query_input(field, query)
                }
                _ => panic!("The right-hand side of the `|||(field, TEXT)` operator must be a text value."),
            }
        }, |field, lhs, rhs| {
            validate_lhs_type_as_text_compatible(lhs, "|||");
            let field = field.expect("The left hand side of the `|||(field, TEXT)` operator must be a field.");
            assert!(get_expr_result_type(rhs) == pg_sys::TEXTOID, "The right-hand side of the `|||(field, TEXT)` operator must be a text value");
            let mut args = PgList::<pg_sys::Node>::new();

            args.push(field.into_const().cast());
            args.push(rhs.cast());

            pg_sys::FuncExpr {
                xpr: pg_sys::Expr { type_: pg_sys::NodeTag::T_FuncExpr },
                funcid: direct_function_call::<pg_sys::Oid>(
                    pg_sys::regprocedurein,
                    &[c"paradedb.match_disjunction(paradedb.fieldname, text)".into_datum()],
                )
                    .expect("`paradedb.match_disjunction(paradedb.fieldname, text)` should exist"),
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

extension_sql!(
    r#"
        ALTER FUNCTION paradedb.search_with_match_disjunction SUPPORT paradedb.search_with_match_disjunction_support;
        ALTER FUNCTION paradedb.search_with_match_disjunction_array SUPPORT paradedb.search_with_match_disjunction_support;
        ALTER FUNCTION paradedb.search_with_match_disjunction_pdb_query SUPPORT paradedb.search_with_match_disjunction_support;
        ALTER FUNCTION paradedb.search_with_match_disjunction_boost SUPPORT paradedb.search_with_match_disjunction_support;
        ALTER FUNCTION paradedb.search_with_match_disjunction_fuzzy SUPPORT paradedb.search_with_match_disjunction_support;
    "#,
    name = "search_with_match_disjunction_support_fn",
    requires = [
        search_with_match_disjunction,
        search_with_match_disjunction_array,
        search_with_match_disjunction_pdb_query,
        search_with_match_disjunction_boost,
        search_with_match_disjunction_fuzzy,
        search_with_match_disjunction_support
    ]
);

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
use crate::api::operator::boost::BoostType;
use crate::api::operator::fuzzy::FuzzyType;
use crate::api::operator::{
    boost_typoid, coerce_to_pdb_query, fuzzy_typoid, get_expr_result_type, pdb_query_typoid,
    request_simplify, searchqueryinput_typoid, validate_lhs_type_as_text_compatible, RHSValue,
    ReturnedNodePointer,
};
use crate::api::FieldName;
use crate::query::pdb_query::{pdb, to_search_query_input};
use crate::query::SearchQueryInput;
use pgrx::{
    direct_function_call, extension_sql, opname, pg_extern, pg_operator, pg_sys, AnyElement,
    Internal, IntoDatum, PgList,
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

/// Converts a pdb::Query to SearchQueryInput for the === operator.
/// Handles UnclassifiedString and UnclassifiedArray by converting them to Term/TermSet.
#[pg_extern(immutable, parallel_safe, name = "term")]
fn term_query(field: FieldName, term: pdb::Query) -> SearchQueryInput {
    match term {
        pdb::Query::ScoreAdjusted { query, score } => {
            let mut query = *query;
            if let pdb::Query::UnclassifiedString {
                string,
                fuzzy_data,
                slop_data,
            } = query
            {
                query = term_str(string);
                query.apply_fuzzy_data(fuzzy_data);
                query.apply_slop_data(slop_data);
            } else if let pdb::Query::UnclassifiedArray {
                array,
                fuzzy_data,
                slop_data,
            } = query
            {
                query = term_set_str(array);
                query.apply_fuzzy_data(fuzzy_data);
                query.apply_slop_data(slop_data);
            }
            to_search_query_input(
                field,
                pdb::Query::ScoreAdjusted {
                    query: Box::new(query),
                    score,
                },
            )
        }
        pdb::Query::UnclassifiedString {
            string,
            fuzzy_data,
            slop_data,
        } => {
            let mut query = term_str(string);
            query.apply_fuzzy_data(fuzzy_data);
            query.apply_slop_data(slop_data);
            to_search_query_input(field, query)
        }
        pdb::Query::UnclassifiedArray {
            array,
            fuzzy_data,
            slop_data,
        } => {
            let mut query = term_set_str(array);
            query.apply_fuzzy_data(fuzzy_data);
            query.apply_slop_data(slop_data);
            to_search_query_input(field, query)
        }
        _ => panic!(
            "The right-hand side of the `===` operator must be text, text[], or an unclassified pdb.* value."
        ),
    }
}

#[pg_extern(immutable, parallel_safe)]
fn search_with_term_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        request_simplify(arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>(), |lhs, field, term| {
            validate_lhs_type_as_text_compatible(lhs, "===");
            let field = field
                .expect("The left hand side of the `===(field, TEXT)` operator must be a field.");
            match term {
                RHSValue::Text(term) => to_search_query_input(field, term_str(term)),
                RHSValue::TextArray(terms) => to_search_query_input(field, term_set_str(terms)),
                RHSValue::PdbQuery(query) => term_query(field, query),
                _ => panic!(
                    "The right-hand side of the `===` operator must be text, text[], or an unclassified pdb.* value."
                )
            }
        }, |field, lhs, rhs| {
            validate_lhs_type_as_text_compatible(lhs, "===");
            let field = field.expect("The left hand side of the `===(field, TEXT)` operator must be a field.");
            let expr_type = get_expr_result_type(rhs);
            let is_text = expr_type == pg_sys::TEXTOID || expr_type == pg_sys::VARCHAROID;
            let is_array = expr_type == pg_sys::TEXTARRAYOID || expr_type == pg_sys::VARCHARARRAYOID;
            let is_pdb_query = expr_type == pdb_query_typoid();
            let is_boost = expr_type == boost_typoid();
            let is_fuzzy = expr_type == fuzzy_typoid();
            assert!(
                is_text || is_array || is_pdb_query || is_boost || is_fuzzy,
                "The right-hand side of the `===` operator must be text, text[], or a pdb.* value"
            );

            // Cast pdb.boost/pdb.fuzzy to pdb.query before calling term
            let rhs = if is_boost {
                coerce_to_pdb_query(rhs, c"paradedb.boost_to_query(pdb.boost)")
            } else if is_fuzzy {
                coerce_to_pdb_query(rhs, c"paradedb.fuzzy_to_query(pdb.fuzzy)")
            } else {
                rhs
            };

            let mut args = PgList::<pg_sys::Node>::new();
            args.push(field.into_const().cast());
            args.push(rhs.cast());

            let funcid = if is_array {
                direct_function_call::<pg_sys::Oid>(
                    pg_sys::regprocedurein,
                    &[c"paradedb.term_set(paradedb.fieldname, text[])".into_datum()],
                )
                .expect("`paradedb.term_set(paradedb.fieldname, text[])` should exist")
            } else if is_text {
                direct_function_call::<pg_sys::Oid>(
                    pg_sys::regprocedurein,
                    &[c"paradedb.term(paradedb.fieldname, text)".into_datum()],
                )
                .expect("`paradedb.term(paradedb.fieldname, text)` should exist")
            } else {
                // pdb.query, pdb.boost (cast to query), pdb.fuzzy (cast to query)
                direct_function_call::<pg_sys::Oid>(
                    pg_sys::regprocedurein,
                    &[c"paradedb.term(paradedb.fieldname, pdb.query)".into_datum()],
                )
                .expect("`paradedb.term(paradedb.fieldname, pdb.query)` should exist")
            };

            pg_sys::FuncExpr {
                xpr: pg_sys::Expr { type_: pg_sys::NodeTag::T_FuncExpr },
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
        })
            .unwrap_or(ReturnedNodePointer(None))
    }
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

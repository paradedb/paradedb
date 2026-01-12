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
use crate::api::operator::boost::BoostType;
use crate::api::operator::slop::SlopType;
use crate::api::operator::{
    boost_typoid, get_expr_result_type, pdb_query_typoid, request_simplify,
    searchqueryinput_typoid, slop_typoid, validate_lhs_type_as_text_compatible, RHSValue,
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

#[pg_extern(immutable, parallel_safe, name = "phrase")]
fn phrase_query(field: FieldName, query: pdb::Query) -> SearchQueryInput {
    match query {
        pdb::Query::ScoreAdjusted { query, score } => {
            let mut query = *query;
            if let pdb::Query::UnclassifiedString {
                string, slop_data, ..
            } = query
            {
                query = phrase_string(string);
                query.apply_slop_data(slop_data);
            } else if let pdb::Query::UnclassifiedArray {
                array, slop_data, ..
            } = query
            {
                query = phrase_array(array);
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
            string, slop_data, ..
        } => {
            let mut query = phrase_string(string);
            query.apply_slop_data(slop_data);
            to_search_query_input(field, query)
        }
        pdb::Query::UnclassifiedArray {
            array, slop_data, ..
        } => {
            let mut query = phrase_array(array);
            query.apply_slop_data(slop_data);
            to_search_query_input(field, query)
        }
        _ => panic!(
            "The right-hand side of the `###` operator must be text, text[], or an unclassified pdb.* value."
        ),
    }
}

#[pg_extern(immutable, parallel_safe, name = "phrase")]
fn phrase_boost(field: FieldName, query: BoostType) -> SearchQueryInput {
    phrase_query(field, query.into())
}

#[pg_extern(immutable, parallel_safe, name = "phrase")]
fn phrase_slop(field: FieldName, query: SlopType) -> SearchQueryInput {
    phrase_query(field, query.into())
}

#[pg_extern(immutable, parallel_safe)]
fn search_with_phrase_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        request_simplify(arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>(), |lhs, field, to_tokenize| {
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
                RHSValue::PdbQuery(query) => phrase_query(field, query),
                _ => panic!(
                    "The right-hand side of the `###` operator must be text, text[], or an unclassified pdb.* value."
                ),
            }
        }, |field, lhs, rhs| {
            validate_lhs_type_as_text_compatible(lhs, "###");
            let field = field.expect("The left hand side of the `###(field, TEXT)` operator must be a field.");
            let expr_type = get_expr_result_type(rhs);
            let is_text = expr_type == pg_sys::TEXTOID || expr_type == pg_sys::VARCHAROID;
            let is_array = expr_type == pg_sys::TEXTARRAYOID || expr_type == pg_sys::VARCHARARRAYOID;
            let is_pdb_query = expr_type == pdb_query_typoid();
            let is_boost = expr_type == boost_typoid();
            let is_slop = expr_type == slop_typoid();
            assert!(
                is_text || is_array || is_pdb_query || is_boost || is_slop,
                "The right-hand side of the `###` operator must be text, text[], or a pdb.* value"
            );

            let mut args = PgList::<pg_sys::Node>::new();
            args.push(field.into_const().cast());
            args.push(rhs.cast());

            pg_sys::FuncExpr {
                xpr: pg_sys::Expr { type_: pg_sys::NodeTag::T_FuncExpr },
                funcid: if is_array {
                    direct_function_call::<pg_sys::Oid>(
                        pg_sys::regprocedurein,
                        &[c"paradedb.phrase_array(paradedb.fieldname, text[])".into_datum()],
                    )
                    .expect("`paradedb.phrase_array(paradedb.fieldname, text[])` should exist")
                } else if is_text {
                    direct_function_call::<pg_sys::Oid>(
                        pg_sys::regprocedurein,
                        &[c"paradedb.phrase(paradedb.fieldname, text)".into_datum()],
                    )
                    .expect("`paradedb.phrase(paradedb.fieldname, text)` should exist")
                } else if is_pdb_query {
                    direct_function_call::<pg_sys::Oid>(
                        pg_sys::regprocedurein,
                        &[c"paradedb.phrase(paradedb.fieldname, pdb.query)".into_datum()],
                    )
                    .expect("`paradedb.phrase(paradedb.fieldname, pdb.query)` should exist")
                } else if is_boost {
                    direct_function_call::<pg_sys::Oid>(
                        pg_sys::regprocedurein,
                        &[c"paradedb.phrase(paradedb.fieldname, pdb.boost)".into_datum()],
                    )
                    .expect("`paradedb.phrase(paradedb.fieldname, pdb.boost)` should exist")
                } else {
                    direct_function_call::<pg_sys::Oid>(
                        pg_sys::regprocedurein,
                        &[c"paradedb.phrase(paradedb.fieldname, pdb.slop)".into_datum()],
                    )
                    .expect("`paradedb.phrase(paradedb.fieldname, pdb.slop)` should exist")
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

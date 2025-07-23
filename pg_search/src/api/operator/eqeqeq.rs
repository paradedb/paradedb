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
    get_expr_result_type, request_simplify, searchqueryinput_typoid, RHSValue, ReturnedNodePointer,
};
use crate::api::FieldName;
use crate::query::fielded_query::FieldedQueryInput;
use crate::query::{SearchQueryInput, TermInput};
use pgrx::{
    direct_function_call, extension_sql, opname, pg_extern, pg_operator, pg_sys, Internal,
    IntoDatum, PgList,
};

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.===)]
fn search_with_term(_field: &str, term: &str) -> bool {
    panic!("query is incompatible with pg_search's `===(field, TEXT)` operator: `{term}`")
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.===)]
fn search_with_term_array(_field: &str, terms: Vec<String>) -> bool {
    panic!("query is incompatible with pg_search's `===(field, TEXT)` operator: `{terms:?}`")
}

#[pg_extern(immutable, parallel_safe)]
fn string_term(field: FieldName, term: String) -> SearchQueryInput {
    SearchQueryInput::FieldedQuery {
        field,
        query: FieldedQueryInput::Term {
            value: term.into(),
            is_datetime: false,
        },
    }
}

#[pg_extern(immutable, parallel_safe)]
fn string_term_array(field: FieldName, terms: Vec<String>) -> SearchQueryInput {
    SearchQueryInput::TermSet {
        terms: terms
            .into_iter()
            .map(|term| TermInput {
                field: field.clone(),
                value: term.into(),
                is_datetime: false,
            })
            .collect(),
    }
}

#[pg_extern(immutable, parallel_safe)]
fn search_with_term_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        request_simplify(arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>(), |field, term| {
            let field = field
                .expect("The left hand side of the `===(field, TEXT)` operator must be a field.");

            match term {
                RHSValue::Text(term) => string_term(field, term),
                RHSValue::TextArray(terms) => string_term_array(field, terms),
                _ => unreachable!("The right-hand side of the `===(field, TEXT)` operator must be a text or text array value")
            }
        }, |field, rhs| {
            let field = field.expect("The left hand side of the `===(field, TEXT)` operator must be a field.");
            let expr_type = get_expr_result_type(rhs);
            assert!({
                expr_type  == pg_sys::TEXTOID || expr_type == pg_sys::VARCHAROID || expr_type == pg_sys::TEXTARRAYOID || expr_type == pg_sys::VARCHARARRAYOID
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
                        &[c"paradedb.string_term_array(paradedb.fieldname, text)".into_datum()],
                    )
                        .expect("`paradedb.string_term_array(paradedb.fieldname, text)` should exist")
                } else {
                    direct_function_call::<pg_sys::Oid>(
                        pg_sys::regprocedurein,
                        &[c"paradedb.string_term(paradedb.fieldname, text)".into_datum()],
                    )
                        .expect("`paradedb.string_term(paradedb.fieldname, text)` should exist")
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
        ALTER FUNCTION paradedb.search_with_term SUPPORT paradedb.search_with_term_support;
        ALTER FUNCTION paradedb.search_with_term_array SUPPORT paradedb.search_with_term_support;
    "#,
    name = "search_with_term_support_fn",
    requires = [
        search_with_term,
        search_with_term_array,
        search_with_term_support
    ]
);

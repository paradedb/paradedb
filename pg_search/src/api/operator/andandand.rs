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
use crate::api::pdb_builder_fns::match_conjunction;
use crate::query::pdb_query::to_search_query_input;
use pgrx::{
    direct_function_call, extension_sql, opname, pg_extern, pg_operator, pg_sys, Internal,
    IntoDatum, PgList,
};

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.&&&)]
fn search_with_match_conjunction(_field: &str, terms_to_tokenize: &str) -> bool {
    panic!(
        "query is incompatible with pg_search's `&&&(field, TEXT)` operator: `{terms_to_tokenize}`"
    )
}

#[pg_extern(immutable, parallel_safe)]
fn search_with_match_conjunction_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        request_simplify(arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>(), |field, to_tokenize| {
            let field = field.expect("The left hand side of the `&&&(field, TEXT)` operator must be a field.");
            let query = match to_tokenize {
                RHSValue::Text(text) => text,
                _ => panic!("The right-hand side of the `&&&(field, TEXT)` operator must be a text value."),
            };

            to_search_query_input(field, match_conjunction(query))
        }, |field, rhs| {
            let field = field.expect("The left hand side of the `|||(field, TEXT)` operator must be a field.");
            assert!(get_expr_result_type(rhs) == pg_sys::TEXTOID, "The right-hand side of the `|||(field, TEXT)` operator must be a text value");
            let mut args = PgList::<pg_sys::Node>::new();

            args.push(field.into_const().cast());
            args.push(rhs.cast());

            pg_sys::FuncExpr {
                xpr: pg_sys::Expr { type_: pg_sys::NodeTag::T_FuncExpr },
                funcid: direct_function_call::<pg_sys::Oid>(
                    pg_sys::regprocedurein,
                    &[c"paradedb.match_conjunction(paradedb.fieldname, text)".into_datum()],
                )
                    .expect("`paradedb.match_conjunction(paradedb.fieldname, text)` should exist"),
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
    "ALTER FUNCTION paradedb.search_with_match_conjunction SUPPORT paradedb.search_with_match_conjunction_support;",
    name = "search_with_match_conjunction_support_fn",
    requires = [search_with_match_conjunction, search_with_match_conjunction_support]
);

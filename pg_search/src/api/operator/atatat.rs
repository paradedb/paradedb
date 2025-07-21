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
use crate::query::SearchQueryInput;
use pgrx::{
    direct_function_call, extension_sql, opname, pg_extern, pg_operator, pg_sys, AnyElement,
    Internal, IntoDatum, PgList,
};

/// This is the function behind the `@@@(anyelement, text)` operator. Since we transform those to
/// use `@@@(anyelement, searchqueryinput`), this function won't be called in normal circumstances, but it
/// could be called if the rhs of the @@@ is some kind of volatile value.
///
/// And in that case we just have to give up.
#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.@@@)]
pub fn search_with_parse(
    _element: AnyElement,
    query: &str,
    _fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    panic!("query is incompatible with pg_search's `@@@(field, TEXT)` operator: `{query}`")
}

#[pg_extern(immutable, parallel_safe)]
pub fn _search_with_parse(field: Option<FieldName>, query_string: String) -> SearchQueryInput {
    match field {
        Some(field) => SearchQueryInput::ParseWithField {
            field,
            query_string,
            lenient: None,
            conjunction_mode: None,
        },
        None => SearchQueryInput::Parse {
            query_string,
            lenient: None,
            conjunction_mode: None,
        },
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn search_with_parse_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        request_simplify(
            arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>(),
            |field, query_value| match query_value {
                RHSValue::Text(query_string) => _search_with_parse(field, query_string),
                _ => {
                    unreachable!(
                        "search_with_parse_support should only ever be called with a text value"
                    )
                }
            },
            |field, rhs| {
                let search_query_input_typoid = searchqueryinput_typoid();
                let expr_type = get_expr_result_type(rhs);

                assert!(
                    expr_type == pg_sys::TEXTOID || expr_type == pg_sys::VARCHAROID,
                    "The right hand side of the `@@@` operator must be a text value"
                );

                let mut args = PgList::<pg_sys::Node>::new();
                args.push(
                    field
                        .map(|field| field.into())
                        .unwrap_or_else(FieldName::null_const)
                        .cast(),
                );
                args.push(rhs.cast());

                pg_sys::FuncExpr {
                    xpr: pg_sys::Expr {
                        type_: pg_sys::NodeTag::T_FuncExpr,
                    },
                    funcid: direct_function_call::<pg_sys::Oid>(
                        pg_sys::regprocedurein,
                        &[c"paradedb._search_with_parse(paradedb.fieldname, text)".into_datum()],
                    )
                    .expect("`paradedb._search_with_parse(paradedb.fieldname, text)` should exist"),
                    funcresulttype: search_query_input_typoid,
                    funcretset: false,
                    funcvariadic: false,
                    funcformat: pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
                    funccollid: pg_sys::Oid::INVALID,
                    inputcollid: pg_sys::Oid::INVALID,
                    args: args.into_pg(),
                    location: -1,
                }
            },
        )
        .unwrap_or(ReturnedNodePointer(None))
    }
}

extension_sql!(
    "ALTER FUNCTION paradedb.search_with_parse SUPPORT paradedb.search_with_parse_support;",
    name = "search_with_parse_support_fn",
    requires = [search_with_parse, search_with_parse_support]
);

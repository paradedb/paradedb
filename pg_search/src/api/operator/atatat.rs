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

use crate::api::builder_fns::{parse, parse_with_field, proximity};
use crate::api::operator::{
    get_expr_result_type, request_simplify, searchqueryinput_typoid, RHSValue, ReturnedNodePointer,
};
use crate::query::pdb_query::{pdb, to_search_query_input};
use crate::query::proximity::ProximityClause;
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
pub fn search_with_parse(_element: AnyElement, query: &str) -> bool {
    panic!("query is incompatible with pg_search's `@@@(field, TEXT)` operator: `{query}`")
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.@@@)]
pub fn search_with_fieled_query_input(_element: AnyElement, query: pdb::Query) -> bool {
    panic!("query is incompatible with pg_search's `@@@(field, pdb.query)` operator: `{query:?}`")
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.@@@)]
pub fn search_with_proximity_clause(_element: AnyElement, query: ProximityClause) -> bool {
    panic!("query is incompatible with pg_search's `@@@(field, pdb.ProximityClause)` operator: `{query:?}`")
}

#[pg_extern(immutable, parallel_safe)]
pub fn atatat_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        request_simplify(
            arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>(),
            |field, query_value| match query_value {
                RHSValue::Text(query_string) => match field {
                    Some(field) => to_search_query_input(field, parse_with_field(query_string, None, None)),
                    None => parse(query_string, None, None),
                }
                RHSValue::PdbQuery(pdb::Query::UnclassifiedString {string, fuzzy_data}) => {
                    assert!(field.is_some());
                    let mut query = parse_with_field(string, None, None);
                    query.apply_fuzzy_data(fuzzy_data);
                    to_search_query_input(field.unwrap(), query)
                }
                RHSValue::PdbQuery(pdb::Query::Boost { query, boost}) => {
                    assert!(field.is_some());
                    let mut query = *query;
                    if let pdb::Query::UnclassifiedString {string, fuzzy_data} = query {
                        query = parse_with_field(string, None, None);
                        query.apply_fuzzy_data(fuzzy_data);
                    }
                    to_search_query_input(field.unwrap(), pdb::Query::Boost { query: Box::new(query), boost})
                }
                RHSValue::PdbQuery(query) => {
                    assert!(field.is_some());
                    to_search_query_input(field.unwrap(), query)
                }
                RHSValue::ProximityClause(prox) => {
                    assert!(field.is_some());
                    to_search_query_input(field.unwrap(), proximity(prox))
                }
                _ => {
                    unreachable!(
                        "atatat_support should only ever be called with a text value"
                    )
                }
            },
            |field, rhs| {
                let search_query_input_typoid = searchqueryinput_typoid();
                let expr_type = get_expr_result_type(rhs);

                assert!(
                    expr_type == pg_sys::TEXTOID || expr_type == pg_sys::VARCHAROID,
                    "The right-hand side of the `@@@` operator must be a text value"
                );

                match field {
                    // here we call the `paradedb.parse_with_field` function
                    Some(field) => {
                        let mut args = PgList::<pg_sys::Node>::new();
                        args.push(field.into_const().cast());
                        args.push(rhs.cast());
                        args.push(pg_sys::makeBoolConst(false, true));
                        args.push(pg_sys::makeBoolConst(false, true));

                        pg_sys::FuncExpr {
                            xpr: pg_sys::Expr {
                                type_: pg_sys::NodeTag::T_FuncExpr,
                            },
                            funcid: direct_function_call::<pg_sys::Oid>(
                                pg_sys::regprocedurein,
                                &[c"paradedb.parse_with_field(paradedb.fieldname, text, bool, bool)".into_datum()],
                            )
                                .expect("`paradedb.parse_with_field(paradedb.fieldname, text, bool, bool)` should exist"),
                            funcresulttype: search_query_input_typoid,
                            funcretset: false,
                            funcvariadic: false,
                            funcformat: pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
                            funccollid: pg_sys::Oid::INVALID,
                            inputcollid: pg_sys::Oid::INVALID,
                            args: args.into_pg(),
                            location: -1,
                        }
                    }

                    // here we call the `paradedb.parse` function without a FieldName
                    None => {
                        let mut args = PgList::<pg_sys::Node>::new();
                        args.push(rhs.cast());
                        args.push(pg_sys::makeBoolConst(false, true));
                        args.push(pg_sys::makeBoolConst(false, true));

                        pg_sys::FuncExpr {
                            xpr: pg_sys::Expr {
                                type_: pg_sys::NodeTag::T_FuncExpr,
                            },
                            funcid: direct_function_call::<pg_sys::Oid>(
                                pg_sys::regprocedurein,
                                &[c"paradedb.parse(text, bool, bool)".into_datum()],
                            )
                                .expect("`paradedb.parse(text, bool, bool)` should exist"),
                            funcresulttype: search_query_input_typoid,
                            funcretset: false,
                            funcvariadic: false,
                            funcformat: pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
                            funccollid: pg_sys::Oid::INVALID,
                            inputcollid: pg_sys::Oid::INVALID,
                            args: args.into_pg(),
                            location: -1,
                        }
                    }
                }
            },
        )
            .unwrap_or(ReturnedNodePointer(None))
    }
}

extension_sql!(
    r#"
        ALTER FUNCTION paradedb.search_with_parse SUPPORT paradedb.atatat_support;
        ALTER FUNCTION paradedb.search_with_fieled_query_input SUPPORT paradedb.atatat_support;
        ALTER FUNCTION paradedb.search_with_proximity_clause SUPPORT paradedb.atatat_support;
    "#,
    name = "atatat_support_fn",
    requires = [
        search_with_parse,
        search_with_fieled_query_input,
        search_with_proximity_clause,
        atatat_support
    ]
);

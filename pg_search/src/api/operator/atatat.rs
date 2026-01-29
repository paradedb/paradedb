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

use crate::api::builder_fns::{parse, parse_with_field, proximity};
use crate::api::operator::{
    boost_typoid, coerce_to_pdb_query, fuzzy_typoid, get_expr_result_type,
    pdb_proximityclause_typoid, pdb_query_typoid, request_simplify, searchqueryinput_typoid,
    RHSValue, ReturnedNodePointer,
};
use crate::api::FieldName;
use crate::query::pdb_query::{pdb, to_search_query_input};
use crate::query::proximity::ProximityClause;
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

/// Converts a pdb::Query to SearchQueryInput for the @@@ operator.
/// Handles UnclassifiedString by converting it to ParseWithField at runtime.
#[pg_extern(immutable, parallel_safe, name = "parse_with_field_query")]
fn parse_with_field_query(field: FieldName, query: pdb::Query) -> SearchQueryInput {
    match query {
        pdb::Query::UnclassifiedString {
            string,
            fuzzy_data,
            slop_data,
        } => {
            let mut query = parse_with_field(string, None, None);
            query.apply_fuzzy_data(fuzzy_data);
            query.apply_slop_data(slop_data);
            to_search_query_input(field, query)
        }
        pdb::Query::ScoreAdjusted { query, score } => {
            let mut inner = *query;
            if let pdb::Query::UnclassifiedString {
                string,
                fuzzy_data,
                slop_data,
            } = inner
            {
                inner = parse_with_field(string, None, None);
                inner.apply_fuzzy_data(fuzzy_data);
                inner.apply_slop_data(slop_data);
            }
            to_search_query_input(
                field,
                pdb::Query::ScoreAdjusted {
                    query: Box::new(inner),
                    score,
                },
            )
        }
        other => to_search_query_input(field, other),
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn atatat_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        request_simplify(
            arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>(),
            |_, field, query_value| match query_value {
                RHSValue::Text(query_string) => match field {
                    Some(field) => to_search_query_input(field, parse_with_field(query_string, None, None)),
                    None => parse(query_string, None, None),
                }
                RHSValue::PdbQuery(pdb::Query::UnclassifiedString { string, fuzzy_data, slop_data }) => {
                    assert!(field.is_some());
                    let mut query = parse_with_field(string, None, None);
                    query.apply_fuzzy_data(fuzzy_data);
                    query.apply_slop_data(slop_data);
                    to_search_query_input(field.unwrap(), query)
                }
                RHSValue::PdbQuery(pdb::Query::ScoreAdjusted { query, score }) => {
                    assert!(field.is_some());
                    let mut query = *query;
                    if let pdb::Query::UnclassifiedString { string, fuzzy_data, slop_data } = query {
                        query = parse_with_field(string, None, None);
                        query.apply_fuzzy_data(fuzzy_data);
                        query.apply_slop_data(slop_data);
                    }
                    to_search_query_input(field.unwrap(), pdb::Query::ScoreAdjusted { query: Box::new(query), score })
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
                        "atatat_support should only ever be called with text, pdb.query, or pdb.ProximityClause values"
                    )
                }
            },
            |field, _, rhs| {
                let search_query_input_typoid = searchqueryinput_typoid();
                let pdb_query_typoid = pdb_query_typoid();
                let expr_type = get_expr_result_type(rhs);
                let is_text = expr_type == pg_sys::TEXTOID || expr_type == pg_sys::VARCHAROID;
                let is_pdb_query = expr_type == pdb_query_typoid;
                let is_boost = expr_type == boost_typoid();
                let is_fuzzy = expr_type == fuzzy_typoid();
                let is_prox = expr_type == pdb_proximityclause_typoid();

                assert!(
                    is_text || is_pdb_query || is_boost || is_fuzzy || is_prox,
                    "The right-hand side of the `@@@` operator must be text, pdb.query, pdb.boost, pdb.fuzzy, or pdb.ProximityClause"
                );

                // Cast pdb.boost/pdb.fuzzy to pdb.query before calling parse_with_field_query
                let rhs = if is_boost {
                    coerce_to_pdb_query(rhs, c"paradedb.boost_to_query(pdb.boost)")
                } else if is_fuzzy {
                    coerce_to_pdb_query(rhs, c"paradedb.fuzzy_to_query(pdb.fuzzy)")
                } else {
                    rhs
                };

                let funcid = if is_pdb_query || is_boost || is_fuzzy {
                    direct_function_call::<pg_sys::Oid>(
                        pg_sys::regprocedurein,
                        &[c"paradedb.parse_with_field_query(paradedb.fieldname, pdb.query)".into_datum()],
                    )
                    .expect("`paradedb.parse_with_field_query(paradedb.fieldname, pdb.query)` should exist")
                } else if is_prox {
                    direct_function_call::<pg_sys::Oid>(
                        pg_sys::regprocedurein,
                        &[c"paradedb.proximity(paradedb.fieldname, pdb.proximityclause)".into_datum()],
                    )
                    .expect("`paradedb.proximity(paradedb.fieldname, pdb.proximityclause)` should exist")
                } else if field.is_some() {
                    direct_function_call::<pg_sys::Oid>(
                        pg_sys::regprocedurein,
                        &[c"paradedb.parse_with_field(paradedb.fieldname, text, bool, bool)".into_datum()],
                    )
                    .expect("`paradedb.parse_with_field(paradedb.fieldname, text, bool, bool)` should exist")
                } else {
                    direct_function_call::<pg_sys::Oid>(
                        pg_sys::regprocedurein,
                        &[c"paradedb.parse(text, bool, bool)".into_datum()],
                    )
                    .expect("`paradedb.parse(text, bool, bool)` should exist")
                };

                match field {
                    // here we call the `paradedb.parse_with_field` function
                    Some(field) => {
                        let mut args = PgList::<pg_sys::Node>::new();
                        args.push(field.into_const().cast());
                        args.push(rhs.cast());

                        if is_text {
                            args.push(pg_sys::makeBoolConst(false, true));
                            args.push(pg_sys::makeBoolConst(false, true));
                        }

                        pg_sys::FuncExpr {
                            xpr: pg_sys::Expr {
                                type_: pg_sys::NodeTag::T_FuncExpr,
                            },
                            funcid,
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
                        assert!(is_text);

                        let mut args = PgList::<pg_sys::Node>::new();
                        args.push(rhs.cast());
                        args.push(pg_sys::makeBoolConst(false, true));
                        args.push(pg_sys::makeBoolConst(false, true));

                        pg_sys::FuncExpr {
                            xpr: pg_sys::Expr {
                                type_: pg_sys::NodeTag::T_FuncExpr,
                            },
                            funcid,
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

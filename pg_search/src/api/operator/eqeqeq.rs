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
    build_text_funcexpr, request_simplify, validate_lhs_type_as_text_compatible, RHSValue,
    ReturnedNodePointer,
};
use crate::query::pdb_query::{pdb, to_search_query_input};
use pgrx::{extension_sql, opname, pg_extern, pg_operator, pg_sys, AnyElement, Internal};

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
        request_simplify(arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>(), |lhs, field, term| {
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
            build_text_funcexpr(
                field, rhs, "===",
                c"paradedb.term(paradedb.fieldname, text)",
                c"paradedb.term_set(paradedb.fieldname, text[])",
            )
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

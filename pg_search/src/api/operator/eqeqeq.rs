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
use crate::api::FieldName;
use crate::api::operator::ReturnedNodePointer;
use crate::api::operator::SearchOperator;
use crate::api::operator::boost::BoostType;
use crate::api::operator::fuzzy::FuzzyType;
use crate::query::SearchQueryInput;
use crate::query::pdb_query::{pdb, to_search_query_input};
use pgrx::{AnyElement, extension_sql, opname, pg_extern, pg_operator};

/// Runtime classification for `===` expressions that cannot be folded during planning.
#[pg_extern(immutable, parallel_safe)]
pub fn term_search_query_input(field: FieldName, query: pdb::Query) -> SearchQueryInput {
    to_search_query_input(field, SearchOperator::Term.classify_query(query))
}

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

operator_support!(fn search_with_term_support, Term);

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

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

//! The phrase operator `###`.
//!
//! The `#[opname]` attributes below spell it `# # #`. Edition 2024 reserves `##` as a token
//! sequence, so the hashes have to be separated in the macro input; pgrx strips the whitespace out
//! of the `opname` token stream, so the operator Postgres sees is still `###`.

use crate::api::operator::ReturnedNodePointer;
use crate::api::operator::boost::BoostType;
use crate::api::operator::slop::SlopType;
use crate::query::pdb_query::pdb;
use pgrx::{AnyElement, extension_sql, opname, pg_operator};

// The `# # #` spelling below is the SQL operator `###`. Edition 2024 reserves `##` as a token
// sequence, so the hashes have to be separated in the macro input; pgrx strips the whitespace out
// of the `opname` token stream, so the operator Postgres sees is still `###`.

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.# # #)]
fn search_with_phrase(_field: AnyElement, terms_to_tokenize: &str) -> bool {
    panic!(
        "query is incompatible with pg_search's `###(field, TEXT)` operator: `{terms_to_tokenize}`"
    )
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.# # #)]
fn search_with_phrase_array(_field: AnyElement, tokens: Vec<String>) -> bool {
    panic!("query is incompatible with pg_search's `###(field, TEXT[])` operator: `{tokens:?}`")
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.# # #)]
fn search_with_phrase_pdb_query(_field: AnyElement, terms_to_tokenize: pdb::Query) -> bool {
    panic!(
        "query is incompatible with pg_search's `###(field, pdb.query)` operator: `{terms_to_tokenize:?}`"
    )
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.# # #)]
fn search_with_phrase_boost(_field: AnyElement, terms_to_tokenize: BoostType) -> bool {
    panic!(
        "query is incompatible with pg_search's `###(field, boost)` operator: `{terms_to_tokenize:?}`"
    )
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000, requires = ["SlopType_final"])]
#[opname(pg_catalog.# # #)]
fn search_with_phrase_slop(_field: AnyElement, terms_to_tokenize: SlopType) -> bool {
    panic!(
        "query is incompatible with pg_search's `###(field, slop)` operator: `{terms_to_tokenize:?}`"
    )
}

operator_support!(fn search_with_phrase_support, Phrase);

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

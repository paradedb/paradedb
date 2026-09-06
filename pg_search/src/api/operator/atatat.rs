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

use crate::api::operator::ReturnedNodePointer;
use crate::query::pdb_query::pdb;
use crate::query::proximity::ProximityClause;
use pgrx::{AnyElement, extension_sql, opname, pg_operator};

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
pub fn search_with_field_query_input(_element: AnyElement, query: pdb::Query) -> bool {
    panic!("query is incompatible with pg_search's `@@@(field, pdb.query)` operator: `{query:?}`")
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.@@@)]
pub fn search_with_proximity_clause(_element: AnyElement, query: ProximityClause) -> bool {
    panic!(
        "query is incompatible with pg_search's `@@@(field, pdb.ProximityClause)` operator: `{query:?}`"
    )
}

operator_support!(pub fn atatat_support, Parse);

extension_sql!(
    r#"
        ALTER FUNCTION paradedb.search_with_parse SUPPORT paradedb.atatat_support;
        ALTER FUNCTION paradedb.search_with_field_query_input SUPPORT paradedb.atatat_support;
        ALTER FUNCTION paradedb.search_with_proximity_clause SUPPORT paradedb.atatat_support;
    "#,
    name = "atatat_support_fn",
    requires = [
        search_with_parse,
        search_with_field_query_input,
        search_with_proximity_clause,
        atatat_support
    ]
);

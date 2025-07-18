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

use crate::api::operator::ReturnedNodePointer;
use crate::query::SearchQueryInput;
use pgrx::{extension_sql, opname, pg_extern, pg_operator, pg_sys, AnyElement, Internal};

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
    panic!("query is incompatible with pg_search's `@@@(key_field, TEXT)` operator: `{query}`")
}

#[pg_extern(immutable, parallel_safe)]
pub fn search_with_parse_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        super::request_simplify(arg, |field, query_string| match field {
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
        })
        .unwrap_or(ReturnedNodePointer(None))
    }
}

extension_sql!(
    "ALTER FUNCTION paradedb.search_with_parse SUPPORT paradedb.search_with_parse_support;",
    name = "search_with_parse_support_fn",
    requires = [search_with_parse, search_with_parse_support]
);

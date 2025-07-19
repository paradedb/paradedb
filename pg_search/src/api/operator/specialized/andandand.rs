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
use crate::api::operator::specialized::RHSValue;
use crate::api::operator::ReturnedNodePointer;
use crate::query::SearchQueryInput;
use pgrx::{extension_sql, opname, pg_extern, pg_operator, Internal};

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
        super::request_simplify(arg, |field, to_tokenize| SearchQueryInput::Match {
            field: field
                .expect("The left hand side of the `&&&(field, TEXT)` operator must be a field."),
            value: match to_tokenize {
                RHSValue::Text(to_tokenize) => to_tokenize,
                _ => unreachable!(
                    "The right hand side of the `&&&(field, TEXT)` operator must be a text value"
                ),
            },
            conjunction_mode: Some(true),
            tokenizer: None,
            distance: None,
            transposition_cost_one: None,
            prefix: None,
        })
        .unwrap_or(ReturnedNodePointer(None))
    }
}

extension_sql!(
    "ALTER FUNCTION paradedb.search_with_match_conjunction SUPPORT paradedb.search_with_match_conjunction_support;",
    name = "search_with_match_conjunction_support_fn",
    requires = [search_with_match_conjunction, search_with_match_conjunction_support]
);

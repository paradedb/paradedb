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
use crate::query::{SearchQueryInput, TermInput};
use pgrx::{extension_sql, opname, pg_extern, pg_operator, Internal};

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.===)]
fn search_with_term(_field: &str, term: &str) -> bool {
    panic!("query is incompatible with pg_search's `===(field, TEXT)` operator: `{term}`")
}

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.===)]
fn search_with_term_array(_field: &str, terms: Vec<String>) -> bool {
    panic!("query is incompatible with pg_search's `===(field, TEXT)` operator: `{terms:?}`")
}

#[pg_extern(immutable, parallel_safe)]
fn search_with_term_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        super::request_simplify(arg, |field, term| {
            let field = field
                .expect("The left hand side of the `===(field, TEXT)` operator must be a field.");

            match term {
                RHSValue::Text(term) => SearchQueryInput::Term {
                    field: Some(field),
                    value: term.into(),
                    is_datetime: false,
                },

                RHSValue::TextArray(terms) => SearchQueryInput::TermSet {
                    terms: terms
                        .into_iter()
                        .map(|term| TermInput {
                            field: field.clone(),
                            value: term.into(),
                            is_datetime: false,
                        })
                        .collect(),
                },

                RHSValue::SearchQueryInput(_) => {
                    unreachable!(
                        "search_with_term_support should never be called with a text array"
                    )
                }
            }
        })
        .unwrap_or(ReturnedNodePointer(None))
    }
}

extension_sql!(
    r#"
        ALTER FUNCTION paradedb.search_with_term SUPPORT paradedb.search_with_term_support;
        ALTER FUNCTION paradedb.search_with_term_array SUPPORT paradedb.search_with_term_support;
    "#,
    name = "search_with_term_support_fn",
    requires = [
        search_with_term,
        search_with_term_array,
        search_with_term_support
    ]
);

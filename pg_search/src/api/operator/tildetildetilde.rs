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

//! The `~~~(vector, vector)` knn search operator: "this row is among the
//! top-`window_size` nearest neighbors of the query vector".
//!
//! Unlike pgvector's distance operators the metric is not encoded in the
//! operator: it comes from the vector column's opclass in the bm25 index.
//!
//! `~~~` only defines *candidacy* (which rows can match); ranking still comes
//! from `ORDER BY pdb.rrf(...)`, which is required for queries using this
//! operator. Predicate scoping follows boolean position: predicates ANDed
//! around the `OR` of a text matcher and a `~~~` matcher apply to both legs,
//! predicates grouped inside a branch apply to that leg only.

use crate::api::operator::{request_simplify, RHSValue, ReturnedNodePointer};
use crate::query::SearchQueryInput;
use pgrx::{extension_sql, opname, pg_extern, pg_operator, pg_sys, AnyElement, Internal};

#[pg_operator(immutable, parallel_safe, cost = 1000000000)]
#[opname(pg_catalog.~~~)]
fn search_with_knn(_field: AnyElement, _vector: AnyElement) -> bool {
    panic!("query is incompatible with pg_search's `~~~(vector, vector)` operator: the left-hand side must be a bm25-indexed vector column and the query must be executed by a ParadeDB scan ordered by `pdb.rrf(...)`")
}

#[pg_extern(immutable, parallel_safe)]
fn search_with_knn_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        request_simplify(
            arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>(),
            |_lhs, field, value| {
                let field = field.expect(
                    "the left-hand side of the `~~~(vector, vector)` operator must be a bm25-indexed vector column",
                );
                match value {
                    RHSValue::Vector(query_vector) => SearchQueryInput::Knn {
                        field,
                        query_vector,
                    },
                    _ => panic!(
                        "the right-hand side of the `~~~(vector, vector)` operator must be a vector value"
                    ),
                }
            },
            |_field, _lhs, _rhs| {
                panic!(
                    "the right-hand side of the `~~~(vector, vector)` operator must be a constant vector: parameters and runtime expressions are not yet supported"
                )
            },
        )
        .unwrap_or(ReturnedNodePointer(None))
    }
}

extension_sql!(
    r#"
        ALTER FUNCTION paradedb.search_with_knn SUPPORT paradedb.search_with_knn_support;
    "#,
    name = "search_with_knn_support_fn",
    requires = [search_with_knn, search_with_knn_support]
);

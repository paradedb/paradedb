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

use proc_macro::TokenStream;

mod builder_fn;
mod generate_tokenizer_sql;

/// A macro that transforms search-related functions into builder functions for ParadeDB's search functionality.
///
/// This macro is used in pg_search to generate builder functions that convert simple non-fielded search
/// functions into ones that return a `SearchQueryInput`, which requires a `FieldName`.
///
/// It must be used in conjunction with `#[pg_extern]`.
///
/// # Example
///
/// ```rust,no_run,compile_fail
/// #[builder_fn]
/// #[pg_extern(name = "foo_bar")]  // name= is required here
/// fn foo_bar(input: String) -> pdb::Query {
///     pdb::Query::FooBar { input }
/// }
/// ```
///
/// This will generate an additional function `match_query_bfn` that takes a field name as the first
/// parameter and converts the Query result into a SearchQueryInput for ParadeDB's search system.
///
/// That generated function will look like:
///
/// ```rust,no_run,compile_fail
/// #[pg_extern]
/// fn foo_bar(field: FieldName, input: String) -> SearchQueryInput {
///     to_search_query_input(field, foo_bar(input))
/// }
/// ```
///
/// From the SQL side of things, the programmer makes `foo_bar(input: String)` and it can be called
/// as
///
/// ```sql
/// SELECT * FROM t WHERE field @@@ pdb.foo_bar('hi mom');
/// ```
///
/// And this macro generates the fielded version, which allows pg_search (or the user) to write the
/// same query as:
///
/// ```sql
/// SELECT * FROM t WHERE key_field @@@ paradedb.foo_bar('field', 'hi mom');
/// ```
#[proc_macro_attribute]
pub fn builder_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    builder_fn::builder_fn(_attr, item)
}

/// Usage:
/// ```ignore
/// generate_tokenizer_type_sql!(
///     sql_name = "foo",
///     inname = "in_t",
///     outname = "out_t",
///     sendname = "send_t",
///     recvname = "recv_t",
///     preferred = true,
/// )
/// ```
///
/// Expands to the expression:
/// `("foo", "in_t", "out_t", "send_t", "recv_t", true)`
#[proc_macro]
pub fn generate_tokenizer_sql(input: TokenStream) -> TokenStream {
    generate_tokenizer_sql::generate_tokenizer_sql(input)
}

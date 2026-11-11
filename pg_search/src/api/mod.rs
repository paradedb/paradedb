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

// TODO: See https://github.com/pgcentralfoundation/pgrx/pull/2089
#![allow(for_loops_over_fallibles)]

#[cfg(any(test, feature = "pg_test"))]
pub use planner::api::vector_test_support;
pub use planner::api::{
    CTID_FIELD_NAME, FieldName, HashMap, HashSet, Regex, fieldname_typoid, version,
};

// SQL-facing `#[pg_extern]` entrypoints should generally live under `pg_search/src/api/`. If a
// `#[pg_extern]` or `#[derive(PostgresType)]` must be defined in a sub-crate instead, make sure it
// is referenced in `_SUBCRATE_PGRX_ANCHORS` in `pg_search/src/lib.rs` so the `cdylib` linker retains it.
mod admin;
pub mod aggregate;
pub mod builder_fns;
pub mod config;
pub mod operator;
mod synonyms;
pub mod tokenize;
pub mod tokenizers;
mod vector;
pub mod window_aggregate;

use pgrx::{extension_sql, pg_cast, pg_extern};

#[pg_cast(implicit)]
fn text_to_fieldname(field: String) -> FieldName {
    FieldName::from(field)
}

extension_sql!(
    r#"
    CREATE CAST (varchar AS paradedb.fieldname) WITH INOUT AS IMPLICIT;
    "#,
    name = "varchar_to_fieldname_cast",
    requires = [text_to_fieldname]
);

#[pg_extern(immutable, parallel_safe)]
pub fn to_search_query_input(
    field: FieldName,
    query: crate::query::pdb_query::pdb::Query,
) -> crate::query::SearchQueryInput {
    crate::query::pdb_query::to_search_query_input(field, query)
}

#[pg_extern(name = "to_search_query_input", immutable, parallel_safe)]
pub fn to_search_query_input_unfielded(
    query: crate::query::pdb_query::pdb::Query,
) -> crate::query::SearchQueryInput {
    crate::query::pdb_query::to_search_query_input_unfielded(query)
}

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

pub mod joingen;
pub mod pagegen;
pub mod wheregen;

use std::fmt::Debug;

use proptest::prelude::*;

use joingen::{JoinExpr, JoinType};
use wheregen::{Expr, SqlValue};

///
/// Generates arbitrary joins and where clauses for the given tables and columns.
///
pub fn arb_joins_and_wheres<V: Clone + Debug + Eq + SqlValue + 'static>(
    join_types: impl Strategy<Value = JoinType> + Clone,
    tables: Vec<impl AsRef<str>>,
    columns: Vec<(impl AsRef<str>, V)>,
) -> impl Strategy<Value = (JoinExpr, Expr<V>)> {
    let table_names = tables
        .into_iter()
        .map(|tn| tn.as_ref().to_string())
        .collect::<Vec<_>>();
    let columns = columns
        .into_iter()
        .map(|(cn, v)| (cn.as_ref().to_string(), v))
        .collect::<Vec<_>>();

    // Choose how many tables will be joined.
    (2..=table_names.len())
        .prop_flat_map(move |join_size| {
            // Then choose tables for that join size.
            proptest::sample::subsequence(table_names.clone(), join_size)
        })
        .prop_flat_map(move |tables| {
            // Finally, choose the joins and where clauses for those tables.
            (
                joingen::arb_joins(
                    join_types.clone(),
                    tables.clone(),
                    columns.iter().map(|(cn, _)| cn.clone()).collect(),
                ),
                wheregen::arb_wheres(tables.clone(), columns.clone()),
            )
        })
}

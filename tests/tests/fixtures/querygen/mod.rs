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
pub mod opexprgen;
pub mod pagegen;
pub mod wheregen;

use proptest::prelude::*;
use sqlx::PgConnection;
use std::fmt::Debug;

use joingen::{JoinExpr, JoinType};
use opexprgen::{ArrayQuantifier, Operator};
use wheregen::{Expr, SqlValue};

use crate::fixtures::db::Query;
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

/// Run the given pg and bm25 queries on the given connection, and compare their results across a
/// variety of configurations of the extension.
///
/// TODO: The configurations of the extension in the loop below could in theory also be proptested
/// properties: if performance becomes a concern, we should lift them out, and apply them using the
/// proptest properties instead.
pub fn compare<R, F>(
    pg_query: String,
    bm25_query: String,
    conn: &mut PgConnection,
    run_query: F,
) -> Result<(), TestCaseError>
where
    R: Eq + Debug,
    F: Fn(&str, &mut PgConnection) -> R,
{
    // the postgres query is always run with the paradedb custom scan turned off
    // this ensures we get the actual, known-to-be-correct result from Postgres'
    // plan, and not from ours where we did some kind of pushdown
    r#"
        SET max_parallel_workers TO 8;
        SET enable_seqscan TO ON;
        SET enable_indexscan TO ON;
        SET paradedb.enable_custom_scan TO OFF;
    "#
    .execute(conn);

    let pg_result = run_query(&pg_query, conn);

    // and for the "bm25" query, we run it a number of times with more and more scan types disabled,
    // always ensuring that paradedb's custom scan is turned on
    "SET paradedb.enable_custom_scan TO ON;".execute(conn);
    for scan_type in [
        "SET enable_seqscan TO OFF",
        "SET enable_indexscan TO OFF",
        "SET max_parallel_workers TO 0",
    ] {
        scan_type.execute(conn);

        let bm25_result = run_query(&bm25_query, conn);
        prop_assert_eq!(
            &pg_result,
            &bm25_result,
            "\nscan_type={}\npg:\n  {}\nbm25:\n  {}\nexplain:\n{}\n",
            scan_type,
            pg_query,
            bm25_query,
            format!("EXPLAIN ANALYZE {bm25_query}")
                .fetch::<(String,)>(conn)
                .into_iter()
                .map(|(s,)| s)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    Ok(())
}

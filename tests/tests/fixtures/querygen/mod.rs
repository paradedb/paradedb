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

pub mod groupbygen;
pub mod joingen;
pub mod opexprgen;
pub mod pagegen;
pub mod wheregen;

use std::fmt::Debug;

use futures::executor::block_on;
use proptest::prelude::*;
use proptest_derive::Arbitrary;
use sqlx::{Connection, PgConnection};

use crate::fixtures::db::Query;
use crate::fixtures::ConnExt;
use joingen::{JoinExpr, JoinType};
use opexprgen::{ArrayQuantifier, Operator};
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

#[derive(Copy, Clone, Debug, Arbitrary)]
pub struct PgGucs {
    aggregate_custom_scan: bool,
    custom_scan: bool,
    custom_scan_without_operator: bool,
    seqscan: bool,
    indexscan: bool,
    parallel_workers: bool,
}

impl PgGucs {
    fn set(&self) -> String {
        let PgGucs {
            aggregate_custom_scan,
            custom_scan,
            custom_scan_without_operator,
            seqscan,
            indexscan,
            parallel_workers,
        } = self;

        let max_parallel_workers = if *parallel_workers { 8 } else { 0 };

        format!(
            r#"
            SET paradedb.enable_aggregate_custom_scan TO {aggregate_custom_scan};
            SET paradedb.enable_custom_scan TO {custom_scan};
            SET paradedb.enable_custom_scan_without_operator TO {custom_scan_without_operator};
            SET enable_seqscan TO {seqscan};
            SET enable_indexscan TO {indexscan};
            SET max_parallel_workers TO {max_parallel_workers};
            "#
        )
    }
}

/// Run the given pg and bm25 queries on the given connection, and compare their results when run
/// with the given GUCs.
pub fn compare<R, F>(
    pg_query: String,
    bm25_query: String,
    gucs: PgGucs,
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
        SET paradedb.enable_aggregate_custom_scan TO OFF;
    "#
    .execute(conn);

    conn.deallocate_all()?;

    let pg_result = run_query(&pg_query, conn);

    // and for the "bm25" query, we run it with the given GUCs set.
    gucs.set().execute(conn);

    conn.deallocate_all()?;

    let bm25_result = run_query(&bm25_query, conn);

    prop_assert_eq!(
        &pg_result,
        &bm25_result,
        "\ngucs={:?}\npg:\n  {}\nbm25:\n  {}\nexplain:\n{}\n",
        gucs,
        pg_query,
        bm25_query,
        format!("EXPLAIN {bm25_query}")
            .fetch::<(String,)>(conn)
            .into_iter()
            .map(|(s,)| s)
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(())
}

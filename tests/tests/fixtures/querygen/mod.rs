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
use wheregen::Expr;

#[derive(Debug, Clone)]
pub struct BM25Options {
    /// "text_fields" or "numeric_fields"
    pub field_type: &'static str,
    /// The JSON config for this field, e.g. `{ "tokenizer": { "type": "keyword" } }`
    pub config_json: &'static str,
}

#[derive(Debug, Clone)]
pub struct Column {
    pub name: &'static str,
    pub sql_type: &'static str,
    pub sample_value: &'static str,
    pub is_primary_key: bool,
    pub is_groupable: bool,
    pub is_whereable: bool,
    pub is_indexed: bool,
    pub bm25_options: Option<BM25Options>,
    pub random_generator_sql: &'static str,
}

impl Column {
    pub const fn new(
        name: &'static str,
        sql_type: &'static str,
        sample_value: &'static str,
    ) -> Self {
        Self {
            name,
            sql_type,
            sample_value,
            is_primary_key: false,
            is_groupable: true,
            is_whereable: true,
            is_indexed: true,
            bm25_options: None,
            random_generator_sql: "NULL",
        }
    }

    pub const fn primary_key(mut self) -> Self {
        self.is_primary_key = true;
        self
    }

    pub const fn groupable(mut self, is_groupable: bool) -> Self {
        self.is_groupable = is_groupable;
        self
    }

    pub const fn whereable(mut self, is_whereable: bool) -> Self {
        self.is_whereable = is_whereable;
        self
    }

    pub const fn indexed(mut self, is_indexed: bool) -> Self {
        self.is_indexed = is_indexed;
        self
    }

    pub const fn bm25_text_field(mut self, config_json: &'static str) -> Self {
        self.bm25_options = Some(BM25Options {
            field_type: "text_fields",
            config_json,
        });
        self
    }

    pub const fn bm25_numeric_field(mut self, config_json: &'static str) -> Self {
        self.bm25_options = Some(BM25Options {
            field_type: "numeric_fields",
            config_json,
        });
        self
    }

    pub const fn random_generator_sql(mut self, random_generator_sql: &'static str) -> Self {
        self.random_generator_sql = random_generator_sql;
        self
    }
}

pub fn generated_queries_setup(
    conn: &mut PgConnection,
    tables: &[(&str, usize)],
    columns_def: &[Column],
) -> String {
    "CREATE EXTENSION pg_search;".execute(conn);
    "SET log_error_verbosity TO VERBOSE;".execute(conn);
    "SET log_min_duration_statement TO 1000;".execute(conn);

    let mut setup_sql = String::new();
    let column_definitions = columns_def
        .iter()
        .map(|col| {
            if col.is_primary_key {
                format!("{} {} NOT NULL PRIMARY KEY", col.name, col.sql_type)
            } else {
                format!("{} {}", col.name, col.sql_type)
            }
        })
        .collect::<Vec<_>>()
        .join(", \n");

    // For bm25 index
    let bm25_columns = columns_def
        .iter()
        .filter(|c| c.is_indexed)
        .map(|c| c.name)
        .collect::<Vec<_>>()
        .join(", ");
    let key_field = columns_def
        .iter()
        .find(|c| c.is_primary_key)
        .map(|c| c.name)
        .expect("At least one column must be a primary key");

    let text_fields = columns_def
        .iter()
        .filter(|c| c.is_indexed)
        .filter_map(|c| c.bm25_options.as_ref())
        .filter(|o| o.field_type == "text_fields")
        .map(|o| o.config_json)
        .collect::<Vec<_>>()
        .join(",\n");

    let numeric_fields = columns_def
        .iter()
        .filter(|c| c.is_indexed)
        .filter_map(|c| c.bm25_options.as_ref())
        .filter(|o| o.field_type == "numeric_fields")
        .map(|o| o.config_json)
        .collect::<Vec<_>>()
        .join(",\n");

    // For INSERT statements
    let insert_columns = columns_def
        .iter()
        .filter(|c| !c.is_primary_key)
        .map(|c| c.name)
        .collect::<Vec<_>>()
        .join(", ");

    let sample_values = columns_def
        .iter()
        .filter(|c| !c.is_primary_key)
        .map(|c| c.sample_value)
        .collect::<Vec<_>>()
        .join(", ");

    let random_generators = columns_def
        .iter()
        .filter(|c| !c.is_primary_key)
        .map(|c| c.random_generator_sql)
        .collect::<Vec<_>>()
        .join(",\n      ");

    for (tname, row_count) in tables {
        let sql = format!(
            r#"
CREATE TABLE {tname} (
    {column_definitions}
);
-- Note: Create the index before inserting rows to encourage multiple segments being created.
CREATE INDEX idx{tname} ON {tname} USING bm25 ({bm25_columns}) WITH (
    key_field = '{key_field}',
    text_fields = '{{ {text_fields} }}',
    numeric_fields = '{{ {numeric_fields} }}'
);

INSERT into {tname} ({insert_columns}) VALUES ({sample_values});

INSERT into {tname} ({insert_columns}) SELECT {random_generators} FROM generate_series(1, {row_count});

{b_tree_indexes}

ANALYZE;
"#,
            b_tree_indexes = columns_def
                .iter()
                .filter(|c| c.is_indexed)
                .map(|c| format!(
                    "CREATE INDEX idx{tname}_{name} ON {tname} ({name});",
                    name = c.name
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );

        (&sql).execute(conn);
        setup_sql.push_str(&sql);
    }

    setup_sql
}

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

impl Default for PgGucs {
    fn default() -> Self {
        Self {
            aggregate_custom_scan: false,
            custom_scan: false,
            custom_scan_without_operator: false,
            seqscan: true,
            indexscan: true,
            parallel_workers: true,
        }
    }
}

impl PgGucs {
    pub fn set(&self) -> String {
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
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    conn: &mut PgConnection,
    setup_sql: &str,
    run_query: F,
) -> Result<(), TestCaseError>
where
    R: Eq + Debug,
    F: Fn(&str, &mut PgConnection) -> R,
{
    match inner_compare(pg_query, bm25_query, gucs, conn, run_query) {
        Ok(()) => Ok(()),
        Err(e) => Err(handle_compare_error(
            e, pg_query, bm25_query, gucs, setup_sql,
        )),
    }
}

fn inner_compare<R, F>(
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
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
    PgGucs::default().set().execute(conn);

    conn.deallocate_all()?;

    let pg_result = run_query(pg_query, conn);

    // and for the "bm25" query, we run it with the given GUCs set.
    gucs.set().execute(conn);

    conn.deallocate_all()?;

    let bm25_result = run_query(bm25_query, conn);

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

/// Helper function to handle comparison errors and generate reproduction scripts
pub fn handle_compare_error(
    error: TestCaseError,
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    setup_sql: &str,
) -> TestCaseError {
    let error_msg = error.to_string();
    let failure_type = if error_msg.contains("error returned from database")
        || error_msg.contains("SQL execution error")
        || error_msg.contains("syntax error")
    {
        "QUERY EXECUTION FAILURE"
    } else {
        "RESULT MISMATCH"
    };

    let repro_script = format!(
        r#"
-- ==== {failure_type} REPRODUCTION SCRIPT ====
-- Copy and paste this entire block to reproduce the issue

-- Prerequisites: Ensure pg_search extension is available
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Table and index setup
{setup_sql}

-- Default GUCs:
{default_gucs}

-- PostgreSQL query:
{pg_query}

-- Set GUCs to match the failing test case
{gucs_sql}

-- BM25 query:
{bm25_query}

-- Original error:
-- {error_msg}

-- To debug further, you can also try:
SET paradedb.enable_aggregate_custom_scan = off;
{bm25_query}

-- ==== END REPRODUCTION SCRIPT ====
"#,
        failure_type = failure_type,
        setup_sql = setup_sql,
        default_gucs = PgGucs::default().set(),
        gucs_sql = gucs.set(),
        pg_query = pg_query,
        bm25_query = bm25_query,
        error_msg = error_msg
    );

    TestCaseError::fail(format!(
        "{}\n{repro_script}",
        if failure_type == "QUERY EXECUTION FAILURE" {
            "Query execution failed"
        } else {
            "Results differ between PostgreSQL and BM25"
        }
    ))
}

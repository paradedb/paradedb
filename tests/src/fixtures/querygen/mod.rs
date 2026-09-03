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

pub mod crossrelgen;
pub mod distinctgen;
pub mod groupbygen;
pub mod joingen;
pub mod numericgen;
pub mod opexprgen;
pub mod orderbygen;
pub mod pagegen;
pub mod pdbagggen;
pub mod wheregen;

use std::fmt::{Debug, Write};
use std::num::NonZeroUsize;
use std::sync::OnceLock;
use std::time::Instant;

use futures::executor::block_on;
use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use sqlx::{Connection, PgConnection};

use crate::fixtures::ConnExt;
use crate::fixtures::db::Query;
use crate::fixtures::fault_grace::{TransientKind, classify_transient};
use crossrelgen::CrossRelExpr;
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
    /// V2 syntax: expression to use in index column list, e.g. "(column::pdb.literal_normalized)"
    /// When set, this is used instead of bm25_options JSON config.
    pub index_expression: Option<&'static str>,
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
            index_expression: None,
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

    pub const fn bm25_json_field(mut self, config_json: &'static str) -> Self {
        self.bm25_options = Some(BM25Options {
            field_type: "json_fields",
            config_json,
        });
        self
    }

    /// Note: should use only the `random()` function to generate random data.
    pub const fn random_generator_sql(mut self, random_generator_sql: &'static str) -> Self {
        self.random_generator_sql = random_generator_sql;
        self
    }

    /// V2 syntax: set index expression, e.g. "(column::pdb.literal_normalized)"
    /// When set, this is used instead of bm25_options JSON config.
    pub const fn bm25_v2_expression(mut self, expression: &'static str) -> Self {
        self.index_expression = Some(expression);
        self
    }
}

/// Builds the schema every generator queries against and returns the SQL as a reproduction
/// script, retrying transient faults per [`crate::fixtures::fault_grace`] (under a plain
/// `cargo test` the first error panics). Each attempt runs in one transaction so a connection
/// killed midway rolls back cleanly and the retry starts from scratch (`CREATE TABLE` has no
/// `IF NOT EXISTS`). `BEGIN`/`COMMIT` are kept out of the returned script, which is replayed
/// statement by statement.
pub fn generated_queries_setup(
    pool: &MutexObjectPool<PgConnection>,
    tables: &[(&str, usize)],
    columns_def: &[Column],
) -> String {
    use crate::fixtures::fault_grace::{RetryError, sql_attempt};
    let attempt = |conn: &mut PgConnection| -> Result<String, sqlx::Error> {
        "BEGIN;".execute_result(conn)?;
        match generated_queries_setup_inner(conn, tables, columns_def) {
            Ok(setup_sql) => {
                "COMMIT;".execute_result(conn)?;
                Ok(setup_sql)
            }
            Err(err) => {
                // Best effort: if the connection is already gone the server rolls back on its own.
                let _ = "ROLLBACK;".execute_result(conn);
                Err(err)
            }
        }
    };
    match crate::fixtures::fault_grace::retry_transient(pool, "generated queries setup", |conn| {
        sql_attempt(attempt(conn))
    }) {
        Ok(Ok(setup_sql)) => setup_sql,
        Ok(Err(e)) => panic!("generated queries setup should succeed: {e:#?}"),
        Err(RetryError::TimedOutUnderPause(e)) => {
            panic!("generated queries setup timed out while faults were paused: {e}")
        }
        Err(RetryError::GraceExpired(reason)) => panic!("{reason}"),
    }
}

fn generated_queries_setup_inner(
    conn: &mut PgConnection,
    tables: &[(&str, usize)],
    columns_def: &[Column],
) -> Result<String, sqlx::Error> {
    "CREATE EXTENSION IF NOT EXISTS vector;".execute_result(conn)?;
    "CREATE EXTENSION IF NOT EXISTS pg_search;".execute_result(conn)?;
    "SET log_error_verbosity TO VERBOSE;".execute_result(conn)?;
    "SET log_min_duration_statement TO 1000;".execute_result(conn)?;

    let qgen_seed = qgen_seed().unwrap_or_else(|| rand::rng().random::<u64>());
    let mut rng = StdRng::seed_from_u64(qgen_seed);
    let pg_seed: f64 = rng.random_range(-1.0..=1.0);
    let bulk_inserts = pick_bulk_inserts(&mut rng);

    let seed_sql = format!("SET seed TO {pg_seed};\n");
    seed_sql.as_str().execute_result(conn)?;

    let mut setup_sql = seed_sql;
    setup_sql.push_str(&format!("-- PARADEDB_QGEN_SEED: {qgen_seed}\n"));
    setup_sql.push_str(&format!("-- qgen bulk inserts: {bulk_inserts}\n"));

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
    // Columns with index_expression use v2 syntax, others use just the name
    let bm25_columns = columns_def
        .iter()
        .filter(|c| c.is_indexed)
        .map(|c| {
            if let Some(expr) = c.index_expression {
                expr.to_string()
            } else {
                c.name.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let key_field = columns_def
        .iter()
        .find(|c| c.is_primary_key)
        .map(|c| c.name)
        .expect("At least one column must be a primary key");

    // Only include columns without index_expression in text_fields (v1 syntax)
    let text_fields = columns_def
        .iter()
        .filter(|c| c.is_indexed && c.index_expression.is_none())
        .filter_map(|c| c.bm25_options.as_ref())
        .filter(|o| o.field_type == "text_fields")
        .map(|o| o.config_json)
        .collect::<Vec<_>>()
        .join(",\n");

    // Only include columns without index_expression in numeric_fields (v1 syntax)
    let numeric_fields = columns_def
        .iter()
        .filter(|c| c.is_indexed && c.index_expression.is_none())
        .filter_map(|c| c.bm25_options.as_ref())
        .filter(|o| o.field_type == "numeric_fields")
        .map(|o| o.config_json)
        .collect::<Vec<_>>()
        .join(",\n");

    let json_fields = columns_def
        .iter()
        .filter(|c| c.is_indexed && c.index_expression.is_none())
        .filter_map(|c| c.bm25_options.as_ref())
        .filter(|o| o.field_type == "json_fields")
        .map(|o| o.config_json)
        .collect::<Vec<_>>()
        .join(",\n");

    // Find the first indexed numeric/date fast field for sort_by (Tantivy doesn't support Str).
    let sortable_types = [
        "INT",
        "BIGINT",
        "SMALLINT",
        "REAL",
        "FLOAT",
        "DOUBLE",
        "NUMERIC",
        "DATE",
        "TIMESTAMP",
    ];
    let sort_by_field = columns_def
        .iter()
        .filter(|c| c.is_indexed)
        .filter(|c| {
            sortable_types
                .iter()
                .any(|t| c.sql_type.to_uppercase().contains(t))
        })
        .filter_map(|c| {
            c.bm25_options
                .as_ref()
                .filter(|o| o.config_json.contains(r#""fast": true"#))
                .map(|_| c.name)
        })
        .next();

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
        // Build sort_by clause if we have a suitable field
        let sort_by_clause = sort_by_field
            .map(|field| format!(",\n    sort_by = '{field} DESC NULLS LAST'"))
            .unwrap_or_default();

        // Total commits per table = the sample-row `INSERT` + `bulk_inserts`
        // bulk chunks. Pin `target_segment_count` to that so the layered merge
        // policy short-circuits (it returns empty layer sizes when
        // `current_segments <= target`) and the chunks survive as distinct
        // segments instead of being merged back into one.
        let target_segments = bulk_inserts.get() + 1;
        let target_segment_clause = format!(",\n    target_segment_count = {target_segments}");

        let bulk_insert_sql = build_bulk_inserts(
            tname,
            *row_count,
            &insert_columns,
            &random_generators,
            bulk_inserts,
        );

        let sql = format!(
            r#"
CREATE TABLE {tname} (
    {column_definitions}
);
-- Note: Create the index before inserting rows to encourage multiple segments being created.
CREATE INDEX idx{tname} ON {tname} USING paradedb ({bm25_columns}) WITH (
    key_field = '{key_field}',
    text_fields = '{{ {text_fields} }}',
    numeric_fields = '{{ {numeric_fields} }}',
    json_fields = '{{ {json_fields} }}'{sort_by_clause}{target_segment_clause}
);

INSERT into {tname} ({insert_columns}) VALUES ({sample_values});

{bulk_insert_sql}

{b_tree_indexes}

ANALYZE {tname};
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

        (&sql).execute_result(conn)?;
        setup_sql.push_str(&sql);
    }

    // Delete a small fraction of each table to force the visibility map and heap resolution to be
    // more interesting.
    for (tname, _) in tables {
        let sql = format!("DELETE FROM {tname} WHERE random() < 0.01;\n");
        sql.as_str().execute_result(conn)?;
        setup_sql.push_str(&sql);
    }

    Ok(setup_sql)
}

///
/// Generates arbitrary joins, where clauses, and optional cross-relation predicates
/// for the given tables and columns.
///
pub fn arb_joins_and_wheres<J, S>(
    join_types: J,
    tables: Vec<S>,
    columns: &[Column],
) -> impl Strategy<Value = (JoinExpr, Expr, Option<CrossRelExpr>)> + use<J, S>
where
    J: Strategy<Value = JoinType> + Clone,
    S: AsRef<str>,
{
    let table_names = tables
        .into_iter()
        .map(|tn| tn.as_ref().to_string())
        .collect::<Vec<_>>();

    let columns = columns.to_vec();
    let numeric_columns: Vec<String> = columns
        .iter()
        .filter(|c| c.sql_type == "INTEGER" && c.is_whereable)
        .map(|c| c.name.to_string())
        .collect();

    // Choose how many tables will be joined (at least 2).
    (2..=table_names.len())
        .prop_flat_map(move |join_size| {
            // Then choose tables for that join size.
            proptest::sample::subsequence(table_names.clone(), join_size)
        })
        .prop_flat_map(move |tables| {
            let cross_rel_strategy = if numeric_columns.is_empty() {
                proptest::strategy::Just(None).boxed()
            } else {
                proptest::option::of(crossrelgen::arb_cross_rel_expr(
                    tables.clone(),
                    numeric_columns.clone(),
                ))
                .boxed()
            };

            // Finally, choose the joins, where clauses, and optional cross-relation predicate for those tables.
            (
                joingen::arb_joins(
                    join_types.clone(),
                    tables.clone(),
                    columns.iter().map(|c| c.name.to_owned()).collect(),
                ),
                wheregen::arb_wheres(tables.clone(), &columns.to_vec()),
                cross_rel_strategy,
            )
        })
}

#[derive(Copy, Clone, Debug)]
pub struct PgGucs {
    pub aggregate_custom_scan: bool,
    pub custom_scan: bool,
    pub custom_scan_without_operator: bool,
    pub filter_pushdown: bool,
    pub join_custom_scan: bool,
    pub seqscan: bool,
    pub indexscan: bool,
    pub parallel_workers: bool,
    /// Toggles Postgres' `parallel_leader_participation` GUC. When `false`,
    /// only background workers emit tuples — useful for shaking out parallel
    /// scans whose leader/worker partitioning is incorrect (e.g. issue #5024).
    pub parallel_leader_participation: bool,
    /// Enable columnar execution (ColumnarExecState).
    pub columnar_exec: bool,
}

/// When `PARADEDB_FORCE_PARALLEL=1` (or `=true`), the proptest `Arbitrary` impl pins
/// `parallel_workers = true` and `PgGucs::set` additionally emits
/// `SET debug_parallel_query = on` so Postgres picks a parallel plan even on
/// the small property-test tables. Other GUCs continue to vary across cases.
fn force_parallel() -> bool {
    static V: OnceLock<bool> = OnceLock::new();
    *V.get_or_init(|| {
        std::env::var("PARADEDB_FORCE_PARALLEL")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    })
}

/// Chunk count used when `PARADEDB_QGEN_SEGMENTATION=multi`. Picked to be
/// larger than the typical Postgres parallel-worker count so the per-worker
/// segment-claim logic actually has work to split.
const MULTI_SEGMENT_CHUNKS: NonZeroUsize = NonZeroUsize::new(8).unwrap();

/// The "single" mode count: one combined bulk `INSERT` per table (plus the
/// always-present sample-row INSERT). A named constant so `pick_bulk_inserts`
/// returns a `NonZeroUsize` in either arm without repeating the literal.
const SINGLE_BULK_INSERT: NonZeroUsize = NonZeroUsize::new(1).unwrap();

/// Reads `PARADEDB_QGEN_SEED`, the optional u64 that pins both the Postgres
/// `SET seed` value and the bulk-insert chunk-count roll. Unset means
/// `generated_queries_setup_result` picks one fresh per call. Either way the seed
/// used lands in the reproduction script, so a failing run can be replayed
/// with `PARADEDB_QGEN_SEED=<n> PROPTEST_RNG_SEED=<m> cargo test ...`.
fn qgen_seed() -> Option<u64> {
    std::env::var("PARADEDB_QGEN_SEED").ok().map(|s| {
        s.parse::<u64>()
            .unwrap_or_else(|_| panic!("PARADEDB_QGEN_SEED must parse as u64; got '{s}'"))
    })
}

/// Picks how many separate bulk `INSERT` statements the setup will emit per
/// table. Each chunk = one Tantivy writer commit = one segment, so the index
/// ends with `bulk_inserts + 1` segments (the +1 is the sample-row INSERT).
///
/// Honors `PARADEDB_QGEN_SEGMENTATION=single|multi|random` first, then falls
/// back to a coin flip on the supplied RNG. One call per
/// `generated_queries_setup_result`; every table built in the same call gets the
/// same count, different `#[test]` functions roll independently.
fn pick_bulk_inserts(rng: &mut impl RngExt) -> NonZeroUsize {
    let mode = std::env::var("PARADEDB_QGEN_SEGMENTATION")
        .ok()
        .unwrap_or_default();
    match mode.to_ascii_lowercase().as_str() {
        "single" => SINGLE_BULK_INSERT,
        "multi" => MULTI_SEGMENT_CHUNKS,
        "" | "random" => {
            if rng.random_bool(0.5) {
                MULTI_SEGMENT_CHUNKS
            } else {
                SINGLE_BULK_INSERT
            }
        }
        other => panic!(
            "PARADEDB_QGEN_SEGMENTATION must be 'single', 'multi', or 'random'; got '{other}'"
        ),
    }
}

/// Emit the bulk-INSERT block for one table. `row_count` rows are split into
/// `bulk_inserts` separate `INSERT ... generate_series` statements,
/// distributed as evenly as possible.
fn build_bulk_inserts(
    tname: &str,
    row_count: usize,
    insert_columns: &str,
    random_generators: &str,
    bulk_inserts: NonZeroUsize,
) -> String {
    let k = bulk_inserts.get();
    (0..k)
        .map(|i| (row_count + i) / k)
        .filter(|chunk| *chunk > 0)
        .map(|chunk| {
            format!(
                "INSERT into {tname} ({insert_columns}) SELECT {random_generators} FROM generate_series(1, {chunk});",
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Server-side `statement_timeout` (ms) emitted by `PgGucs::set`, so a hung query
/// surfaces as a failure instead of stalling the run. Override with
/// `PARADEDB_QGEN_STATEMENT_TIMEOUT_MS`; defaults to 60000.
fn statement_timeout_ms() -> u64 {
    static V: OnceLock<u64> = OnceLock::new();
    *V.get_or_init(|| {
        std::env::var("PARADEDB_QGEN_STATEMENT_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60_000)
    })
}

impl Arbitrary for PgGucs {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        any::<[bool; 10]>()
            .prop_map(|b| {
                let mut g = Self {
                    aggregate_custom_scan: b[0],
                    custom_scan: b[1],
                    custom_scan_without_operator: b[2],
                    filter_pushdown: b[3],
                    join_custom_scan: b[4],
                    seqscan: b[5],
                    indexscan: b[6],
                    parallel_workers: b[7],
                    parallel_leader_participation: b[8],
                    columnar_exec: b[9],
                };
                if force_parallel() {
                    g.parallel_workers = true;
                }
                g
            })
            .boxed()
    }
}

impl PgGucs {
    /// Creates an instance of PgGucs with all pg_search scans disabled.
    pub fn pg_search_disabled() -> Self {
        Self {
            aggregate_custom_scan: false,
            custom_scan: false,
            custom_scan_without_operator: false,
            filter_pushdown: false,
            join_custom_scan: false,
            seqscan: true,
            indexscan: true,
            parallel_workers: true,
            parallel_leader_participation: true,
            columnar_exec: false,
        }
    }

    pub fn set(&self) -> String {
        let PgGucs {
            aggregate_custom_scan,
            custom_scan,
            custom_scan_without_operator,
            filter_pushdown,
            join_custom_scan,
            seqscan,
            indexscan,
            parallel_workers,
            parallel_leader_participation,
            columnar_exec,
        } = self;

        let max_parallel_workers = if *parallel_workers { 8 } else { 0 };
        let max_parallel_workers_per_gather = if *parallel_workers { 4 } else { 0 };

        let mut gucs = String::with_capacity(512);
        writeln!(
            gucs,
            "SET paradedb.enable_aggregate_custom_scan TO {aggregate_custom_scan};"
        )
        .unwrap();
        writeln!(gucs, "SET paradedb.enable_custom_scan TO {custom_scan};").unwrap();
        writeln!(
            gucs,
            "SET paradedb.enable_custom_scan_without_operator TO {custom_scan_without_operator};"
        )
        .unwrap();
        writeln!(
            gucs,
            "SET paradedb.enable_filter_pushdown TO {filter_pushdown};"
        )
        .unwrap();
        writeln!(
            gucs,
            "SET paradedb.enable_join_custom_scan TO {join_custom_scan};"
        )
        .unwrap();
        writeln!(gucs, "SET enable_seqscan TO {seqscan};").unwrap();
        writeln!(gucs, "SET enable_indexscan TO {indexscan};").unwrap();
        writeln!(gucs, "SET max_parallel_workers TO {max_parallel_workers};").unwrap();
        writeln!(
            gucs,
            "SET max_parallel_workers_per_gather TO {max_parallel_workers_per_gather};"
        )
        .unwrap();
        writeln!(
            gucs,
            "SET parallel_leader_participation TO {parallel_leader_participation};"
        )
        .unwrap();
        writeln!(gucs, "SET paradedb.add_doc_count_to_aggs TO true;").unwrap();
        writeln!(
            gucs,
            "SET paradedb.enable_columnar_exec TO {columnar_exec};"
        )
        .unwrap();
        // Pin `min_rows_per_worker` low when we want parallel workers to be used.
        if *parallel_workers {
            writeln!(gucs, "SET paradedb.min_rows_per_worker TO 10;").unwrap();
        } else {
            writeln!(gucs, "RESET paradedb.min_rows_per_worker;").unwrap();
        }
        writeln!(gucs, "SET statement_timeout TO {};", statement_timeout_ms()).unwrap();
        if force_parallel() {
            writeln!(gucs, "SET debug_parallel_query TO on;").unwrap();
        }
        gucs
    }
}

/// Fully-resolved outcome of a single qgen comparison case: good, transient, or bad.
pub enum CaseOutcome {
    /// Good: PostgreSQL and ParadeDB produced identical results.
    Match,
    /// Transient: a query hit a classified fault-induced error. Ride it out by retrying (see
    /// [`crate::fixtures::fault_grace::retry_transient`]); never a verdict.
    Transient(TransientKind, sqlx::Error),
    /// Bad: a result mismatch, a panic during comparison, or a hard SQL error. Carries a
    /// `TestCaseError` with the reproduction script embedded.
    Failure(TestCaseError),
}

impl CaseOutcome {
    pub fn into_test_result(self) -> Result<(), TestCaseError> {
        match self {
            CaseOutcome::Match => Ok(()),
            CaseOutcome::Failure(e) => Err(e),
            // Unreachable through the retrying path, and a plain `cargo test` never classifies
            // anything transient; kept total for direct `compare_outcome` callers.
            CaseOutcome::Transient(_, e) => Err(TestCaseError::fail(format!(
                "{e}: transient database fault"
            ))),
        }
    }
}

/// The session settings each side of a comparison runs under.
pub struct Sides {
    pub baseline: String,
    pub candidate: String,
}

impl Sides {
    /// Plain Postgres, the known-correct baseline, against the custom scans under `gucs`.
    pub fn postgres_vs(gucs: &PgGucs) -> Self {
        Self {
            baseline: PgGucs::pg_search_disabled().set(),
            candidate: gucs.set(),
        }
    }
}

/// Run one generated case on `conn`: execute `pg_query` (custom scan off, the known-correct
/// baseline) and `bm25_query` (with `gucs`), then compare their results.
pub fn compare_outcome<R, F>(
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    conn: &mut PgConnection,
    setup_sql: &str,
    run_query: F,
) -> CaseOutcome
where
    R: Eq + Debug,
    F: Fn(&str, &mut PgConnection) -> Result<R, sqlx::Error>,
{
    let sides = Sides::postgres_vs(gucs);
    compare_outcome_on(
        &sides, pg_query, bm25_query, gucs, conn, setup_sql, run_query,
    )
}

/// [`compare_outcome`] with the two sessions spelled out, for a baseline other than plain
/// Postgres, such as one backend of pg_search against another.
pub fn compare_outcome_on<R, F>(
    sides: &Sides,
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    conn: &mut PgConnection,
    setup_sql: &str,
    run_query: F,
) -> CaseOutcome
where
    R: Eq + Debug,
    F: Fn(&str, &mut PgConnection) -> Result<R, sqlx::Error>,
{
    // A panic (vs a returned sqlx::Error) still becomes a Failure, so it trips the oracle and
    // carries a repro script instead of aborting the driver.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        compare_outcome_inner(
            sides, pg_query, bm25_query, gucs, conn, setup_sql, run_query,
        )
    }));
    match outcome {
        Ok(o) => o,
        Err(panic) => {
            let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                format!("Panic: {s}")
            } else if let Some(s) = panic.downcast_ref::<String>() {
                format!("Panic: {s}")
            } else {
                "Panic occurred".to_string()
            };
            CaseOutcome::Failure(handle_compare_error(
                TestCaseError::fail(msg),
                pg_query,
                bm25_query,
                gucs,
                setup_sql,
            ))
        }
    }
}

/// Runs one case, retrying transient faults until it completes, so every case in the proptest
/// budget ends in a real verdict. Bounding and liveness are [`crate::fixtures::fault_grace`]'s
/// job. Takes the pool because a fault usually kills the connection in use.
pub fn compare_outcome_retrying<R, F>(
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    pool: &MutexObjectPool<PgConnection>,
    setup_sql: &str,
    run_query: F,
) -> CaseOutcome
where
    R: Eq + Debug,
    F: Fn(&str, &mut PgConnection) -> Result<R, sqlx::Error>,
{
    let sides = Sides::postgres_vs(gucs);
    compare_outcome_retrying_on(
        &sides, pg_query, bm25_query, gucs, pool, setup_sql, run_query,
    )
}

/// [`compare_outcome_retrying`] with the two sessions spelled out; see [`compare_outcome_on`].
pub fn compare_outcome_retrying_on<R, F>(
    sides: &Sides,
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    pool: &MutexObjectPool<PgConnection>,
    setup_sql: &str,
    run_query: F,
) -> CaseOutcome
where
    R: Eq + Debug,
    F: Fn(&str, &mut PgConnection) -> Result<R, sqlx::Error>,
{
    use crate::fixtures::fault_grace::{Attempt, RetryError};
    let fail = |msg: String| {
        CaseOutcome::Failure(handle_compare_error(
            TestCaseError::fail(msg),
            pg_query,
            bm25_query,
            gucs,
            setup_sql,
        ))
    };
    let outcome = crate::fixtures::fault_grace::retry_transient(pool, "qgen case", |conn| {
        match compare_outcome_on(
            sides, pg_query, bm25_query, gucs, conn, setup_sql, &run_query,
        ) {
            CaseOutcome::Transient(kind, e) => Attempt::Transient(kind, e),
            verdict => Attempt::Done(verdict),
        }
    });
    match outcome {
        Ok(o) => o,
        Err(RetryError::TimedOutUnderPause(e)) => fail(format!(
            "statement timed out while faults were paused for the whole attempt \
             (the query cannot finish inside statement_timeout on a healthy database): {e}"
        )),
        Err(RetryError::GraceExpired(reason)) => fail(reason),
    }
}

fn compare_outcome_inner<R, F>(
    sides: &Sides,
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    conn: &mut PgConnection,
    setup_sql: &str,
    run_query: F,
) -> CaseOutcome
where
    R: Eq + Debug,
    F: Fn(&str, &mut PgConnection) -> Result<R, sqlx::Error>,
{
    let mut queries = || -> Result<(R, R), sqlx::Error> {
        sides
            .baseline
            .as_str()
            .execute_result(conn)
            .and_then(|()| conn.deallocate_all())?;
        let pg_result = run_query(pg_query, conn)?;

        sides
            .candidate
            .as_str()
            .execute_result(conn)
            .and_then(|()| conn.deallocate_all())?;
        let bm25_result = run_query(bm25_query, conn)?;
        Ok((pg_result, bm25_result))
    };
    let (pg_result, bm25_result) = match queries() {
        Ok(results) => results,
        Err(e) => match classify_transient(&e) {
            Some(kind) => return CaseOutcome::Transient(kind, e),
            None => {
                return CaseOutcome::Failure(handle_compare_error(
                    TestCaseError::fail(format!("{e}: error in query execution")),
                    pg_query,
                    bm25_query,
                    gucs,
                    setup_sql,
                ));
            }
        },
    };
    match assert_results_match(&pg_result, &bm25_result, pg_query, bm25_query, gucs, conn) {
        Ok(()) => CaseOutcome::Match,
        Err(e) => CaseOutcome::Failure(handle_compare_error(
            e, pg_query, bm25_query, gucs, setup_sql,
        )),
    }
}

/// Assert the two result sets are equal, attaching the ParadeDB plan to the failure message. The
/// EXPLAIN is built lazily inside the assert message so it runs only on mismatch, and best-effort
/// so a fault while composing it cannot itself panic.
fn assert_results_match<R>(
    pg_result: &R,
    bm25_result: &R,
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    conn: &mut PgConnection,
) -> Result<(), TestCaseError>
where
    R: Eq + Debug,
{
    prop_assert_eq!(
        pg_result,
        bm25_result,
        "\ngucs={:?}\npg:\n  {}\nbm25:\n  {}\nexplain:\n{}\n",
        gucs,
        pg_query,
        bm25_query,
        format!("EXPLAIN {bm25_query}")
            .fetch_result::<(String,)>(conn)
            .map(|rows| rows
                .into_iter()
                .map(|(s,)| s)
                .collect::<Vec<_>>()
                .join("\n"))
            .unwrap_or_else(|e| format!("<EXPLAIN unavailable: {e}>"))
    );
    Ok(())
}

/// Panic-based comparison kept for the non-Antithesis generator tests (`json_pushdown`,
/// `scalar_array_pushdown`), whose `run_query` closures panic on DB errors. Thin wrapper over
/// [`compare_outcome`].
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
    match compare_outcome(
        pg_query,
        bm25_query,
        gucs,
        conn,
        setup_sql,
        |query, conn| Ok::<R, sqlx::Error>(run_query(query, conn)),
    ) {
        // run_query panics on DB errors here, so a `Transient` means the GUC set failed (and a
        // plain `cargo test` never classifies anything transient anyway).
        CaseOutcome::Transient(_, e) => Err(handle_compare_error(
            TestCaseError::fail(format!("{e}: error in query execution")),
            pg_query,
            bm25_query,
            gucs,
            setup_sql,
        )),
        verdict => verdict.into_test_result(),
    }
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
        || error_msg.contains("Panic")
    {
        "QUERY EXECUTION FAILURE"
    } else {
        "RESULT MISMATCH"
    };

    let qgen_seed = setup_sql
        .lines()
        .find_map(|l| l.strip_prefix("-- PARADEDB_QGEN_SEED: "))
        .unwrap_or_else(|| {
            panic!(
                "qgen seed marker missing from setup_sql; \
                 `generated_queries_setup_result` must emit `-- PARADEDB_QGEN_SEED: <n>`"
            )
        });
    let proptest_seed = std::env::var("PROPTEST_RNG_SEED")
        .ok()
        .unwrap_or_else(|| "<from proptest output above>".to_string());

    let repro_script = format!(
        r#"
-- ==== {failure_type} REPRODUCTION SCRIPT ====
-- Copy and paste this entire block to reproduce the issue
--
-- Prerequisites: Ensure pg_search extension is available
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_search;
--
-- Table and index setup
{setup_sql}
--
-- Default GUCs:
{default_gucs}
--
-- PostgreSQL query:
{pg_query};
--
-- Set GUCs to match the failing test case
{gucs_sql}
--
-- ParadeDB query:
{bm25_query};
--
-- ==== END REPRODUCTION SCRIPT ====

Replay this proptest case end-to-end:
  PARADEDB_QGEN_SEED={qgen_seed} PROPTEST_RNG_SEED={proptest_seed} \
    cargo test --package tests --test qgen <test_fn_name>

Original error:
{error_msg}
"#,
        failure_type = failure_type,
        qgen_seed = qgen_seed,
        proptest_seed = proptest_seed,
        setup_sql = setup_sql,
        default_gucs = PgGucs::pg_search_disabled().set(),
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
            "Results differ between PostgreSQL and ParadeDB"
        }
    ))
}

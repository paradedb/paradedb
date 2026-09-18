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
pub mod mutationgen;
pub mod numericgen;
pub mod opexprgen;
pub mod orderbygen;
pub mod pagegen;
pub mod pdbagggen;
pub mod wheregen;
pub mod windowgen;

use std::fmt::{Debug, Write};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, OnceLock};
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
use mutationgen::CaseChurn;
use opexprgen::{ArrayQuantifier, Operator};
use wheregen::Expr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexExpression {
    Literal,
    UnicodeWordsColumnar,
    Upper,
    LiteralNormalized,
}

impl IndexExpression {
    /// Generate the column definition for `CREATE INDEX ... USING paradedb(...)`
    pub fn to_index_sql(&self, column_name: &str) -> String {
        match self {
            Self::Literal => format!("({column_name}::pdb.literal)"),
            Self::UnicodeWordsColumnar => {
                format!("({column_name}::pdb.unicode_words('columnar=true'))")
            }
            Self::Upper => format!("(upper({column_name})::pdb.literal)"),
            Self::LiteralNormalized => format!("({column_name}::pdb.literal_normalized)"),
        }
    }
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
    pub is_orderable: Option<bool>,
    pub random_generator_sql: &'static str,
    pub index_expression: Option<IndexExpression>,
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
            is_orderable: None,
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

    pub const fn orderable(mut self, is_orderable: bool) -> Self {
        self.is_orderable = Some(is_orderable);
        self
    }

    pub fn is_orderable(&self) -> bool {
        if let Some(orderable) = self.is_orderable {
            return orderable;
        }
        let ty = self.sql_type.to_ascii_uppercase();
        ty.starts_with("INT")
            || ty.starts_with("SERIAL")
            || ty.starts_with("BIGINT")
            || ty.starts_with("SMALLINT")
            || ty.starts_with("NUMERIC")
            || ty.starts_with("DECIMAL")
            || ty.starts_with("FLOAT")
            || ty.starts_with("REAL")
            || ty.starts_with("DOUBLE")
            || ty.starts_with("DATE")
            || ty.starts_with("TIME")
    }

    /// Note: should use only the `random()` function to generate random data.
    pub const fn random_generator_sql(mut self, random_generator_sql: &'static str) -> Self {
        self.random_generator_sql = random_generator_sql;
        self
    }

    pub const fn index_expression(mut self, expression: IndexExpression) -> Self {
        self.index_expression = Some(expression);
        self
    }

    /// Whether this column has an array SQL type (e.g. "TEXT[]", "INTEGER[]").
    pub const fn is_array(&self) -> bool {
        let bytes = self.sql_type.as_bytes();
        bytes.len() >= 2 && bytes[bytes.len() - 2] == b'[' && bytes[bytes.len() - 1] == b']'
    }
}

#[derive(Debug, Clone)]
pub struct SetupScript {
    pub sql: String,
    pub tables: Vec<String>,
    pub qgen_seed: Option<u64>,
    /// Churn committed so far, shared by every per-case copy of this script, so a repro rebuilds
    /// the heap the failing case queried rather than the one the setup built.
    pub churn_log: Arc<Mutex<String>>,
    /// The uncommitted churn of the case this copy belongs to; `compare_outcome_on` frames the
    /// case with it. `None` on the script `generated_queries_setup` returns.
    pub case_churn: Option<CaseChurn>,
}

impl SetupScript {
    pub fn new(sql: String, tables: Vec<String>) -> Self {
        Self {
            sql,
            tables,
            qgen_seed: None,
            churn_log: Arc::new(Mutex::new(String::new())),
            case_churn: None,
        }
    }

    /// A copy for a failure that did not run inside the case's transaction, or that ran after it
    /// closed. Its churn is in the log by then, so the script must not frame the queries in a
    /// transaction they never ran in, nor replay the same rows twice.
    pub fn without_case_churn(&self) -> Self {
        Self {
            case_churn: None,
            ..self.clone()
        }
    }

    /// The setup section of a reproduction script: the schema, then every mutation committed
    /// since.
    pub fn repro_setup_sql(&self) -> String {
        let log = self
            .churn_log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if log.is_empty() {
            return self.sql.clone();
        }
        format!("{}\n{log}", self.sql)
    }

    pub fn drop_tables_sql(&self) -> String {
        self.tables
            .iter()
            .map(|t| format!("DROP TABLE {t};"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl std::fmt::Display for SetupScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.sql)
    }
}

/// Builds the schema every generator queries against and returns the SQL as a reproduction
/// script, retrying transient faults per [`crate::fixtures::fault_grace`] (under a plain
/// `cargo test` the first error panics). Each attempt runs in one transaction so a connection
/// killed midway rolls back cleanly and the retry starts from scratch (`CREATE TABLE` has no
/// `IF NOT EXISTS`). `BEGIN`/`COMMIT` are kept out of the returned script, which is replayed
/// statement by statement.
///
/// The seed also decides whether the BM25 indexes are `partition_by` (see `pick_partition_by`).
pub fn generated_queries_setup(
    pool: &MutexObjectPool<PgConnection>,
    tables: &[(&str, usize)],
    columns_def: &[Column],
) -> SetupScript {
    use crate::fixtures::fault_grace::{RetryError, sql_attempt};
    let attempt = |conn: &mut PgConnection| -> Result<SetupScript, sqlx::Error> {
        "BEGIN;".execute_result(conn)?;
        match generated_queries_setup_inner(conn, tables, columns_def) {
            Ok(setup_script) => {
                "COMMIT;".execute_result(conn)?;
                Ok(setup_script)
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
        Ok(Ok(setup_script)) => setup_script,
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
) -> Result<SetupScript, sqlx::Error> {
    "CREATE EXTENSION IF NOT EXISTS vector;".execute_result(conn)?;
    "CREATE EXTENSION IF NOT EXISTS pg_search;".execute_result(conn)?;
    "SET log_error_verbosity TO VERBOSE;".execute_result(conn)?;
    "SET log_min_duration_statement TO 1000;".execute_result(conn)?;

    let qgen_seed = qgen_seed().unwrap_or_else(|| rand::rng().random::<u64>());
    let mut rng = StdRng::seed_from_u64(qgen_seed);
    let pg_seed: f64 = rng.random_range(-1.0..=1.0);
    let bulk_inserts = pick_bulk_inserts(&mut rng);
    let partition_by = pick_partition_by(&mut rng, columns_def);

    let seed_sql = format!("SET seed TO {pg_seed};\n");
    seed_sql.as_str().execute_result(conn)?;

    let mut setup_sql = seed_sql;
    setup_sql.push_str(&format!("-- PARADEDB_QGEN_SEED: {qgen_seed}\n"));
    setup_sql.push_str(&format!("-- qgen bulk inserts: {bulk_inserts}\n"));
    setup_sql.push_str(&format!(
        "-- qgen partition_by: {}\n",
        partition_by.as_deref().unwrap_or("none")
    ));

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

    let index_columns = columns_def
        .iter()
        .filter(|c| c.is_indexed)
        .map(|c| {
            if let Some(expr) = c.index_expression {
                expr.to_index_sql(c.name)
            } else {
                c.name.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

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
        .map(|c| c.name)
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

        // In incremental mode this equals the insert-commit count, preventing the generated
        // segments from immediately merging together. A partitioned build gets two partitions:
        // more would leave segments of a handful of rows on these tables, and a segment in
        // which no row has a given JSON key trips the aggregate scan (#6353).
        let target_segments = if partition_by.is_some() {
            2
        } else {
            bulk_inserts.get() + 1
        };

        let partition_by_clause = partition_by
            .as_deref()
            .map(|fields| format!(",\n    partition_by = '{fields}'"))
            .unwrap_or_default();

        let bulk_insert_sql = build_bulk_inserts(
            tname,
            *row_count,
            &insert_columns,
            &random_generators,
            bulk_inserts,
        );

        let create_index_sql = format!(
            r#"CREATE INDEX idx{tname} ON {tname} USING paradedb ({index_columns}) WITH (
    target_segment_count = {target_segments}{sort_by_clause}{partition_by_clause}
);
"#,
        );
        // TODO(#5738): drop this toggle once partitioning also applies to rows inserted after
        // CREATE INDEX (partitioning M3); then every index can be created before the data.
        let (index_before_data, index_after_data) = if partition_by.is_some() {
            ("", create_index_sql.as_str())
        } else {
            (create_index_sql.as_str(), "")
        };

        let sql = format!(
            r#"
CREATE TABLE {tname} (
    {column_definitions}
);
-- Churn picks rows in heap order, so a background vacuum between a run and its replay would
-- hand the replay other rows.
ALTER TABLE {tname} SET (autovacuum_enabled = off);
{index_before_data}

INSERT into {tname} ({insert_columns}) VALUES ({sample_values});

{bulk_insert_sql}

{index_after_data}

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

    let table_names = tables
        .iter()
        .map(|(tname, _)| (*tname).to_string())
        .collect::<Vec<_>>();

    Ok(SetupScript {
        sql: setup_sql,
        tables: table_names,
        qgen_seed: Some(qgen_seed),
        churn_log: Arc::new(Mutex::new(String::new())),
        case_churn: None,
    })
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
                joingen::arb_joins(join_types.clone(), tables.clone(), &columns),
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
    /// Enable range co-partitioning for joins whose indexes declare compatible `partition_by`
    /// fields.
    pub range_partitioned_join: bool,
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

/// `PARADEDB_QGEN_PARTITION_BY` overrides the seeded choice. Only integer fields are candidates:
/// text fields would need a raw normalizer the fixture does not set.
///
/// A partitioned layout under a parallel join currently aborts the backend at plan time
/// (#6364). The roll stays on so the generators keep reaching that path; `none` is the
/// escape hatch until the fix lands.
fn pick_partition_by(rng: &mut impl RngExt, columns_def: &[Column]) -> Option<String> {
    let mode = std::env::var("PARADEDB_QGEN_PARTITION_BY")
        .ok()
        .unwrap_or_default();
    match mode.to_ascii_lowercase().as_str() {
        "none" => return None,
        "" | "random" => {}
        _ => {
            let names: Vec<&str> = columns_def.iter().map(|c| c.name).collect();
            if mode.split(',').all(|field| names.contains(&field)) {
                return Some(mode);
            }
        }
    }

    let integer_types = [
        "INT",
        "INTEGER",
        "BIGINT",
        "SMALLINT",
        "SERIAL8",
        "BIGSERIAL",
    ];
    let candidates = columns_def
        .iter()
        .filter(|c| c.is_indexed && c.index_expression.is_none())
        .filter(|c| integer_types.contains(&c.sql_type.to_uppercase().as_str()))
        .map(|c| c.name)
        .collect::<Vec<_>>();
    if candidates.is_empty() || rng.random_bool(1.0 / 3.0) {
        return None;
    }
    let first = rng.random_range(0..candidates.len());
    let mut fields = vec![candidates[first]];
    if candidates.len() > 1 && rng.random_bool(0.25) {
        let mut second = rng.random_range(0..candidates.len() - 1);
        if second >= first {
            second += 1;
        }
        fields.push(candidates[second]);
    }
    Some(fields.join(","))
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
        any::<[bool; 11]>()
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
                    range_partitioned_join: b[10],
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
            range_partitioned_join: false,
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
            range_partitioned_join,
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
        writeln!(
            gucs,
            "SET paradedb.enable_range_partitioned_join TO {range_partitioned_join};"
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

/// Which side of a query comparison is currently being executed.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum QuerySide {
    Baseline,
    Candidate,
}

impl QuerySide {
    pub fn is_candidate(self) -> bool {
        self == Self::Candidate
    }

    pub fn is_baseline(self) -> bool {
        self == Self::Baseline
    }
}

/// Run one generated case on `conn`: execute `pg_query` (custom scan off, the known-correct
/// baseline) and `bm25_query` (with `gucs`), then compare their results.
pub fn compare_outcome<R, F>(
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    conn: &mut PgConnection,
    setup: &SetupScript,
    run_query: F,
) -> CaseOutcome
where
    R: Eq + Debug,
    F: Fn(&str, QuerySide, &mut PgConnection) -> Result<R, sqlx::Error>,
{
    let sides = Sides::postgres_vs(gucs);
    compare_outcome_on(&sides, pg_query, bm25_query, gucs, conn, setup, run_query)
}

/// The transaction a case's uncommitted churn holds open around both sides (see
/// [`mutationgen`]). Opened before the sides run, closed after them whatever happened in
/// between, so no pooled connection goes back with a transaction still open.
struct CaseFrame<'a> {
    churn: Option<&'a CaseChurn>,
    own_open: bool,
}

impl<'a> CaseFrame<'a> {
    fn open(
        churn: Option<&'a CaseChurn>,
        conn: &mut PgConnection,
        setup: &SetupScript,
    ) -> Result<Self, sqlx::Error> {
        let mut frame = Self {
            churn,
            own_open: false,
        };
        if let Some(churn) = churn
            && let Err(e) = frame.open_inner(churn, conn)
        {
            frame.close(conn, setup);
            return Err(e);
        }
        Ok(frame)
    }

    fn open_inner(
        &mut self,
        churn: &CaseChurn,
        conn: &mut PgConnection,
    ) -> Result<(), sqlx::Error> {
        "BEGIN;".execute_result(conn)?;
        self.own_open = true;
        for statement in &churn.own {
            statement.execute_result(conn)?;
        }
        Ok(())
    }

    /// Best effort: a session that is already gone rolls back on its own. The rolled-back
    /// transaction then joins the churn log, since the aborted tuples and index entries it
    /// leaves behind are part of the heap every later case queries.
    fn close(&mut self, conn: &mut PgConnection, setup: &SetupScript) {
        let own_ran = self.own_open;
        if self.own_open {
            let _ = "ROLLBACK;".execute_result(conn);
            self.own_open = false;
        }
        if let Some(churn) = self.churn
            && own_ran
        {
            let mut log = setup
                .churn_log
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            churn.log_rolled_back(&mut log);
        }
    }
}

/// [`compare_outcome`] with the two sessions spelled out, for a baseline other than plain
/// Postgres, such as one backend of pg_search against another.
pub fn compare_outcome_on<R, F>(
    sides: &Sides,
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    conn: &mut PgConnection,
    setup: &SetupScript,
    run_query: F,
) -> CaseOutcome
where
    R: Eq + Debug,
    F: Fn(&str, QuerySide, &mut PgConnection) -> Result<R, sqlx::Error>,
{
    let mut frame = match CaseFrame::open(setup.case_churn.as_ref(), conn, setup) {
        Ok(frame) => frame,
        Err(e) => {
            return match classify_transient(&e) {
                Some(kind) => CaseOutcome::Transient(kind, e),
                None => CaseOutcome::Failure(handle_compare_error(
                    TestCaseError::fail(format!("{e}: error applying the case's churn")),
                    pg_query,
                    bm25_query,
                    gucs,
                    &setup.without_case_churn(),
                )),
            };
        }
    };
    // A panic (vs a returned sqlx::Error) still becomes a Failure, so it trips the oracle and
    // carries a repro script instead of aborting the driver.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        compare_outcome_inner(sides, pg_query, bm25_query, gucs, conn, setup, run_query)
    }));
    // Rendered before the frame closes, since closing appends the case's churn to the log and
    // the script carries it inline.
    let outcome = match outcome {
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
                setup,
            ))
        }
    };
    frame.close(conn, setup);
    outcome
}

/// Runs one case, retrying transient faults until it completes, so every case in the proptest
/// budget ends in a real verdict. Bounding and liveness are [`crate::fixtures::fault_grace`]'s
/// job. Takes the pool because a fault usually kills the connection in use.
pub fn compare_outcome_retrying<R, F>(
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    pool: &MutexObjectPool<PgConnection>,
    setup: &SetupScript,
    run_query: F,
) -> CaseOutcome
where
    R: Eq + Debug,
    F: Fn(&str, QuerySide, &mut PgConnection) -> Result<R, sqlx::Error>,
{
    let sides = Sides::postgres_vs(gucs);
    compare_outcome_retrying_on(&sides, pg_query, bm25_query, gucs, pool, setup, run_query)
}

/// [`compare_outcome_retrying`] with the two sessions spelled out; see [`compare_outcome_on`].
pub fn compare_outcome_retrying_on<R, F>(
    sides: &Sides,
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    pool: &MutexObjectPool<PgConnection>,
    setup: &SetupScript,
    run_query: F,
) -> CaseOutcome
where
    R: Eq + Debug,
    F: Fn(&str, QuerySide, &mut PgConnection) -> Result<R, sqlx::Error>,
{
    use crate::fixtures::fault_grace::{Attempt, RetryError};
    let fail = |msg: String| {
        CaseOutcome::Failure(handle_compare_error(
            TestCaseError::fail(msg),
            pg_query,
            bm25_query,
            gucs,
            &setup.without_case_churn(),
        ))
    };
    let outcome = crate::fixtures::fault_grace::retry_transient(pool, "qgen case", |conn| {
        match compare_outcome_on(sides, pg_query, bm25_query, gucs, conn, setup, &run_query) {
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

/// Runs an EXPLAIN check on `bm25_query` under `gucs`, retrying transient faults, and asserts
/// that the resulting plan contains at least one of `expected_any`.
pub fn compare_plan_retrying(
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    pool: &MutexObjectPool<PgConnection>,
    setup: &SetupScript,
    expected_any: &[&str],
    forbidden_any: &[&str],
) -> CaseOutcome {
    use crate::fixtures::fault_grace::{RetryError, retry_transient, sql_attempt};

    let fail = |msg: String| {
        // The plan check runs on its own pooled session, outside the case's transaction.
        CaseOutcome::Failure(handle_compare_error(
            TestCaseError::fail(msg),
            pg_query,
            bm25_query,
            gucs,
            &setup.without_case_churn(),
        ))
    };

    let outcome = retry_transient(pool, "qgen plan check", |conn| {
        sql_attempt(gucs.set().execute_result(conn).and_then(|()| {
            format!("EXPLAIN (FORMAT JSON) {bm25_query}")
                .fetch_one_result::<(serde_json::Value,)>(conn)
        }))
    });

    let plan = match outcome {
        Ok(Ok(plan)) => plan,
        Ok(Err(e)) => return fail(format!("{e}: EXPLAIN failed for '{bm25_query}'")),
        Err(RetryError::TimedOutUnderPause(e)) => {
            return fail(format!(
                "EXPLAIN timed out while faults were paused, for '{bm25_query}': {e}"
            ));
        }
        Err(RetryError::GraceExpired(reason)) => return fail(reason),
    };

    let plan_str = format!("{:#?}", plan.0);
    if !expected_any
        .iter()
        .any(|expected| plan_str.contains(expected))
    {
        let expected_desc = expected_any.join(" or ");
        return fail(format!(
            "Query should use {expected_desc} but got plan: {plan_str}\nQuery: {bm25_query}"
        ));
    }

    if let Some(forbidden) = forbidden_any
        .iter()
        .find(|forbidden| plan_str.contains(**forbidden))
    {
        return fail(format!(
            "Query should not use {forbidden} but got plan: {plan_str}\nQuery: {bm25_query}"
        ));
    }

    CaseOutcome::Match
}

fn compare_outcome_inner<R, F>(
    sides: &Sides,
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    conn: &mut PgConnection,
    setup: &SetupScript,
    run_query: F,
) -> CaseOutcome
where
    R: Eq + Debug,
    F: Fn(&str, QuerySide, &mut PgConnection) -> Result<R, sqlx::Error>,
{
    let mut queries = || -> Result<(R, R), sqlx::Error> {
        sides
            .baseline
            .as_str()
            .execute_result(conn)
            .and_then(|()| conn.deallocate_all())?;
        let pg_result = run_query(pg_query, QuerySide::Baseline, conn)?;

        sides
            .candidate
            .as_str()
            .execute_result(conn)
            .and_then(|()| conn.deallocate_all())?;
        let bm25_result = run_query(bm25_query, QuerySide::Candidate, conn)?;
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
                    setup,
                ));
            }
        },
    };
    match assert_results_match(&pg_result, &bm25_result, pg_query, bm25_query, gucs, conn) {
        Ok(()) => CaseOutcome::Match,
        Err(e) => CaseOutcome::Failure(handle_compare_error(e, pg_query, bm25_query, gucs, setup)),
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
    setup: &SetupScript,
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
        setup,
        |query, _side, conn| Ok::<R, sqlx::Error>(run_query(query, conn)),
    ) {
        // run_query panics on DB errors here, so a `Transient` means the GUC set failed (and a
        // plain `cargo test` never classifies anything transient anyway).
        CaseOutcome::Transient(_, e) => Err(handle_compare_error(
            TestCaseError::fail(format!("{e}: error in query execution")),
            pg_query,
            bm25_query,
            gucs,
            setup,
        )),
        verdict => verdict.into_test_result(),
    }
}

/// The seeds a reproduction script replays under.
fn repro_seeds(setup: &SetupScript) -> (String, String) {
    let qgen_seed = setup
        .qgen_seed
        .map(|s| s.to_string())
        .or_else(|| {
            setup
                .sql
                .lines()
                .find_map(|l| l.strip_prefix("-- PARADEDB_QGEN_SEED: "))
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "<unknown>".to_string());
    let proptest_seed = std::env::var("PROPTEST_RNG_SEED")
        .ok()
        .unwrap_or_else(|| "<from proptest output above>".to_string());
    (qgen_seed, proptest_seed)
}

/// A reproduction script for a failure in the fixture rather than in a query under test: the
/// schema, the churn that landed before it, and the statement that failed. No oracle ran, so
/// there is no pair of queries to print.
pub fn handle_setup_error(
    setup: &SetupScript,
    what: &str,
    detail: &str,
    sql: &str,
) -> TestCaseError {
    let (qgen_seed, proptest_seed) = repro_seeds(setup);
    TestCaseError::fail(format!(
        r#"{what} failed: {detail}

-- ==== FIXTURE FAILURE REPRODUCTION SCRIPT ====
-- Copy and paste this entire block to reproduce the issue
--
-- Prerequisites: Ensure pg_search extension is available
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_search;
--
-- Table and index setup
{setup_sql}
--
-- The statement that failed:
{sql}
--
-- Cleanup:
{drop_tables_sql}
--
-- ==== END REPRODUCTION SCRIPT ====

Replay this proptest case end-to-end:
  PARADEDB_QGEN_SEED={qgen_seed} PROPTEST_RNG_SEED={proptest_seed} \
    cargo test --package tests --test qgen <test_fn_name>
"#,
        setup_sql = setup.repro_setup_sql(),
        drop_tables_sql = setup.drop_tables_sql(),
    ))
}

/// Helper function to handle comparison errors and generate reproduction scripts
pub fn handle_compare_error(
    error: TestCaseError,
    pg_query: &str,
    bm25_query: &str,
    gucs: &PgGucs,
    setup: &SetupScript,
) -> TestCaseError {
    let error_msg = error.to_string();
    let failure_type = if error_msg.contains("Query should use")
        || error_msg.contains("EXPLAIN failed")
        || error_msg.contains("EXPLAIN timed out")
    {
        "PLANNING FAILURE"
    } else if error_msg.contains("error returned from database")
        || error_msg.contains("SQL execution error")
        || error_msg.contains("syntax error")
        || error_msg.contains("Panic")
    {
        "QUERY EXECUTION FAILURE"
    } else {
        "RESULT MISMATCH"
    };

    let (qgen_seed, proptest_seed) = repro_seeds(setup);

    let drop_tables_sql = setup.drop_tables_sql();
    let (case_sql, rollback_sql) = match &setup.case_churn {
        Some(churn) => (
            format!("--\n-- Case churn\n{}", churn.repro_sql()),
            "ROLLBACK;\n",
        ),
        None => (String::new(), ""),
    };

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
{case_sql}--
-- Default GUCs:
{default_gucs}
--
-- PostgreSQL query:
{pg_query};
--
-- Set GUCs to match the failing test case
{gucs_sql}
--
-- ParadeDB explain:
EXPLAIN
{bm25_query};
--
-- ParadeDB query:
{bm25_query};
--
-- Cleanup:
{rollback_sql}{drop_tables_sql}
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
        setup_sql = setup.repro_setup_sql(),
        default_gucs = PgGucs::pg_search_disabled().set(),
        gucs_sql = gucs.set(),
        pg_query = pg_query,
        bm25_query = bm25_query,
        drop_tables_sql = drop_tables_sql,
        error_msg = error_msg
    );

    TestCaseError::fail(format!(
        "{}\n{repro_script}",
        match failure_type {
            "QUERY EXECUTION FAILURE" => "Query execution failed",
            "PLANNING FAILURE" => "Query plan did not match expectations",
            _ => "Results differ between PostgreSQL and ParadeDB",
        }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The plan check and the post-close failure paths run outside the case's transaction, so
    /// their scripts must not open one: replaying the churn there changes the rows the queries
    /// read.
    #[test]
    fn a_script_frames_the_case_only_when_the_case_ran_framed() {
        let mut setup = SetupScript::new(
            "CREATE TABLE t (id int);".to_string(),
            vec!["t".to_string()],
        );
        setup.case_churn = Some(mutationgen::CaseChurn {
            own: vec!["UPDATE t SET id = id + 1;".to_string()],
        });
        let render = |setup: &SetupScript| {
            handle_compare_error(
                TestCaseError::fail("Query should use a ParadeDB scan"),
                "SELECT 1",
                "SELECT 1",
                &PgGucs::pg_search_disabled(),
                setup,
            )
            .to_string()
        };

        let framed = render(&setup);
        assert!(framed.contains("-- Case churn"), "{framed}");
        assert!(framed.contains("UPDATE t SET id = id + 1;"), "{framed}");

        let unframed = render(&setup.without_case_churn());
        assert!(!unframed.contains("-- Case churn"), "{unframed}");
        assert!(!unframed.contains("ROLLBACK;"), "{unframed}");
    }
}

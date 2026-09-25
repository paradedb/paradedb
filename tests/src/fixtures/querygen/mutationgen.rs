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

//! Heap churn for the qgen fixture, so the comparison does not assume a freshly built,
//! all-visible table.
//!
//! One churn runs per session, right after the schema, and every case then queries the heap it
//! left: dead tuples, HOT chains, cleared visibility-map bits, reclaimed and reused ctids, and
//! index segments that still carry docs for rows that are gone.
//!
//! The heap holds still for the whole run, which is what keeps the property test's own
//! guarantees: a failing case shrinks its query against the same heap it failed on, and the
//! printed script rebuilds that heap from the schema plus one block of SQL.
//!
//! The mutations come from `PARADEDB_QGEN_SEED`, the same seed the fixture's rows come from, so
//! a run replays whole. Row picks and written values come from Postgres `random()`, seeded once
//! from that draw. `PARADEDB_QGEN_CHURN=off` skips the churn, to tell a churn-dependent failure
//! from a plain query bug.

use std::fmt::Write;
use std::sync::OnceLock;

use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{RngAlgorithm, TestRng, TestRunner};
use sqlx::PgConnection;

use super::{Column, PgGucs, SetupScript, handle_setup_error};
use crate::fixtures::db::Query;
use crate::fixtures::fault_grace::{RetryError, retry_transient, sql_attempt};

/// The `INSERT` column list and the matching random value list, derived once from the schema.
#[derive(Clone, PartialEq, Eq)]
pub struct InsertShape {
    pub columns: String,
    pub generators: String,
}

/// Every case carries the same shape, so proptest's failing-input dump keeps it out of the way.
impl std::fmt::Debug for InsertShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InsertShape { .. }")
    }
}

impl InsertShape {
    pub fn new(columns: &[Column]) -> Self {
        let non_key = columns.iter().filter(|c| !c.is_primary_key);
        Self {
            columns: non_key
                .clone()
                .map(|c| c.name)
                .collect::<Vec<_>>()
                .join(", "),
            generators: non_key
                .map(|c| c.random_generator_sql)
                .collect::<Vec<_>>()
                .join(",\n      "),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mutation {
    Insert {
        table: String,
        rows: usize,
    },
    Delete {
        table: String,
        rows: usize,
    },
    /// Deletes `rows` random rows and inserts as many fresh ones, so the heap churns while the
    /// row count (and with it the join fan-out the test was sized for) holds.
    Replace {
        table: String,
        rows: usize,
    },
    /// Rewrites one column on `rows` random rows. A column outside every index keeps these
    /// updates HOT, which is the only way to grow a HOT chain behind a ctid the index holds.
    Update {
        table: String,
        column: &'static str,
        generator: &'static str,
        rows: usize,
    },
    /// Reclaims dead tuples, marks their docs deleted in the index, and sets visibility-map bits
    /// again. Truncation is drawn per mutation, since it takes an exclusive lock and returns the
    /// space to the filesystem rather than to the free space map.
    Vacuum {
        table: String,
        truncate: bool,
    },
}

impl Mutation {
    pub fn is_vacuum(&self) -> bool {
        matches!(self, Mutation::Vacuum { .. })
    }

    pub fn statements(&self, shape: &InsertShape) -> Vec<String> {
        let pick = |table: &str, rows: usize| {
            format!("id IN (SELECT id FROM {table} ORDER BY random() LIMIT {rows})")
        };
        let insert = |table: &str, rows: usize| {
            format!(
                "INSERT INTO {table} ({}) SELECT {} FROM generate_series(1, {rows});",
                shape.columns, shape.generators
            )
        };
        match self {
            Mutation::Insert { table, rows } => vec![insert(table, *rows)],
            Mutation::Delete { table, rows } => {
                vec![format!("DELETE FROM {table} WHERE {};", pick(table, *rows))]
            }
            Mutation::Replace { table, rows } => vec![
                format!("DELETE FROM {table} WHERE {};", pick(table, *rows)),
                insert(table, *rows),
            ],
            Mutation::Update {
                table,
                column,
                generator,
                rows,
            } => vec![format!(
                "UPDATE {table} SET {column} = {generator} WHERE {};",
                pick(table, *rows)
            )],
            Mutation::Vacuum { table, truncate } => {
                vec![format!("VACUUM (TRUNCATE {truncate}) {table};")]
            }
        }
    }
}

/// Session settings for one churn phase.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Session {
    /// `setseed()` argument, so the rows a phase picks replay under the same proptest seed.
    pub seed: f64,
    /// `paradedb.global_mutable_segment_rows` for the phase's inserts. `None` leaves the index
    /// option in charge; `Some(0)` sends every insert straight to an immutable segment.
    pub mutable_segment_rows: Option<usize>,
}

impl Session {
    /// The churn is a fixture, not a subject: its DML runs on plain Postgres paths, whatever the
    /// previous case left the custom-scan GUCs at, and `SET LOCAL` keeps those settings off the
    /// pooled session. The seed is drawn once per phase, so batches split at a `VACUUM` go on
    /// drawing from the same stream.
    fn session_statements(&self) -> Vec<String> {
        vec![
            PgGucs::pg_search_disabled().set_local(),
            format!("SELECT setseed({});", self.seed),
        ]
    }

    /// Re-applied for each transaction of a phase, since `SET LOCAL` ends with it. Row picks hand
    /// out `random()` in scan order, so they stay off index-only scans, which read the bm25 index
    /// in segment order, and off parallel plans, whose row order varies from run to run. Either
    /// would pick other rows on replay.
    fn local_statements(&self) -> Vec<String> {
        let mut statements = vec![
            "SET LOCAL enable_indexonlyscan TO off;".to_string(),
            "SET LOCAL max_parallel_workers_per_gather TO 0;".to_string(),
        ];
        match self.mutable_segment_rows {
            Some(rows) => statements.push(format!(
                "SET LOCAL paradedb.global_mutable_segment_rows TO {rows};"
            )),
            None => {
                statements.push("SET LOCAL paradedb.global_mutable_segment_rows TO -1;".to_string())
            }
        }
        statements
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Churn {
    pub mutations: Vec<Mutation>,
    pub session: Session,
    pub shape: InsertShape,
}

/// `PARADEDB_QGEN_CHURN=off` turns the churn off.
pub fn churn_enabled() -> bool {
    static V: OnceLock<bool> = OnceLock::new();
    *V.get_or_init(|| {
        let mode = std::env::var("PARADEDB_QGEN_CHURN").unwrap_or_default();
        match mode.to_ascii_lowercase().as_str() {
            "" | "on" => true,
            "off" => false,
            other => panic!("PARADEDB_QGEN_CHURN must be 'on' or 'off'; got '{other}'"),
        }
    })
}

/// Rows per mutation: a slice of the table that is felt on a ten-row table without turning a
/// hundred-thousand-row one into a sort benchmark.
fn max_rows(table_rows: usize) -> usize {
    (table_rows / 5).clamp(1, 50)
}

fn arb_table(tables: &[(&str, usize)]) -> impl Strategy<Value = (String, usize)> + Clone + use<> {
    let tables: Vec<(String, usize)> = tables
        .iter()
        .map(|(name, rows)| ((*name).to_string(), max_rows(*rows)))
        .collect();
    proptest::sample::select(tables)
}

fn arb_size_neutral(
    tables: &[(&str, usize)],
    columns: &[Column],
) -> impl Strategy<Value = Mutation> + use<> {
    let updatable: Vec<(&'static str, &'static str, bool)> = columns
        .iter()
        .filter(|c| !c.is_primary_key && c.random_generator_sql != "NULL")
        .map(|c| (c.name, c.random_generator_sql, c.is_indexed))
        .collect();
    // HOT chains only grow behind un-indexed columns, so those get most of the updates.
    let hot: Vec<_> = updatable.iter().filter(|c| !c.2).cloned().collect();
    let cold: Vec<_> = updatable.iter().filter(|c| c.2).cloned().collect();
    let column = match (hot.is_empty(), cold.is_empty()) {
        (true, true) => None,
        (false, true) => Some(proptest::sample::select(hot).boxed()),
        (true, false) => Some(proptest::sample::select(cold).boxed()),
        (false, false) => Some(
            prop_oneof![
                2 => proptest::sample::select(hot),
                1 => proptest::sample::select(cold),
            ]
            .boxed(),
        ),
    };
    let replace = arb_table(tables)
        .prop_flat_map(|(table, max)| (Just(table), 1..=max))
        .prop_map(|(table, rows)| Mutation::Replace { table, rows });
    match column {
        None => replace.boxed(),
        Some(column) => {
            let update = (arb_table(tables), column)
                .prop_flat_map(|((table, max), column)| (Just(table), Just(column), 1..=max))
                .prop_map(|(table, (column, generator, _), rows)| Mutation::Update {
                    table,
                    column,
                    generator,
                    rows,
                });
            prop_oneof![replace, update].boxed()
        }
    }
}

/// Size-neutral by default, so the row counts each generator sized its joins around hold. A run
/// draws more of these than a single case would: the heap has to be worth querying 256 times.
fn arb_mutations(
    tables: &[(&str, usize)],
    columns: &[Column],
) -> impl Strategy<Value = Vec<Mutation>> + use<> {
    let vacuum = (arb_table(tables), any::<bool>())
        .prop_map(|((table, _), truncate)| Mutation::Vacuum { table, truncate });
    let mutation = prop_oneof![
        5 => arb_size_neutral(tables, columns),
        1 => vacuum,
    ];
    proptest::collection::vec(mutation, 3..=8)
}

fn arb_session() -> impl Strategy<Value = Session> + Clone {
    (
        -1.0..=1.0f64,
        prop_oneof![
            2 => Just(None),
            1 => Just(Some(0)),
            1 => Just(Some(1)),
            1 => Just(Some(10)),
        ],
    )
        .prop_map(|(seed, mutable_segment_rows)| Session {
            seed,
            mutable_segment_rows,
        })
}

/// Churn over `tables` (name and row count, as given to [`super::generated_queries_setup`]) with
/// the schema in `columns`.
fn arb_churn(tables: &[(&str, usize)], columns: &[Column]) -> impl Strategy<Value = Churn> + use<> {
    let shape = InsertShape::new(columns);
    (arb_mutations(tables, columns), arb_session()).prop_map(move |(mutations, session)| Churn {
        mutations,
        session,
        shape: shape.clone(),
    })
}

/// Churns the fixture once, before any case runs, and returns the script with the churn appended
/// so a failure replays the whole heap. The heap then holds still, which is what lets proptest
/// shrink a failing query against the heap it failed on. `enabled` false, or `PARADEDB_QGEN_CHURN=off`, hands back
/// the pristine fixture: that is the clean run each test keeps beside the churned one, as the
/// control that tells a churn-dependent failure apart.
pub fn churn_setup(
    pool: &MutexObjectPool<PgConnection>,
    setup: SetupScript,
    tables: &[(&str, usize)],
    columns: &[Column],
    enabled: bool,
) -> SetupScript {
    if !enabled || !churn_enabled() {
        return setup;
    }
    // Drawn from the fixture's own seed rather than from proptest's RNG. A per-case draw would
    // move the heap under a shrinking run, so the minimized case would query a heap that never
    // failed. One seed also replays the rows and the churn together.
    let seed = setup.qgen_seed.unwrap_or_default();
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&seed.to_le_bytes());
    let rng = TestRng::from_seed(RngAlgorithm::ChaCha, &bytes);
    let mut runner = TestRunner::new_with_rng(ProptestConfig::default(), rng);
    let churn = arb_churn(tables, columns)
        .new_tree(&mut runner)
        .expect("the churn strategy should produce a value")
        .current();
    churn.apply(pool, setup)
}

impl Churn {
    /// Runs the mutations on a pooled session and hands back the script with their SQL appended.
    /// `VACUUM` refuses a transaction block, so the run splits into batches at each vacuum, in
    /// the order drawn: a vacuum drawn first reclaims what the fixture's own build left, and the
    /// inserts after it reuse the space.
    fn apply(&self, pool: &MutexObjectPool<PgConnection>, mut setup: SetupScript) -> SetupScript {
        let mut preamble = self.session.session_statements();
        let mut dml: Vec<String> = Vec::new();
        setup.sql.push_str("\n-- churn\n");

        for mutation in &self.mutations {
            let statements = mutation.statements(&self.shape);
            if !mutation.is_vacuum() {
                dml.extend(statements);
                continue;
            }
            if !dml.is_empty() {
                self.run_batch(pool, &mut setup, &mut preamble, &mut dml);
            }
            for vacuum in statements {
                run_retrying(pool, &setup, "qgen churn vacuum", &vacuum);
                // Appended as it lands, so a failure later in the churn still prints a script
                // that rebuilds the heap up to that point.
                writeln!(setup.sql, "{vacuum}").unwrap();
            }
        }
        if !dml.is_empty() {
            self.run_batch(pool, &mut setup, &mut preamble, &mut dml);
        }
        setup
    }

    /// One transaction of the churn. The session-level settings and the seed go in the first
    /// batch only: re-seeding would hand the batches after a vacuum the same rows.
    fn run_batch(
        &self,
        pool: &MutexObjectPool<PgConnection>,
        setup: &mut SetupScript,
        preamble: &mut Vec<String>,
        dml: &mut Vec<String>,
    ) {
        let mut statements = std::mem::take(preamble);
        statements.extend(self.session.local_statements());
        statements.append(dml);
        let batch = format!("BEGIN;\n{}\nCOMMIT;", statements.join("\n"));
        run_retrying(pool, setup, "qgen churn", &batch);
        writeln!(setup.sql, "{batch}").unwrap();
    }
}

/// The churn runs before proptest does, so a failure here cannot become a case failure. It
/// panics with the same script a case would have printed.
fn run_retrying(pool: &MutexObjectPool<PgConnection>, setup: &SetupScript, what: &str, sql: &str) {
    let outcome = retry_transient(pool, what, |conn| {
        let result = sql.execute_result(conn);
        if result.is_err() {
            // A failed batch leaves the session in an aborted transaction; clear it so the
            // connection goes back to the pool usable.
            let _ = "ROLLBACK;".execute_result(conn);
        }
        sql_attempt(result)
    });
    let detail = match outcome {
        Ok(Ok(())) => return,
        Ok(Err(e)) => e.to_string(),
        Err(RetryError::TimedOutUnderPause(e)) => {
            format!("timed out while faults were paused: {e}")
        }
        Err(RetryError::GraceExpired(reason)) => reason,
    };
    panic!("{}", handle_setup_error(setup, what, &detail, sql));
}

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

//! Heap churn around a qgen case, so the comparison does not assume a freshly built,
//! all-visible table.
//!
//! Each case draws a [`Churn`] with two parts:
//!
//! 1. `committed`: mutations committed on the case's own session before either side runs. They
//!    pile up over the run, so later cases query heaps with dead tuples, HOT chains, cleared
//!    visibility-map bits, reclaimed and reused ctids, and index segments that still carry docs
//!    for rows that are gone.
//! 2. `own`: mutations left uncommitted in the transaction that runs both sides, rolled back once
//!    the case is over. Both sides see the session's own writes; the rollback then leaves aborted
//!    tuples behind for the cases after it.
//!
//! Both sides of a case run back to back on one session, and a case with uncommitted churn runs
//! them inside its transaction, so they read the same rows. Nothing else writes to the test's
//! database while a case runs. The index, on the other hand, sees every one of these rows and
//! has to filter them. Rows another backend holds in flight are a stressgres subject: they need
//! a second live session, which no reproduction script can replay.
//!
//! Row picks and written values come from Postgres `random()`, re-seeded per phase from proptest,
//! so a case replays under the same `PROPTEST_RNG_SEED`. `PARADEDB_QGEN_CHURN=off` disables the
//! whole thing, to tell a churn-dependent failure from a plain query bug.

use std::fmt::Write;
use std::sync::OnceLock;

use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
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
    /// previous case left the custom-scan GUCs at. Seeded once per phase, so batches split at a
    /// `VACUUM` go on drawing from the same stream.
    fn session_statements(&self) -> Vec<String> {
        vec![
            PgGucs::pg_search_disabled().set(),
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

    /// For a phase whose transaction goes on to run the case: the queries must plan with the
    /// settings the case chose, not the churn's.
    fn closing_statements() -> Vec<String> {
        vec![
            "RESET enable_indexonlyscan;".to_string(),
            "RESET max_parallel_workers_per_gather;".to_string(),
            "RESET paradedb.global_mutable_segment_rows;".to_string(),
        ]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Churn {
    pub committed: Vec<Mutation>,
    pub own: Vec<Mutation>,
    pub sessions: [Session; 2],
    pub shape: InsertShape,
}

/// The uncommitted part of one case's churn, carried by the per-case [`SetupScript`] so the
/// comparison helpers can frame the case with it. Every field is plain SQL: the framing is
/// what `compare_outcome_on` does with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseChurn {
    /// Run on the comparison session after `begin`, before either side.
    pub own: Vec<String>,
}

impl CaseChurn {
    /// The case block of a reproduction script. The comparison queries follow it, inside the
    /// transaction it opens, and the rollback comes after them.
    pub fn repro_sql(&self) -> String {
        let mut sql = String::new();
        writeln!(
            sql,
            "-- Case transaction; both queries below run inside it and it is rolled back after:"
        )
        .unwrap();
        writeln!(sql, "BEGIN;").unwrap();
        for statement in &self.own {
            writeln!(sql, "{statement}").unwrap();
        }
        sql
    }

    /// What the case leaves behind for the cases after it: its rolled-back transaction. Appended
    /// to the churn log once the case is over, so a later failure's script rebuilds the aborted
    /// tuples and index entries too.
    pub fn log_rolled_back(&self, log: &mut String) {
        if !self.own.is_empty() {
            log.push_str("-- churn: the case transaction, rolled back after the case\n");
            log.push_str(&rolled_back_block(&self.own));
        }
    }
}

fn rolled_back_block(statements: &[String]) -> String {
    let mut sql = String::from("BEGIN;\n");
    for statement in statements {
        sql.push_str(statement.trim_end());
        sql.push('\n');
    }
    sql.push_str("ROLLBACK;\n");
    sql
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

fn arb_any(
    tables: &[(&str, usize)],
    columns: &[Column],
) -> impl Strategy<Value = Mutation> + use<> {
    let sized = arb_table(tables).prop_flat_map(|(table, max)| (Just(table), 1..=max));
    prop_oneof![
        2 => arb_size_neutral(tables, columns),
        1 => sized.clone().prop_map(|(table, rows)| Mutation::Insert { table, rows }),
        1 => sized.prop_map(|(table, rows)| Mutation::Delete { table, rows }),
    ]
}

fn arb_committed(
    tables: &[(&str, usize)],
    columns: &[Column],
) -> impl Strategy<Value = Vec<Mutation>> + use<> {
    let vacuum = (arb_table(tables), any::<bool>())
        .prop_map(|((table, _), truncate)| Mutation::Vacuum { table, truncate });
    let mutation = prop_oneof![
        5 => arb_size_neutral(tables, columns),
        1 => vacuum,
    ];
    proptest::collection::vec(mutation, 0..=3)
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

/// Churn for one case over `tables` (name and row count, as given to
/// [`super::generated_queries_setup`]) with the schema in `columns`. With `enabled` false, or
/// under `PARADEDB_QGEN_CHURN=off`, every case gets an empty churn: that is the clean run each
/// test keeps beside the churned one, as the control that tells a churn-dependent failure apart.
pub fn arb_churn(
    tables: &[(&str, usize)],
    columns: &[Column],
    enabled: bool,
) -> impl Strategy<Value = Churn> + use<> {
    let shape = InsertShape::new(columns);
    if !enabled || !churn_enabled() {
        return Just(Churn::none(shape)).boxed();
    }
    (
        arb_committed(tables, columns),
        proptest::collection::vec(arb_any(tables, columns), 0..=3),
        proptest::array::uniform2(arb_session()),
    )
        .prop_map(move |(committed, own, sessions)| Churn {
            committed,
            own,
            sessions,
            shape: shape.clone(),
        })
        .boxed()
}

impl Churn {
    pub fn none(shape: InsertShape) -> Self {
        Self {
            committed: Vec::new(),
            own: Vec::new(),
            sessions: [Session {
                seed: 0.0,
                mutable_segment_rows: None,
            }; 2],
            shape,
        }
    }

    /// Runs the committed part on a pooled session and returns the per-case [`SetupScript`]:
    /// the base script plus the log so far as its SQL, and the uncommitted part for the
    /// comparison helpers to frame the case with.
    pub fn apply(
        &self,
        pool: &MutexObjectPool<PgConnection>,
        base: &SetupScript,
    ) -> Result<SetupScript, TestCaseError> {
        self.commit(pool, base)?;
        // The log stays out of `sql`: `repro_setup_sql` appends it at render time.
        let mut setup = SetupScript {
            sql: base.sql.clone(),
            tables: base.tables.clone(),
            qgen_seed: base.qgen_seed,
            churn_log: base.churn_log.clone(),
            case_churn: None,
        };
        let case = self.case_churn();
        if !case.own.is_empty() {
            setup.case_churn = Some(case);
        }
        Ok(setup)
    }

    /// The committed phase. `VACUUM` refuses a transaction block, so the phase runs as batches
    /// split at each vacuum, in the order drawn: a vacuum drawn first reclaims what earlier cases
    /// left, and the inserts after it reuse the space. Each group joins the churn log as soon as
    /// it lands, so a failure halfway through still leaves a script that rebuilds the heap.
    fn commit(
        &self,
        pool: &MutexObjectPool<PgConnection>,
        base: &SetupScript,
    ) -> Result<(), TestCaseError> {
        if self.committed.is_empty() {
            return Ok(());
        }
        let session = self.sessions[0];
        let mut preamble = session.session_statements();
        let mut dml: Vec<String> = Vec::new();

        for mutation in &self.committed {
            let statements = mutation.statements(&self.shape);
            if !mutation.is_vacuum() {
                dml.extend(statements);
                continue;
            }
            if !dml.is_empty() {
                self.run_batch(pool, base, &mut preamble, &session, &mut dml)?;
            }
            for vacuum in statements {
                run_retrying(pool, base, "qgen churn vacuum", &vacuum)?;
                append_to_log(base, &vacuum);
            }
        }
        if !dml.is_empty() {
            self.run_batch(pool, base, &mut preamble, &session, &mut dml)?;
        }
        Ok(())
    }

    /// One transaction of the committed phase. The session-level settings and the seed go in the
    /// first batch only: re-seeding would hand the batches after a vacuum the same rows.
    fn run_batch(
        &self,
        pool: &MutexObjectPool<PgConnection>,
        base: &SetupScript,
        preamble: &mut Vec<String>,
        session: &Session,
        dml: &mut Vec<String>,
    ) -> Result<(), TestCaseError> {
        let mut statements = std::mem::take(preamble);
        statements.extend(session.local_statements());
        statements.append(dml);
        let batch = format!("BEGIN;\n{}\nCOMMIT;", statements.join("\n"));
        run_retrying(pool, base, "qgen churn", &batch)?;
        append_to_log(base, &batch);
        Ok(())
    }

    fn case_churn(&self) -> CaseChurn {
        let mut own = Vec::new();
        if !self.own.is_empty() {
            own.extend(self.sessions[1].session_statements());
            own.extend(self.sessions[1].local_statements());
            for mutation in &self.own {
                own.extend(mutation.statements(&self.shape));
            }
            own.extend(Session::closing_statements());
        }
        CaseChurn { own }
    }
}

fn run_retrying(
    pool: &MutexObjectPool<PgConnection>,
    base: &SetupScript,
    what: &str,
    sql: &str,
) -> Result<(), TestCaseError> {
    let outcome = retry_transient(pool, what, |conn| {
        let result = sql.execute_result(conn);
        if result.is_err() {
            // A failed batch leaves the session in an aborted transaction; clear it so the
            // connection goes back to the pool usable.
            let _ = "ROLLBACK;".execute_result(conn);
        }
        sql_attempt(result)
    });
    match outcome {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(handle_setup_error(base, what, &e.to_string(), sql)),
        Err(RetryError::TimedOutUnderPause(e)) => Err(handle_setup_error(
            base,
            what,
            &format!("timed out while faults were paused: {e}"),
            sql,
        )),
        Err(RetryError::GraceExpired(reason)) => Err(handle_setup_error(base, what, &reason, sql)),
    }
}

/// Every statement the churn commits joins the shared log, since later cases query the heap it
/// leaves and a failure script has to rebuild it.
fn append_to_log(base: &SetupScript, sql: &str) {
    let mut log = base
        .churn_log
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    log.push_str("-- churn: committed before the case\n");
    log.push_str(sql.trim_end());
    log.push('\n');
}

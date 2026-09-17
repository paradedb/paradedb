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
//! Each case draws a [`Churn`] with up to three parts:
//!
//! 1. `committed`: mutations committed on the case's own session before either side runs. They
//!    pile up over the run, so later cases query heaps with dead tuples, HOT chains, cleared
//!    visibility-map bits, reclaimed and reused ctids, and index segments that still carry docs
//!    for rows that are gone.
//! 2. `own`: mutations left uncommitted in the transaction that runs both sides, rolled back once
//!    the case is over. Both sides see the session's own writes; the rollback then leaves aborted
//!    tuples behind for the cases after it.
//! 3. `concurrent`: mutations a second session holds open across both sides, either rolled back
//!    afterwards (in-progress rows from another backend) or committed between the two sides
//!    under `REPEATABLE READ` (the candidate reads an index that already holds rows its snapshot
//!    must exclude).
//!
//! The oracle stays sound because both sides of a case run under one snapshot. The index, on the
//! other hand, sees every one of these rows and has to filter them.
//!
//! Row picks and written values come from Postgres `random()`, re-seeded per phase from proptest,
//! so a case replays under the same `PROPTEST_RNG_SEED`. `PARADEDB_QGEN_CHURN=off` disables the
//! whole thing, to tell a churn-dependent failure from a plain query bug.

use std::fmt::Write;
use std::sync::OnceLock;

use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use sqlx::PgConnection;

use super::{Column, PgGucs, SetupScript};
use crate::fixtures::db::Query;
use crate::fixtures::fault_grace::{RetryError, retry_transient, sql_attempt};

/// Which rows a mutation may touch. The two transactions a case holds open take disjoint halves
/// of the key space so neither waits on the other's row locks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdScope {
    Any,
    Odd,
    Even,
}

impl IdScope {
    fn predicate(self) -> &'static str {
        match self {
            IdScope::Any => "TRUE",
            IdScope::Odd => "id % 2 = 1",
            IdScope::Even => "id % 2 = 0",
        }
    }
}

/// The `INSERT` column list and the matching random value list, derived once from the schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsertShape {
    pub columns: String,
    pub generators: String,
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
    /// again. Truncation takes a conditional exclusive lock, so it stays off while another
    /// transaction of the case holds the table.
    Vacuum {
        table: String,
        truncate: bool,
    },
}

impl Mutation {
    fn table(&self) -> &str {
        match self {
            Mutation::Insert { table, .. }
            | Mutation::Delete { table, .. }
            | Mutation::Replace { table, .. }
            | Mutation::Update { table, .. }
            | Mutation::Vacuum { table, .. } => table,
        }
    }

    pub fn is_vacuum(&self) -> bool {
        matches!(self, Mutation::Vacuum { .. })
    }

    pub fn statements(&self, scope: IdScope, shape: &InsertShape) -> Vec<String> {
        let pick = |table: &str, rows: usize| {
            format!(
                "id IN (SELECT id FROM {table} WHERE {} ORDER BY random() LIMIT {rows})",
                scope.predicate()
            )
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

/// What the second session does with its open transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fate {
    /// Rolled back once the case is over: both sides saw in-progress rows from another backend.
    Rollback,
    /// Committed after the baseline ran, so the candidate reads an index holding rows its
    /// snapshot excludes. With `vacuum`, the dead ones are also reclaimed before the candidate
    /// runs.
    CommitBetween { vacuum: bool },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Concurrent {
    pub mutations: Vec<Mutation>,
    pub fate: Fate,
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
    /// previous case left the custom-scan GUCs at. The mutable-segment setting is `SET LOCAL`
    /// so the enclosing transaction's end restores it.
    fn statements(&self) -> Vec<String> {
        let mut statements = vec![
            PgGucs::pg_search_disabled().set(),
            format!("SELECT setseed({});", self.seed),
        ];
        match self.mutable_segment_rows {
            Some(rows) => statements.push(format!(
                "SET LOCAL paradedb.global_mutable_segment_rows TO {rows};"
            )),
            None => statements.push("RESET paradedb.global_mutable_segment_rows;".to_string()),
        }
        statements
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Churn {
    pub committed: Vec<Mutation>,
    pub own: Vec<Mutation>,
    pub concurrent: Option<Concurrent>,
    pub sessions: [Session; 3],
    pub shape: InsertShape,
}

/// The uncommitted part of one case's churn, carried by the per-case [`SetupScript`] so the
/// comparison helpers can frame the case with it. Every field is plain SQL: the framing is
/// what `compare_outcome_on` does with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseChurn {
    /// Opens the case's own transaction on the comparison session.
    pub begin: String,
    /// Run on the comparison session after `begin`, before either side.
    pub own: Vec<String>,
    /// Run by the second session inside its own transaction, before either side.
    pub concurrent: Vec<String>,
    pub fate: Option<Fate>,
    /// Run by the second session right after committing between the sides.
    pub between: Vec<String>,
}

impl CaseChurn {
    /// Whether the second session commits between the two sides.
    pub fn commits_between(&self) -> bool {
        matches!(self.fate, Some(Fate::CommitBetween { .. }))
    }

    /// The case block of a reproduction script. The comparison queries follow it, inside the
    /// transaction it opens. A second session's rolled-back churn is replayed as a rolled-back
    /// block up front: that rebuilds the heap it left behind, which is what later cases see too,
    /// though not the in-progress timing the failing case itself saw.
    pub fn repro_sql(&self) -> String {
        let mut sql = String::new();
        match self.fate {
            Some(Fate::CommitBetween { .. }) if !self.concurrent.is_empty() => {
                writeln!(
                    sql,
                    "-- Second session, opened before the queries below and committed between \
                     them (its statements are in the churn log above once committed):"
                )
                .unwrap();
                writeln!(sql, "--   BEGIN;").unwrap();
                for line in self.concurrent.iter().flat_map(|s| s.lines()) {
                    writeln!(sql, "--   {line}").unwrap();
                }
                writeln!(sql, "--   COMMIT;").unwrap();
                for line in self.between.iter().flat_map(|s| s.lines()) {
                    writeln!(sql, "--   {line}").unwrap();
                }
            }
            Some(_) if !self.concurrent.is_empty() => {
                writeln!(
                    sql,
                    "-- Second session, held open across both queries below and rolled back \
                     after them; replayed here as rolled back beforehand:"
                )
                .unwrap();
                sql.push_str(&rolled_back_block(&self.concurrent));
            }
            _ => {}
        }
        writeln!(
            sql,
            "-- Case transaction; both queries below run inside it and it is rolled back after:"
        )
        .unwrap();
        writeln!(sql, "{}", self.begin).unwrap();
        for statement in &self.own {
            writeln!(sql, "{statement}").unwrap();
        }
        sql
    }

    /// What the case leaves behind for the cases after it: its rolled-back transactions. Appended
    /// to the churn log once the case is over, so a later failure's script rebuilds the aborted
    /// tuples and index entries too.
    pub fn log_rolled_back(&self, log: &mut String, peer_rolled_back: bool) {
        if !self.own.is_empty() {
            log.push_str("-- churn: the case transaction, rolled back after the case\n");
            log.push_str(&rolled_back_block(&self.own));
        }
        if peer_rolled_back && !self.concurrent.is_empty() {
            log.push_str("-- churn: a second session's transaction, rolled back after the case\n");
            log.push_str(&rolled_back_block(&self.concurrent));
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

fn arb_concurrent(
    tables: &[(&str, usize)],
    columns: &[Column],
) -> impl Strategy<Value = Concurrent> + use<> {
    let rollback =
        proptest::collection::vec(arb_any(tables, columns), 1..=3).prop_map(|mutations| {
            Concurrent {
                mutations,
                fate: Fate::Rollback,
            }
        });
    let commit = (
        proptest::collection::vec(arb_size_neutral(tables, columns), 1..=3),
        any::<bool>(),
    )
        .prop_map(|(mutations, vacuum)| Concurrent {
            mutations,
            fate: Fate::CommitBetween { vacuum },
        });
    prop_oneof![rollback, commit]
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
/// [`super::generated_queries_setup`]) with the schema in `columns`. Empty parts are as likely
/// as not, so pristine tables stay in the mix.
pub fn arb_churn(
    tables: &[(&str, usize)],
    columns: &[Column],
) -> impl Strategy<Value = Churn> + use<> {
    let shape = InsertShape::new(columns);
    if !churn_enabled() {
        return Just(Churn {
            committed: Vec::new(),
            own: Vec::new(),
            concurrent: None,
            sessions: [Session {
                seed: 0.0,
                mutable_segment_rows: None,
            }; 3],
            shape,
        })
        .boxed();
    }
    (
        arb_committed(tables, columns),
        proptest::collection::vec(arb_any(tables, columns), 0..=3),
        proptest::option::of(arb_concurrent(tables, columns)),
        proptest::array::uniform3(arb_session()),
    )
        .prop_map(move |(committed, own, concurrent, sessions)| Churn {
            committed,
            own,
            concurrent,
            sessions,
            shape: shape.clone(),
        })
        .boxed()
}

impl Churn {
    /// Runs the committed part on a pooled session, appends it to the shared churn log, and
    /// returns the per-case [`SetupScript`]: the base script plus the log so far as its SQL, and
    /// the uncommitted parts for the comparison helpers to frame the case with.
    pub fn apply(
        &self,
        pool: &MutexObjectPool<PgConnection>,
        base: &SetupScript,
    ) -> Result<SetupScript, TestCaseError> {
        let committed_sql = self.commit(pool)?;
        if !committed_sql.is_empty() {
            base.churn_log
                .lock()
                .expect("churn log lock should not be poisoned")
                .push_str(&committed_sql);
        }
        // The log stays out of `sql`: `repro_setup_sql` appends it at render time.
        let mut setup = SetupScript {
            sql: base.sql.clone(),
            tables: base.tables.clone(),
            qgen_seed: base.qgen_seed,
            churn_log: base.churn_log.clone(),
            case_churn: None,
        };
        let case = self.case_churn();
        if !case.own.is_empty() || !case.concurrent.is_empty() {
            setup.case_churn = Some(case);
        }
        Ok(setup)
    }

    /// The committed phase: one transaction for the DML (a fault mid-way rolls back cleanly and
    /// the retry starts over), then any `VACUUM`s on their own, since they refuse a transaction
    /// block. Returns the SQL that ran, for the log.
    fn commit(&self, pool: &MutexObjectPool<PgConnection>) -> Result<String, TestCaseError> {
        if self.committed.is_empty() {
            return Ok(String::new());
        }
        let session = self.sessions[0];
        let mut dml: Vec<String> = session.statements();
        let mut vacuums = Vec::new();
        for mutation in &self.committed {
            let statements = mutation.statements(IdScope::Any, &self.shape);
            if mutation.is_vacuum() {
                vacuums.extend(statements);
            } else {
                dml.extend(statements);
            }
        }

        let mut log = String::from("-- churn: committed before the case\n");
        let batch = format!("BEGIN;\n{}\nCOMMIT;", dml.join("\n"));
        run_retrying(pool, "qgen churn", &batch)?;
        writeln!(log, "{batch}").unwrap();
        for vacuum in vacuums {
            run_retrying(pool, "qgen churn vacuum", &vacuum)?;
            writeln!(log, "{vacuum}").unwrap();
        }
        Ok(log)
    }

    fn case_churn(&self) -> CaseChurn {
        let fate = self.concurrent.as_ref().map(|c| c.fate);
        let isolation = match fate {
            // One snapshot for both sides is what makes a commit in between harmless.
            Some(Fate::CommitBetween { .. }) => " ISOLATION LEVEL REPEATABLE READ",
            _ => "",
        };
        let mut own = Vec::new();
        if !self.own.is_empty() {
            own.extend(self.sessions[1].statements());
            for mutation in &self.own {
                own.extend(mutation.statements(IdScope::Odd, &self.shape));
            }
        }
        let mut concurrent = Vec::new();
        let mut between = Vec::new();
        if let Some(c) = &self.concurrent {
            concurrent.extend(self.sessions[2].statements());
            for mutation in &c.mutations {
                concurrent.extend(mutation.statements(IdScope::Even, &self.shape));
            }
            if let Fate::CommitBetween { vacuum: true } = c.fate {
                let mut tables: Vec<&str> = c.mutations.iter().map(Mutation::table).collect();
                tables.sort_unstable();
                tables.dedup();
                for table in tables {
                    between.push(format!("VACUUM (TRUNCATE false) {table};"));
                }
            }
        }
        CaseChurn {
            begin: format!("BEGIN{isolation};"),
            own,
            concurrent,
            fate,
            between,
        }
    }
}

fn run_retrying(
    pool: &MutexObjectPool<PgConnection>,
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
        Ok(Err(e)) => Err(TestCaseError::fail(format!("{what} failed: {e}\n{sql}"))),
        Err(RetryError::TimedOutUnderPause(e)) => Err(TestCaseError::fail(format!(
            "{what} timed out while faults were paused: {e}\n{sql}"
        ))),
        Err(RetryError::GraceExpired(reason)) => Err(TestCaseError::fail(reason)),
    }
}

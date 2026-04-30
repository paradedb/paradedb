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

//! This module fakes the behavior of pg17+'s `aminsertcleanup` by hooking the executor's "run",
//! "finish", and "process utility" hooks.
#![allow(static_mut_refs)]

use crate::api::HashMap;
use crate::postgres::insert::{insertcleanup, InsertMode, InsertState};
use pgrx::pg_sys::{uint64, QueryDesc, ScanDirection};
use pgrx::{pg_guard, pg_sys};
use std::collections::hash_map::Entry;

/// # Stack Nesting Invariant (Issue #4843)
///
/// `EXECUTOR_RUN_STACK` must never have `None` on top of `Some(...)`.
/// When a nested `ExecutorRun` occurs (e.g., from `bingo.cansmiles` via
/// internal C++ SPI calls during expression evaluation), the depth counter
/// is incremented instead of pushing a new slot.
///
/// ```ignore
/// // Before fix (buggy):
/// // outer ExecutorRun  → push(None)              → [None]
/// //   aminsert         → push_insert_state       → [Some({oid→state})]
/// //   nested ExecutorRun → push(None)              → [Some({oid→state}), None]
/// //   get_insert_state → .last_mut() sees None   → unreachable! PANIC
///
/// // After fix:
/// // outer ExecutorRun  → push(None)              → [None]
/// //   aminsert         → push_insert_state       → [Some({oid→state, depth:0})]
/// //   nested ExecutorRun → depth += 1             → [Some({oid→state, depth:1})]
/// //   get_insert_state → .last_mut() sees Some  → OK
/// //   nested Finish    → depth -= 1             → [Some({oid→state, depth:0})]
/// // outer Finish       → depth==0, pop & cleanup → []
/// ```
///
/// This invariant is enforced by the depth counter in `ExecutorRunEntry`
/// and the early-return logic in `executor_run_hook` / `executor_finish_hook`.
#[derive(Default)]
struct ExecutorRunEntry {
    active: HashMap<pg_sys::Oid, InsertState>,
    depth: u32,
}

static mut EXECUTOR_RUN_STACK: Vec<Option<ExecutorRunEntry>> = Vec::new();

#[inline]
pub unsafe fn get_insert_state(indexrelid: pg_sys::Oid) -> Option<&'static mut InsertState> {
    let entry = EXECUTOR_RUN_STACK.last_mut().expect(
        "get_insert_state: ExecutorRunEntry should have already been pushed onto the stack",
    );
    match entry {
        Some(state) => state.active.get_mut(&indexrelid),
        None => unreachable!("get_insert_state: tried to get the InsertState for relation {indexrelid:?} but it was never pushed"),
    }
}

#[inline]
pub unsafe fn push_insert_state(insert_state: InsertState) {
    if EXECUTOR_RUN_STACK.is_empty() {
        pgrx::register_xact_callback(pgrx::PgXactCallbackEvent::Abort, || {
            EXECUTOR_RUN_STACK.clear();
        });
    }

    let entry = EXECUTOR_RUN_STACK
        .last_mut()
        .expect(
            "push_insert_state: ExecutorRunEntry should have already been pushed onto the stack",
        )
        .get_or_insert_default();
    match entry.active.entry(insert_state.indexrelid) {
        Entry::Vacant(slot) => slot.insert(insert_state),

        // this shouldn't ever happen unless there's some logic bug between here and `postgres/insert.rs`
        Entry::Occupied(_) => unreachable!(
            "already have an ExecutorRunEntry for index {:?}",
            insert_state.indexrelid
        ),
    };
}

pub unsafe fn register() {
    static mut PREV_PROCESS_UTILITY_HOOK: pg_sys::ProcessUtility_hook_type = None;
    static mut PREV_EXECUTOR_RUN_HOOK: pg_sys::ExecutorRun_hook_type = None;
    static mut PREV_EXECUTOR_FINISH_HOOK: pg_sys::ExecutorFinish_hook_type = None;

    PREV_PROCESS_UTILITY_HOOK = pg_sys::ProcessUtility_hook;
    pg_sys::ProcessUtility_hook = Some(process_utility_hook);

    PREV_EXECUTOR_RUN_HOOK = pg_sys::ExecutorRun_hook;
    #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
    {
        pg_sys::ExecutorRun_hook = Some(executor_run_hook_pg15_17);
    }
    #[cfg(feature = "pg18")]
    {
        pg_sys::ExecutorRun_hook = Some(executor_run_hook_pg18);
    }

    PREV_EXECUTOR_FINISH_HOOK = pg_sys::ExecutorFinish_hook;
    pg_sys::ExecutorFinish_hook = Some(executor_finish_hook);

    #[allow(clippy::too_many_arguments)]
    #[rustfmt::skip]
    #[pg_guard]
    unsafe extern "C-unwind" fn process_utility_hook(
        pstmt: *mut pg_sys::PlannedStmt,
        query_string: *const ::core::ffi::c_char,
        read_only_tree: bool,
        context: pg_sys::ProcessUtilityContext::Type,
        params: pg_sys::ParamListInfo,
        query_env: *mut pg_sys::QueryEnvironment,
        dest: *mut pg_sys::DestReceiver,
        qc: *mut pg_sys::QueryCompletion,
    ) {
        EXECUTOR_RUN_STACK.push(None);
        if let Some(prev_hook) = PREV_PROCESS_UTILITY_HOOK {
            prev_hook(pstmt, query_string, read_only_tree, context, params, query_env, dest, qc);
        } else {
            pg_sys::standard_ProcessUtility(pstmt, query_string, read_only_tree, context, params, query_env, dest, qc)
        }

        aminsertcleanup_stack();
    }

    #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
    #[pg_guard]
    unsafe extern "C-unwind" fn executor_run_hook_pg15_17(
        query_desc: *mut QueryDesc,
        direction: ScanDirection::Type,
        count: uint64,
        execute_once: bool,
    ) {
        if let Some(Some(entry)) = EXECUTOR_RUN_STACK.last_mut() {
            entry.depth += 1;
            if let Some(prev_hook) = PREV_EXECUTOR_RUN_HOOK {
                prev_hook(query_desc, direction, count, execute_once);
            } else {
                pg_sys::standard_ExecutorRun(query_desc, direction, count, execute_once);
            }
            return;
        }

        EXECUTOR_RUN_STACK.push(None);
        if let Some(prev_hook) = PREV_EXECUTOR_RUN_HOOK {
            prev_hook(query_desc, direction, count, execute_once);
        } else {
            pg_sys::standard_ExecutorRun(query_desc, direction, count, execute_once);
        }
    }

    #[cfg(feature = "pg18")]
    #[pg_guard]
    unsafe extern "C-unwind" fn executor_run_hook_pg18(
        query_desc: *mut QueryDesc,
        direction: ScanDirection::Type,
        count: uint64,
    ) {
        if let Some(Some(entry)) = EXECUTOR_RUN_STACK.last_mut() {
            entry.depth += 1;
            if let Some(prev_hook) = PREV_EXECUTOR_RUN_HOOK {
                prev_hook(query_desc, direction, count);
            } else {
                pg_sys::standard_ExecutorRun(query_desc, direction, count);
            }
            return;
        }

        EXECUTOR_RUN_STACK.push(None);
        if let Some(prev_hook) = PREV_EXECUTOR_RUN_HOOK {
            prev_hook(query_desc, direction, count);
        } else {
            pg_sys::standard_ExecutorRun(query_desc, direction, count);
        }
    }

    #[pg_guard]
    unsafe extern "C-unwind" fn executor_finish_hook(query_desc: *mut pg_sys::QueryDesc) {
        if let Some(Some(entry)) = EXECUTOR_RUN_STACK.last_mut() {
            if entry.depth > 0 {
                entry.depth -= 1;
                if let Some(prev_hook) = PREV_EXECUTOR_FINISH_HOOK {
                    prev_hook(query_desc);
                } else {
                    pg_sys::standard_ExecutorFinish(query_desc);
                }
                return;
            }
        }

        aminsertcleanup_stack();
        if let Some(prev_hook) = PREV_EXECUTOR_FINISH_HOOK {
            prev_hook(query_desc);
        } else {
            pg_sys::standard_ExecutorFinish(query_desc);
        }
    }

    unsafe fn aminsertcleanup_stack() -> Option<()> {
        let entry = EXECUTOR_RUN_STACK.pop()??;
        for (_, mut insert_state) in entry.active {
            let mode = std::mem::replace(&mut insert_state.mode, InsertMode::Completed);
            insertcleanup(&insert_state, mode);
        }
        None
    }
}
#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;

    /// A dummy pg_extern that executes an SPI query internally.
    ///
    /// When this function is used as a BM25 index expression, Postgres evaluates
    /// it during aminsert. The internal Spi::run call fires its own ExecutorRun,
    /// which — before the depth-counter fix — would push a new None onto
    /// EXECUTOR_RUN_STACK and cause get_insert_state to hit unreachable!.
    ///
    /// This function must be #[pg_extern] (not just a Rust fn) so that the pgrx
    /// schema generator emits its SQL and Postgres can reference it by name in
    /// the index expression.
    #[pg_extern(immutable, strict)]
    fn spi_identity(input: &str) -> String {
        // This Spi::run internally calls SPI_execute, which triggers
        // ExecutorRun_hook — exactly the nesting that caused #4843.
        Spi::run("SELECT 1").expect("spi_identity: inner SPI query failed");
        input.to_string()
    }

    /// Regression test for issue #4843.
    ///
    /// Verifies that inserting multiple rows into a table whose BM25 index
    /// contains an expression that triggers a nested ExecutorRun (via SPI)
    /// does not panic with "entered unreachable code: get_insert_state".
    ///
    /// # How the nesting is triggered
    ///
    /// 1. Outer INSERT → ExecutorRun_hook fires → push(None) onto stack.
    /// 2. aminsert evaluates the index expression `spi_identity(val)`.
    /// 3. Inside spi_identity, Spi::run("SELECT 1") fires another
    ///    ExecutorRun_hook (the nested one).
    /// 4. The depth counter in ExecutorRunEntry absorbs the nested call
    ///    instead of pushing a new None that would shadow the outer Some.
    /// 5. The nested ExecutorFinish decrements depth and returns early.
    /// 6. The outer ExecutorFinish sees depth == 0 and calls aminsertcleanup.
    ///
    /// Without the fix, step 4 would push None and step 4's get_insert_state
    /// would hit unreachable!, crashing the backend.
    #[pg_test]
    fn test_nested_executor_run_does_not_panic() {
        // Setup
        Spi::run(
            "CREATE TABLE nested_exec_test (
            id  SERIAL PRIMARY KEY,
            val TEXT NOT NULL
        );",
        )
        .expect("failed to create test table");

        // The expression index calls spi_identity(), which internally runs
        // an SPI query, reproducing the bingo.cansmiles nesting pattern.
        Spi::run(
            "CREATE INDEX nested_exec_test_bm25_idx
           ON nested_exec_test
           USING bm25 (id, (tests.spi_identity(val)::pdb.simple))
           WITH (key_field = 'id');",
        )
        .expect("failed to create BM25 expression index");

        // This is the actual regression guard for #4843.
        // Before the depth-counter fix, this panicked with:
        //   "entered unreachable code: get_insert_state: tried to get the
        //    InsertState for relation <oid> but it was never pushed"
        //
        // The multi-row INSERT evaluates spi_identity() per row, each
        // evaluation fires Spi::run() internally, which triggers a nested
        // ExecutorRun_hook while the outer stack slot is still live.
        Spi::run(
            "INSERT INTO nested_exec_test (val)
         SELECT 'row_' || g
         FROM generate_series(1, 5) AS g;",
        )
        .expect("multi-row insert panicked — nested ExecutorRun bug (#4843) has regressed");

        // Sanity-check: rows actually landed in the table.
        let count = Spi::get_one::<i64>("SELECT COUNT(*) FROM nested_exec_test;")
            .expect("COUNT query failed")
            .expect("COUNT returned NULL");

        assert_eq!(count, 5, "expected 5 rows, got {count}");
    }
}

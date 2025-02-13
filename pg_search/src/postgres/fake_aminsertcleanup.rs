// Copyright (c) 2023-2025 Retake, Inc.
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

use crate::postgres::insert::{paradedb_aminsertcleanup, InsertState};
use pgrx::pg_sys::{uint64, QueryDesc, ScanDirection};
use pgrx::{pg_guard, pg_sys};
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;

#[derive(Default)]
struct ExecutorRunEntry {
    active: FxHashMap<pg_sys::Oid, InsertState>,
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
    pg_sys::ExecutorRun_hook = Some(executor_run_hook);

    PREV_EXECUTOR_FINISH_HOOK = pg_sys::ExecutorFinish_hook;
    pg_sys::ExecutorFinish_hook = Some(executor_finish_hook);

    #[cfg(not(feature = "pg13"))]
    #[allow(clippy::too_many_arguments)]
    #[rustfmt::skip]
    #[pg_guard]
    unsafe extern "C" fn process_utility_hook(
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

    #[cfg(feature = "pg13")]
    #[allow(clippy::too_many_arguments)]
    #[rustfmt::skip]
    #[pg_guard]
    unsafe extern "C" fn process_utility_hook(
        pstmt: *mut pg_sys::PlannedStmt,
        query_string: *const ::core::ffi::c_char,
        context: pg_sys::ProcessUtilityContext::Type,
        params: pg_sys::ParamListInfo,
        query_env: *mut pg_sys::QueryEnvironment,
        dest: *mut pg_sys::DestReceiver,
        qc: *mut pg_sys::QueryCompletion,
    ) {
        EXECUTOR_RUN_STACK.push(None);
        if let Some(prev_hook) = PREV_PROCESS_UTILITY_HOOK {
            prev_hook(pstmt, query_string, context, params, query_env, dest, qc);
        } else {
            pg_sys::standard_ProcessUtility(pstmt, query_string, context, params, query_env, dest, qc)
        }

        aminsertcleanup_stack();
    }

    #[pg_guard]
    unsafe extern "C" fn executor_run_hook(
        query_desc: *mut QueryDesc,
        direction: ScanDirection::Type,
        count: uint64,
        execute_once: bool,
    ) {
        EXECUTOR_RUN_STACK.push(None);
        pg_sys::standard_ExecutorRun(query_desc, direction, count, execute_once);
    }

    #[pg_guard]
    unsafe extern "C" fn executor_finish_hook(query_desc: *mut pg_sys::QueryDesc) {
        aminsertcleanup_stack();
        if let Some(prev_hook) = PREV_EXECUTOR_FINISH_HOOK {
            prev_hook(query_desc);
        } else {
            pg_sys::standard_ExecutorFinish(query_desc);
        }
    }

    unsafe fn aminsertcleanup_stack() -> Option<()> {
        let entry = EXECUTOR_RUN_STACK
            .pop()
            .expect("should have an ExecutorRuntimeState entry")?;
        for (_, insert_state) in entry.active {
            paradedb_aminsertcleanup(insert_state.writer);
        }
        None
    }
}

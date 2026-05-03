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

//! Polyfill for pg17+'s `aminsertcleanup` on pg15/pg16.
//!
//! # How it works
//!
//! Postgres 17 added `aminsertcleanup`, which lets an index AM defer work until after
//! `ExecutorFinish`. On pg15/pg16 we approximate this by hooking three executor entry points:
//! `ExecutorRun`, `ExecutorFinish`, and `ProcessUtility`.
//!
//! Each hook invocation that can produce `aminsert` calls (i.e. every `ExecutorRun` and every
//! `ProcessUtility`) pushes a fresh [`InsertFrame`] onto [`EXECUTOR_RUN_STACK`] via a
//! [`FrameGuard`]. The guard's `Drop` impl runs `insertcleanup` for every [`InsertState`]
//! accumulated in that frame, then pops it off the stack — automatically, whether the hook
//! returns normally or unwinds.
//!
//! `ExecutorFinish` no longer does any cleanup itself; it only chains to the previous hook.
//! Cleanup for the matching `ExecutorRun` has already happened when that hook returned and
//! its [`FrameGuard`] was dropped, which is always before `ExecutorFinish` is called.
//!
//! # Stack invariants
//!
//! * `EXECUTOR_RUN_STACK.len()` equals the number of live [`FrameGuard`] instances. Each guard
//!   pushes exactly one frame on creation and pops exactly one frame on drop.
//!
//! * `push_insert_state` always targets the **top** frame. `aminsert` is always called from
//!   within the hook invocation that created its frame, so the top of the stack is always the
//!   correct frame.
//!
//! * Two `aminsert` calls for the **same** `indexrelid` within the **same** hook invocation
//!   cannot occur. Nested DML (recursive triggers, SPI inserts) fires a new `ExecutorRun` hook,
//!   which pushes a new frame via a new [`FrameGuard`]. The inner frame is independent of the
//!   outer one, so there is no `HashMap` collision even when the same index is targeted at both
//!   nesting levels.
//!
//! * Frames may be empty. A `ProcessUtility` invocation that contains no DML will push and pop
//!   a frame whose `active` map is empty; that frame's drop is a no-op.
//!
//! * **What is NOT guaranteed**: the stack is not "all non-empty below all empty". A
//!   `ProcessUtility` hook can push a frame on top of an active `ExecutorRun` frame. This is
//!   fine — each frame is independent and cleaned up by its own guard.
//!
//! # Panic / unwind safety
//!
//! `insertcleanup` can panic (e.g. if called with `InsertMode::Completed`, or via `.expect()`
//! calls inside the inner cleanup functions). If `FrameGuard::drop` ran cleanup during an
//! already-active unwind, a second panic would abort the process. To prevent this, `Drop` checks
//! `std::thread::panicking()` and skips cleanup when already unwinding. The transaction-abort
//! xact callback registered in `push_insert_state` clears the stack in that case, which is safe
//! because Postgres rolls back all storage changes on error anyway.

#![allow(static_mut_refs)]

use crate::api::HashMap;
use crate::postgres::insert::{insertcleanup, InsertMode, InsertState};
use pgrx::pg_sys::{uint64, QueryDesc, ScanDirection};
use pgrx::{pg_guard, pg_sys};
use std::collections::hash_map::Entry;

// ---------------------------------------------------------------------------
// Stack storage
// ---------------------------------------------------------------------------

/// One nesting level's worth of in-progress index insert states.
///
/// Pushed onto [`EXECUTOR_RUN_STACK`] by [`FrameGuard::new`] and popped by [`FrameGuard::drop`].
/// Starts empty; `aminsert` calls populate it via [`push_insert_state`].
struct InsertFrame {
    active: HashMap<pg_sys::Oid, InsertState>,
}

/// The executor hook nesting stack.
///
/// Each live [`FrameGuard`] corresponds to exactly one element in this `Vec`.
/// **Only [`FrameGuard`] is allowed to push or pop this stack.**
static mut EXECUTOR_RUN_STACK: Vec<InsertFrame> = Vec::new();

// ---------------------------------------------------------------------------
// RAII frame guard
// ---------------------------------------------------------------------------

/// Pushes a fresh [`InsertFrame`] onto [`EXECUTOR_RUN_STACK`] when created, and pops + cleans
/// it up when dropped.
///
/// This is the **only** mechanism that may push or pop the stack.  One `FrameGuard` is created
/// at the top of each executor hook function that can receive `aminsert` calls and bound to a
/// `let _frame` local so it drops at the end of that hook invocation.
struct FrameGuard;

impl FrameGuard {
    /// Push a new empty frame onto the stack.
    ///
    /// # Safety
    /// Must be called from within a Postgres executor hook (main thread, valid memory context).
    unsafe fn new() -> Self {
        // Register the abort callback the first time we touch the stack in this transaction so
        // that a Postgres ERROR during cleanup cannot leak frames across transactions.
        // `is_empty()` prevents re-registering on every nested hook call.
        if EXECUTOR_RUN_STACK.is_empty() {
            pgrx::register_xact_callback(pgrx::PgXactCallbackEvent::Abort, || {
                // We are already inside a Postgres error unwind.  Drop every frame without
                // running insertcleanup — Postgres will roll back all storage changes, so there
                // is nothing to commit.  clear() drops each InsertFrame (and its InsertStates)
                // via their normal Drop impls, which is safe here.
                EXECUTOR_RUN_STACK.clear();
            });
        }

        EXECUTOR_RUN_STACK.push(InsertFrame {
            active: HashMap::default(),
        });

        FrameGuard
    }
}

impl Drop for FrameGuard {
    fn drop(&mut self) {
        // Safety: we are on the Postgres main thread, inside an executor hook whose entry pushed
        // one frame, so the stack is non-empty.
        unsafe {
            if std::thread::panicking() {
                // We are already unwinding.  Calling insertcleanup now risks a second panic,
                // which would abort the process.  Leave the frame in place; the xact-abort
                // callback registered above will clear the stack when the transaction rolls back.
                return;
            }

            let frame = EXECUTOR_RUN_STACK
                .pop()
                .expect("FrameGuard::drop: stack underflow — frame was never pushed");

            for (_, mut insert_state) in frame.active {
                // Replace the mode with Completed *before* calling insertcleanup.  If
                // insertcleanup panics partway through, the xact-abort callback will clear the
                // remaining frames; having Completed in place prevents a double-cleanup if the
                // same state were somehow encountered again (it won't be, but this is defensive).
                let mode = std::mem::replace(&mut insert_state.mode, InsertMode::Completed);
                insertcleanup(&insert_state, mode);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public API — called from postgres/insert.rs :: init_insert_state
// ---------------------------------------------------------------------------

/// Return a mutable reference to the [`InsertState`] for `indexrelid` in the current (top)
/// frame, or `None` if no state has been pushed for that index in this frame yet.
///
/// # Safety
/// Must be called from within a live executor hook invocation (i.e. while a [`FrameGuard`]
/// exists on the call stack).
#[inline]
pub unsafe fn get_insert_state(indexrelid: pg_sys::Oid) -> Option<&'static mut InsertState> {
    EXECUTOR_RUN_STACK
        .last_mut()
        .expect(
            "get_insert_state: called outside of an executor hook — EXECUTOR_RUN_STACK is empty",
        )
        .active
        .get_mut(&indexrelid)
}

/// Insert `insert_state` into the current (top) frame.
///
/// # Panics
/// Panics if called outside of an executor hook (empty stack), or if a state for
/// `insert_state.indexrelid` is already present in the current frame.
///
/// The latter case is `unreachable!` rather than a graceful error because it can only be reached
/// if Postgres calls `aminsert` twice for the same index within a single `ExecutorRun` invocation
/// without an intervening `ExecutorRun` hook call.  Postgres does not do this:
///
/// * Within one executor node, `aminsert` calls are serialised.
/// * Nested DML (recursive triggers, SPI inserts into the same table) always fires a new
///   `ExecutorRun` hook, which pushes a new [`FrameGuard`] → new frame → clean `HashMap`.
///   The inner `aminsert` therefore lands in the inner frame, not the outer one.
///
/// # Safety
/// Must be called from within a live executor hook invocation.
#[inline]
pub unsafe fn push_insert_state(insert_state: InsertState) {
    let frame = EXECUTOR_RUN_STACK.last_mut().expect(
        "push_insert_state: called outside of an executor hook — EXECUTOR_RUN_STACK is empty",
    );

    match frame.active.entry(insert_state.indexrelid) {
        Entry::Vacant(slot) => {
            slot.insert(insert_state);
        }
        Entry::Occupied(_) => unreachable!(
            "push_insert_state: duplicate indexrelid {:?} in the same executor frame. \
             This indicates two aminsert calls for the same index within a single \
             ExecutorRun invocation without an intervening ExecutorRun hook call, \
             which Postgres does not produce.",
            insert_state.indexrelid
        ),
    }
}

// ---------------------------------------------------------------------------
// Hook registration
// ---------------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // ProcessUtility hook
    //
    // DDL and utility statements (COPY, DO blocks, etc.) can contain DML that
    // fires aminsert.  We push a frame for the full duration of the utility
    // statement via FrameGuard so any accumulated InsertStates are cleaned up
    // when the utility statement completes.
    // -----------------------------------------------------------------------
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
        let _frame = FrameGuard::new();

        if let Some(prev_hook) = PREV_PROCESS_UTILITY_HOOK {
            prev_hook(pstmt, query_string, read_only_tree, context, params, query_env, dest, qc);
        } else {
            pg_sys::standard_ProcessUtility(pstmt, query_string, read_only_tree, context, params, query_env, dest, qc);
        }

        // _frame drops here → FrameGuard::drop → insertcleanup for all accumulated states.
    }

    // -----------------------------------------------------------------------
    // ExecutorRun hooks (version-gated by pg feature flag)
    //
    // Every invocation — including nested ones from recursive triggers or SPI
    // inserts — gets its own independent InsertFrame.  Because each nesting
    // level has its own frame, two aminsert calls targeting the same index at
    // different nesting depths land in different HashMaps and never collide.
    // -----------------------------------------------------------------------
    #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
    #[pg_guard]
    unsafe extern "C-unwind" fn executor_run_hook_pg15_17(
        query_desc: *mut QueryDesc,
        direction: ScanDirection::Type,
        count: uint64,
        execute_once: bool,
    ) {
        let _frame = FrameGuard::new();

        if let Some(prev_hook) = PREV_EXECUTOR_RUN_HOOK {
            prev_hook(query_desc, direction, count, execute_once);
        } else {
            pg_sys::standard_ExecutorRun(query_desc, direction, count, execute_once);
        }

        // _frame drops here → FrameGuard::drop → insertcleanup for all accumulated states.
    }

    #[cfg(feature = "pg18")]
    #[pg_guard]
    unsafe extern "C-unwind" fn executor_run_hook_pg18(
        query_desc: *mut QueryDesc,
        direction: ScanDirection::Type,
        count: uint64,
    ) {
        let _frame = FrameGuard::new();

        if let Some(prev_hook) = PREV_EXECUTOR_RUN_HOOK {
            prev_hook(query_desc, direction, count);
        } else {
            pg_sys::standard_ExecutorRun(query_desc, direction, count);
        }

        // _frame drops here → FrameGuard::drop → insertcleanup for all accumulated states.
    }

    // -----------------------------------------------------------------------
    // ExecutorFinish hook
    //
    // This hook no longer has any cleanup responsibility.  By the time Postgres
    // calls ExecutorFinish, ExecutorRun has already returned and its FrameGuard
    // has already run insertcleanup for every InsertState accumulated during
    // that run.  We keep this hook registered solely to maintain the hook chain
    // for any other extensions that installed a hook before us.
    // -----------------------------------------------------------------------
    #[pg_guard]
    unsafe extern "C-unwind" fn executor_finish_hook(query_desc: *mut pg_sys::QueryDesc) {
        if let Some(prev_hook) = PREV_EXECUTOR_FINISH_HOOK {
            prev_hook(query_desc);
        } else {
            pg_sys::standard_ExecutorFinish(query_desc);
        }
    }
}

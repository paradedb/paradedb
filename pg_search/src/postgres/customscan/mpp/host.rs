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

//! Shape-agnostic MPP worker exec dispatcher.
//!
//! Each CustomScan provider that hosts an MPP plan implements [`MppHostState`] on its
//! `CustomScanStateWrapper<T>` to expose its provider-specific state to the shared
//! [`exec_mpp_worker_impl`]. AggregateScan and JoinScan used to keep two ~100-LOC
//! near-duplicate `exec_mpp_worker` functions; the only real differences are where the
//! tokio runtime lives in the typed state and which `SessionContext` profile is used
//! to deserialize the plan. The trait isolates those two concerns; the rest of the
//! body is shared.

use datafusion::execution::context::SessionContext;
use pgrx::pg_sys;

use crate::postgres::customscan::mpp::exec_worker::{run_mpp_worker, MppWorkerInputs};

/// Per-scan glue the shared dispatcher needs to host an MPP worker.
///
/// Implementations live alongside their CustomScan provider (see
/// `aggregatescan::mpp` and `joinscan::mpp`). They own the typed state and know
/// where the runtime slot and `MppExecState::Worker` variant are kept; the trait
/// is the smallest interface that lets [`exec_mpp_worker_impl`] drive the worker
/// without knowing which scan provider it's hosted under.
pub(crate) trait MppHostState {
    /// `true` if a tokio runtime is already installed in this state.
    ///
    /// **Contract:** must return `true` after a prior call to [`Self::install_runtime`]
    /// on the same state. Implementations must check the same slot they wrote in
    /// `install_runtime` — a slot-incoherent impl (write to slot A, check slot B)
    /// would rebuild the runtime on every PG re-entry and crash on the second pass.
    ///
    /// Workers can call `exec_mpp_worker` more than once (PG re-enters scan exec after
    /// EOS); only the first call should build the runtime and drive the plan, so
    /// subsequent calls short-circuit to EOF.
    fn already_drained(&self) -> bool;

    /// Pulls worker-thread inputs out of the typed state. Called exactly once per
    /// worker exec — implementations are expected to `mem::take` `outbound_senders`
    /// out of the `MppExecState::Worker` variant rather than cloning it.
    fn take_worker_inputs(&mut self) -> MppWorkerInputs;

    /// Builds the seed `SessionContext` used only for plan deserialization
    /// (`ctx.task_ctx()`). The distributed planner config (worker resolver, transport,
    /// estimators, codec) is layered on top inside `run_mpp_worker` via
    /// `build_mpp_session_context`. Both procs have to agree on stage shape; this is how.
    fn build_seed_ctx(&self) -> SessionContext;

    /// Installs the tokio runtime in the provider-specific slot and returns a borrowed
    /// reference to it. The runtime must live for the entire body of `run_mpp_worker`,
    /// which is why we return the reference rather than dropping the value back in
    /// after install.
    fn install_runtime(&mut self, runtime: tokio::runtime::Runtime) -> &tokio::runtime::Runtime;
}

/// Shape-agnostic body of `exec_mpp_worker`. Workers emit zero rows back to PG;
/// `null_mut()` signals end-of-stream. Per-scan wrappers should simply call this with
/// `self` after their wrapper-side state checks.
pub(crate) fn exec_mpp_worker_impl<S: MppHostState>(state: &mut S) -> *mut pg_sys::TupleTableSlot {
    if state.already_drained() {
        return std::ptr::null_mut();
    }
    let inputs = state.take_worker_inputs();
    let seed_ctx = state.build_seed_ctx();
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => pgrx::error!("mpp worker: tokio runtime build failed: {e}"),
    };
    // Extending the runtime borrow through `run_mpp_worker` is sound because `inputs`
    // and `seed_ctx` are owned values (no `state` borrow held) and `run_mpp_worker`
    // never reaches back into `state`.
    let runtime = state.install_runtime(runtime);
    run_mpp_worker(inputs, seed_ctx, runtime);
    std::ptr::null_mut()
}

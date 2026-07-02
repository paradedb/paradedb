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

//! Cooperative cancel/die handling for the MPP execution loops.
//!
//! The producers and the leader consumer drive their DataFusion streams under a
//! current-thread `tokio` runtime via `Runtime::block_on`. A backend-die (SIGTERM, which PG
//! sends to the surviving parallel workers as soon as one of them errors) processes through
//! `ProcessInterrupts` into `proc_exit`, which does NOT unwind the Rust stack. Taken from a
//! `CHECK_FOR_INTERRUPTS()` inside `block_on`, that runs transaction cleanup while the tokio
//! runtime is still live on the same stack, the customscan-state drop then drops the
//! mid-flight runtime, and tokio aborts the process. (Query-cancel reaches PG via an
//! `ereport` that pgrx catches and unwinds, so it's already safe; the die path is the one
//! that crashes.)
//!
//! Two pieces cooperate:
//!
//! - [`HeldInterrupts`] wraps `block_on` in PG's `HOLD_INTERRUPTS()` / `RESUME_INTERRUPTS()`,
//!   so a `CHECK_FOR_INTERRUPTS()` anywhere under the runtime (the scanner's own check, a
//!   buffer wait, our drain loops) can't `proc_exit` mid-flight. The hold covers the
//!   subroutine checks our cooperative polling can't reach.
//! - The drain/send loops still poll [`cancel_pending`] and bail with [`interrupted`] so a
//!   pending cancel/die makes `block_on` return promptly instead of running the held query to
//!   completion.
//!
//! `block_on` returns, the runtime goes idle, every fragment future (with its DSM senders)
//! drops while the segment is still mapped, the hold resumes, and the caller calls
//! [`check_for_interrupts`] to let PG act on the cancel/die with nothing left on the stack.

use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::DataFusionError;
use datafusion::execution::SendableRecordBatchStream;
use futures::StreamExt;

/// RAII holdoff for cancel/die, mirroring PG's `HOLD_INTERRUPTS()` / `RESUME_INTERRUPTS()`.
/// While one of these is alive, `ProcessInterrupts` returns without acting, so a backend-die
/// can't `proc_exit` out of the live tokio runtime even if a subroutine runs
/// `CHECK_FOR_INTERRUPTS()`. `Drop` resumes, including when a panic unwinds through `block_on`.
pub(crate) struct HeldInterrupts {
    _not_send: std::marker::PhantomData<*const ()>,
}

impl HeldInterrupts {
    pub(crate) fn hold() -> Self {
        // SAFETY: backend-thread-only increment of the holdoff counter, paired with the
        // decrement in `Drop`. Matches the `HOLD_INTERRUPTS()` macro.
        #[cfg(not(test))]
        unsafe {
            pgrx::pg_sys::InterruptHoldoffCount += 1;
        }
        Self {
            _not_send: std::marker::PhantomData,
        }
    }
}

impl Drop for HeldInterrupts {
    fn drop(&mut self) {
        #[cfg(not(test))]
        unsafe {
            pgrx::pg_sys::InterruptHoldoffCount -= 1;
        }
    }
}

/// True when a query-cancel (SIGINT) or backend-die (SIGTERM) is pending. Reads the signal
/// flags directly so it never enters `ProcessInterrupts`, which could `proc_exit` out of the
/// live runtime.
#[cfg(not(test))]
pub(crate) fn cancel_pending() -> bool {
    // SAFETY: plain reads of the backend's interrupt flags on the backend thread.
    unsafe { pgrx::pg_sys::QueryCancelPending != 0 || pgrx::pg_sys::ProcDiePending != 0 }
}

/// The lib-test binary doesn't link the PG globals; nothing in tests drives a real backend
/// interrupt, so treat it as never pending.
#[cfg(test)]
pub(crate) fn cancel_pending() -> bool {
    false
}

/// Error an MPP loop returns when it bails on a pending cancel/die. The dispatcher discards
/// it and calls [`check_for_interrupts`] once `block_on` has returned.
pub(crate) fn interrupted() -> DataFusionError {
    DataFusionError::Execution("mpp: query interrupted".into())
}

/// Act on the cancel/die that [`HeldInterrupts`] held off and the drain loops bailed on
/// cooperatively, now that `block_on` has returned and the runtime is idle. Acting mid-`block_on`
/// would `proc_exit` out of the live runtime; this is the safe point. Cancel `longjmp`s into PG
/// error handling (caught and unwound by pgrx); die `proc_exit`s. Either way the runtime is off
/// the stack, so the later customscan-state drop tears down cleanly. It's a function so the
/// lib-test build, which doesn't link PG, can stub it to a no-op.
#[cfg(not(test))]
pub(crate) fn check_for_interrupts() {
    pgrx::check_for_interrupts!();
}

#[cfg(test)]
pub(crate) fn check_for_interrupts() {}

/// Drive one batch out of an MPP gather `stream` under the cancel/die holdoff, then service any
/// interrupt the drain deferred. The holdoff wraps the synchronous `block_on`, not the async
/// poll, so it can't move into the `Stream` itself: a `CHECK_FOR_INTERRUPTS` taken while
/// `block_on` drives the runtime would `proc_exit` out of it. Shared by the JoinScan and
/// AggregateScan consumers.
pub(crate) fn block_on_next(
    runtime: &tokio::runtime::Runtime,
    stream: &mut SendableRecordBatchStream,
) -> Option<Result<RecordBatch, DataFusionError>> {
    let next = {
        let _held = HeldInterrupts::hold();
        runtime.block_on(async { stream.next().await })
    };
    check_for_interrupts();
    next
}

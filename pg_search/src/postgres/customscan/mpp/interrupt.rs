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

//! Postgres-integrated Tokio runtime ([`PgTokioRuntime`]) and cooperative interrupt handling.
//!
//! All async DataFusion planning, stream execution, and MPP Tonic/gRPC servers run under
//! [`PgTokioRuntime`], which bridges Postgres's `MyLatch`, asynchronous signal handlers (`SIGINT`,
//! `SIGTERM`, `SIGUSR1`), and `postmaster_alive_fds[POSTMASTER_FD_WATCH]` into Tokio's
//! `current_thread` `mio` reactor without polling timers:
//!
//! 1. [`HeldInterrupts`] wraps `block_on` in Postgres's `HOLD_INTERRUPTS()` /
//!    `RESUME_INTERRUPTS()` (`InterruptHoldoffCount`), so `ProcessInterrupts()` cannot `proc_exit`
//!    out of a live Tokio runtime while `block_on` is on the stack.
//! 2. [`PgTokioRuntime::new`] installs `on_thread_park` and `on_thread_unpark` hooks on the
//!    `current_thread` scheduler that maintain `MyLatch->maybe_sleeping`, arm a signal-safe wakeup
//!    descriptor ([`super::wakeup`]), call `ResetLatch(MyLatch)` when set, and wake all registered
//!    [`futures::task::AtomicWaker`]s whenever a signal or latch event occurs.
//! 3. A background `AsyncFd` task on [`PgTokioRuntime`] watches the backend's wakeup `socketpair`
//!    and `postmaster_alive_fds[POSTMASTER_FD_WATCH]`, synthesizing `ProcDiePending` and waking all
//!    tasks immediately on `postmaster` death.
//! 4. [`PgTokioRuntime::block_on`] and [`block_on_next`] wake immediately when an interrupt arrives
//!    while a future is `Poll::Pending`, exit the inner `tokio::runtime::Runtime::block_on`, drop
//!    [`HeldInterrupts`], and call [`check_for_interrupts`] with Tokio's `block_on` frame off the
//!    stack (resuming the pinned future if the interrupt was a benign parallel worker notice).
//!
//! ## Caveats (`tokio::net`, `spawn_blocking`, and Postgres FFI)
//!
//! - `current_thread` progress: background tasks spawned via [`PgTokioRuntime::spawn`] (such as
//!   HTTP/2 connection drivers or keepalive loops) only advance while [`PgTokioRuntime::block_on`]
//!   is on the stack. Between `ExecCustomScan` calls or across idle periods, the `mio` reactor is
//!   paused and Postgres's own `CHECK_FOR_INTERRUPTS()` / `WaitLatch` is in charge.
//! - `spawn_blocking` and DNS resolution: `tokio::net::lookup_host` (and passing `"host:port"`
//!   strings to `TcpStream::connect`) dispatches synchronous libc `getaddrinfo` onto Tokio's
//!   `spawn_blocking` OS threadpool. Those OS threads must never call into `pgrx` / `pg_sys`, and
//!   [`PgTokioRuntime::drop`] uses `Runtime::shutdown_background()` so an unwinding query cancel
//!   never blocks waiting for a hung `spawn_blocking` thread.
//! - Postgres VFD accounting (`src/backend/storage/file/fd.c`): raw OS sockets opened via
//!   `tokio::net` bypass Postgres's VFD cache (`AcquireExternalFD` / `ReleaseExternalFD`). Opening
//!   many concurrent sockets per backend near `RLIMIT_NOFILE` should bracket socket lifetimes with
//!   `AcquireExternalFD` / `ReleaseExternalFD` so Postgres knows to evict cached relation FDs.
//! - `PgTryBuilder` boundaries: [`HeldInterrupts`] prevents `CHECK_FOR_INTERRUPTS()` from
//!   `siglongjmp`ing across Tokio frames, but any synchronous call to a Postgres C function that
//!   can unconditionally `ereport(ERROR)` (e.g. `palloc` OOM) must still be wrapped in a
//!   synchronous `PgTryBuilder` block without crossing an `.await` point or `spawn` boundary.

use std::future::Future;
use std::io;
use std::mem::ManuallyDrop;
use std::sync::Arc;
use std::task::Poll;

use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::DataFusionError;
use datafusion::execution::SendableRecordBatchStream;
use futures::StreamExt;

use crate::postgres::customscan::mpp::wakeup::{
    PgReactorState, disarm_thread_park_state, ensure_signal_wakeup_hooks_installed,
    register_reactor, self_wakeup_slot, unregister_reactor,
};

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

    /// Re-establishes the hold after a caught Postgres ERROR. `errfinish` zeroes the
    /// count, so a worker that turns a fragment's error into a `DataFusionError` and
    /// keeps running would do so with interrupts live. Sets rather than increments, so
    /// the live guard's single decrement still returns the count to zero.
    pub(crate) fn reassert() {
        #[cfg(not(test))]
        unsafe {
            if pgrx::pg_sys::InterruptHoldoffCount == 0 {
                pgrx::pg_sys::InterruptHoldoffCount = 1;
            }
        }
    }
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because the body is a single decrement
// of `InterruptHoldoffCount` that cannot raise, and skipping it could only leave interrupts held:
// `errfinish` zeroes the count for a Postgres ERROR, but nothing does for a panic caught on the
// Rust side.
impl Drop for HeldInterrupts {
    fn drop(&mut self) {
        disarm_thread_park_state();
        // `errfinish` zeroes the count before a caught Postgres ERROR unwinds through
        // here, so a plain decrement would underflow. Skipping it matches PG's own
        // holdoff sections, which never resume after an ERROR either.
        #[cfg(not(test))]
        unsafe {
            pgrx::pg_sys::InterruptHoldoffCount =
                pgrx::pg_sys::InterruptHoldoffCount.saturating_sub(1);
        }
    }
}

/// True when a query-cancel (SIGINT) or backend-die (SIGTERM / postmaster death) is pending.
/// Reads the signal flags directly so it never enters `ProcessInterrupts`, which could `proc_exit`
/// out of the live runtime.
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

/// True when an interrupt is pending and [`check_for_interrupts`] will be able to service and
/// clear `InterruptPending` as soon as the single [`HeldInterrupts`] guard inside
/// [`PgTokioRuntime::block_on`] drops (`InterruptHoldoffCount <= 1 && CritSectionCount == 0`).
///
/// Covers `QueryCancelPending` (`SIGINT`), `ProcDiePending` (`SIGTERM` / postmaster death), and
/// `ParallelMessagePending` (`SIGUSR1` from a parallel worker notice, error, or crash).
/// When an outer caller holds an additional [`HeldInterrupts`] guard (`InterruptHoldoffCount > 1`,
/// e.g. `run_mpp_worker`), returns `false` so the inner future finishes its own cooperative
/// shutdown before the outer guard drops.
#[cfg(not(test))]
fn can_service_interrupts_on_exit() -> bool {
    unsafe {
        if pgrx::pg_sys::InterruptHoldoffCount > 1 || pgrx::pg_sys::CritSectionCount != 0 {
            return false;
        }
        if pgrx::pg_sys::QueryCancelPending != 0 || pgrx::pg_sys::ProcDiePending != 0 {
            pgrx::pg_sys::InterruptPending = 1;
            return true;
        }
        pgrx::pg_sys::InterruptPending != 0
    }
}

#[cfg(test)]
fn can_service_interrupts_on_exit() -> bool {
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
/// the stack, so the later customscan-state drop tears down cleanly.
#[cfg(not(test))]
pub(crate) fn check_for_interrupts() {
    pgrx::check_for_interrupts!();
}

#[cfg(test)]
pub(crate) fn check_for_interrupts() {}

/// A single-threaded (`current_thread`) Tokio runtime integrated with Postgres's `MyLatch`,
/// asynchronous signal handlers (`SIGINT`, `SIGTERM`, `SIGUSR1`), and `postmaster_alive_fds`.
pub struct PgTokioRuntime {
    runtime: ManuallyDrop<tokio::runtime::Runtime>,
    reactor_state: Arc<PgReactorState>,
    reactor_tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl PgTokioRuntime {
    /// Build a new Postgres-integrated `current_thread` Tokio runtime with I/O and time drivers
    /// enabled, `MyLatch` park/unpark hooks registered, and background `AsyncFd` watchers for the
    /// process's wakeup slot and `postmaster_alive_fds`.
    pub fn new() -> io::Result<Self> {
        ensure_signal_wakeup_hooks_installed();
        let slot = self_wakeup_slot();
        let reactor_state = PgReactorState::new(slot);
        let park_state = Arc::clone(&reactor_state);
        let unpark_state = Arc::clone(&reactor_state);

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .on_thread_park(move || park_state.on_thread_park())
            .on_thread_unpark(move || unpark_state.on_thread_unpark())
            .build()?;

        let reactor_tasks = reactor_state.spawn_reactor_tasks(runtime.handle())?;
        register_reactor(runtime.handle().id(), &reactor_state);

        Ok(Self {
            runtime: ManuallyDrop::new(runtime),
            reactor_state,
            reactor_tasks,
        })
    }

    pub fn handle(&self) -> &tokio::runtime::Handle {
        self.runtime.handle()
    }

    pub fn enter(&self) -> tokio::runtime::EnterGuard<'_> {
        self.runtime.enter()
    }

    /// Drive `future` to completion on this runtime under [`HeldInterrupts`], servicing any
    /// Postgres interrupts (`QueryCancelPending`, `ProcDiePending`, `ParallelMessagePending`, or
    /// postmaster death) outside the inner `tokio::runtime::Runtime::block_on` frame.
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        loop {
            check_for_interrupts();
            enum Step<T> {
                Output(T),
                InterruptPending,
            }
            self.reactor_state.reset_cancel_notification();
            let step = {
                let _held = HeldInterrupts::hold();
                let interrupt_waker = self.reactor_state.interrupt_waker();
                self.runtime.block_on(std::future::poll_fn(|cx| {
                    if can_service_interrupts_on_exit() {
                        return Poll::Ready(Step::InterruptPending);
                    }
                    match future.as_mut().poll(cx) {
                        Poll::Ready(out) => Poll::Ready(Step::Output(out)),
                        Poll::Pending => {
                            interrupt_waker.register(cx.waker());
                            if can_service_interrupts_on_exit() {
                                return Poll::Ready(Step::InterruptPending);
                            }
                            Poll::Pending
                        }
                    }
                }))
            };
            check_for_interrupts();
            match step {
                Step::Output(out) => return out,
                Step::InterruptPending => {
                    // `check_for_interrupts()` just ran `ProcessInterrupts()` with Tokio's
                    // `block_on` off the stack. Fatal/error interrupts (`SIGINT`, `SIGTERM`,
                    // postmaster death, or worker error/crash) already unwound or exited; benign
                    // parallel worker notices were emitted and cleared `InterruptPending`, so loop
                    // and continue polling `future`.
                }
            }
        }
    }
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because `drop` only unregisters from
// the Rust `REACTOR_REGISTRY` and aborts local Tokio tasks without calling into Postgres FFI, and
// `PgTokioRuntime` is used in standalone `#[test]` binaries.
impl Drop for PgTokioRuntime {
    fn drop(&mut self) {
        unregister_reactor(self.runtime.handle().id());
        for task in self.reactor_tasks.drain(..) {
            task.abort();
        }
        // SAFETY: `self.runtime` is only taken once here in `Drop`. Using `shutdown_background()`
        // ensures dropping `PgTokioRuntime` during a query-cancel unwind never blocks waiting for
        // `spawn_blocking` threads (such as libc `getaddrinfo` from `tokio::net`).
        let runtime = unsafe { ManuallyDrop::take(&mut self.runtime) };
        runtime.shutdown_background();
        disarm_thread_park_state();
    }
}

/// Drive one batch out of a DataFusion `stream` on `runtime` under [`HeldInterrupts`], servicing
/// any Postgres interrupts outside `block_on`. Shared by `JoinScan`, `AggregateScan`, and
/// `BaseScan`.
pub(crate) fn block_on_next(
    runtime: &PgTokioRuntime,
    stream: &mut SendableRecordBatchStream,
) -> Option<Result<RecordBatch, DataFusionError>> {
    runtime.block_on(async {
        if cancel_pending() {
            return Some(Err(interrupted()));
        }
        stream.next().await
    })
}

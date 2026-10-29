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

//! Postgres-integrated Tokio runtime ([`PgTokioRuntime`]), main-thread FFI executor
//! ([`MainThreadExecutor`]), and cooperative interrupt handling.
//!
//! All async DataFusion planning, stream execution, and MPP Tonic/gRPC servers run under
//! [`PgTokioRuntime`], which drives a multi-threaded Tokio runtime (`worker_threads(2)`) while
//! keeping all Postgres FFI, non-`Send` state, and signal delivery strictly on the backend's
//! main thread:
//!
//! 1. Background Tokio worker threads are spawned under [`ScopedSignalMask`] (and mask async
//!    signals in `on_thread_start`) so `SIGINT`, `SIGTERM`, `SIGHUP`, `SIGUSR1`, `SIGUSR2`,
//!    `SIGALRM`, and `SIGQUIT` are delivered exclusively to the Postgres main thread.
//! 2. [`MainThreadExecutor`] (`run_on_main_thread`, `drop_on_main_thread`, [`MainThread`])
//!    routes every Postgres FFI call, Tantivy segment/reader operation, and non-`Send` destructor
//!    from background worker threads to the main thread over its job and drop queues drained inside
//!    [`PgTokioRuntime::block_on`].
//! 3. [`HeldInterrupts`] wraps `block_on` in Postgres's `HOLD_INTERRUPTS()` /
//!    `RESUME_INTERRUPTS()` (`InterruptHoldoffCount`), so `ProcessInterrupts()` cannot `proc_exit`
//!    or `siglongjmp` across live Rust frames while `block_on` is on the stack.
//! 4. When the main thread is idle inside [`PgTokioRuntime::block_on`], it calls
//!    [`super::wakeup::PgReactorState::on_thread_park`] and [`super::wakeup::PgReactorState::on_thread_unpark`]
//!    to maintain `MyLatch->maybe_sleeping`, arm a signal-safe wakeup descriptor
//!    ([`super::wakeup`]), call `ResetLatch(MyLatch)` when set, and wake all registered
//!    [`futures::task::AtomicWaker`]s whenever a signal or latch event occurs.
//! 5. A background `AsyncFd` task on [`PgTokioRuntime`] watches the backend's wakeup `socketpair`
//!    and `postmaster_alive_fds[POSTMASTER_FD_WATCH]`, synthesizing `ProcDiePending` and waking all
//!    tasks immediately on `postmaster` death.
//! 6. [`PgTokioRuntime::block_on`] and [`block_on_next`] wake immediately when an interrupt arrives,
//!    drop [`HeldInterrupts`], and call [`check_for_interrupts`] with `block_on` off the stack
//!    (resuming the pinned future if the interrupt was a benign parallel worker notice).
//! 7. [`PgTokioRuntime::quiesce_tasks`] (called on scan teardown and when the last handle for a
//!    scan drops) aborts any pending [`run_on_main_thread`] jobs with [`MainThreadAbort`], waits for
//!    all non-reactor Tokio tasks to finish unwinding, and drains all deferred
//!    [`drop_on_main_thread`] items on the main thread before relations or `EState` are closed.
//! 8. [`PgTokioRuntime::shutdown_workers`] (`Runtime::shutdown_timeout`) joins the background
//!    worker threads while servicing any final [`drop_on_main_thread`] items on the main thread
//!    before DSM segments are detached or `proc_exit` runs.
//!
//! ## Caveats (`tokio::net`, `spawn_blocking`, and Postgres FFI)
//!
//! - Main-thread FFI invariant: Tokio worker threads and `spawn_blocking` threads must never call
//!   into `pgrx` / `pg_sys` directly; all Postgres FFI must go through [`run_on_main_thread`] or
//!   [`drop_on_main_thread`].
//! - Postgres VFD accounting (`src/backend/storage/file/fd.c`): raw OS sockets opened via
//!   `tokio::net` bypass Postgres's VFD cache (`AcquireExternalFD` / `ReleaseExternalFD`). Opening
//!   many concurrent sockets per backend near `RLIMIT_NOFILE` should bracket socket lifetimes with
//!   `AcquireExternalFD` / `ReleaseExternalFD` so Postgres knows to evict cached relation FDs.
//! - `PgTryBuilder` boundaries: [`HeldInterrupts`] prevents `CHECK_FOR_INTERRUPTS()` from
//!   `siglongjmp`ing across Tokio frames, but any synchronous call to a Postgres C function that
//!   can unconditionally `ereport(ERROR)` (e.g. `palloc` OOM) must still run on the main thread
//!   under a `pg_guard` / `PgTryBuilder` boundary without crossing an `.await` point.

use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::Poll;

use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::DataFusionError;
use datafusion::execution::SendableRecordBatchStream;
use futures::StreamExt;
use futures::task::AtomicWaker;

use crate::postgres::customscan::mpp::wakeup::{
    PgReactorState, disarm_thread_park_state, ensure_signal_wakeup_hooks_installed,
    register_reactor, self_wakeup_slot, unregister_reactor,
};

struct UnsafeSend<T>(T);

// SAFETY: Used only by `MainThreadExecutor` to transfer ownership of main-thread-affine values
// (`drop_on_main_thread`) or stack job pointers (`JobState::Pending`) back to the Postgres main
// thread where they are accessed or dropped exclusively.
unsafe impl<T> Send for UnsafeSend<T> {}

enum JobState {
    /// Queued and waiting for the main thread. Holds a type-erased pointer to a closure on the
    /// submitting background thread's stack frame.
    Pending(*mut dyn FnMut()),
    /// Claimed by the main thread and currently executing. The background thread never exits
    /// `run_blocking` while the state is `Running`.
    Running,
    /// Completed by the main thread (`result` has been written on the background thread's stack).
    Done,
    /// Aborted because the main thread captured a leader panic, is quiescing scan tasks, or is
    /// shutting down the runtime.
    Aborted,
}

// SAFETY: The raw pointer in `JobState::Pending` points to a stack closure on the calling
// background thread, and that thread blocks on `JobSlot::cvar` until the state transitions to
// `Done` or `Aborted`.
unsafe impl Send for JobState {}

struct JobSlot {
    state: parking_lot::Mutex<JobState>,
    cvar: parking_lot::Condvar,
}

/// Unwind marker payload used to abort a background task when the main thread has already captured
/// a leader panic in [`MainThreadExecutor::captured_panic`], is quiescing in-flight tasks at scan
/// teardown ([`PgTokioRuntime::quiesce_tasks`]), or is shutting down the runtime.
pub(crate) struct MainThreadAbort;

/// Queue and synchronization state allowing background Tokio worker threads to execute Postgres
/// and Tantivy operations synchronously on the Postgres main thread inside
/// [`PgTokioRuntime::block_on`].
pub(crate) struct MainThreadExecutor {
    jobs: parking_lot::Mutex<VecDeque<Arc<JobSlot>>>,
    drops: parking_lot::Mutex<Vec<Box<dyn FnOnce() + Send + 'static>>>,
    has_pending_jobs: AtomicBool,
    has_pending_drops: AtomicBool,
    has_captured_panic_flag: AtomicBool,
    shutting_down: AtomicBool,
    quiescing: AtomicBool,
    in_flight_jobs: AtomicUsize,
    captured_panic: parking_lot::Mutex<Option<Box<dyn std::any::Any + Send + 'static>>>,
    main_waker: AtomicWaker,
    is_parallel_worker: bool,
}

thread_local! {
    static WORKER_MAIN_EXECUTOR: std::cell::RefCell<Option<Arc<MainThreadExecutor>>> =
        const { std::cell::RefCell::new(None) };
}

/// Returns `true` when called on the Postgres main thread, or `false` when called on a background
/// Tokio worker or `spawn_blocking` thread of a [`PgTokioRuntime`].
#[inline]
pub(crate) fn is_main_thread() -> bool {
    WORKER_MAIN_EXECUTOR.with(|cell| cell.borrow().is_none())
}

/// Thread-safe check for whether execution is currently inside [`PgTokioRuntime`] on a
/// Postgres parallel worker (either on a background Tokio worker thread or on the main thread
/// inside [`PgTokioRuntime::block_on`]).
///
/// Returns `false` when called outside [`PgTokioRuntime::block_on`] (for example, during
/// parallel `CREATE INDEX`), where `pgrx` `CaughtError` panics must unwind natively to the
/// parallel worker's `#[pg_guard]` boundary rather than being converted into
/// `DataFusionError`.
#[inline]
pub(crate) fn is_parallel_worker() -> bool {
    if let Some(is_worker) =
        WORKER_MAIN_EXECUTOR.with(|cell| cell.borrow().as_ref().map(|e| e.is_parallel_worker))
    {
        return is_worker;
    }
    BACKEND_RUNTIME.with(|cell| {
        cell.borrow()
            .as_ref()
            .is_some_and(|rt| rt.is_running() && rt.main_executor.is_parallel_worker)
    })
}

/// Zero-sized token proving that execution is currently on the Postgres main thread inside
/// [`run_on_main_thread`].
///
/// The private field prevents construction outside `interrupt.rs`, and `PhantomData<*const ()>`
/// makes the token `!Send` and `!Sync`, so it cannot escape a [`run_on_main_thread`] closure
/// across threads.
#[derive(Clone, Copy)]
pub(crate) struct MainThreadToken {
    _not_send_sync: std::marker::PhantomData<*const ()>,
}

impl MainThreadToken {
    #[inline]
    fn new() -> Self {
        Self {
            _not_send_sync: std::marker::PhantomData,
        }
    }
}

/// Execute `f` synchronously on the Postgres main thread, passing a [`MainThreadToken`] that
/// authorizes borrowing or taking [`MainThread`] values.
///
/// - When called on the Postgres main thread, executes `f` directly inline with zero overhead.
/// - When called on a background Tokio worker thread of [`PgTokioRuntime`], dispatches `f` to the
///   Postgres main thread (which is parked inside [`PgTokioRuntime::block_on`]) and blocks the
///   calling worker thread inside `tokio::task::block_in_place` until `f` completes.
///
/// # When to use [`try_run_on_main_thread`] instead
///
/// On a Postgres parallel worker (`is_parallel_worker() == true`), an uncaught panic inside
/// `run_on_main_thread` is resumed on the calling background Tokio thread. Because
/// `datafusion-distributed` runs plan decoding (`Worker::coordinator_channel`) and stream
/// execution (`Worker::execute_task`) inside detached `tokio::spawn` tasks, an uncaught panic on a
/// worker thread will abort the spawned task without sending a gRPC `Status` error (or hang waiting
/// for plan setup).
///
/// Any fallible code path that can execute on a parallel worker (DataFusion `PhysicalExtensionCodec`,
/// `ExecutionPlan::execute`, stream `poll_next` bodies, `ScalarUDFImpl::invoke_with_args`,
/// `Accumulator`, or `BufFile` spill I/O) should therefore use [`try_run_on_main_thread`] (or wrap
/// its outer entry point in [`catch_worker_panic`]) so main-thread panics (`pgrx::error!` /
/// `ereport(ERROR)`) are converted into `Err(DataFusionError::Execution(...))` on workers while
/// still unwinding natively on the leader.
pub(crate) fn run_on_main_thread<F, R>(f: F) -> R
where
    F: FnOnce(MainThreadToken) -> R,
{
    let exec_opt = WORKER_MAIN_EXECUTOR.with(|cell| cell.borrow().clone());
    match exec_opt {
        None => f(MainThreadToken::new()),
        Some(exec) => exec.run_blocking(|| f(MainThreadToken::new())),
    }
}

/// Execute fallible `f` synchronously on the Postgres main thread, converting any main-thread
/// panic (`pgrx::error!` / `ereport(ERROR)`) on a parallel worker into
/// `Err(DataFusionError::Execution(...))`.
///
/// - On a parallel worker (`is_parallel_worker() == true`), catches any panic inside `f` on the
///   main thread, reasserts [`HeldInterrupts`], and returns `Err(DataFusionError::Execution(msg))`
///   so Tonic streams the error back to the coordinator over gRPC without unwinding background
///   Tokio worker threads.
/// - On the leader backend (`is_parallel_worker() == false`), panics propagate normally to
///   [`MainThreadExecutor`] and are resumed on the leader's main thread by
///   [`PgTokioRuntime::block_on`], preserving the original `pgrx` `ErrorReportWithLevel` /
///   `CaughtError` payload.
pub(crate) fn try_run_on_main_thread<F, R>(f: F) -> datafusion::common::Result<R>
where
    F: FnOnce(MainThreadToken) -> datafusion::common::Result<R>,
{
    let is_worker = is_parallel_worker();
    run_on_main_thread(|token| {
        if is_worker {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(token))) {
                Ok(res) => res,
                Err(payload) => {
                    HeldInterrupts::reassert();
                    if payload.is::<MainThreadAbort>() {
                        std::panic::resume_unwind(payload);
                    }
                    let msg =
                        crate::postgres::customscan::mpp::panic_payload_message(payload.as_ref());
                    Err(DataFusionError::Execution(msg))
                }
            }
        } else {
            f(token)
        }
    })
}

/// Run `f`, catching any panic on a parallel worker and converting it into
/// `Err(DataFusionError::Execution(...))` so Tonic streams the error back to the coordinator.
/// On the leader backend, panics propagate normally.
pub(crate) fn catch_worker_panic<F, R>(f: F) -> datafusion::common::Result<R>
where
    F: FnOnce() -> datafusion::common::Result<R>,
{
    if is_parallel_worker() {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(res) => res,
            Err(payload) => {
                HeldInterrupts::reassert();
                if payload.is::<MainThreadAbort>() {
                    std::panic::resume_unwind(payload);
                }
                let msg = crate::postgres::customscan::mpp::panic_payload_message(payload.as_ref());
                Err(DataFusionError::Execution(msg))
            }
        }
    } else {
        f()
    }
}

/// Drop `val` on the Postgres main thread.
///
/// - When called on the Postgres main thread, drops `val` inline.
/// - When called on a background Tokio worker thread of [`PgTokioRuntime`], enqueues `val` onto a
///   non-blocking drop queue drained by the main thread inside [`PgTokioRuntime::block_on`] and
///   [`PgTokioRuntime::drop`].
pub(crate) fn drop_on_main_thread<T: 'static>(val: T) {
    let exec_opt = WORKER_MAIN_EXECUTOR.with(|cell| cell.borrow().clone());
    match exec_opt {
        None => drop(val),
        Some(exec) => {
            let wrapped = UnsafeSend(val);
            exec.enqueue_drop(Box::new(move || {
                drop(wrapped);
            }));
        }
    }
}

/// Wrapper that binds access and destruction of `T` to the Postgres main thread.
///
/// - Accessing `&T`, `&mut T`, or taking `Option<T>` requires a [`MainThreadToken`], which can
///   only be obtained inside [`run_on_main_thread`].
/// - Dropping `MainThread<T>` routes `T`'s destructor through [`drop_on_main_thread`].
///
/// Because there is only a single Postgres main thread per backend/worker process and `T` can
/// only ever be accessed or dropped on that thread, `MainThread<T>` is soundly `Send` and
/// `Sync` for any `T: 'static`.
pub(crate) struct MainThread<T: 'static>(Option<T>);

// SAFETY: `T` can only be borrowed or taken with a `MainThreadToken` (which is `!Send + !Sync`
// and only exists on the single Postgres main thread inside `run_on_main_thread`), and `Drop`
// routes `T` to the main thread via `drop_on_main_thread`.
unsafe impl<T: 'static> Send for MainThread<T> {}
unsafe impl<T: 'static> Sync for MainThread<T> {}

impl<T: 'static> MainThread<T> {
    #[inline]
    pub(crate) fn new(value: T) -> Self {
        Self(Some(value))
    }

    #[inline]
    pub(crate) fn get(&self, _token: MainThreadToken) -> &T {
        self.0.as_ref().expect("MainThread accessed after drop")
    }

    #[inline]
    pub(crate) fn get_mut(&mut self, _token: MainThreadToken) -> &mut T {
        self.0.as_mut().expect("MainThread accessed after drop")
    }

    #[inline]
    pub(crate) fn take(&mut self, _token: MainThreadToken) -> Option<T> {
        self.0.take()
    }
}

impl<T: Clone + 'static> Clone for MainThread<T> {
    fn clone(&self) -> Self {
        run_on_main_thread(|token| Self::new(self.get(token).clone()))
    }
}

impl<T: Default + 'static> Default for MainThread<T> {
    fn default() -> Self {
        run_on_main_thread(|_| Self::new(T::default()))
    }
}

impl<T: std::fmt::Debug + 'static> std::fmt::Debug for MainThread<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MainThread").finish_non_exhaustive()
    }
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because this only forwards the inner
// value to `drop_on_main_thread`, where its own `Drop` impl handles unwinding and process exit.
impl<T: 'static> Drop for MainThread<T> {
    fn drop(&mut self) {
        if let Some(value) = self.0.take() {
            drop_on_main_thread(value);
        }
    }
}

impl MainThreadExecutor {
    fn new(is_parallel_worker: bool) -> Arc<Self> {
        Arc::new(Self {
            jobs: parking_lot::Mutex::new(VecDeque::new()),
            drops: parking_lot::Mutex::new(Vec::new()),
            has_pending_jobs: AtomicBool::new(false),
            has_pending_drops: AtomicBool::new(false),
            has_captured_panic_flag: AtomicBool::new(false),
            shutting_down: AtomicBool::new(false),
            quiescing: AtomicBool::new(false),
            in_flight_jobs: AtomicUsize::new(0),
            captured_panic: parking_lot::Mutex::new(None),
            main_waker: AtomicWaker::new(),
            is_parallel_worker,
        })
    }

    #[inline]
    fn has_captured_panic(&self) -> bool {
        self.has_captured_panic_flag.load(Ordering::Acquire)
    }

    fn record_leader_panic(&self, payload: Box<dyn std::any::Any + Send + 'static>) {
        let mut guard = self.captured_panic.lock();
        if guard.is_none() {
            *guard = Some(payload);
            self.has_captured_panic_flag.store(true, Ordering::Release);
        }
    }

    #[inline]
    fn take_captured_panic(&self) -> Option<Box<dyn std::any::Any + Send + 'static>> {
        if !self.has_captured_panic_flag.load(Ordering::Acquire) {
            return None;
        }
        let mut guard = self.captured_panic.lock();
        let taken = guard.take();
        self.has_captured_panic_flag.store(false, Ordering::Release);
        taken
    }

    fn enqueue_drop(&self, drop_fn: Box<dyn FnOnce() + Send + 'static>) {
        {
            let mut drops = self.drops.lock();
            drops.push(drop_fn);
            self.has_pending_drops.store(true, Ordering::Release);
        }
        self.main_waker.wake();
    }

    #[inline]
    fn has_pending_work(&self) -> bool {
        self.has_pending_drops.load(Ordering::Acquire)
            || self.has_pending_jobs.load(Ordering::Acquire)
    }

    #[inline]
    fn drain_drops(&self) -> bool {
        if !self.has_pending_drops.load(Ordering::Acquire) {
            return false;
        }
        self.drain_drops_slow()
    }

    fn drain_drops_slow(&self) -> bool {
        let mut did_work = false;
        loop {
            let batch: Vec<Box<dyn FnOnce() + Send + 'static>> = {
                let mut drops = self.drops.lock();
                if drops.is_empty() {
                    self.has_pending_drops.store(false, Ordering::Release);
                    break;
                }
                let taken = std::mem::take(&mut *drops);
                self.has_pending_drops.store(false, Ordering::Release);
                taken
            };
            did_work = true;
            #[cfg(not(test))]
            if crate::postgres::utils::proc_exit_in_progress()
                || unsafe { pgrx::pg_sys::ProcDiePending != 0 }
            {
                std::mem::forget(batch);
                continue;
            }
            for drop_fn in batch {
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(drop_fn)).is_err() {
                    HeldInterrupts::reassert();
                }
            }
        }
        did_work
    }

    fn abort_pending_jobs(&self) {
        if !self.has_pending_jobs.load(Ordering::Acquire) {
            return;
        }
        let pending: Vec<Arc<JobSlot>> = {
            let mut jobs = self.jobs.lock();
            let drained = jobs.drain(..).collect();
            self.has_pending_jobs.store(false, Ordering::Release);
            drained
        };
        for slot in pending {
            let mut state = slot.state.lock();
            if matches!(*state, JobState::Pending(_)) {
                *state = JobState::Aborted;
                slot.cvar.notify_one();
            }
        }
    }

    fn begin_shutdown(&self) {
        self.shutting_down.store(true, Ordering::Release);
        self.abort_pending_jobs();
    }

    /// Run all queued drops and main-thread jobs on the Postgres main thread. Returns `true` if
    /// any work was executed.
    #[inline]
    fn drain_work(&self) -> bool {
        let mut did_work = self.drain_drops();
        if !self.has_pending_jobs.load(Ordering::Acquire) {
            return did_work;
        }
        did_work |= self.drain_jobs_slow();
        did_work
    }

    fn drain_jobs_slow(&self) -> bool {
        let mut did_work = false;
        loop {
            if !self.is_parallel_worker && self.has_captured_panic() {
                self.abort_pending_jobs();
                break;
            }
            let Some(slot) = ({
                let mut jobs = self.jobs.lock();
                let next = jobs.pop_front();
                if jobs.is_empty() {
                    self.has_pending_jobs.store(false, Ordering::Release);
                }
                next
            }) else {
                break;
            };
            let task_ptr = {
                let mut state = slot.state.lock();
                match *state {
                    JobState::Pending(ptr) => {
                        *state = JobState::Running;
                        ptr
                    }
                    _ => continue,
                }
            };
            did_work = true;
            // SAFETY: While `*state` is `JobState::Running`, the submitting background thread is
            // guaranteed to remain blocked inside `slot.cvar.wait`, keeping `task_ptr`'s stack
            // frame alive and exclusively borrowed by the main thread.
            unsafe { (*task_ptr)() };

            let panicked_on_leader = !self.is_parallel_worker && self.has_captured_panic();
            {
                let mut state = slot.state.lock();
                *state = if panicked_on_leader {
                    JobState::Aborted
                } else {
                    JobState::Done
                };
                slot.cvar.notify_one();
            }
            if panicked_on_leader {
                self.abort_pending_jobs();
                break;
            }
            let _ = self.drain_drops();
        }
        did_work
    }

    /// Synchronously executes a non-`'static` closure `f` on the Postgres main thread from a
    /// background Tokio worker thread.
    ///
    /// - Stack-lifetime safety: `task` and `result` live on this worker thread's stack frame, and
    ///   `JobSlotDropGuard` guarantees this frame cannot return or unwind while the slot is
    ///   `Pending` (cancelling it to `Aborted` under the mutex) or `Running` (waiting on `cvar`
    ///   until the main thread finishes calling `task_ptr`).
    /// - Panic routing: on a parallel worker (`is_parallel_worker`), panics (including `pgrx`
    ///   `CaughtError` from `ereport(ERROR)`) are returned in `result` and resumed on the worker
    ///   task so `try_run_on_main_thread` / `catch_worker_panic` can surface them as a gRPC
    ///   `DataFusionError`. On the leader, panics are stashed via `record_leader_panic` to unwind
    ///   `PgTokioRuntime::block_on` on the main thread, while the worker task unwinds with
    ///   `MainThreadAbort`.
    /// - Scheduler handoff: uses `tokio::task::block_in_place` when called inside an async Tokio
    ///   task so another worker thread can take over this thread's run queue while blocked on
    ///   `slot.cvar`.
    fn run_blocking<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        struct InFlightJobGuard<'a>(&'a AtomicUsize);
        // NOTE: We intentionally do NOT use `impl_safe_drop!` here because this guard only
        // decrements `in_flight_jobs` on normal return or unwind.
        impl Drop for InFlightJobGuard<'_> {
            fn drop(&mut self) {
                self.0.fetch_sub(1, Ordering::AcqRel);
            }
        }
        self.in_flight_jobs.fetch_add(1, Ordering::AcqRel);
        let _in_flight_guard = InFlightJobGuard(&self.in_flight_jobs);

        if self.shutting_down.load(Ordering::Acquire)
            || self.quiescing.load(Ordering::Acquire)
            || (!self.is_parallel_worker && self.has_captured_panic())
        {
            std::panic::resume_unwind(Box::new(MainThreadAbort));
        }

        let mut f_opt = Some(f);
        let mut result: Option<Result<R, Box<dyn std::any::Any + Send + 'static>>> = None;
        let is_worker = self.is_parallel_worker;

        let mut task = || {
            let func = f_opt.take().expect("main-thread job closure called once");
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(func)) {
                Ok(val) => {
                    result = Some(Ok(val));
                }
                Err(payload) => {
                    HeldInterrupts::reassert();
                    if is_worker || payload.is::<MainThreadAbort>() {
                        result = Some(Err(payload));
                    } else {
                        self.record_leader_panic(payload);
                    }
                }
            }
        };

        // SAFETY: We erase the stack lifetime of `&mut task` into `*mut dyn FnMut()` and block the
        // current stack frame on `slot.cvar` until the state transitions to `Done` or `Aborted`
        // (never returning while `Pending` or `Running`), so `task` always outlives the main
        // thread's call through `task_ptr`.
        let task_ptr: *mut dyn FnMut() =
            unsafe { std::mem::transmute::<&mut dyn FnMut(), *mut dyn FnMut()>(&mut task) };

        let slot = Arc::new(JobSlot {
            state: parking_lot::Mutex::new(JobState::Pending(task_ptr)),
            cvar: parking_lot::Condvar::new(),
        });

        struct JobSlotDropGuard<'a>(&'a JobSlot);
        // NOTE: We intentionally do NOT use `impl_safe_drop!` here because this guard upholds memory
        // safety on unwind: `task` on this stack frame must not be dropped while `slot` is still
        // `Pending(task_ptr)` or `Running` on the main thread.
        impl Drop for JobSlotDropGuard<'_> {
            fn drop(&mut self) {
                let mut state = self.0.state.lock();
                loop {
                    match *state {
                        JobState::Pending(_) => {
                            *state = JobState::Aborted;
                            break;
                        }
                        JobState::Running => {
                            self.0.cvar.wait(&mut state);
                        }
                        JobState::Done | JobState::Aborted => break,
                    }
                }
            }
        }
        let _slot_guard = JobSlotDropGuard(&slot);

        let wait_for_slot = || {
            {
                let mut jobs = self.jobs.lock();
                if self.shutting_down.load(Ordering::Acquire)
                    || self.quiescing.load(Ordering::Acquire)
                    || (!self.is_parallel_worker && self.has_captured_panic())
                {
                    std::panic::resume_unwind(Box::new(MainThreadAbort));
                }
                jobs.push_back(Arc::clone(&slot));
                self.has_pending_jobs.store(true, Ordering::Release);
            }
            self.main_waker.wake();

            let mut state = slot.state.lock();
            loop {
                match *state {
                    JobState::Pending(_) | JobState::Running => {
                        slot.cvar.wait(&mut state);
                    }
                    JobState::Done => break false,
                    JobState::Aborted => break true,
                }
            }
        };

        let aborted = if tokio::task::try_id().is_some() {
            tokio::task::block_in_place(wait_for_slot)
        } else {
            wait_for_slot()
        };

        if aborted {
            std::panic::resume_unwind(Box::new(MainThreadAbort));
        }

        match result.expect("completed main-thread job must populate result") {
            Ok(val) => val,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }
}

/// Builds the signal mask installed on background Tokio worker threads.
///
/// Starts from `sigfillset` so every asynchronous process-directed signal (`SIGINT` query cancel,
/// `SIGTERM` backend die, `SIGUSR1` procsignal/parallel messages, `SIGHUP` config reload,
/// `SIGALRM` statement/lock timeouts, `SIGQUIT` crash shutdown, and any signals registered by
/// other extensions) is blocked on Tokio worker threads and delivered by the kernel exclusively to
/// the Postgres main thread, whose signal handlers assume main-thread state (`MyLatch`,
/// `PG_SETMASK(&BlockSig)`, etc.).
///
/// Only the synchronous, thread-directed hardware fault and trap signals (`SIGSEGV`, `SIGBUS`,
/// `SIGFPE`, `SIGILL`, `SIGABRT`, `SIGTRAP`) are left unblocked (`sigdelset`): POSIX leaves
/// blocking a synchronous fault signal undefined when a fault occurs (which would also break
/// Rust's guard-page stack-overflow handler), and `abort()` / debugger traps must fire on the
/// faulting thread itself.
fn build_background_sigset() -> libc::sigset_t {
    unsafe {
        let mut set: libc::sigset_t = std::mem::zeroed();
        libc::sigfillset(&mut set);
        for sig in [
            libc::SIGSEGV,
            libc::SIGBUS,
            libc::SIGFPE,
            libc::SIGILL,
            libc::SIGABRT,
            libc::SIGTRAP,
        ] {
            libc::sigdelset(&mut set, sig);
        }
        set
    }
}

/// Temporarily applies [`build_background_sigset`] on the calling thread while spawning Tokio
/// worker threads so newly created threads inherit the blocked signal mask from birth (before
/// their first instruction runs), then restores the caller's original signal mask on drop.
struct ScopedSignalMask {
    old_set: libc::sigset_t,
}

impl ScopedSignalMask {
    fn block_async_signals() -> Self {
        unsafe {
            let new_set = build_background_sigset();
            let mut old_set: libc::sigset_t = std::mem::zeroed();
            libc::pthread_sigmask(libc::SIG_BLOCK, &new_set, &mut old_set);
            Self { old_set }
        }
    }
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because the thread's signal mask must
// always be restored when `ScopedSignalMask` leaves scope.
impl Drop for ScopedSignalMask {
    fn drop(&mut self) {
        unsafe {
            libc::pthread_sigmask(libc::SIG_SETMASK, &self.old_set, std::ptr::null_mut());
        }
    }
}

fn mask_async_signals_current_thread() {
    unsafe {
        let new_set = build_background_sigset();
        libc::pthread_sigmask(libc::SIG_BLOCK, &new_set, std::ptr::null_mut());
    }
}

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
        if is_main_thread() {
            unsafe {
                if pgrx::pg_sys::InterruptHoldoffCount == 0 {
                    pgrx::pg_sys::InterruptHoldoffCount = 1;
                }
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
    // SAFETY: plain reads of the backend's interrupt flags.
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

/// True when a cancel (`SIGINT`) or die (`SIGTERM` / postmaster death) is pending and
/// [`check_for_interrupts`] will service it immediately (`InterruptHoldoffCount == 0 &&
/// CritSectionCount == 0`).
#[cfg(not(test))]
fn can_service_cancel_now() -> bool {
    unsafe {
        pgrx::pg_sys::InterruptHoldoffCount == 0
            && pgrx::pg_sys::CritSectionCount == 0
            && (pgrx::pg_sys::QueryCancelPending != 0 || pgrx::pg_sys::ProcDiePending != 0)
    }
}

#[cfg(test)]
fn can_service_cancel_now() -> bool {
    false
}

/// Error an MPP loop returns when it bails on a pending cancel/die. The dispatcher discards
/// it and calls [`check_for_interrupts`] once `block_on` has returned.
pub(crate) fn interrupted() -> DataFusionError {
    DataFusionError::Execution("mpp: query interrupted".into())
}

/// Returns `Err(interrupted())` when a query-cancel or backend-die is pending so DataFusion
/// scan streams unwind cooperatively out of [`PgTokioRuntime::block_on`].
pub(crate) fn check_for_cancel() -> Result<(), DataFusionError> {
    if cancel_pending() {
        Err(interrupted())
    } else {
        Ok(())
    }
}

/// Services pending Postgres interrupts immediately when not under [`HeldInterrupts`]
/// (`InterruptHoldoffCount == 0 && CritSectionCount == 0`), or returns `true` when interrupts are
/// held off and a query cancel/die is pending so the caller can unwind cooperatively without
/// repeatedly entering C `ProcessInterrupts()` while `InterruptPending` remains set.
#[cfg(not(test))]
pub(crate) fn check_for_interrupts_or_cancel() -> bool {
    if unsafe { pgrx::pg_sys::InterruptHoldoffCount == 0 && pgrx::pg_sys::CritSectionCount == 0 } {
        pgrx::check_for_interrupts!();
        false
    } else {
        cancel_pending()
    }
}

#[cfg(test)]
pub(crate) fn check_for_interrupts_or_cancel() -> bool {
    false
}

/// Act on the cancel/die that [`HeldInterrupts`] held off and the drain loops bailed on
/// cooperatively, now that `block_on` has returned and the runtime is idle. Acting mid-`block_on`
/// would `proc_exit` out of the live runtime; this is the safe point. Cancel `longjmp`s into PG
/// error handling (caught and unwound by pgrx); die `proc_exit`s. Either way the runtime is off
/// the stack, so the later customscan-state drop tears down cleanly.
#[cfg(not(test))]
pub(crate) fn check_for_interrupts() {
    unsafe {
        if pgrx::pg_sys::QueryCancelPending != 0 || pgrx::pg_sys::ProcDiePending != 0 {
            pgrx::pg_sys::InterruptPending = 1;
        }
    }
    pgrx::check_for_interrupts!();
}

#[cfg(test)]
pub(crate) fn check_for_interrupts() {}

struct PgTokioRuntimeInner {
    runtime: parking_lot::Mutex<Option<tokio::runtime::Runtime>>,
    handle: tokio::runtime::Handle,
    reactor_state: Arc<PgReactorState>,
    reactor_tasks: parking_lot::Mutex<Vec<tokio::task::JoinHandle<()>>>,
    reactor_task_count: usize,
    active_handles: AtomicUsize,
    main_executor: Arc<MainThreadExecutor>,
}

thread_local! {
    static BACKEND_RUNTIME: std::cell::RefCell<Option<Arc<PgTokioRuntimeInner>>> =
        const { std::cell::RefCell::new(None) };
}

/// Handle to the backend's multi-threaded Tokio runtime integrated with Postgres's `MyLatch`,
/// asynchronous signal handlers (`SIGINT`, `SIGTERM`, `SIGUSR1`), `postmaster_alive_fds`, and a
/// [`MainThreadExecutor`] that runs Postgres/Tantivy batch work on the Postgres main thread while
/// background worker threads drive async I/O (`tonic`, `h2`, `DsmAsyncStream`) and DataFusion
/// operators.
pub struct PgTokioRuntime {
    inner: Arc<PgTokioRuntimeInner>,
}

impl Clone for PgTokioRuntime {
    fn clone(&self) -> Self {
        self.inner.active_handles.fetch_add(1, Ordering::AcqRel);
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl PgTokioRuntimeInner {
    fn build() -> io::Result<Arc<Self>> {
        let is_worker = {
            #[cfg(not(test))]
            unsafe {
                pgrx::pg_sys::ParallelWorkerNumber >= 0
            }
            #[cfg(test)]
            {
                false
            }
        };
        let main_executor = MainThreadExecutor::new(is_worker);
        let worker_exec = Arc::clone(&main_executor);

        let slot = self_wakeup_slot()?;
        let reactor_state = PgReactorState::new(slot)?;

        let runtime = {
            let _sig_guard = ScopedSignalMask::block_async_signals();
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .on_thread_start(move || {
                    mask_async_signals_current_thread();
                    WORKER_MAIN_EXECUTOR.with(|cell| {
                        *cell.borrow_mut() = Some(Arc::clone(&worker_exec));
                    });
                })
                .build()?
        };

        let handle = runtime.handle().clone();
        let reactor_tasks = reactor_state.spawn_reactor_tasks(&handle)?;
        let reactor_task_count = reactor_tasks.len();
        register_reactor(handle.id(), &reactor_state);

        Ok(Arc::new(Self {
            runtime: parking_lot::Mutex::new(Some(runtime)),
            handle,
            reactor_state,
            reactor_tasks: parking_lot::Mutex::new(reactor_tasks),
            reactor_task_count,
            active_handles: AtomicUsize::new(0),
            main_executor,
        }))
    }

    fn is_running(&self) -> bool {
        self.runtime.lock().is_some()
    }
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because `PgTokioRuntimeInner` is also
// used in standalone `#[test]` binaries and must join its worker threads if dropped directly.
impl Drop for PgTokioRuntimeInner {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.get_mut().take() {
            unregister_reactor(self.handle.id());
            for task in std::mem::take(self.reactor_tasks.get_mut()) {
                task.abort();
            }
            self.main_executor.begin_shutdown();
            runtime.shutdown_timeout(std::time::Duration::from_secs(5));
            self.main_executor.drain_drops();
            disarm_thread_park_state();
        }
    }
}

impl PgTokioRuntime {
    /// Return a handle to the current backend's Postgres-integrated multi-threaded Tokio runtime,
    /// initializing it on first use (or recreating it if a previous query tore down the worker
    /// pool during cancel/panic cleanup).
    pub fn new() -> io::Result<Self> {
        debug_assert!(
            is_main_thread(),
            "PgTokioRuntime::new must be called on the Postgres main thread"
        );
        ensure_signal_wakeup_hooks_installed();
        // Ensure `pgrx`'s `ACTIVE_THREAD` is claimed by the Postgres main thread before any
        // background worker threads are spawned (especially after `fork()` in parallel workers,
        // where `pgrx`'s `pthread_atfork` child handler resets `ACTIVE_THREAD` to 0).
        #[cfg(not(test))]
        unsafe {
            pgrx::pg_sys::submodules::ffi::pg_guard_ffi_boundary(|| ());
        }

        BACKEND_RUNTIME.with(|cell| {
            if let Some(existing) = cell.borrow().as_ref()
                && existing.is_running()
                && (existing.active_handles.load(Ordering::Acquire) == 0
                    || existing.handle.metrics().num_alive_tasks() <= existing.reactor_task_count)
            {
                existing.active_handles.fetch_add(1, Ordering::AcqRel);
                return Ok(Self {
                    inner: Arc::clone(existing),
                });
            }
            let inner = PgTokioRuntimeInner::build()?;
            inner.active_handles.store(1, Ordering::Release);
            if cell.borrow().as_ref().is_none_or(|rt| !rt.is_running()) {
                *cell.borrow_mut() = Some(Arc::clone(&inner));
            }
            Ok(Self { inner })
        })
    }

    pub fn handle(&self) -> &tokio::runtime::Handle {
        &self.inner.handle
    }

    pub fn enter(&self) -> tokio::runtime::EnterGuard<'_> {
        self.inner.handle.enter()
    }

    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.inner.handle.spawn(future)
    }

    pub(crate) fn is_running(&self) -> bool {
        self.inner.is_running()
    }

    fn drain_drops(&self) {
        if is_main_thread() {
            self.inner.main_executor.drain_drops();
        }
    }

    /// Abort any in-flight `run_on_main_thread` jobs, wait for all non-reactor Tokio tasks on this
    /// runtime to finish unwinding/dropping, and drain all deferred main-thread drops synchronously
    /// on the Postgres main thread.
    ///
    /// Called when tearing down a scan stream (such as early `LIMIT` exit, `ReScan`, or
    /// `end_custom_scan`) and when the last active [`PgTokioRuntime`] handle drops so no background
    /// worker thread can remain blocked in `slot.cvar.wait` or hold [`MainThread`] Postgres
    /// resources across scan / transaction teardown.
    pub(crate) fn quiesce_tasks(&self) {
        if !is_main_thread() {
            return;
        }
        if !self.inner.is_running() {
            self.inner.main_executor.drain_drops();
            return;
        }
        #[cfg(not(test))]
        if crate::postgres::utils::proc_exit_in_progress()
            || unsafe { pgrx::pg_sys::ProcDiePending != 0 }
        {
            self.shutdown_workers();
            return;
        }

        let _held = HeldInterrupts::hold();
        let baseline_tasks = self.inner.reactor_task_count;
        self.inner
            .main_executor
            .quiescing
            .store(true, Ordering::SeqCst);
        self.inner.main_executor.abort_pending_jobs();
        self.inner.main_executor.drain_drops();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while self.inner.is_running() {
            self.inner.main_executor.abort_pending_jobs();
            self.inner.main_executor.drain_drops();

            let alive_tasks = self.inner.handle.metrics().num_alive_tasks();
            let in_flight = self
                .inner
                .main_executor
                .in_flight_jobs
                .load(Ordering::Acquire);
            if alive_tasks <= baseline_tasks
                && in_flight == 0
                && !self.inner.main_executor.has_pending_work()
            {
                break;
            }
            if std::time::Instant::now() >= deadline {
                self.inner
                    .main_executor
                    .quiescing
                    .store(false, Ordering::SeqCst);
                self.shutdown_workers();
                debug_assert!(
                    false,
                    "PgTokioRuntime::quiesce_tasks timed out with {alive_tasks} alive tasks \
                     (baseline {baseline_tasks}, in_flight {in_flight}); shut down worker pool"
                );
                return;
            }
            std::thread::sleep(std::time::Duration::from_micros(50));
        }

        self.inner.main_executor.abort_pending_jobs();
        self.inner.main_executor.drain_drops();
        self.inner
            .main_executor
            .quiescing
            .store(false, Ordering::SeqCst);
    }

    /// Shut down and join the background Tokio worker threads if still running.
    ///
    /// Called on the Postgres main thread before servicing a terminal interrupt
    /// (`QueryCancelPending`, `ProcDiePending`, or a worker error/panic) and before
    /// `run_mpp_worker` releases its [`HeldInterrupts`] guard, so no `tokio-rt-worker` thread
    /// remains alive during `AbortTransaction` (`AtEOXact_Parallel` DSM unmap) or `proc_exit(1)`.
    pub(crate) fn shutdown_workers(&self) {
        let Some(runtime) = self.inner.runtime.lock().take() else {
            return;
        };
        let _held = HeldInterrupts::hold();
        unregister_reactor(self.inner.handle.id());
        let reactor_tasks: Vec<_> = std::mem::take(&mut *self.inner.reactor_tasks.lock());
        for task in reactor_tasks {
            task.abort();
        }
        // Abort any in-flight or future `run_on_main_thread` calls so background worker threads
        // cannot block waiting for the main thread while we join the runtime.
        self.inner.main_executor.begin_shutdown();
        runtime.shutdown_timeout(std::time::Duration::from_secs(5));
        // Now that all background worker threads have exited, run (or forget, if the backend is
        // about to `proc_exit`) any deferred main-thread drops they enqueued during task teardown.
        self.inner.main_executor.drain_drops();
        disarm_thread_park_state();
    }

    /// Drive `future` to completion on this runtime under [`HeldInterrupts`], servicing any
    /// [`run_on_main_thread`] / [`drop_on_main_thread`] requests from background worker threads
    /// and servicing Postgres interrupts (`QueryCancelPending`, `ProcDiePending`,
    /// `ParallelMessagePending`, or postmaster death) outside the inner
    /// `tokio::runtime::Handle::block_on` frame.
    ///
    /// - Outer loop: `future` is pinned as `Option<F>` so the inner `handle.block_on` can exit
    ///   early on `Step::InterruptPending`, drop [`HeldInterrupts`], and let
    ///   `check_interrupts_with_cleanup` call `ProcessInterrupts()` with `handle.block_on` off the
    ///   stack. Terminal interrupts drop `future` and join worker threads before unwinding; benign
    ///   `SIGUSR1` worker notices clear `InterruptPending` and resume polling `future` on the next
    ///   iteration.
    /// - Inner `poll_fn` loop: drains `main_executor` jobs/drops before polling `future`. On
    ///   `Poll::Pending`, registers `cx.waker()`, arms the Postgres latch (`on_thread_park()`), and
    ///   re-checks `has_pending_work()` and `can_service_interrupts_on_exit()` before returning
    ///   `Poll::Pending` so no wakeup arriving during park setup can be lost.
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        let inner = &self.inner;
        let mut future = std::pin::pin!(Some(future));
        let shutdown_and_resume_panic = |future: &mut std::pin::Pin<&mut Option<F>>,
                                         payload: Box<dyn std::any::Any + Send + 'static>|
         -> ! {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                future.set(None);
            }));
            self.shutdown_workers();
            std::panic::resume_unwind(payload);
        };
        let check_interrupts_with_cleanup = |future: &mut std::pin::Pin<&mut Option<F>>| {
            if can_service_cancel_now() {
                future.set(None);
                self.shutdown_workers();
                check_for_interrupts();
                return;
            }
            if let Err(payload) = std::panic::catch_unwind(check_for_interrupts) {
                shutdown_and_resume_panic(future, payload);
            }
        };

        loop {
            if let Some(payload) = inner.main_executor.take_captured_panic() {
                shutdown_and_resume_panic(&mut future, payload);
            }
            check_interrupts_with_cleanup(&mut future);
            enum Step<T> {
                Output(T),
                InterruptPending,
                LeaderPanic(Box<dyn std::any::Any + Send + 'static>),
            }
            inner.reactor_state.reset_cancel_notification();
            let step = {
                let _held = HeldInterrupts::hold();
                let interrupt_waker = inner.reactor_state.interrupt_waker();
                assert!(
                    inner.is_running(),
                    "PgTokioRuntime::block_on called after shutdown"
                );
                let mut parked = false;
                inner.handle.block_on(std::future::poll_fn(|cx| {
                    if parked {
                        parked = false;
                        inner.reactor_state.on_thread_unpark();
                    }

                    loop {
                        inner.main_executor.drain_work();
                        if let Some(payload) = inner.main_executor.take_captured_panic() {
                            return Poll::Ready(Step::LeaderPanic(payload));
                        }
                        if can_service_interrupts_on_exit() {
                            return Poll::Ready(Step::InterruptPending);
                        }

                        let poll_res =
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                let fut = future
                                    .as_mut()
                                    .as_pin_mut()
                                    .expect("future polled after completion");
                                fut.poll(cx)
                            }));
                        let polled = match poll_res {
                            Ok(polled) => polled,
                            Err(payload) => {
                                HeldInterrupts::reassert();
                                return Poll::Ready(Step::LeaderPanic(payload));
                            }
                        };
                        match polled {
                            Poll::Ready(out) => {
                                inner.main_executor.drain_drops();
                                if let Some(payload) = inner.main_executor.take_captured_panic() {
                                    return Poll::Ready(Step::LeaderPanic(payload));
                                }
                                return Poll::Ready(Step::Output(out));
                            }
                            Poll::Pending => {
                                if inner.main_executor.has_pending_work() {
                                    continue;
                                }
                                if can_service_interrupts_on_exit() {
                                    return Poll::Ready(Step::InterruptPending);
                                }
                                interrupt_waker.register(cx.waker());
                                inner.main_executor.main_waker.register(cx.waker());
                                inner.reactor_state.on_thread_park();
                                parked = true;
                                if inner.main_executor.has_pending_work() {
                                    parked = false;
                                    inner.reactor_state.on_thread_unpark();
                                    continue;
                                }
                                if can_service_interrupts_on_exit() {
                                    parked = false;
                                    inner.reactor_state.on_thread_unpark();
                                    return Poll::Ready(Step::InterruptPending);
                                }
                                return Poll::Pending;
                            }
                        }
                    }
                }))
            };
            if let Step::LeaderPanic(payload) = step {
                shutdown_and_resume_panic(&mut future, payload);
            }
            if let Some(payload) = inner.main_executor.take_captured_panic() {
                shutdown_and_resume_panic(&mut future, payload);
            }
            check_interrupts_with_cleanup(&mut future);
            match step {
                Step::Output(out) => return out,
                Step::LeaderPanic(payload) => shutdown_and_resume_panic(&mut future, payload),
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

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because `PgTokioRuntime` is also used
// in standalone `#[test]` binaries and must quiesce background tasks and drain queued main-thread
// drops when the last handle drops.
impl Drop for PgTokioRuntime {
    fn drop(&mut self) {
        let prev = self.inner.active_handles.fetch_sub(1, Ordering::AcqRel);
        if prev == 1 {
            self.quiesce_tasks();
        } else {
            self.drain_drops();
        }
    }
}

/// Drive one batch out of a DataFusion `stream` on the Postgres main thread under
/// [`HeldInterrupts`], while servicing any [`run_on_main_thread`] requests from background worker
/// tasks and checking for Postgres interrupts. Shared by `JoinScan`, `AggregateScan`, and
/// `BaseScan`.
pub(crate) fn block_on_next(
    runtime: &PgTokioRuntime,
    stream: &mut SendableRecordBatchStream,
) -> Option<Result<RecordBatch, DataFusionError>> {
    runtime.block_on(std::future::poll_fn(|cx| {
        if cancel_pending() {
            return Poll::Ready(Some(Err(interrupted())));
        }
        stream.poll_next_unpin(cx)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn multi_thread_runtime_dispatches_jobs_drops_and_panics_to_main_thread() {
        let main_tid = std::thread::current().id();
        assert!(is_main_thread());

        let dropped_on_main = Arc::new(AtomicUsize::new(0));
        struct MainThreadDropChecker {
            main_tid: std::thread::ThreadId,
            counter: Arc<AtomicUsize>,
        }
        // NOTE: We intentionally do NOT use `impl_safe_drop!` here because this test helper only
        // asserts the drop thread and increments an atomic counter.
        impl Drop for MainThreadDropChecker {
            fn drop(&mut self) {
                assert_eq!(std::thread::current().id(), self.main_tid);
                assert!(is_main_thread());
                self.counter.fetch_add(1, Ordering::SeqCst);
            }
        }

        let rt = PgTokioRuntime::new().expect("create runtime");

        // 1. Concurrent background tasks running non-'static closures on the main thread and
        //    deferring drops to the main thread.
        let mut handles = Vec::new();
        for i in 0..8usize {
            let counter = Arc::clone(&dropped_on_main);
            handles.push(rt.spawn(async move {
                assert!(!is_main_thread());
                assert_ne!(std::thread::current().id(), main_tid);

                let mut local_stack_val = i * 10;
                let observed_tid = run_on_main_thread(|_| {
                    assert!(is_main_thread());
                    local_stack_val += 5;
                    std::thread::current().id()
                });
                assert_eq!(observed_tid, main_tid);
                assert_eq!(local_stack_val, i * 10 + 5);

                drop_on_main_thread(MainThreadDropChecker { main_tid, counter });
                local_stack_val
            }));
        }

        let sum: usize = rt.block_on(async move {
            let mut total = 0;
            for h in handles {
                total += h.await.expect("task succeeded");
            }
            total
        });
        assert_eq!(sum, (0..8usize).map(|i| i * 10 + 5).sum::<usize>());
        assert_eq!(dropped_on_main.load(Ordering::SeqCst), 8);

        // 2. A background task aborted during `PgTokioRuntime::drop` still drops its deferred
        //    main-thread resource on `main_tid`.
        let counter_for_shutdown = Arc::clone(&dropped_on_main);
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        rt.spawn(async move {
            let _guard = MainThread::new(MainThreadDropChecker {
                main_tid,
                counter: counter_for_shutdown,
            });
            let _ = started_tx.send(());
            std::future::pending::<()>().await;
        });
        rt.block_on(async move {
            let _ = started_rx.await;
        });

        // 3. A panic inside `run_on_main_thread` on the leader (`is_parallel_worker == false`)
        //    unwinds directly out of `rt.block_on` on the main thread.
        let panic_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let h = rt.spawn(async move {
                run_on_main_thread(|_| -> usize {
                    panic!("expected main-thread failure");
                })
            });
            rt.block_on(h)
        }));
        let err = panic_res.expect_err("leader main-thread panic must unwind block_on");
        let msg = crate::postgres::customscan::mpp::panic_payload_message(err.as_ref());
        assert!(
            msg.contains("expected main-thread failure"),
            "unexpected panic message: {msg}"
        );

        drop(rt);
        assert_eq!(dropped_on_main.load(Ordering::SeqCst), 9);
    }

    #[test]
    fn early_exit_quiesces_in_flight_jobs_and_nested_spawned_tasks() {
        let main_tid = std::thread::current().id();
        let dropped_on_main = Arc::new(AtomicUsize::new(0));
        let stale_job_ran = Arc::new(AtomicBool::new(false));

        struct MainThreadResource {
            main_tid: std::thread::ThreadId,
            counter: Arc<AtomicUsize>,
        }
        crate::impl_safe_drop!(MainThreadResource, |self| {
            assert_eq!(std::thread::current().id(), self.main_tid);
            assert!(is_main_thread());
            self.counter.fetch_add(1, Ordering::SeqCst);
        });

        // Case 1: A background task queues a `run_on_main_thread` job right as the main thread
        // returns early from `block_on` (e.g., `LIMIT 1` satisfied), aborts the outer task handle,
        // and drops `PgTokioRuntime`. The blocked worker thread must be woken with
        // `MainThreadAbort`, unwind its stack, and have its `MainThread<MainThreadResource>`
        // drained on the main thread before `drop(rt)` returns, without leaking the stale job into
        // the next `PgTokioRuntime` scope.
        {
            let rt = PgTokioRuntime::new().expect("create runtime");
            let exec = Arc::clone(&rt.inner.main_executor);
            let counter = Arc::clone(&dropped_on_main);
            let stale_ran = Arc::clone(&stale_job_ran);
            let (ready_tx, ready_rx) = std::sync::mpsc::channel();
            let (go_tx, go_rx) = std::sync::mpsc::channel();

            let handle = rt.spawn(async move {
                let _resource = MainThread::new(MainThreadResource { main_tid, counter });
                ready_tx.send(()).unwrap();
                go_rx.recv().unwrap();
                let _ = try_run_on_main_thread(|_| {
                    stale_ran.store(true, Ordering::SeqCst);
                    Ok(())
                });
            });

            rt.block_on(async move {
                ready_rx.recv().unwrap();
            });

            // Now the main thread has exited `block_on`. Let the background task enter
            // `run_on_main_thread` and park inside `slot.cvar.wait` while no `block_on` loop is
            // running on the main thread.
            go_tx.send(()).unwrap();
            while !exec.has_pending_jobs.load(Ordering::Acquire) {
                std::thread::yield_now();
            }

            handle.abort();
            drop(rt);

            assert_eq!(dropped_on_main.load(Ordering::SeqCst), 1);
            assert!(!stale_job_ran.load(Ordering::SeqCst));
        }

        // Verify a subsequent query reusing the cached `BACKEND_RUNTIME` does not execute any
        // leftover job from the previous query.
        {
            let rt = PgTokioRuntime::new().expect("reuse runtime");
            let val = rt.block_on(rt.spawn(async move {
                run_on_main_thread(|_| {
                    assert!(is_main_thread());
                    42usize
                })
            }));
            assert_eq!(val.unwrap(), 42);
            assert!(!stale_job_ran.load(Ordering::SeqCst));
            drop(rt);
        }

        // Case 2: Multi-hop nested `tokio::spawn` tasks (mirroring `Spawner`'s outer supervisor
        // task -> inner `JoinSet` task holding `MainThread<T>`). Aborting the outer handle when
        // the stream drops is asynchronous; `drop(rt)` must wait for both the outer and inner
        // tasks to unwind and must drain the inner task's `MainThread<MainThreadResource>` on the
        // main thread before returning.
        {
            let rt = PgTokioRuntime::new().expect("reuse runtime");
            let counter = Arc::clone(&dropped_on_main);
            let (inner_started_tx, inner_started_rx) = tokio::sync::oneshot::channel();

            let outer_handle = rt.spawn(async move {
                let mut join_set = tokio::task::JoinSet::new();
                join_set.spawn(async move {
                    let _resource = MainThread::new(MainThreadResource { main_tid, counter });
                    let _ = inner_started_tx.send(());
                    std::future::pending::<()>().await;
                });
                while join_set.join_next().await.is_some() {}
            });

            rt.block_on(async move {
                inner_started_rx.await.unwrap();
            });

            outer_handle.abort();
            drop(rt);

            assert_eq!(dropped_on_main.load(Ordering::SeqCst), 2);
        }
    }
}

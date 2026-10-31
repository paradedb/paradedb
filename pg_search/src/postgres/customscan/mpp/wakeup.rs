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

//! Pre-allocated `AF_UNIX` `socketpair` wakeup pool and per-process Tokio `AsyncFd` fanout.
//!
//! Why `socketpair`s instead of Postgres `MyLatch`:
//! When running Tonic/gRPC on a Tokio runtime, the thread parks inside `mio::Poll::poll`
//! (`kqueue` on macOS, `epoll` on Linux). `pg_sys::SetLatch` writes to Postgres's private
//! `selfpipe_readfd`, which is not registered in Tokio's `mio` reactor, so a `SetLatch` will
//! not wake a parked Tokio runtime.
//!
//! Why a pre-allocated pool in `_PG_init`:
//! Postgres parallel workers are forked from `postmaster` (`fork()`, not `pthread_create`), so
//! file descriptors created by a query leader after `fork()` are not present in worker FD tables.
//! Allocating a pool of non-blocking `AF_UNIX` `socketpair`s in `_PG_init` before `postmaster`
//! forks any children gives every backend and parallel worker an inherited table of wakeup FDs.
//! Note that `_PG_init` runs during `process_shared_preload_libraries()`, before Postgres's
//! `set_max_safe_fds()` probes the remaining openable descriptors, so Postgres's VFD cache
//! automatically accounts for these descriptors without calling `ReserveExternalFD()`.
//!
//! Why `WakeupDispatcher` fans out to `AtomicWaker`s:
//! `tokio::io::unix::AsyncFd::poll_read_ready` stores only a single reader `Waker` (each call
//! overwrites the previous waker). A process participating in an MPP query has `W` concurrent
//! HTTP/2 connections (`2 * W` SPSC rings, each polling `poll_read` and `poll_write`
//! independently). Polling `AsyncFd` from one dedicated background task and fanning out to
//! per-ring `AtomicWaker`s prevents waker overwrites across rings and connections.

use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(test))]
use std::sync::atomic::{AtomicI32, AtomicUsize, fence};
use std::sync::{Arc, OnceLock, Weak};

use futures::task::AtomicWaker;
use parking_lot::Mutex;
use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

#[cfg(not(test))]
use crate::postgres::customscan::mpp::interrupt::cancel_pending;

/// Default number of socketpairs when initialized outside `_PG_init` (e.g. unit tests).
const DEFAULT_POOL_SIZE: usize = 128;

/// A non-blocking `AF_UNIX` socketpair inherited across `fork()`.
#[derive(Clone, Copy, Debug)]
pub struct WakeupSocketPair {
    pub read_fd: RawFd,
    pub write_fd: RawFd,
}

impl WakeupSocketPair {
    fn new() -> io::Result<Self> {
        let mut fds = [0 as libc::c_int; 2];
        // SAFETY: `fds` is a valid 2-element array.
        let rc = unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) };
        if rc != 0 {
            return Err(io::Error::last_os_error());
        }
        for &fd in &fds {
            // SAFETY: `fd` was just returned by `socketpair`.
            unsafe {
                let flags = libc::fcntl(fd, libc::F_GETFL);
                if flags >= 0 {
                    libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
                }
                let fd_flags = libc::fcntl(fd, libc::F_GETFD);
                if fd_flags >= 0 {
                    libc::fcntl(fd, libc::F_SETFD, fd_flags | libc::FD_CLOEXEC);
                }
            }
        }
        Ok(Self {
            read_fd: fds[0],
            write_fd: fds[1],
        })
    }
}

/// Index into the global `WAKEUP_POOL`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WakeupSlot(pub u32);

impl WakeupSlot {
    pub const UNSET: Self = Self(u32::MAX);
}

static WAKEUP_POOL: OnceLock<Vec<WakeupSocketPair>> = OnceLock::new();

fn allocate_pool(count: usize) -> Vec<WakeupSocketPair> {
    let mut pairs = Vec::with_capacity(count);
    for i in 0..count {
        match WakeupSocketPair::new() {
            Ok(pair) => pairs.push(pair),
            Err(e) => {
                // If the host has a tight `RLIMIT_NOFILE`, keep whatever pairs we opened so
                // modulo-indexing over `pairs` still works as long as at least 1 pair succeeded.
                if pairs.is_empty() {
                    panic!("mpp: failed to allocate wakeup socketpair {i}/{count}: {e}");
                }
                break;
            }
        }
    }
    pairs
}

/// Initialize the wakeup socketpair pool from `_PG_init` before `postmaster` forks backends.
pub fn init_wakeup_pool() {
    let _ = WAKEUP_POOL.get_or_init(|| {
        let count = unsafe {
            use pgrx::pg_sys;
            (pg_sys::MaxConnections
                + pg_sys::max_worker_processes
                + pg_sys::autovacuum_max_workers
                + pg_sys::max_wal_senders
                + 64)
                .clamp(DEFAULT_POOL_SIZE as i32, 1024) as usize
        };
        allocate_pool(count)
    });
}

fn pool() -> &'static [WakeupSocketPair] {
    WAKEUP_POOL.get_or_init(|| allocate_pool(DEFAULT_POOL_SIZE))
}

/// Return the [`WakeupSlot`] for the calling Postgres backend or parallel worker.
#[cfg(not(test))]
pub fn self_wakeup_slot() -> WakeupSlot {
    use pgrx::pg_sys;
    // `pg_sys::MyProcNumber` is the PG17+ global; PG15/16 carry the same value on
    // `MyProc->pgprocno`.
    #[cfg(any(feature = "pg15", feature = "pg16"))]
    let my_pgprocno: i32 = unsafe { (*pg_sys::MyProc).pgprocno };
    #[cfg(not(any(feature = "pg15", feature = "pg16")))]
    let my_pgprocno: i32 = unsafe { pg_sys::MyProcNumber };

    let len = pool().len();
    WakeupSlot((my_pgprocno.max(0) as usize % len) as u32)
}

/// Standalone unit-test binaries (`cargo test --lib`) do not link Postgres globals
/// (`MyProcNumber`). Assign a deterministic per-thread slot starting at index 16 so concurrent
/// test threads do not collide with each other or with explicit `WakeupSlot(0..4)` ring tests.
#[cfg(test)]
pub fn self_wakeup_slot() -> WakeupSlot {
    use std::sync::atomic::AtomicU32;
    static NEXT_TEST_SLOT: AtomicU32 = AtomicU32::new(16);
    thread_local! {
        static THREAD_SLOT: WakeupSlot = {
            let len = pool().len() as u32;
            let idx = NEXT_TEST_SLOT.fetch_add(1, Ordering::Relaxed);
            WakeupSlot(idx % len)
        };
    }
    THREAD_SLOT.with(|s| *s)
}

/// Write a 1-byte wakeup signal to `slot`. Non-blocking: if the socket buffer is already
/// non-empty (`EAGAIN`), the target's `AsyncFd` is already readable.
pub fn wake_slot(slot: WakeupSlot) {
    if slot == WakeupSlot::UNSET {
        return;
    }
    let pairs = pool();
    let pair = pairs[slot.0 as usize % pairs.len()];
    let byte: u8 = 1;
    // SAFETY: `pair.write_fd` is a valid non-blocking socket descriptor.
    unsafe {
        let _ = libc::write(pair.write_fd, &byte as *const u8 as *const libc::c_void, 1);
    }
}

/// Drain all pending wakeup bytes from `read_fd`.
fn drain_fd(read_fd: RawFd) {
    let mut buf = [0u8; 256];
    loop {
        // SAFETY: `read_fd` is a valid non-blocking socket descriptor and `buf` is valid.
        let n = unsafe { libc::read(read_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n < buf.len() as isize {
            break;
        }
    }
}

/// Wrapper around `RawFd` that implements `AsRawFd` without closing the descriptor on drop.
/// `WAKEUP_POOL` and `postmaster_alive_fds` own their descriptors for the lifetime of the
/// process across queries; dropping `AsyncFd<UnownedFd>` at query teardown must only deregister
/// from `mio` (`kqueue`/`epoll`).
#[repr(transparent)]
struct UnownedFd(RawFd);

impl AsRawFd for UnownedFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

// ============================================================================
// Postgres Signal & MyLatch <-> Tokio Reactor Bridge
// ============================================================================

/// Non-negative wakeup socket `write_fd` while the current backend/worker thread is between
/// `on_thread_park` and `on_thread_unpark` inside a [`super::interrupt::PgTokioRuntime`], or `-1`
/// when running on CPU / outside Tokio.
///
/// Read from [`pg_tokio_signal_trampoline`] so a signal (`SIGINT`, `SIGTERM`, `SIGUSR1`) arriving
/// in the few instructions between `on_thread_park` and the `kevent`/`epoll_wait` syscall writes
/// 1 byte into the wakeup socketpair and prevents `mio::Poll::poll` from entering sleep.
#[cfg(not(test))]
static TOKIO_PARKED_WAKEUP_FD: AtomicI32 = AtomicI32::new(-1);

#[cfg(not(test))]
static SIGNAL_HOOKS_PID: AtomicI32 = AtomicI32::new(0);
#[cfg(not(test))]
static ORIG_SIGINT_HANDLER: AtomicUsize = AtomicUsize::new(0);
#[cfg(not(test))]
static ORIG_SIGTERM_HANDLER: AtomicUsize = AtomicUsize::new(0);
#[cfg(not(test))]
static ORIG_SIGUSR1_HANDLER: AtomicUsize = AtomicUsize::new(0);

#[cfg(all(not(test), target_os = "macos"))]
unsafe fn errno_ptr() -> *mut libc::c_int {
    unsafe { libc::__error() }
}

#[cfg(all(not(test), target_os = "linux"))]
unsafe fn errno_ptr() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

/// Async-signal-safe trampoline chained onto Postgres's `SIGINT`, `SIGTERM`, and `SIGUSR1`
/// handlers. Invokes Postgres's handler (`wrapper_handler`, which sets `InterruptPending` /
/// `QueryCancelPending` / `ProcDiePending` / `ParallelMessagePending` and `MyLatch->is_set`), and
/// then writes 1 byte to [`TOKIO_PARKED_WAKEUP_FD`] if a [`super::interrupt::PgTokioRuntime`] is
/// currently parking or parked in `mio::Poll::poll`.
#[cfg(not(test))]
extern "C" fn pg_tokio_signal_trampoline(signo: libc::c_int) {
    unsafe {
        let saved_errno = *errno_ptr();
        let orig = match signo {
            libc::SIGINT => ORIG_SIGINT_HANDLER.load(Ordering::SeqCst),
            libc::SIGTERM => ORIG_SIGTERM_HANDLER.load(Ordering::SeqCst),
            libc::SIGUSR1 => ORIG_SIGUSR1_HANDLER.load(Ordering::SeqCst),
            _ => 0,
        };
        if orig != 0 && orig != libc::SIG_DFL && orig != libc::SIG_IGN {
            let handler: extern "C" fn(libc::c_int) = std::mem::transmute(orig);
            handler(signo);
        }
        let fd = TOKIO_PARKED_WAKEUP_FD.load(Ordering::SeqCst);
        if fd >= 0 {
            let byte: u8 = 1;
            let _ = libc::write(fd, &byte as *const u8 as *const libc::c_void, 1);
        }
        *errno_ptr() = saved_errno;
    }
}

/// Idempotently chain [`pg_tokio_signal_trampoline`] onto `SIGINT`, `SIGTERM`, and `SIGUSR1` in
/// the current backend or parallel worker process.
#[cfg(not(test))]
pub(crate) fn ensure_signal_wakeup_hooks_installed() {
    let pid = unsafe { libc::getpid() };
    if SIGNAL_HOOKS_PID.load(Ordering::Acquire) == pid {
        return;
    }
    let trampoline_addr = pg_tokio_signal_trampoline as *const () as usize;
    for (signo, slot) in [
        (libc::SIGINT, &ORIG_SIGINT_HANDLER),
        (libc::SIGTERM, &ORIG_SIGTERM_HANDLER),
        (libc::SIGUSR1, &ORIG_SIGUSR1_HANDLER),
    ] {
        unsafe {
            let mut old_act: libc::sigaction = std::mem::zeroed();
            if libc::sigaction(signo, std::ptr::null(), &mut old_act) != 0 {
                continue;
            }
            let cur = old_act.sa_sigaction;
            if cur == 0
                || cur == libc::SIG_DFL
                || cur == libc::SIG_IGN
                || cur == trampoline_addr
                || (old_act.sa_flags & libc::SA_SIGINFO) != 0
            {
                continue;
            }
            slot.store(cur, Ordering::SeqCst);
            let mut new_act = old_act;
            new_act.sa_sigaction = trampoline_addr;
            let _ = libc::sigaction(signo, &new_act, std::ptr::null_mut());
        }
    }
    SIGNAL_HOOKS_PID.store(pid, Ordering::Release);
}

#[cfg(test)]
pub(crate) fn ensure_signal_wakeup_hooks_installed() {}

/// Defensively clear [`TOKIO_PARKED_WAKEUP_FD`] and `MyLatch->maybe_sleeping` when exiting a
/// [`super::interrupt::HeldInterrupts`] scope so Postgres C code can never observe
/// `MyLatch->maybe_sleeping != 0` (`ResetLatch` asserts `maybe_sleeping == false`).
#[cfg(not(test))]
pub(crate) fn disarm_thread_park_state() {
    TOKIO_PARKED_WAKEUP_FD.store(-1, Ordering::Relaxed);
    unsafe {
        let latch = pgrx::pg_sys::MyLatch;
        if !latch.is_null() && (*latch).owner_pid == pgrx::pg_sys::MyProcPid {
            (*latch).maybe_sleeping = 0;
        }
    }
}

#[cfg(test)]
pub(crate) fn disarm_thread_park_state() {}

type WakerGroup = Arc<Mutex<Vec<Arc<AtomicWaker>>>>;

/// Shared state bridging a [`super::interrupt::PgTokioRuntime`]'s `on_thread_park` /
/// `on_thread_unpark` hooks, its `WakeupSlot` `AsyncFd`, and `postmaster_alive_fds` with its
/// interrupt waker and any attached [`WakeupDispatcher`]s.
pub(crate) struct PgReactorState {
    slot: WakeupSlot,
    read_fd: RawFd,
    #[cfg_attr(test, allow(dead_code))]
    write_fd: RawFd,
    interrupt_waker: Arc<AtomicWaker>,
    dispatcher_groups: Mutex<Vec<WakerGroup>>,
    cancel_notified: AtomicBool,
}

impl PgReactorState {
    pub(crate) fn new(slot: WakeupSlot) -> Arc<Self> {
        let pairs = pool();
        let pair = pairs[slot.0 as usize % pairs.len()];
        Arc::new(Self {
            slot,
            read_fd: pair.read_fd,
            write_fd: pair.write_fd,
            interrupt_waker: Arc::new(AtomicWaker::new()),
            dispatcher_groups: Mutex::new(Vec::new()),
            cancel_notified: AtomicBool::new(false),
        })
    }

    pub(crate) fn slot(&self) -> WakeupSlot {
        self.slot
    }

    pub(crate) fn interrupt_waker(&self) -> &Arc<AtomicWaker> {
        &self.interrupt_waker
    }

    pub(crate) fn reset_cancel_notification(&self) {
        self.cancel_notified.store(false, Ordering::Relaxed);
    }

    pub(crate) fn attach_dispatcher(&self, group: &WakerGroup) {
        let mut groups = self.dispatcher_groups.lock();
        if !groups.iter().any(|g| Arc::ptr_eq(g, group)) {
            groups.push(Arc::clone(group));
        }
    }

    pub(crate) fn detach_dispatcher(&self, group: &WakerGroup) {
        self.dispatcher_groups
            .lock()
            .retain(|g| !Arc::ptr_eq(g, group));
    }

    /// Wake the runtime's interrupt waker and every [`AtomicWaker`] registered across attached
    /// [`WakeupDispatcher`]s.
    pub(crate) fn wake_all(&self) {
        self.interrupt_waker.wake();
        let groups: Vec<WakerGroup> = self.dispatcher_groups.lock().clone();
        for group in groups {
            let list: Vec<Arc<AtomicWaker>> = group.lock().clone();
            for w in list {
                w.wake();
            }
        }
    }

    /// Tokio `on_thread_park` hook: mirrors Postgres's `WaitEventSetWait` pre-sleep protocol.
    ///
    /// Sets `MyLatch->maybe_sleeping = 1` and arms [`TOKIO_PARKED_WAKEUP_FD`], executes a
    /// `SeqCst` memory barrier, and checks whether `MyLatch->is_set` or `cancel_pending()` is
    /// already true. If so, disarms `maybe_sleeping`, resets `MyLatch`, and wakes all wakers
    /// before Tokio checks `has_pending_work`, preventing `mio::Poll::poll` from blocking.
    #[cfg(not(test))]
    pub(crate) fn on_thread_park(&self) {
        use pgrx::pg_sys;

        TOKIO_PARKED_WAKEUP_FD.store(self.write_fd, Ordering::SeqCst);
        let latch = unsafe { pg_sys::MyLatch };
        let owns_latch = !latch.is_null() && unsafe { (*latch).owner_pid == pg_sys::MyProcPid };
        if owns_latch && unsafe { (*latch).is_set == 0 } {
            unsafe {
                (*latch).maybe_sleeping = 1;
            }
        }
        fence(Ordering::SeqCst);

        let latch_was_set = owns_latch && unsafe { (*latch).is_set != 0 };
        if latch_was_set {
            unsafe {
                (*latch).maybe_sleeping = 0;
                pg_sys::ResetLatch(latch);
            }
        }
        let new_cancel = cancel_pending() && !self.cancel_notified.swap(true, Ordering::SeqCst);
        if latch_was_set || new_cancel {
            TOKIO_PARKED_WAKEUP_FD.store(-1, Ordering::SeqCst);
            if owns_latch {
                unsafe {
                    (*latch).maybe_sleeping = 0;
                }
            }
            self.wake_all();
        }
    }

    #[cfg(test)]
    pub(crate) fn on_thread_park(&self) {}

    /// Tokio `on_thread_unpark` hook: clears `MyLatch->maybe_sleeping` and
    /// [`TOKIO_PARKED_WAKEUP_FD`], resets `MyLatch` if set, and wakes all wakers if a signal or
    /// latch event interrupted `mio::Poll::poll`.
    #[cfg(not(test))]
    pub(crate) fn on_thread_unpark(&self) {
        use pgrx::pg_sys;

        TOKIO_PARKED_WAKEUP_FD.store(-1, Ordering::SeqCst);
        let latch = unsafe { pg_sys::MyLatch };
        let owns_latch = !latch.is_null() && unsafe { (*latch).owner_pid == pg_sys::MyProcPid };
        if owns_latch {
            unsafe {
                (*latch).maybe_sleeping = 0;
            }
        }
        let latch_was_set = owns_latch && unsafe { (*latch).is_set != 0 };
        if latch_was_set {
            unsafe {
                pg_sys::ResetLatch(latch);
            }
        }
        let new_cancel = cancel_pending() && !self.cancel_notified.swap(true, Ordering::SeqCst);
        if latch_was_set || new_cancel {
            self.wake_all();
        }
    }

    #[cfg(test)]
    pub(crate) fn on_thread_unpark(&self) {}

    /// Spawn the background `AsyncFd` tasks on `handle`:
    /// 1. `pool[self.slot].read_fd` for cross-process DSM SPSC ring wakeups and signal-trampoline
    ///    bytes.
    /// 2. `pg_sys::postmaster_alive_fds[POSTMASTER_FD_WATCH]` (when under postmaster) so an
    ///    unclean postmaster death (`SIGKILL` -> pipe EOF) immediately sets `ProcDiePending` and
    ///    wakes the runtime without polling timers.
    pub(crate) fn spawn_reactor_tasks(
        self: &Arc<Self>,
        handle: &tokio::runtime::Handle,
    ) -> io::Result<Vec<tokio::task::JoinHandle<()>>> {
        let _enter = handle.enter();
        let mut tasks = Vec::with_capacity(2);

        let read_fd = self.read_fd;
        drain_fd(read_fd);
        let async_fd = AsyncFd::with_interest(UnownedFd(read_fd), Interest::READABLE)?;
        let state_for_wakeup = Arc::clone(self);
        tasks.push(handle.spawn(async move {
            loop {
                let mut guard = match async_fd.readable().await {
                    Ok(g) => g,
                    Err(_) => break,
                };
                drain_fd(read_fd);
                guard.clear_ready();
                state_for_wakeup.wake_all();
            }
        }));

        #[cfg(not(test))]
        {
            use pgrx::pg_sys;
            let pm_fd =
                unsafe { pg_sys::postmaster_alive_fds[pg_sys::POSTMASTER_FD_WATCH as usize] };
            if pm_fd >= 0
                && let Ok(pm_async_fd) =
                    AsyncFd::with_interest(UnownedFd(pm_fd), Interest::READABLE)
            {
                let state_for_pm = Arc::clone(self);
                tasks.push(handle.spawn(async move {
                    loop {
                        let mut guard = match pm_async_fd.readable().await {
                            Ok(g) => g,
                            Err(_) => break,
                        };
                        if unsafe { !pg_sys::PostmasterIsAliveInternal() } {
                            // Synthesize the exact flags `die(SIGTERM)` sets so all
                            // cooperative `cancel_pending()` checks unwind out of `block_on`
                            // and the subsequent `check_for_interrupts()` exits cleanly.
                            unsafe {
                                pg_sys::ProcDiePending = 1;
                                pg_sys::InterruptPending = 1;
                            }
                            state_for_pm.wake_all();
                            break;
                        }
                        guard.clear_ready();
                    }
                }));
            }
        }

        Ok(tasks)
    }
}

type ReactorRegistry = Mutex<Vec<(tokio::runtime::Id, Weak<PgReactorState>)>>;

static REACTOR_REGISTRY: OnceLock<ReactorRegistry> = OnceLock::new();

fn reactor_registry() -> &'static ReactorRegistry {
    REACTOR_REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

pub(crate) fn register_reactor(id: tokio::runtime::Id, state: &Arc<PgReactorState>) {
    let mut reg = reactor_registry().lock();
    reg.retain(|(existing_id, weak)| *existing_id != id && weak.strong_count() > 0);
    reg.push((id, Arc::downgrade(state)));
}

pub(crate) fn unregister_reactor(id: tokio::runtime::Id) {
    reactor_registry()
        .lock()
        .retain(|(existing_id, weak)| *existing_id != id && weak.strong_count() > 0);
}

fn find_reactor(id: tokio::runtime::Id) -> Option<Arc<PgReactorState>> {
    reactor_registry()
        .lock()
        .iter()
        .find_map(|(existing_id, weak)| (*existing_id == id).then(|| weak.upgrade()).flatten())
}

/// Per-process wakeup fanout for a single MPP query execution.
///
/// Watches the calling process's `WakeupSlot` `read_fd` on the query's Tokio runtime and wakes
/// all registered per-ring [`AtomicWaker`]s whenever a peer writes a wakeup byte, a Postgres
/// cancel/die interrupt arrives, `postmaster` dies, or the dispatcher is stopped.
pub struct WakeupDispatcher {
    slot: WakeupSlot,
    wakers: WakerGroup,
    attached_reactor: Option<Arc<PgReactorState>>,
    driver_task: Option<tokio::task::JoinHandle<()>>,
}

impl WakeupDispatcher {
    /// Create a dispatcher for `slot`.
    pub fn new(slot: WakeupSlot) -> Self {
        Self {
            slot,
            wakers: Arc::new(Mutex::new(Vec::new())),
            attached_reactor: None,
            driver_task: None,
        }
    }

    pub fn slot(&self) -> WakeupSlot {
        self.slot
    }

    pub fn is_started(&self) -> bool {
        self.attached_reactor.is_some() || self.driver_task.is_some()
    }

    /// Detach or abort the background reader and synchronously wake all registered [`AtomicWaker`]s
    /// so any local shutdown watcher (`DsmWorkerShutdownWatcher`) or stream wakes immediately.
    pub fn stop(&mut self) {
        if let Some(reactor) = self.attached_reactor.take() {
            reactor.detach_dispatcher(&self.wakers);
        }
        if let Some(task) = self.driver_task.take() {
            task.abort();
        }
        let list: Vec<Arc<AtomicWaker>> = self.wakers.lock().clone();
        for w in list {
            w.wake();
        }
    }

    /// Register a new [`AtomicWaker`] that will be woken whenever `slot` receives a wakeup byte
    /// or a Postgres interrupt occurs.
    pub fn register_waker(&self) -> Arc<AtomicWaker> {
        let waker = Arc::new(AtomicWaker::new());
        self.wakers.lock().push(Arc::clone(&waker));
        waker
    }

    /// Attach this dispatcher to `handle`'s [`PgReactorState`] if `handle` belongs to a
    /// [`super::interrupt::PgTokioRuntime`] watching `self.slot`, or spawn a standalone `AsyncFd`
    /// reader loop onto `handle` (used in multi-slot unit tests).
    pub fn start(&mut self, handle: &tokio::runtime::Handle) -> io::Result<()> {
        if self.is_started() {
            return Ok(());
        }
        if let Some(reactor) = find_reactor(handle.id())
            && reactor.slot() == self.slot
        {
            reactor.attach_dispatcher(&self.wakers);
            self.attached_reactor = Some(reactor);
            return Ok(());
        }

        let pairs = pool();
        let read_fd = pairs[self.slot.0 as usize % pairs.len()].read_fd;
        drain_fd(read_fd);
        let _enter = handle.enter();
        let async_fd = AsyncFd::with_interest(UnownedFd(read_fd), Interest::READABLE)?;
        let wakers = Arc::clone(&self.wakers);
        let task = handle.spawn(async move {
            loop {
                let mut guard = match async_fd.readable().await {
                    Ok(g) => g,
                    Err(_) => break,
                };
                drain_fd(read_fd);
                guard.clear_ready();
                let list: Vec<Arc<AtomicWaker>> = wakers.lock().clone();
                for w in list {
                    w.wake();
                }
            }
        });
        self.driver_task = Some(task);
        Ok(())
    }
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because `drop` only detaches wakers and
// aborts the local Tokio `JoinHandle` without touching Postgres FFI, and `WakeupDispatcher` is
// exercised in standalone `#[test]` binaries that do not link Postgres globals.
impl Drop for WakeupDispatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

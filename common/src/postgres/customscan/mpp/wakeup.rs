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

//! Pre-allocated cross-process wakeup descriptor pool (`eventfd` on Linux, `AF_UNIX` `socketpair`
//! on macOS) and per-process Tokio `AsyncFd` fanout.
//!
//! Why wakeup descriptors instead of Postgres `MyLatch`:
//! When running Tonic/gRPC on a Tokio runtime, the thread parks inside `mio::Poll::poll`
//! (`kqueue` on macOS, `epoll` on Linux). `pg_sys::SetLatch` writes to Postgres's private
//! `selfpipe_readfd`, which is not registered in Tokio's `mio` reactor, so a `SetLatch` will
//! not wake a parked Tokio runtime.
//!
//! Why a pre-allocated pool in `_PG_init` sized by `max_worker_processes`:
//! Postgres parallel workers are forked from `postmaster` (`fork()`, not `pthread_create`), so
//! file descriptors created by a query leader after `fork()` are not present in worker FD tables.
//! Allocating a small pool of non-blocking wakeup descriptors in `_PG_init` before `postmaster`
//! forks any children gives every backend and parallel worker an inherited table of cross-process
//! wakeup FDs. Because every MPP query requires at least 2 parallel workers plus 1 leader, at most
//! `1.5 * max_worker_processes` processes can ever participate in MPP queries concurrently across
//! the cluster; sizing the inherited pool to `2 * max_worker_processes` keeps the inherited FD
//! count small and independent of `MaxConnections` and `autovacuum_worker_slots`.
//!
//! Meanwhile, intra-process signal wakeups (`PgReactorState` / `TOKIO_PARKED_WAKEUP_FD`) only need
//! a descriptor within the current process, so each backend lazily allocates a single
//! process-local wakeup descriptor on first `PgTokioRuntime` use and registers it with
//! `ReserveExternalFD()`.
//!
//! Why `WakeupDispatcher` fans out to `AtomicWaker`s:
//! `tokio::io::unix::AsyncFd::poll_read_ready` stores only a single reader `Waker` (each call
//! overwrites the previous waker). A process participating in an MPP query has `W` concurrent
//! HTTP/2 connections (`2 * W` SPSC rings, each polling `poll_read` and `poll_write`
//! independently). Polling `AsyncFd` from one dedicated background task and fanning out to
//! per-ring `AtomicWaker`s prevents waker overwrites across rings and connections.

use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
#[cfg(not(test))]
use std::sync::atomic::{AtomicUsize, fence};
use std::sync::{Arc, OnceLock};

use futures::task::AtomicWaker;
use parking_lot::Mutex;
use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

#[cfg(not(test))]
use crate::postgres::customscan::mpp::interrupt::cancel_pending;

/// Minimum number of cross-process wakeup slots allocated in `_PG_init` (and in unit tests).
const DEFAULT_POOL_SIZE: usize = 32;

/// A non-blocking wakeup descriptor pair (`eventfd` on Linux where `read_fd == write_fd`, or an
/// `AF_UNIX` `socketpair` on macOS).
#[derive(Clone, Copy, Debug)]
struct WakeupSocketPair {
    read_fd: RawFd,
    write_fd: RawFd,
}

impl WakeupSocketPair {
    #[cfg(target_os = "linux")]
    const FDS_PER_SLOT: usize = 1;
    #[cfg(not(target_os = "linux"))]
    const FDS_PER_SLOT: usize = 2;

    #[cfg(target_os = "linux")]
    fn new() -> io::Result<Self> {
        // SAFETY: `eventfd` with `EFD_NONBLOCK | EFD_CLOEXEC` is safe and returns a new fd or -1.
        let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            read_fd: fd,
            write_fd: fd,
        })
    }

    #[cfg(not(target_os = "linux"))]
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

    fn close(self) {
        unsafe {
            libc::close(self.read_fd);
            #[cfg(not(target_os = "linux"))]
            if self.write_fd != self.read_fd {
                libc::close(self.write_fd);
            }
        }
    }
}

/// Index into the global `WAKEUP_POOL`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WakeupSlot(u32);

impl WakeupSlot {
    pub const UNSET: Self = Self(u32::MAX);

    #[inline]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Reconstruct a [`WakeupSlot`] from the raw `u32` stored in the DSM wakeup slot table.
    #[inline]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }
}

struct WakeupPool {
    pairs: Vec<WakeupSocketPair>,
    owners_ptr: *mut AtomicI32,
}

// SAFETY: `pairs` is immutable after initialization, and `owners_ptr` points to a
// `MAP_SHARED | MAP_ANONYMOUS` region of `AtomicI32`s accessed exclusively through atomics.
unsafe impl Send for WakeupPool {}
unsafe impl Sync for WakeupPool {}

impl WakeupPool {
    fn new(count: usize) -> Self {
        let Some(owners_ptr) = allocate_shared_owners(count) else {
            return Self {
                pairs: Vec::new(),
                owners_ptr: std::ptr::NonNull::<AtomicI32>::dangling().as_ptr(),
            };
        };
        let pairs = allocate_pool(count);
        Self { pairs, owners_ptr }
    }

    fn owners(&self) -> &[AtomicI32] {
        if self.pairs.is_empty() {
            return &[];
        }
        unsafe { std::slice::from_raw_parts(self.owners_ptr, self.pairs.len()) }
    }
}

static WAKEUP_POOL: OnceLock<WakeupPool> = OnceLock::new();

/// File descriptors kept free in `postmaster` so `set_max_safe_fds()`, lockfiles, and listen
/// sockets never run out of descriptors at startup.
const POSTMASTER_FD_HEADROOM: usize = 128;

fn cap_pool_count_to_rlimit(needed: usize) -> usize {
    let fds_per_slot = WakeupSocketPair::FDS_PER_SLOT;
    unsafe {
        let mut rlim: libc::rlimit = std::mem::zeroed();
        if libc::getrlimit(libc::RLIMIT_NOFILE, &mut rlim) != 0 {
            return needed;
        }
        let required_fds = (needed
            .saturating_mul(fds_per_slot)
            .saturating_add(POSTMASTER_FD_HEADROOM)) as libc::rlim_t;
        if rlim.rlim_cur < required_fds && rlim.rlim_cur < rlim.rlim_max {
            let new_rlim = libc::rlimit {
                rlim_cur: required_fds.min(rlim.rlim_max),
                rlim_max: rlim.rlim_max,
            };
            if libc::setrlimit(libc::RLIMIT_NOFILE, &new_rlim) == 0 {
                rlim = new_rlim;
            }
        }
        let cur = usize::try_from(rlim.rlim_cur).unwrap_or(usize::MAX);
        let max_slots = cur.saturating_sub(POSTMASTER_FD_HEADROOM) / fds_per_slot;
        needed.min(max_slots)
    }
}

fn allocate_pool(count: usize) -> Vec<WakeupSocketPair> {
    let mut pairs = Vec::with_capacity(count);
    for _ in 0..count {
        match WakeupSocketPair::new() {
            Ok(pair) => pairs.push(pair),
            Err(_) => {
                // Release a batch of pairs so `postmaster` retains safe FD headroom for
                // `set_max_safe_fds()` and startup files.
                let release_pairs =
                    (POSTMASTER_FD_HEADROOM / WakeupSocketPair::FDS_PER_SLOT).min(pairs.len());
                for _ in 0..release_pairs {
                    if let Some(pair) = pairs.pop() {
                        pair.close();
                    }
                }
                break;
            }
        }
    }
    pairs
}

fn allocate_shared_owners(len: usize) -> Option<*mut AtomicI32> {
    if len == 0 {
        return None;
    }
    let byte_len = (len * std::mem::size_of::<AtomicI32>()).max(4096);
    unsafe {
        let ptr = libc::mmap(
            std::ptr::null_mut(),
            byte_len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        if ptr == libc::MAP_FAILED {
            return None;
        }
        let atomics = ptr as *mut AtomicI32;
        for i in 0..len {
            std::ptr::write(atomics.add(i), AtomicI32::new(0));
        }
        Some(atomics)
    }
}

/// Initialize the cross-process wakeup descriptor pool from `_PG_init` before `postmaster` forks
/// backends.
///
/// Sized from `2 * max_worker_processes` (floored at `DEFAULT_POOL_SIZE = 32`) and capped so the
/// inherited descriptors stay well below `max_files_per_process`. Because every MPP query requires
/// at least 2 parallel workers + 1 leader, at most `1.5 * max_worker_processes` processes can ever
/// participate in MPP concurrently, so `2 * max_worker_processes` can always saturate
/// `max_parallel_workers` without scaling with `MaxConnections` or `autovacuum_worker_slots`.
pub fn init_wakeup_pool() {
    let _ = WAKEUP_POOL.get_or_init(|| {
        let (needed, max_safe_slots) = unsafe {
            use pgrx::pg_sys;
            let needed = (pg_sys::max_worker_processes.saturating_mul(2))
                .max(DEFAULT_POOL_SIZE as i32) as usize;
            let max_files = pg_sys::max_files_per_process.max(0) as usize;
            let safe_fds = max_files.saturating_sub(POSTMASTER_FD_HEADROOM) / 2;
            let max_safe_slots = (safe_fds / WakeupSocketPair::FDS_PER_SLOT).max(DEFAULT_POOL_SIZE);
            (needed, max_safe_slots)
        };
        WakeupPool::new(cap_pool_count_to_rlimit(needed.min(max_safe_slots)))
    });
}

fn pool() -> &'static WakeupPool {
    WAKEUP_POOL.get_or_init(|| WakeupPool::new(cap_pool_count_to_rlimit(DEFAULT_POOL_SIZE)))
}

fn lookup_slot_pair(slot: WakeupSlot) -> io::Result<WakeupSocketPair> {
    let pairs = &pool().pairs;
    pairs.get(slot.0 as usize).copied().ok_or_else(|| {
        io::Error::other(format!(
            "no more filehandles: wakeup slot {} exceeds pre-allocated wakeup pool of {} slots (increase RLIMIT_NOFILE / ulimit -n)",
            slot.0,
            pairs.len()
        ))
    })
}

#[cfg(not(test))]
fn current_owner_id() -> i32 {
    unsafe { libc::getpid().max(1) }
}

#[cfg(test)]
fn current_owner_id() -> i32 {
    1
}

#[cfg(not(test))]
fn is_owner_dead(pid: i32) -> bool {
    if pid <= 0 {
        return true;
    }
    unsafe { libc::kill(pid, 0) != 0 && *errno_ptr() == libc::ESRCH }
}

#[cfg(test)]
fn is_owner_dead(pid: i32) -> bool {
    pid <= 0
}

/// True if at least `min_count` cross-process [`WakeupSlot`]s are currently available in
/// `WAKEUP_POOL` (either free or held by a process that exited uncleanly).
pub fn has_available_wakeup_slots(min_count: usize) -> bool {
    if min_count == 0 {
        return true;
    }
    let p = pool();
    if p.pairs.len() < min_count {
        return false;
    }
    let mut available = 0usize;
    for owner in p.owners() {
        let cur = owner.load(Ordering::Acquire);
        if cur == 0 || is_owner_dead(cur) {
            available += 1;
            if available >= min_count {
                return true;
            }
        }
    }
    false
}

/// RAII reservation of `1 + launched` [`WakeupSlot`]s from the shared `WAKEUP_POOL` for a single
/// MPP query.
pub struct WakeupSlotReservation {
    slots: Mutex<Vec<WakeupSlot>>,
}

impl WakeupSlotReservation {
    pub fn slots(&self) -> Vec<WakeupSlot> {
        self.slots.lock().clone()
    }

    /// Release all reserved slots back to `WAKEUP_POOL`. Idempotent.
    pub fn release(&self) {
        let taken = std::mem::take(&mut *self.slots.lock());
        if taken.is_empty() {
            return;
        }
        let p = pool();
        let owners = p.owners();
        for slot in taken {
            let idx = slot.0 as usize;
            if let Some(pair) = p.pairs.get(idx) {
                drain_fd(pair.read_fd);
            }
            if let Some(owner) = owners.get(idx) {
                owner.store(0, Ordering::Release);
            }
        }
    }
}

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because `release` only drains
// non-blocking wakeup descriptors and clears atomic slot-owner integers without touching Postgres
// FFI, and must return slots to the shared pool on unwind or in standalone tests.
impl Drop for WakeupSlotReservation {
    fn drop(&mut self) {
        self.release();
    }
}

/// Reserve `count` distinct [`WakeupSlot`]s from `WAKEUP_POOL` for an MPP query, returning `None`
/// if fewer than `count` slots are available so the caller can fall back to serial execution.
pub fn reserve_wakeup_slots(count: usize) -> Option<WakeupSlotReservation> {
    if count == 0 {
        return Some(WakeupSlotReservation {
            slots: Mutex::new(Vec::new()),
        });
    }
    let p = pool();
    if p.pairs.len() < count {
        return None;
    }
    let owners = p.owners();
    let my_id = current_owner_id();
    let mut claimed = Vec::with_capacity(count);

    // Pass 1: claim free slots (`owner == 0`).
    for (idx, owner) in owners.iter().enumerate() {
        if owner
            .compare_exchange(0, my_id, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            drain_fd(p.pairs[idx].read_fd);
            claimed.push(WakeupSlot(idx as u32));
            if claimed.len() == count {
                return Some(WakeupSlotReservation {
                    slots: Mutex::new(claimed),
                });
            }
        }
    }

    // Pass 2: reclaim slots whose owning process died uncleanly (`SIGKILL` / `ESRCH`).
    for (idx, owner) in owners.iter().enumerate() {
        let cur = owner.load(Ordering::Acquire);
        if cur != 0
            && cur != my_id
            && is_owner_dead(cur)
            && owner
                .compare_exchange(cur, my_id, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        {
            drain_fd(p.pairs[idx].read_fd);
            claimed.push(WakeupSlot(idx as u32));
            if claimed.len() == count {
                return Some(WakeupSlotReservation {
                    slots: Mutex::new(claimed),
                });
            }
        }
    }

    for slot in claimed {
        owners[slot.0 as usize].store(0, Ordering::Release);
    }
    None
}

#[inline]
unsafe fn write_wakeup_fd(write_fd: RawFd) {
    #[cfg(target_os = "linux")]
    unsafe {
        let val: u64 = 1;
        let _ = libc::write(
            write_fd,
            &val as *const u64 as *const libc::c_void,
            std::mem::size_of::<u64>(),
        );
    }
    #[cfg(not(target_os = "linux"))]
    unsafe {
        let byte: u8 = 1;
        let _ = libc::write(write_fd, &byte as *const u8 as *const libc::c_void, 1);
    }
}

/// Write a wakeup signal to `slot`. Non-blocking: if the descriptor is already readable (`EAGAIN`),
/// the target's `AsyncFd` will already wake.
pub fn wake_slot(slot: WakeupSlot) {
    if slot == WakeupSlot::UNSET {
        return;
    }
    let Some(&pair) = pool().pairs.get(slot.0 as usize) else {
        return;
    };
    // SAFETY: `pair.write_fd` is a valid non-blocking wakeup descriptor.
    unsafe {
        write_wakeup_fd(pair.write_fd);
    }
}

/// Drain all pending wakeup notifications from `read_fd`.
fn drain_fd(read_fd: RawFd) {
    #[cfg(target_os = "linux")]
    unsafe {
        let mut val: u64 = 0;
        let _ = libc::read(
            read_fd,
            &mut val as *mut u64 as *mut libc::c_void,
            std::mem::size_of::<u64>(),
        );
    }
    #[cfg(not(target_os = "linux"))]
    {
        let mut buf = [0u8; 256];
        loop {
            // SAFETY: `read_fd` is a valid non-blocking socket descriptor and `buf` is valid.
            let n =
                unsafe { libc::read(read_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if n < buf.len() as isize {
                break;
            }
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

/// Non-negative wakeup `write_fd` while the Postgres main thread is between
/// `on_thread_park` and `on_thread_unpark` inside [`super::interrupt::PgTokioRuntime::block_on`],
/// or `-1` when running on CPU / outside `block_on`.
///
/// Read from [`pg_tokio_signal_trampoline`] so a signal (`SIGINT`, `SIGTERM`, `SIGUSR1`) arriving
/// after `on_thread_park` writes a wakeup notification into the process-local wakeup descriptor and
/// wakes the `AsyncFd` reactor watcher, which in turn unparks the main thread.
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
/// then writes a wakeup notification to [`TOKIO_PARKED_WAKEUP_FD`] if a
/// [`super::interrupt::PgTokioRuntime`] is currently parking or parked in `mio::Poll::poll`.
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
            write_wakeup_fd(fd);
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

/// Defensively clear [`TOKIO_PARKED_WAKEUP_FD`] when exiting a [`super::interrupt::HeldInterrupts`]
/// scope so signal delivery outside `block_on` never writes to a stale wakeup descriptor.
#[cfg(not(test))]
pub(crate) fn disarm_thread_park_state() {
    if TOKIO_PARKED_WAKEUP_FD.load(Ordering::Relaxed) < 0 {
        return;
    }
    TOKIO_PARKED_WAKEUP_FD.store(-1, Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn disarm_thread_park_state() {}

type WakerGroup = Arc<Mutex<Vec<Arc<AtomicWaker>>>>;

#[cfg(not(test))]
static PROCESS_REACTOR_WAKEUP_PAIR: OnceLock<Mutex<Option<(i32, WakeupSocketPair)>>> =
    OnceLock::new();

fn process_local_reactor_wakeup_pair() -> io::Result<WakeupSocketPair> {
    #[cfg(not(test))]
    {
        let pid = unsafe { libc::getpid() };
        let lock = PROCESS_REACTOR_WAKEUP_PAIR.get_or_init(|| Mutex::new(None));
        let mut guard = lock.lock();
        if let Some((owner_pid, pair)) = *guard
            && owner_pid == pid
        {
            return Ok(pair);
        }
        let pair = WakeupSocketPair::new()?;
        // Inform Postgres's VFD cache (`fd.c`) of the long-lived descriptor(s) held by this
        // backend's intra-process reactor wakeup pair.
        unsafe {
            for _ in 0..WakeupSocketPair::FDS_PER_SLOT {
                pgrx::pg_sys::ReserveExternalFD();
            }
        }
        *guard = Some((pid, pair));
        Ok(pair)
    }
    #[cfg(test)]
    {
        thread_local! {
            static TEST_REACTOR_PAIR: std::cell::OnceCell<WakeupSocketPair> =
                const { std::cell::OnceCell::new() };
        }
        TEST_REACTOR_PAIR.with(|cell| {
            if let Some(&pair) = cell.get() {
                return Ok(pair);
            }
            let pair = WakeupSocketPair::new()?;
            let _ = cell.set(pair);
            Ok(pair)
        })
    }
}

/// Shared state bridging [`super::interrupt::PgTokioRuntime::block_on`]'s main-thread park/unpark
/// protocol, its process-local signal wakeup `AsyncFd`, and `postmaster_alive_fds` with its
/// interrupt waker and any attached [`WakeupDispatcher`]s.
pub struct PgReactorState {
    read_fd: RawFd,
    #[cfg_attr(test, allow(dead_code))]
    write_fd: RawFd,
    interrupt_waker: Arc<AtomicWaker>,
    dispatcher_groups: Mutex<Vec<WakerGroup>>,
    cancel_notified: AtomicBool,
}

impl PgReactorState {
    pub(crate) fn new() -> io::Result<Arc<Self>> {
        let pair = process_local_reactor_wakeup_pair()?;
        Ok(Arc::new(Self {
            read_fd: pair.read_fd,
            write_fd: pair.write_fd,
            interrupt_waker: Arc::new(AtomicWaker::new()),
            dispatcher_groups: Mutex::new(Vec::new()),
            cancel_notified: AtomicBool::new(false),
        }))
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

    /// Main-thread pre-park protocol called by [`super::interrupt::PgTokioRuntime::block_on`].
    ///
    /// Arms [`TOKIO_PARKED_WAKEUP_FD`], executes a `SeqCst` memory barrier, and checks whether
    /// `MyLatch->is_set` or `cancel_pending()` is already true (set by `SetLatch` in a signal
    /// handler that ran before `TOKIO_PARKED_WAKEUP_FD` was armed). If so, resets `MyLatch`,
    /// disarms `TOKIO_PARKED_WAKEUP_FD`, and wakes all wakers before the main thread parks.
    #[cfg(not(test))]
    pub(crate) fn on_thread_park(&self) {
        use pgrx::pg_sys;

        TOKIO_PARKED_WAKEUP_FD.store(self.write_fd, Ordering::SeqCst);
        fence(Ordering::SeqCst);

        let latch = unsafe { pg_sys::MyLatch };
        let owns_latch = !latch.is_null() && unsafe { (*latch).owner_pid == pg_sys::MyProcPid };
        let latch_was_set = owns_latch && unsafe { (*latch).is_set != 0 };
        if latch_was_set {
            unsafe {
                pg_sys::ResetLatch(latch);
            }
        }
        let new_cancel = cancel_pending() && !self.cancel_notified.swap(true, Ordering::SeqCst);
        if latch_was_set || new_cancel {
            TOKIO_PARKED_WAKEUP_FD.store(-1, Ordering::SeqCst);
            self.wake_all();
        }
    }

    #[cfg(test)]
    pub(crate) fn on_thread_park(&self) {}

    /// Main-thread post-park protocol called by [`super::interrupt::PgTokioRuntime::block_on`]:
    /// clears [`TOKIO_PARKED_WAKEUP_FD`], resets `MyLatch` if set, and wakes all wakers if a
    /// signal or latch event arrived while parked.
    #[cfg(not(test))]
    pub(crate) fn on_thread_unpark(&self) {
        use pgrx::pg_sys;

        TOKIO_PARKED_WAKEUP_FD.store(-1, Ordering::SeqCst);
        let latch = unsafe { pg_sys::MyLatch };
        let owns_latch = !latch.is_null() && unsafe { (*latch).owner_pid == pg_sys::MyProcPid };
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
    /// 1. `self.read_fd` for process-local signal-trampoline wakeups.
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
        // SAFETY: `read_fd` is owned by `LOCAL_WAKEUP_PAIR` / `WAKEUP_POOL` for the process
        // lifetime and outlives `async_fd`.
        let async_fd =
            unsafe { AsyncFd::register_with_interest(UnownedFd(read_fd), Interest::READABLE) }?;
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
                    // SAFETY: `pm_fd` is owned by `postmaster_alive_fds` for the backend lifetime
                    // and outlives `pm_async_fd`.
                    unsafe {
                        AsyncFd::register_with_interest(UnownedFd(pm_fd), Interest::READABLE)
                    }
            {
                let state_for_pm = Arc::clone(self);
                tasks.push(handle.spawn(async move {
                    loop {
                        let mut guard = match pm_async_fd.readable().await {
                            Ok(g) => g,
                            Err(_) => break,
                        };
                        let mut c: u8 = 0;
                        let rc =
                            unsafe { libc::read(pm_fd, &mut c as *mut u8 as *mut libc::c_void, 1) };
                        let postmaster_dead = if rc < 0 {
                            let err = io::Error::last_os_error();
                            !matches!(
                                err.kind(),
                                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                            )
                        } else {
                            true
                        };
                        if postmaster_dead {
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

    /// Attach this dispatcher to `handle`'s [`PgReactorState`] (so Postgres signals, latch events,
    /// and postmaster death wake registered wakers) and spawn an `AsyncFd` reader loop watching
    /// `self.slot` for cross-process DSM ring wakeups.
    pub fn start(&mut self, handle: &tokio::runtime::Handle) -> io::Result<()> {
        if self.is_started() {
            return Ok(());
        }
        let read_fd = lookup_slot_pair(self.slot)?.read_fd;
        drain_fd(read_fd);
        let _enter = handle.enter();
        // SAFETY: `read_fd` is owned by `WAKEUP_POOL` for the process lifetime and outlives
        // `async_fd`.
        let async_fd =
            unsafe { AsyncFd::register_with_interest(UnownedFd(read_fd), Interest::READABLE) }?;
        if let Some(reactor) = super::interrupt::current_reactor_state(handle) {
            reactor.attach_dispatcher(&self.wakers);
            self.attached_reactor = Some(reactor);
        }
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

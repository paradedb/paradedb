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
use std::sync::{Arc, OnceLock};

use futures::task::AtomicWaker;
use parking_lot::Mutex;
use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

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
/// `WAKEUP_POOL` owns the descriptors for the lifetime of the process across queries; dropping
/// `AsyncFd<UnownedFd>` at query teardown must only deregister from `mio` (`kqueue`/`epoll`).
#[repr(transparent)]
struct UnownedFd(RawFd);

impl AsRawFd for UnownedFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

/// Per-process wakeup fanout for a single MPP query execution.
///
/// Watches the calling process's `WakeupSlot` `read_fd` on the query's Tokio runtime and wakes
/// all registered per-ring [`AtomicWaker`]s whenever a peer writes a wakeup byte.
pub struct WakeupDispatcher {
    slot: WakeupSlot,
    wakers: Arc<Mutex<Vec<Arc<AtomicWaker>>>>,
    driver_task: Option<tokio::task::JoinHandle<()>>,
}

impl WakeupDispatcher {
    /// Create a dispatcher for `slot` and drain any leftover bytes from a prior query.
    pub fn new(slot: WakeupSlot) -> Self {
        let pairs = pool();
        let read_fd = pairs[slot.0 as usize % pairs.len()].read_fd;
        drain_fd(read_fd);
        Self {
            slot,
            wakers: Arc::new(Mutex::new(Vec::new())),
            driver_task: None,
        }
    }

    pub fn slot(&self) -> WakeupSlot {
        self.slot
    }

    pub fn is_started(&self) -> bool {
        self.driver_task.is_some()
    }

    pub fn stop(&mut self) {
        if let Some(task) = self.driver_task.take() {
            task.abort();
        }
    }

    /// Register a new [`AtomicWaker`] that will be woken whenever `slot` receives a wakeup byte.
    pub fn register_waker(&self) -> Arc<AtomicWaker> {
        let waker = Arc::new(AtomicWaker::new());
        self.wakers.lock().push(Arc::clone(&waker));
        waker
    }

    /// Spawn the `AsyncFd` reader loop onto `handle`.
    pub fn start(&mut self, handle: &tokio::runtime::Handle) -> io::Result<()> {
        let pairs = pool();
        let read_fd = pairs[self.slot.0 as usize % pairs.len()].read_fd;
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

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because `drop` only aborts the local
// Tokio `JoinHandle` without touching Postgres FFI, and `WakeupDispatcher` is exercised in
// standalone `#[test]` binaries that do not link Postgres globals.
impl Drop for WakeupDispatcher {
    fn drop(&mut self) {
        if let Some(task) = self.driver_task.take() {
            task.abort();
        }
    }
}

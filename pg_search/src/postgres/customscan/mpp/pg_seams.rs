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

//! The two PostgreSQL primitives the shared-memory transport leaves to the embedder: how to wake a
//! blocked consumer, and how to check for cancellation.
//!
//! The transport itself lives in `datafusion_distributed::shm` and is PG-free; these impls are
//! the only MPP transport code that still touches `pg_sys`.

use datafusion::common::{DataFusionError, Result};
use datafusion_distributed::shm::{Interrupt, Wakeup};

use crate::postgres::customscan::mpp::interrupt::{cancel_pending, interrupted};

/// A receiver backend as `(pgprocno, pid)`. The `pid` rules out a recycled `PGPROC` slot.
type ReceiverProc = (i32, i32);

/// Pack `pgprocno` (low 32 bits) + `pid` (high 32 bits) into the one `u64` token the ring stores.
/// A producer's single `Acquire` load then can't observe a torn `(new_pgprocno, old_pid)` pair and
/// wake the wrong backend.
#[inline]
pub fn pack_receiver(pgprocno: i32, pid: i32) -> u64 {
    ((pid as u32 as u64) << 32) | (pgprocno as u32 as u64)
}

#[inline]
fn unpack_receiver(packed: u64) -> ReceiverProc {
    (packed as u32 as i32, (packed >> 32) as u32 as i32)
}

/// Wakes a consumer backend by `SetLatch`. The token is the `(pgprocno, pid)` the receiver packed
/// via [`pack_receiver`]; the transport hands it back here after every publish.
pub struct PgWakeup;

impl Wakeup for PgWakeup {
    fn wake(&self, token: u64) {
        let (pgprocno, expected_pid) = unpack_receiver(token);
        if pgprocno < 0 {
            return;
        }
        // The pg_sys path is cfg'd out of the lib-test binary: the macOS flat-namespace linker
        // aborts at process start on an unresolved extern static like `ProcGlobal`, so any code
        // referencing it must be absent from the test binary entirely.
        #[cfg(not(test))]
        unsafe {
            wake_receiver_via_pg_sys(pgprocno, expected_pid);
        }
        #[cfg(test)]
        {
            let _ = expected_pid;
        }
    }
}

/// Resolve `ProcGlobal->allProcs[pgprocno]`, confirm `proc->pid == expected_pid` (so a recycled
/// PGPROC slot doesn't disturb an unrelated tenant), and `SetLatch` it.
///
/// Resolving by `pgprocno + pid` rather than `BackendPidGetProc(pid)` avoids scanning the whole
/// proc array on the send hot path.
///
/// # Safety
/// Must run on the backend thread (the plan-node poll); `pg_sys::SetLatch` is itself cross-thread
/// safe but pgrx's `check_active_thread` wrapper requires the backend thread.
#[cfg(not(test))]
unsafe fn wake_receiver_via_pg_sys(pgprocno: i32, expected_pid: i32) {
    use pgrx::pg_sys;

    let proc_global = unsafe { pg_sys::ProcGlobal };
    if proc_global.is_null() {
        return;
    }
    let all_proc_count = unsafe { (*proc_global).allProcCount };
    // Defense in depth: any attached backend can corrupt the receiver token in DSM. The negative
    // range is guarded in `wake`; this guards the positive range against the array's actual size.
    if pgprocno < 0 || (pgprocno as u32) >= all_proc_count {
        return;
    }
    let all_procs = unsafe { (*proc_global).allProcs };
    if all_procs.is_null() {
        return;
    }
    let proc = unsafe { all_procs.add(pgprocno as usize) };
    let current_pid = unsafe { (*proc).pid };
    if current_pid != expected_pid {
        return;
    }
    // PGPROC owns the Latch by value at `procLatch`; we want a `*mut Latch` into that slot.
    unsafe { pg_sys::SetLatch(&raw mut (*proc).procLatch) };
}

/// Cancellation seam, checked at the transport's block points (the send spin and the consumer pull
/// loop). Those run under `Runtime::block_on`, so it bails cooperatively with an error rather than
/// servicing the interrupt here: a die taken inside `block_on` would `proc_exit` out of the live
/// runtime. The caller services the deferred interrupt once `block_on` returns. See `mpp::interrupt`.
pub struct PgInterrupt;

impl Interrupt for PgInterrupt {
    fn check(&self) -> Result<(), DataFusionError> {
        if cancel_pending() {
            return Err(interrupted());
        }
        Ok(())
    }
}

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

//! Low-level shm_mq primitives for the MPP mesh: `ShmMqSender`,
//! `ShmMqReceiver`, and the alignment helpers used to size queue slots.
//!
//! The N×K queue layout is described in
//! [`super::dsm`](crate::postgres::customscan::mpp::dsm); this module owns
//! only the per-slot FFI wrappers.

use pgrx::pg_sys;

use crate::mpp_log;
use crate::parallel_worker::mqueue::{
    MessageQueueReceiver, MessageQueueRecvError, MessageQueueSendError, MessageQueueSender,
};
use crate::postgres::customscan::mpp::transport::{
    BatchChannelReceiver, BatchChannelSender, RecvOutcome,
};
use datafusion::common::DataFusionError;

/// MAXALIGN-DOWN of `queue_bytes`. Postgres requires shm_mq slot sizes to be
/// MAXALIGN-aligned; this rounds the user request down so every slot in a
/// queue array is the same size and slot indexing is a simple multiply.
#[inline]
pub(super) fn aligned_queue_bytes(queue_bytes: usize) -> usize {
    const MAXIMUM_ALIGNOF: usize = pg_sys::MAXIMUM_ALIGNOF as usize;
    queue_bytes & !(MAXIMUM_ALIGNOF - 1)
}

/// MAXALIGN-UP `n` to the next multiple of `MAXIMUM_ALIGNOF`. Returns `None`
/// on overflow. Used by [`super::dsm::compute_dsm_layout`] to keep section
/// boundaries inside the DSM region MAXALIGN-aligned.
#[inline]
pub(super) fn align_up_maxalign_checked(n: usize) -> Option<usize> {
    const MAXIMUM_ALIGNOF: usize = pg_sys::MAXIMUM_ALIGNOF as usize;
    let mask = MAXIMUM_ALIGNOF - 1;
    n.checked_add(mask).map(|x| x & !mask)
}

/// shm_mq-backed `BatchChannelSender`. Wraps `MessageQueueSender` so we reuse
/// its detach-on-drop behavior and the pgrx-safe FFI.
///
/// `unsafe impl Send` below is safe **only** when the sender is *used* from
/// a thread that owns a valid `PGPROC` — i.e., the main backend thread or a
/// dedicated parallel-worker backend. The blocking `shm_mq_send(nowait=false)`
/// path uses `WaitLatch` + `CHECK_FOR_INTERRUPTS` — both process-global
/// Postgres primitives, not thread-safe off a backend thread.
pub(crate) struct ShmMqSender {
    inner: MessageQueueSender,
    attach_thread: std::thread::ThreadId,
    /// Async serialization lock around the shm_mq handle. Held by [`MppSender::send_*_traced`]
    /// across the cooperative-drain spin to keep partial sends from interleaving — see the
    /// [`BatchChannelSender::send_lock`] doc and PG's `shm_mq_send` invariants. Multiple
    /// [`MppSender`] clones share the same `Arc<dyn BatchChannelSender>` to multiplex
    /// `(stage_id, partition)` channels onto one shm_mq, and yielding between `try_send_bytes`
    /// retries can otherwise stitch one sender's length prefix to another sender's payload and
    /// corrupt the queue.
    send_lock: tokio::sync::Mutex<()>,
}

// SAFETY: see struct doc.
unsafe impl Send for ShmMqSender {}
// SAFETY: `MppSender` ensures all `send_*` paths execute on the attach thread (a
// `debug_assert!` in `send_bytes` / `try_send_bytes` pins this), so cross-thread sharing of
// `&ShmMqSender` never actually crosses threads at the FFI boundary. The Sync bound is there so
// multiple `MppSender`s can hold `Arc<dyn BatchChannelSender>` clones of the same underlying
// queue (the multi-partition fan-out pattern) without the type system rejecting the `Arc::clone`
// at compile time.
unsafe impl Sync for ShmMqSender {}

impl ShmMqSender {
    /// # Safety
    /// - `seg` must be a valid `dsm_segment*` (or NULL on workers).
    /// - `mq` must point to a shm_mq region that has been `shm_mq_create`'d
    ///   at its address with the expected size and has had
    ///   `shm_mq_set_receiver` called by the peer.
    pub(super) unsafe fn attach(seg: *mut pg_sys::dsm_segment, mq: *mut pg_sys::shm_mq) -> Self {
        unsafe {
            Self {
                inner: MessageQueueSender::new(seg, mq),
                attach_thread: std::thread::current().id(),
                send_lock: tokio::sync::Mutex::new(()),
            }
        }
    }
}

impl BatchChannelSender for ShmMqSender {
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), DataFusionError> {
        debug_assert_eq!(
            std::thread::current().id(),
            self.attach_thread,
            "ShmMqSender::send_bytes called off the attach thread"
        );
        self.inner.send(bytes).map_err(|e| match e {
            MessageQueueSendError::Detached => {
                DataFusionError::Execution("mpp: shm_mq sender detached".into())
            }
            MessageQueueSendError::WouldBlock => {
                DataFusionError::Execution("mpp: shm_mq send would block".into())
            }
            MessageQueueSendError::Unknown(code) => {
                mpp_log!("mpp: shm_mq send unknown code {code}");
                DataFusionError::Execution(format!("mpp: shm_mq send unknown code {code}"))
            }
        })
    }

    fn try_send_bytes(&self, bytes: &[u8]) -> Result<bool, DataFusionError> {
        debug_assert_eq!(
            std::thread::current().id(),
            self.attach_thread,
            "ShmMqSender::try_send_bytes called off the attach thread"
        );
        match self.inner.try_send(bytes) {
            Ok(Some(())) => Ok(true),
            Ok(None) => Ok(false),
            Err(MessageQueueSendError::Detached) => Err(DataFusionError::Execution(
                "mpp: shm_mq sender detached".into(),
            )),
            Err(MessageQueueSendError::WouldBlock) => Ok(false),
            Err(MessageQueueSendError::Unknown(code)) => {
                mpp_log!("mpp: shm_mq try_send unknown code {code}");
                Err(DataFusionError::Execution(format!(
                    "mpp: shm_mq try_send unknown code {code}"
                )))
            }
        }
    }

    fn send_lock(&self) -> &tokio::sync::Mutex<()> {
        &self.send_lock
    }
}

/// shm_mq-backed `BatchChannelReceiver`. The leader creates the shm_mq via
/// `shm_mq_create` during DSM init; this attaches as receiver to an already-
/// initialized queue.
pub(super) struct ShmMqReceiver {
    inner: MessageQueueReceiver,
}

// SAFETY: `NonNull<shm_mq_handle>` is a pointer into DSM valid across threads
// within the same backend process; the drain thread is the only reader after
// attach, and only via `shm_mq_receive(nowait=true)` (no thread-unsafe latch
// or CFI calls).
unsafe impl Send for ShmMqReceiver {}

// SAFETY: production invariant is that only the backend thread calls `try_recv` (the cooperative
// drain runs inline on `DrainGatherStream::poll_next`; pgrx's `check_active_thread` would panic
// on a non-backend caller anyway). `shm_mq_receive(nowait=true)` is therefore not reentered
// concurrently on the same handle. We need `Sync` to satisfy the `BatchChannelReceiver: Send +
// Sync` bound, which in turn lets `DrainHandle: Sync` come from the concrete types instead of
// from the `Mutex<Vec<…>>` wrapper on `coop_receivers` having to provide it.
unsafe impl Sync for ShmMqReceiver {}

impl ShmMqReceiver {
    /// Attach as receiver to an *already-created* shm_mq.
    ///
    /// # Safety
    /// - `mq` must point to a shm_mq previously initialized by `shm_mq_create`.
    /// - `seg` may be NULL on workers.
    /// - No other proc has already set itself as receiver for `mq`.
    pub(super) unsafe fn attach_existing(
        seg: *mut pg_sys::dsm_segment,
        mq: *mut pg_sys::shm_mq,
    ) -> Self {
        unsafe {
            pg_sys::shm_mq_set_receiver(mq, pg_sys::MyProc);
            let handle = pg_sys::shm_mq_attach(mq, seg, std::ptr::null_mut());
            Self {
                inner: MessageQueueReceiver::from_raw_handle(handle),
            }
        }
    }
}

impl BatchChannelReceiver for ShmMqReceiver {
    fn try_recv(&self) -> RecvOutcome {
        match self.inner.try_recv() {
            Ok(Some(bytes)) => RecvOutcome::Bytes(bytes),
            Ok(None) => RecvOutcome::Empty,
            Err(MessageQueueRecvError::Detached) => RecvOutcome::Detached,
            Err(MessageQueueRecvError::WouldBlock) => RecvOutcome::Empty,
            Err(MessageQueueRecvError::Unknown(code)) => {
                mpp_log!("mpp: shm_mq recv unknown code {code}, treating as detached");
                RecvOutcome::Detached
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligned_queue_bytes_rounds_down_to_maxalign() {
        let maxalign = pg_sys::MAXIMUM_ALIGNOF as usize;
        for req in [0, 1, 7, 8, 15, 16, 1024, 1025, 1_048_576, 8 * 1024 * 1024] {
            let got = aligned_queue_bytes(req);
            assert!(got <= req);
            assert_eq!(got % maxalign, 0, "not aligned: {got} for req {req}");
            assert!(req - got < maxalign);
        }
    }

    #[test]
    fn align_up_maxalign_rounds_up_to_maxalign() {
        let maxalign = pg_sys::MAXIMUM_ALIGNOF as usize;
        for req in [0, 1, 7, 8, 15, 16, 1024, 1025] {
            let got = align_up_maxalign_checked(req).unwrap();
            assert!(got >= req);
            assert_eq!(got % maxalign, 0);
            assert!(got - req < maxalign);
        }
        assert!(align_up_maxalign_checked(usize::MAX).is_none());
    }
}

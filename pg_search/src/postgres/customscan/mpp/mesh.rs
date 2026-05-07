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

use std::sync::Arc;

use parking_lot::Mutex;
use pgrx::pg_sys;

use crate::mpp_log;
use crate::parallel_worker::mqueue::{
    MessageQueueReceiver, MessageQueueRecvError, MessageQueueSendError, MessageQueueSender,
};
use crate::postgres::customscan::mpp::dsm::WorkerPeerAttach;
use crate::postgres::customscan::mpp::transport::{
    BatchChannelReceiver, BatchChannelSender, DemuxDrainHandle, MppReceiver, MppSender, RecvOutcome,
};
use datafusion::common::DataFusionError;

/// MAXALIGN-DOWN of `queue_bytes`. Postgres requires shm_mq slot sizes to be
/// MAXALIGN-aligned; this rounds the user request down so every slot in a
/// queue array is the same size and slot indexing is a simple multiply.
#[inline]
pub fn aligned_queue_bytes(queue_bytes: usize) -> usize {
    const MAXIMUM_ALIGNOF: usize = pg_sys::MAXIMUM_ALIGNOF as usize;
    queue_bytes & !(MAXIMUM_ALIGNOF - 1)
}

/// MAXALIGN-UP `n` to the next multiple of `MAXIMUM_ALIGNOF`. Returns `None`
/// on overflow. Used by [`super::dsm::compute_dsm_layout`] to keep section
/// boundaries inside the DSM region MAXALIGN-aligned.
#[inline]
pub fn align_up_maxalign_checked(n: usize) -> Option<usize> {
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
pub struct ShmMqSender {
    inner: MessageQueueSender,
    attach_thread: std::thread::ThreadId,
}

// SAFETY: see struct doc.
unsafe impl Send for ShmMqSender {}

impl ShmMqSender {
    /// # Safety
    /// - `seg` must be a valid `dsm_segment*` (or NULL on workers).
    /// - `mq` must point to a shm_mq region that has been `shm_mq_create`'d
    ///   at its address with the expected size and has had
    ///   `shm_mq_set_receiver` called by the peer.
    pub unsafe fn attach(seg: *mut pg_sys::dsm_segment, mq: *mut pg_sys::shm_mq) -> Self {
        unsafe {
            Self {
                inner: MessageQueueSender::new(seg, mq),
                attach_thread: std::thread::current().id(),
            }
        }
    }
}

impl ShmMqSender {
    /// True iff the caller is on the thread that originally attached this
    /// sender. Reserved for the future multi-thread Tokio path (G7-MT)
    /// where compute threads dispatch FFI ops back to the attach thread
    /// via channel.
    #[allow(dead_code)]
    pub fn on_attach_thread(&self) -> bool {
        std::thread::current().id() == self.attach_thread
    }
}

impl BatchChannelSender for ShmMqSender {
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), DataFusionError> {
        // Non-blocking shm_mq send under the hood is thread-safe within
        // the same process (the underlying `shm_mq_send(nowait=true)` only
        // touches DSM-resident atomics + `SetLatch`, which is documented
        // as safe from any thread / signal handlers). The blocking variant
        // would call `WaitLatch` which is backend-only — but we use the
        // cooperative-drain spin instead of the blocking path.
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
}

/// shm_mq-backed `BatchChannelReceiver`. The leader creates the shm_mq via
/// `shm_mq_create` during DSM init; this attaches as receiver to an already-
/// initialized queue.
pub struct ShmMqReceiver {
    inner: MessageQueueReceiver,
}

// SAFETY: `NonNull<shm_mq_handle>` is a pointer into DSM valid across threads
// within the same backend process; the drain thread is the only reader after
// attach, and only via `shm_mq_receive(nowait=true)` (no thread-unsafe latch
// or CFI calls).
unsafe impl Send for ShmMqReceiver {}

impl ShmMqReceiver {
    /// Attach as receiver to an *already-created* shm_mq.
    ///
    /// # Safety
    /// - `mq` must point to a shm_mq previously initialized by `shm_mq_create`.
    /// - `seg` may be NULL on workers.
    /// - No other participant has already set itself as receiver for `mq`.
    pub unsafe fn attach_existing(seg: *mut pg_sys::dsm_segment, mq: *mut pg_sys::shm_mq) -> Self {
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

/// Per-worker handle to the peer mesh (Track B).
///
/// Each worker is both a producer (its row of the N×N grid) and a consumer
/// (its column). The leader is not part of the peer mesh.
///
/// Inbound demux drains: one [`DemuxDrainHandle`] per producer worker.
/// Each drain reads tagged framed messages from the `(producer, this_worker)`
/// shm_mq queue and routes by `tag = partition_id` into per-partition
/// sub-buffers. `n_partitions` is the number of distinct partitions a
/// producer can address — for the post-aggregate peer-mesh shuffle this
/// is the *global* partition count (`n_workers²` after the fork's
/// `RepartitionExec` scaling) so every consumer task K can demux its
/// partitions `N*K..(K+1)*N - 1`.
///
/// The `outbound` senders are held inside a `Mutex<Option<...>>` so the
/// inner producer fragment can take ownership exactly once via
/// [`Self::take_outbound`] before it begins pushing.
pub struct MppPeerMesh {
    pub n_workers: u32,
    pub self_idx: u32,
    pub n_partitions: u32,
    outbound: Mutex<Option<Vec<MppSender>>>,
    /// Inbound demux drain handles, one per producer worker. Cloned out at
    /// transport `open()` time so a peer-mesh shuffle can stream from
    /// each peer independently and demux by partition tag.
    inbound_drains: Vec<Arc<DemuxDrainHandle>>,
}

impl std::fmt::Debug for MppPeerMesh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MppPeerMesh")
            .field("n_workers", &self.n_workers)
            .field("self_idx", &self.self_idx)
            .field("n_partitions", &self.n_partitions)
            .field("outbound_taken", &self.outbound.lock().is_none())
            .field("inbound_drains", &self.inbound_drains.len())
            .finish()
    }
}

impl MppPeerMesh {
    /// Build from `worker_peer_attach`'s output. Wraps each shm_mq receiver
    /// in a cooperative [`DemuxDrainHandle`] sized to `n_partitions` tag
    /// buckets — the producer side will frame each batch with `tag =
    /// partition_id` and the consumer demuxes accordingly.
    pub fn from_worker_attach(
        self_idx: u32,
        n_workers: u32,
        n_partitions: u32,
        attach: WorkerPeerAttach,
    ) -> Self {
        let outbound: Vec<MppSender> = attach
            .peer_outbound
            .into_iter()
            .map(|s| MppSender::new(Box::new(s)))
            .collect();
        let inbound_drains: Vec<Arc<DemuxDrainHandle>> = attach
            .peer_inbound
            .into_iter()
            .map(|r| {
                let mpp_recv = MppReceiver::new(Box::new(r));
                Arc::new(DemuxDrainHandle::cooperative(vec![mpp_recv], n_partitions))
            })
            .collect();
        debug_assert_eq!(outbound.len(), n_workers as usize);
        debug_assert_eq!(inbound_drains.len(), n_workers as usize);
        Self {
            n_workers,
            self_idx,
            n_partitions,
            outbound: Mutex::new(Some(outbound)),
            inbound_drains,
        }
    }

    /// Take ownership of the outbound senders. Returns `None` if already
    /// taken. The inner producer fragment calls this exactly once before it
    /// begins pushing.
    pub fn take_outbound(&self) -> Option<Vec<MppSender>> {
        self.outbound.lock().take()
    }

    /// Demux drain handle for the queue from `producer` to this worker.
    /// Indexed by producer worker idx; each handle exposes per-tag
    /// sub-buffers via [`DemuxDrainHandle::buffer`].
    pub fn drain_for_producer(&self, producer: u32) -> Option<&Arc<DemuxDrainHandle>> {
        if producer >= self.n_workers {
            return None;
        }
        self.inbound_drains.get(producer as usize)
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

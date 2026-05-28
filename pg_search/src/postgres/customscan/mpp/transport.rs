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

//! Transport layer for MPP shuffle.
//!
//! Layout:
//! - [`MppFrameHeader`] is a fixed 16-byte prefix every wire message carries. It tags the
//!   payload with `(stage_id, partition)`, so one underlying queue can carry frames for many
//!   logical channels at once. That's what the multi-stage natural-shape path needs.
//! - [`encode_frame_into`] / [`decode_frame`] serialize a `RecordBatch` with a header prefix via
//!   Arrow IPC. They're the only codec entry points; tests round-trip through the same path so
//!   the wire format under test always matches production.
//! - [`DrainBuffer`] is the local per-proc queue that the drain thread
//!   writes into and the DataFusion consumer reads from. It decouples
//!   consumer-side backpressure from producer-side backpressure: the drain thread
//!   always makes forward progress on the inbound shm_mqs, so a stalled consumer
//!   cannot propagate backpressure to remote producers and cause an N×N
//!   peer-stall cycle.
//!
//! The shm_mq-backed sender/receiver and drain thread spawn logic build on
//! top of these primitives.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::ipc::reader::StreamReader;
use datafusion::arrow::ipc::writer::StreamWriter;
use datafusion::common::DataFusionError;

use super::fail_loud;

/// Magic bytes "MPPF" (MPP Frame) at the start of every wire message.
/// Lets receivers reject misrouted / corrupt frames before they hit Arrow IPC.
const MPP_FRAME_MAGIC: u32 = 0x4D505046;

/// Wire-format size of [`MppFrameHeader`] in bytes. Asserted at compile time
/// below via `const _: ()`.
const MPP_FRAME_HEADER_SIZE: usize = 16;

/// Kind of payload following [`MppFrameHeader`].
///
/// `Batch` is the common case. The header is followed by an Arrow IPC stream containing one
/// `RecordBatch`. `Eof` carries no payload. It signals the receiver that the named
/// `(stage_id, partition)` channel is done, even though the underlying shm_mq queue may still
/// carry frames for other channels.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MppFrameKind {
    Batch = 0,
    Eof = 1,
}

/// 16-byte prefix on every transport frame.
///
/// The fixed layout `[magic, flags, stage_id, partition]` (4×u32) is what
/// senders prepend before the Arrow IPC stream bytes and what receivers
/// parse before deciding which channel buffer the payload belongs to.
///
/// See the `flags` bit-layout block below for the encoding of the `flags` word.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MppFrameHeader {
    pub magic: u32,
    pub flags: u32,
    pub stage_id: u32,
    pub partition: u32,
}

/// `flags` bit layout:
///   bits  0..8:  frame kind (Batch | Eof)
///   bits  8..16: reserved (must be 0)
///   bits 16..32: sender_proc (mesh peer that wrote the frame)
const FRAME_KIND_MASK: u32 = 0x0000_00FF;
const FRAME_RESERVED_MASK: u32 = 0x0000_FF00;
const FRAME_SENDER_SHIFT: u32 = 16;
/// Maximum `sender_proc` representable in the header. Asserted at construction time so an
/// overflow becomes a hard error in the producer rather than silent flag corruption on the wire.
pub const MPP_MAX_SENDER_PROC: u32 = 0xFFFF;

const _: () = {
    // shm_mq slot layout calculations depend on this being exact.
    assert!(std::mem::size_of::<MppFrameHeader>() == MPP_FRAME_HEADER_SIZE);
};

#[inline]
fn pack_flags(kind: MppFrameKind, sender_proc: u32) -> u32 {
    // fail_loud rather than debug_assert: in release builds the check would be compiled out and
    // an out-of-range value would silently truncate to `sender_proc & 0xFFFF`. Catching the case
    // where a refactor accidentally passes a task_id or partition here is the whole point.
    if sender_proc > MPP_MAX_SENDER_PROC {
        fail_loud(format!(
            "mpp: sender_proc {sender_proc} > MPP_MAX_SENDER_PROC ({MPP_MAX_SENDER_PROC})"
        ));
    }
    (kind as u32) | (sender_proc << FRAME_SENDER_SHIFT)
}

impl MppFrameHeader {
    /// Build a `Batch` header for the given `(stage_id, partition)` stamped with `sender_proc`.
    pub fn batch(stage_id: u32, partition: u32, sender_proc: u32) -> Self {
        Self {
            magic: MPP_FRAME_MAGIC,
            flags: pack_flags(MppFrameKind::Batch, sender_proc),
            stage_id,
            partition,
        }
    }

    /// Build an `Eof` header for the given `(stage_id, partition)` stamped with `sender_proc`.
    /// Carries no payload; receivers route it to the channel buffer's source-done counter.
    pub fn eof(stage_id: u32, partition: u32, sender_proc: u32) -> Self {
        Self {
            magic: MPP_FRAME_MAGIC,
            flags: pack_flags(MppFrameKind::Eof, sender_proc),
            stage_id,
            partition,
        }
    }

    /// The mesh peer that wrote this frame. Phase 4 of mesh multiplexing uses this to key the
    /// drain-side per-channel buffer registry by `(sender_proc, stage_id, partition)`.
    pub fn sender_proc(&self) -> u32 {
        (self.flags >> FRAME_SENDER_SHIFT) & 0xFFFF
    }

    /// Read the kind out of `flags`. Returns an error if the kind byte is
    /// unknown or if any reserved bit (bits 8..16) is set, which catches wire-format
    /// drift early. Sender_proc bits (16..32) are not validated here; readers extract
    /// them with `sender_proc()`.
    pub(super) fn kind(&self) -> Result<MppFrameKind, DataFusionError> {
        let reserved = self.flags & FRAME_RESERVED_MASK;
        if reserved != 0 {
            return Err(DataFusionError::Internal(format!(
                "mpp: reserved frame flag bits set ({reserved:#x})"
            )));
        }
        match self.flags & FRAME_KIND_MASK {
            0 => Ok(MppFrameKind::Batch),
            1 => Ok(MppFrameKind::Eof),
            other => Err(DataFusionError::Internal(format!(
                "mpp: unknown frame kind {other:#x}"
            ))),
        }
    }

    /// Serialize into the first `MPP_FRAME_HEADER_SIZE` bytes of `out`.
    /// `out.len()` must be `>= MPP_FRAME_HEADER_SIZE`.
    fn write_to(&self, out: &mut [u8]) {
        debug_assert!(out.len() >= MPP_FRAME_HEADER_SIZE);
        out[0..4].copy_from_slice(&self.magic.to_le_bytes());
        out[4..8].copy_from_slice(&self.flags.to_le_bytes());
        out[8..12].copy_from_slice(&self.stage_id.to_le_bytes());
        out[12..16].copy_from_slice(&self.partition.to_le_bytes());
    }

    /// Parse from the first `MPP_FRAME_HEADER_SIZE` bytes of `bytes`. Returns
    /// `Err` if the slice is too short or the magic doesn't match.
    fn parse(bytes: &[u8]) -> Result<Self, DataFusionError> {
        if bytes.len() < MPP_FRAME_HEADER_SIZE {
            // No encoder in this file emits sub-header output, so a short frame means the
            // shm_mq stitched together payloads from different senders. Hex-dump the bytes
            // so the source is identifiable from log output without a debugger.
            let hex = bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            return Err(DataFusionError::Internal(format!(
                "mpp: frame too short for header ({} < {}); bytes = [{hex}]",
                bytes.len(),
                MPP_FRAME_HEADER_SIZE
            )));
        }
        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        if magic != MPP_FRAME_MAGIC {
            return Err(DataFusionError::Internal(format!(
                "mpp: bad frame magic {magic:#x} (expected {MPP_FRAME_MAGIC:#x})"
            )));
        }
        Ok(Self {
            magic,
            flags: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            stage_id: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            partition: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
        })
    }
}

/// Serialize `batch` into `buf` with a 16-byte [`MppFrameHeader`] prefix
/// addressing it to `(stage_id, partition)`. Wire format:
///
/// ```text
/// [ magic | flags | stage_id | partition ] [ Arrow IPC stream bytes ]
/// |---------- 16 bytes --------|           |---- variable ----|
/// ```
///
/// `flags` encodes kind + sender_proc; see the bit-layout block near
/// `FRAME_KIND_MASK` for details.
///
/// Caller is expected to hold `buf` alive across many encodes so the peak-sized
/// allocation amortizes (~500 KB/batch on the 25M GROUP BY bench).
fn encode_frame_into(
    header: MppFrameHeader,
    batch: &RecordBatch,
    buf: &mut Vec<u8>,
) -> Result<(), DataFusionError> {
    buf.clear();
    buf.resize(MPP_FRAME_HEADER_SIZE, 0);
    header.write_to(&mut buf[..MPP_FRAME_HEADER_SIZE]);
    let mut writer = StreamWriter::try_new(&mut *buf, batch.schema_ref())?;
    writer.write(batch)?;
    writer.finish()?;
    Ok(())
}

/// Serialize a payload-less [`MppFrameKind::Eof`] frame for `(stage_id, partition)`
/// into `buf`. The shm_mq peer reads this as a 16-byte message and routes it to
/// the channel buffer's source-done counter without touching Arrow IPC.
/// Consumed by [`MppSender::send_eof_traced`] when a producer fragment's
/// per-partition stream exhausts, so the receiver's `(stage_id, partition)`
/// channel buffer transitions to `Eof` even though the multiplexed shm_mq queue
/// stays attached for other channels.
fn encode_eof_frame_into(
    stage_id: u32,
    partition: u32,
    sender_proc: u32,
    buf: &mut Vec<u8>,
) -> Result<(), DataFusionError> {
    buf.clear();
    buf.resize(MPP_FRAME_HEADER_SIZE, 0);
    MppFrameHeader::eof(stage_id, partition, sender_proc)
        .write_to(&mut buf[..MPP_FRAME_HEADER_SIZE]);
    Ok(())
}

/// Inverse of [`encode_frame_into`]. Parses the 16-byte header and, for `Batch` frames, decodes
/// the trailing Arrow IPC stream. `Eof` frames return `(header, None)`. Receivers branch on
/// `header.kind()` to decide routing.
fn decode_frame(bytes: &[u8]) -> Result<(MppFrameHeader, Option<RecordBatch>), DataFusionError> {
    let header = MppFrameHeader::parse(bytes)?;
    match header.kind()? {
        MppFrameKind::Eof => {
            if bytes.len() != MPP_FRAME_HEADER_SIZE {
                return Err(DataFusionError::Internal(format!(
                    "mpp: Eof frame carries payload ({} > {})",
                    bytes.len(),
                    MPP_FRAME_HEADER_SIZE
                )));
            }
            Ok((header, None))
        }
        MppFrameKind::Batch => {
            let payload = &bytes[MPP_FRAME_HEADER_SIZE..];
            let mut reader = StreamReader::try_new(payload, None)?;
            let batch = reader.next().ok_or_else(|| {
                DataFusionError::Execution("mpp: empty arrow-ipc stream in decode_frame".into())
            })??;
            Ok((header, Some(batch)))
        }
    }
}

/// Local queue between a drain (either the cooperative `try_drain_pass` or the test-only thread
/// variant) and the consumer that pops batches.
///
/// In the cooperative path each `DrainBuffer` corresponds to one logical channel: one
/// `(stage_id, partition)` entry in the owning [`DrainHandle`]'s registry. `num_sources` is
/// always `1` there because a given drain serves a single sender_proc, which is the only producer
/// for any channel routed through it. The test-only thread path uses a single shared buffer with
/// `num_sources = N` over an N-sender setup.
///
/// Push side: callers append deserialized batches; on source detach (or per-channel `Eof` frame)
/// [`DrainBuffer::notify_source_done`] is called. Once `sources_done >= num_sources` AND the
/// queue is empty, `try_pop` returns [`DrainItem::Eof`].
///
/// Pop side: cooperative consumers loop on `try_pop` + `yield_now`. The test-only `pop_front`
/// blocks on the condvar.
#[derive(Debug)]
pub(super) struct DrainBuffer {
    inner: Mutex<DrainBufferInner>,
    cond: Condvar,
}

#[derive(Debug)]
struct DrainBufferInner {
    queue: VecDeque<RecordBatch>,
    num_sources: u32,
    sources_done: u32,
    /// Consumer-side cancel flag. When set (e.g., query cancelled or `DrainHandle` dropped),
    /// `try_pop`/`pop_front` returns `Eof` even if `sources_done` hasn't reached `num_sources`.
    cancelled: bool,
}

/// Yielded by [`DrainBuffer::pop_front`].
#[derive(Debug)]
pub(super) enum DrainItem {
    /// A batch produced by one of the inbound shm_mqs.
    Batch(RecordBatch),
    /// All source queues have detached and the local queue is drained.
    Eof,
}

impl DrainBuffer {
    /// Create a drain buffer expecting `num_sources` inbound queues. For a
    /// proc in an N-proc mesh, `num_sources == N - 1` (all peers
    /// excluding self — the self-partition bypasses the buffer).
    pub fn new(num_sources: u32) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(DrainBufferInner {
                queue: VecDeque::new(),
                num_sources,
                sources_done: 0,
                cancelled: false,
            }),
            cond: Condvar::new(),
        })
    }

    /// Push a freshly-received batch into the local queue.
    pub fn push_batch(&self, batch: RecordBatch) {
        let mut guard = self.inner.lock().expect("DrainBuffer mutex poisoned");
        guard.queue.push_back(batch);
        self.cond.notify_one();
    }

    /// Mark one source queue as detached. Safe to call from the drain thread
    /// after observing `SHM_MQ_DETACHED` on a given inbound queue.
    pub fn notify_source_done(&self) {
        let mut guard = self.inner.lock().expect("DrainBuffer mutex poisoned");
        guard.sources_done = guard.sources_done.saturating_add(1);
        if guard.sources_done >= guard.num_sources {
            self.cond.notify_all();
        }
    }

    /// Cancel all further pushes and wake all consumers with EOF.
    pub fn cancel(&self) {
        let mut guard = self.inner.lock().expect("DrainBuffer mutex poisoned");
        guard.cancelled = true;
        self.cond.notify_all();
    }

    /// Non-blocking variant. Returns the front item, or `DrainItem::Eof` if
    /// all sources have detached and the queue is drained, or `None` if more
    /// data may yet arrive. Cooperative consumers loop on
    /// `try_drain_pass` + `try_pop`, yielding to the executor between
    /// iterations.
    pub fn try_pop(&self) -> Option<DrainItem> {
        let mut guard = self.inner.lock().expect("DrainBuffer mutex poisoned");
        Self::try_pop_locked(&mut guard)
    }

    /// Shared body of [`try_pop`] and the test-only [`Self::pop_front`].
    /// Returns `Some(Batch)` if the queue has data, `Some(Eof)` if all
    /// sources have detached or the buffer is cancelled, and `None`
    /// otherwise. Lets the two entry points stay in lockstep on the
    /// "buffered data wins over cancellation/EOF" invariant locked in by
    /// the `drain_buffer_drains_buffered_before_eof` test.
    fn try_pop_locked(guard: &mut MutexGuard<'_, DrainBufferInner>) -> Option<DrainItem> {
        if let Some(batch) = guard.queue.pop_front() {
            return Some(DrainItem::Batch(batch));
        }
        if guard.cancelled || guard.sources_done >= guard.num_sources {
            return Some(DrainItem::Eof);
        }
        None
    }
}

/// Outcome of a single non-blocking receive attempt.
#[derive(Debug)]
pub(super) enum RecvOutcome {
    /// One serialized Arrow IPC message ready to decode.
    Bytes(Vec<u8>),
    /// No data currently available but the peer is still attached.
    Empty,
    /// The peer has detached; no more bytes will ever arrive on this channel.
    Detached,
}

/// Non-blocking byte channel receiver. Implementations: shm_mq (production),
/// `std::sync::mpsc` (tests). Must be `Send` because the drain thread takes
/// ownership.
pub(super) trait BatchChannelReceiver: Send + Sync {
    fn try_recv(&self) -> RecvOutcome;

    /// Tell producers to stop sending. Default no-op; concrete impls override when the
    /// underlying transport doesn't get a "drop = detach" signal for free.
    ///
    /// `ShmMqReceiver` doesn't override: shm_mq's `MessageQueueReceiver::Drop` calls
    /// `shm_mq_detach`, which is enough to make producers see Detached.
    ///
    /// `DsmInboxReceiver` overrides: the DSM MPSC ring's detach is mostly handled by
    /// `DsmMpscSender::Drop` flipping `detached` when the last sender goes away, but
    /// the consumer side can force-detach early (e.g., query teardown before producers
    /// finish) via this method. Phase 4 wires the call from `DrainHandle` teardown.
    #[allow(dead_code)]
    fn set_detached(&self) {}
}

/// Byte channel sender paired with [`BatchChannelReceiver`]. `send` blocks when
/// the channel is full. Dropping the sender signals EOF to the receiver.
///
/// `Send` is required because unit tests and future producer-pump threads move
/// senders across thread boundaries. Production shm_mq senders, however, must
/// only be *used* from the main backend thread — the blocking shm_mq send path
/// (`nowait=false`) touches `WaitLatch`/`CHECK_FOR_INTERRUPTS`, which is not
/// safe off-thread. See [`crate::postgres::customscan::mpp::mesh::ShmMqSender`]
/// for the safety contract.
pub(crate) trait BatchChannelSender: Send + Sync {
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), DataFusionError>;

    /// Non-blocking variant. Returns `Ok(true)` on success, `Ok(false)`
    /// when the channel is full (caller should retry), `Err` on detach /
    /// transport error. Default falls back to the blocking send — safe
    /// for in-proc channels used by tests where "full" doesn't arise.
    fn try_send_bytes(&self, bytes: &[u8]) -> Result<bool, DataFusionError> {
        self.send_bytes(bytes).map(|()| true)
    }

    /// Async lock the send paths hold across the cooperative-drain spin so two tasks can't
    /// interleave partial writes on the same handle. PG's `shm_mq_send` requires the same
    /// `(nbytes, data)` on retry after `SHM_MQ_WOULD_BLOCK`. Multiple [`MppSender`] clones
    /// multiplex onto one channel, and the spin's `yield_now().await` would otherwise let a
    /// sibling task land a different payload mid-message and corrupt the queue. In-proc
    /// channels return a per-instance mutex too, just to keep the call sites uniform.
    fn send_lock(&self) -> &tokio::sync::Mutex<()>;
}

/// Pluggable "drain everything inbound" hook for [`MppSender`]'s cooperative send spin. The
/// peer-mesh deadlock-breaking pattern needs the producer to pump ALL inbound queues (not just
/// one) while waiting for a full outbound, so the implementation typically delegates to
/// `MppMesh::drain_all_inbound()` which iterates every per-sender-proc drain.
pub trait CooperativeDrainSet: Send + Sync {
    fn try_drain_pass(&self) -> Result<(), DataFusionError>;
}

impl CooperativeDrainSet for DrainHandle {
    fn try_drain_pass(&self) -> Result<(), DataFusionError> {
        DrainHandle::try_drain_pass(self)
    }
}

/// High-level sender: encodes a `RecordBatch` then pushes bytes through the underlying channel.
///
/// With `cooperative_drain` set, `send_batch` breaks the symmetric-send deadlock on a
/// single-threaded tokio runtime by interleaving send-retries with
/// `CooperativeDrainSet::try_drain_pass` on the same mesh's inbound side. Each proc's
/// sender doing the same guarantees mutual progress: our drain pulls peer-shipped rows out of
/// our inbound queues, which frees peers' outbound-to-us send space, which lets their sends
/// un-stall.
pub struct MppSender {
    /// Underlying byte channel. Held behind `Arc` so multiple `MppSender`s can share one
    /// `shm_mq` queue while tagging frames with different `(stage_id, partition)` headers, which
    /// is the multiplexed path's natural pattern. Clone the Arc, build a new `MppSender` with a
    /// different header, both write into the same queue.
    pub(super) channel: Arc<dyn BatchChannelSender>,
    cooperative_drain: Option<Arc<dyn CooperativeDrainSet>>,
    /// Frame header prepended to every outgoing batch. Identifies the logical
    /// `(stage_id, partition)` channel the receiver demultiplexes on. Per-sender rather than
    /// per-call: each partition gets its own `MppSender` via `clone_with_header`, all sharing
    /// the underlying `Arc<dyn BatchChannelSender>` of a single shm_mq queue.
    pub(super) header: MppFrameHeader,
    /// Scratch buffer reused across every `encode_frame_into` on this sender. Sized by the
    /// first batch; subsequent batches clear and re-fill without reallocating. Interior
    /// mutability lets the caller keep the `&self` signature (each producer fragment holds
    /// its `MppSender` clones behind shared borrows for the duration of
    /// `worker::run_worker_fragment`).
    scratch: std::cell::RefCell<Vec<u8>>,
}

// SAFETY: only `scratch: RefCell<Vec<u8>>` and the trait-object `Arc`s are `!Sync`. Callers
// compose `send_*_traced` futures via `tokio::spawn` / `join_all`, which makes the compiler
// require `&Self: Send` and therefore `Self: Sync`. At runtime those futures run on the
// current-thread tokio runtime pinned to the PG backend thread, so the cell is never actually
// observed from another thread. Same single-thread-by-construction contract as
// `unsafe impl Send for ShmMqSender` in `mesh.rs`.
unsafe impl Sync for MppSender {}

impl MppSender {
    /// Construct a sender that tags every outgoing batch with `header`. Production call sites
    /// clone one shared `Arc<dyn BatchChannelSender>` across N senders, each with a different
    /// `MppFrameHeader::batch(stage, p)`. That's the multiplexed pattern for fanning multiple
    /// partitions over one shm_mq queue.
    pub(super) fn with_header(
        channel: Arc<dyn BatchChannelSender>,
        header: MppFrameHeader,
    ) -> Self {
        Self {
            channel,
            cooperative_drain: None,
            header,
            scratch: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Build a new `MppSender` that shares this sender's underlying channel
    /// but tags every frame with `header` instead. Used by callers that know
    /// the physical plan's output partition count and need one sender per
    /// partition, all multiplexed over the same shm_mq queue.
    pub fn clone_with_header(&self, header: MppFrameHeader) -> Self {
        Self {
            channel: Arc::clone(&self.channel),
            cooperative_drain: self.cooperative_drain.as_ref().map(Arc::clone),
            header,
            scratch: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Attach a [`CooperativeDrainSet`] so [`Self::send_batch_traced`]'s spin
    /// can drain inbound peer traffic while waiting for outbound space.
    /// Required for peer-mesh fragments where every worker is both sender and
    /// consumer; without it, symmetric full-queue stalls deadlock the
    /// single-threaded Tokio runtime.
    pub fn with_cooperative_drain(mut self, drain: Arc<dyn CooperativeDrainSet>) -> Self {
        self.cooperative_drain = Some(drain);
        self
    }

    /// `send_batch` variant that accumulates per-call timings and spin counts into `stats`.
    /// Callers that report at EOF (e.g. `ShuffleStream`) use this to diagnose where time
    /// goes when the outbound queue is full.
    ///
    /// Async because the spin awaits the per-handle send lock and yields between
    /// `try_send_bytes` retries; see `send_with_scratch`.
    pub(super) async fn send_batch_traced(
        &self,
        batch: &RecordBatch,
        stats: &mut SendBatchStats,
    ) -> Result<(), DataFusionError> {
        // Take the scratch buffer out of the `RefCell` rather than
        // holding a `RefMut` across the spin below. The spin contains
        // `pgrx::check_for_interrupts!()`, which can `longjmp` through
        // Rust frames; a `longjmp` does not run `Drop`, so a `RefMut`
        // held across it would leave the cell perpetually borrowed and
        // panic the next caller. `replace` is atomic — the cell is
        // never observed in a borrowed state — and we put the buffer
        // back at the end so its heap allocation survives across calls.
        // If the spin longjmps anyway, the cell holds the default empty
        // `Vec` and the next call simply re-allocates.
        let mut scratch = self.scratch.replace(Vec::new());
        let result = self.send_with_scratch(batch, &mut scratch, stats).await;
        self.scratch.replace(scratch);
        result
    }

    /// Send a payload-less [`MppFrameKind::Eof`] frame so the receiver's `(stage_id, partition)`
    /// channel buffer transitions to `Eof` and the consumer's pull loop terminates cleanly.
    ///
    /// Producer fragments must call this exactly once per `(stage_id, partition)` channel after
    /// the local stream exhausts. Without it the multiplexed shm_mq queue stays attached (other
    /// channels still flow) and the consumer channel buffer never reaches `sources_done == 1`. The
    /// receive-side [`DrainHandle::try_drain_pass`] decodes the frame and calls
    /// `notify_source_done` on the matching channel buffer.
    ///
    /// Uses the same cooperative-spin path as [`Self::send_batch_traced`] so a full outbound
    /// queue doesn't deadlock the EOF send. `stats.spin_iters` / `send_wait` capture any
    /// contention.
    ///
    /// Symmetric-EOF safety: when every peer reaches EOF simultaneously with full outbound
    /// queues, each peer's cooperative [`CooperativeDrainSet::try_drain_pass`] inside the spin
    /// pulls peer-sent frames out of its own inbound queues, freeing space the peers are blocked
    /// on. Progress is monotone: at least one `try_send_bytes` succeeds per spin iteration
    /// somewhere in the mesh, so symmetric stalls resolve within a few iterations rather than
    /// deadlocking.
    pub(super) async fn send_eof_traced(
        &self,
        stats: &mut SendBatchStats,
    ) -> Result<(), DataFusionError> {
        let mut scratch = self.scratch.replace(Vec::new());
        let result = self.send_eof_with_scratch(&mut scratch, stats).await;
        self.scratch.replace(scratch);
        result
    }

    async fn send_eof_with_scratch(
        &self,
        scratch: &mut Vec<u8>,
        stats: &mut SendBatchStats,
    ) -> Result<(), DataFusionError> {
        encode_eof_frame_into(
            self.header.stage_id,
            self.header.partition,
            self.header.sender_proc(),
            scratch,
        )?;
        let Some(drain) = self.cooperative_drain.as_ref() else {
            return self.channel.send_bytes(scratch);
        };
        // Lock the channel before the spin so a sibling task can't interleave a different
        // partial write through the shared shm_mq handle. See `BatchChannelSender::send_lock`.
        let _send_guard = self.channel.send_lock().lock().await;
        let mut first_try = true;
        let t_wait_start = Instant::now();
        loop {
            self.spin_check_for_interrupts().await?;
            let send_ok = self.spin_try_send_bytes(scratch).await?;
            if send_ok {
                if !first_try {
                    stats.send_wait += t_wait_start.elapsed();
                }
                return Ok(());
            }
            first_try = false;
            stats.spin_iters += 1;
            let t_drain = Instant::now();
            self.spin_try_drain_pass(drain).await?;
            stats.coop_drain_in_spin += t_drain.elapsed();
            tokio::task::yield_now().await;
        }
    }

    /// Spin-loop helper: call `check_for_interrupts`. Gated out of test builds because the
    /// pgrx macro pulls in symbols the lib-test binary doesn't link.
    async fn spin_check_for_interrupts(&self) -> Result<(), DataFusionError> {
        #[cfg(not(test))]
        pgrx::check_for_interrupts!();
        Ok(())
    }

    /// Spin-loop helper: call `channel.try_send_bytes(scratch)`.
    async fn spin_try_send_bytes(&self, scratch: &[u8]) -> Result<bool, DataFusionError> {
        self.channel.try_send_bytes(scratch)
    }

    /// Spin-loop helper: call `drain.try_drain_pass()`.
    async fn spin_try_drain_pass(
        &self,
        drain: &Arc<dyn CooperativeDrainSet>,
    ) -> Result<(), DataFusionError> {
        drain.try_drain_pass()
    }

    async fn send_with_scratch(
        &self,
        batch: &RecordBatch,
        scratch: &mut Vec<u8>,
        stats: &mut SendBatchStats,
    ) -> Result<(), DataFusionError> {
        let t_enc = Instant::now();
        encode_frame_into(self.header, batch, scratch)?;
        stats.encode += t_enc.elapsed();
        let Some(drain) = self.cooperative_drain.as_ref() else {
            // No drain attached (unit tests, in-proc channels): fall
            // back to the blocking send path.
            return self.channel.send_bytes(scratch);
        };
        // Lock the channel before the spin so a sibling task can't interleave a different
        // partial write through the shared shm_mq handle. See `BatchChannelSender::send_lock`.
        // Long-term, switching shm_mq for an async-friendly ring buffer (cf. #4184) drops the
        // partial-send invariant entirely and removes the need for this lock.
        //
        // G7-MT phase 3d TODO: under the multi_thread runtime, holding `_send_guard` across
        // a relay round-trip + `yield_now().await` lets one fragment task starve a sibling
        // that shares the same `Arc<dyn BatchChannelSender>` (the multi-partition fan-out
        // pattern). The `tokio::sync::Mutex` is FIFO-fair, so the sibling can sit blocked
        // for an entire 25M-row shuffle. The real fix is moving the whole spin to the
        // service task so the lock contention happens on the backend thread at FFI speed,
        // not at compute-thread round-trip speed. Today's linear topology has at most one
        // sender per `Arc`, so this is latent until phase 3d ships.
        let _send_guard = self.channel.send_lock().lock().await;
        let mut first_try = true;
        let t_wait_start = Instant::now();
        // The spin runs inside a tokio task on the backend thread's current-thread runtime
        // (DataFusion needs one to drive `Stream`s). The deadlock we're breaking is
        // *cross-proc*: two peers each blocked on a full outbound. `try_drain_pass` pulls
        // peer batches off our inbound on the same OS thread, freeing their slots so their
        // sends advance. `yield_now().await` between iterations hands the runtime back to
        // siblings if any are ready, mostly a no-op under today's linear MPP topology.
        loop {
            self.spin_check_for_interrupts().await?;
            let send_ok = self.spin_try_send_bytes(scratch).await?;
            if send_ok {
                if !first_try {
                    stats.send_wait += t_wait_start.elapsed();
                }
                return Ok(());
            }
            first_try = false;
            stats.spin_iters += 1;
            // Would-block. Pull from our inbound so peers' outbound-to-us frees up and their
            // sends to us unblock; without this, symmetric full-queue sends deadlock. Errors
            // propagate so a peer detaching mid-spin doesn't leave us spinning on a closed
            // mesh.
            let t_drain = Instant::now();
            self.spin_try_drain_pass(drain).await?;
            stats.coop_drain_in_spin += t_drain.elapsed();
            tokio::task::yield_now().await;
        }
    }
}

/// Per-call timing + spin metrics for [`MppSender::send_batch_traced`].
/// All fields accumulate; callers zero or reuse as needed.
#[derive(Default, Debug, Clone)]
pub(super) struct SendBatchStats {
    /// Cumulative time spent inside `encode_frame_into` (header + Arrow IPC serialization).
    pub(super) encode: Duration,
    /// Cumulative wall time in the send-retry spin after the first failed
    /// `try_send_bytes`. Zero if the first try succeeded.
    pub(super) send_wait: Duration,
    /// Cumulative time spent in `try_drain_pass` while spinning on a
    /// full outbound. A subset of `send_wait`; the remainder is the
    /// `tokio::task::yield_now()` await + the (small) cost of
    /// `try_send_bytes` itself.
    pub(super) coop_drain_in_spin: Duration,
    /// Count of `try_send_bytes` calls that returned `Ok(false)` (full).
    pub(super) spin_iters: u64,
}

/// High-level receiver: pulls bytes via the underlying channel and decodes them
/// into `RecordBatch`. Used by the drain thread.
pub(super) struct MppReceiver {
    channel: Box<dyn BatchChannelReceiver>,
}

impl MppReceiver {
    pub fn new(channel: Box<dyn BatchChannelReceiver>) -> Self {
        Self { channel }
    }

    pub(super) fn try_recv_batch(&self) -> RecvBatchOutcome {
        match self.channel.try_recv() {
            RecvOutcome::Bytes(bytes) => match decode_frame(&bytes) {
                Ok((header, Some(batch))) => RecvBatchOutcome::Batch { header, batch },
                Ok((header, None)) => RecvBatchOutcome::Eof { header },
                Err(e) => RecvBatchOutcome::Error(e),
            },
            RecvOutcome::Empty => RecvBatchOutcome::Empty,
            RecvOutcome::Detached => RecvBatchOutcome::Detached,
        }
    }
}

/// Decoded result of an [`MppReceiver::try_recv_batch`]. Carries the
/// parsed [`MppFrameHeader`] so the drain thread can route the payload to
/// the right `(stage_id, partition)` channel buffer.
#[derive(Debug)]
pub(super) enum RecvBatchOutcome {
    Batch {
        header: MppFrameHeader,
        batch: RecordBatch,
    },
    /// A payload-less `Eof` frame for `header.(stage_id, partition)`. The
    /// underlying shm_mq queue is still attached. The sender is just
    /// signalling that this logical channel is done, so we can EOF
    /// per-channel without dropping the whole queue.
    Eof {
        header: MppFrameHeader,
    },
    Empty,
    Detached,
    Error(DataFusionError),
}

/// Per-`(sender_proc, stage_id, partition)` channel buffer registry owned by a cooperative
/// [`DrainHandle`]. The handle may host several cooperative receivers (DSM MPSC inbox + self-loop
/// in-proc), each demultiplexed by the [`MppFrameHeader`] prefix into the same `map`.
/// `try_drain_pass` looks up the right channel buffer on every frame and pushes the payload into
/// it. Consumers waiting on a given key only see frames matching that key.
///
/// Each entry is a `DrainBuffer::new(1)`: exactly one sender_proc emits frames for any given
/// channel. Per-channel EOF flows via the `Eof` frame demuxed onto the matching buffer; query-
/// teardown unblock flows via [`DrainHandle::cancel_channel_buffers`] from the handle's `Drop`.
#[derive(Default)]
struct ChannelBufferRegistry {
    /// Keyed by `(sender_proc, stage_id, partition)`. Under mesh-multiplexing the unified
    /// inbox carries frames from every peer, so each `(stage, partition)` consumer gets its
    /// own per-sender buffer — preserving the implicit "one stream per sender" semantics
    /// that `WorkerConnection::stream_partition` consumers rely on.
    map: HashMap<(u32, u32, u32), Arc<DrainBuffer>>,
}

/// Per-sender-proc drain: stashes the receivers and polls them inline from the cooperative spin
/// (no background thread), demuxing each frame into a per-`(stage_id, partition)` channel buffer.
///
/// Inline polling is the production requirement: pgrx's `check_active_thread` guard panics on any
/// pg FFI call (including `shm_mq_receive`) from a non-backend thread, so the drain work has to
/// run on the backend thread. Tests that need a true thread-backed drain use
/// [`ThreadedDrainHandle`] instead.
///
/// On drop, the handle cancels every channel buffer so any consumer blocked on `try_pop` unblocks
/// with `Eof` — the drain can therefore never outlive its query, even on a panicked teardown.
pub struct DrainHandle {
    /// Per-(stage_id, partition) channel buffer registry. Populated lazily on first frame for a
    /// channel, or up-front by callers (e.g. `WorkerConnection::stream_partition`) that need a
    /// buffer to wait on before any frame arrives.
    channel_buffers: Mutex<ChannelBufferRegistry>,
    /// Receivers owned by the handle and polled inline from `DrainGatherStream::poll_next` via
    /// [`Self::try_drain_pass`]. The `Mutex` is for interior mutability: `try_drain_pass(&self)`
    /// marks each slot as `None` after observing `Detached` so subsequent passes skip the dead
    /// receiver. `BatchChannelReceiver: Send + Sync` makes `Vec<Option<MppReceiver>>: Sync`
    /// already, so the lock is no longer doubling as the `Sync` provider — replacing it with a
    /// non-locking primitive would need either an atomic per-slot detached flag or accepting
    /// that detached receivers get polled once per pass (fast-returning `Detached`). The lock
    /// is uncontended in production (single backend thread) so the marginal cost is in the
    /// type system, not the runtime.
    coop_receivers: Mutex<Vec<Option<MppReceiver>>>,
}

impl DrainHandle {
    /// Construct a cooperative drain handle. Channel buffers are populated lazily by
    /// [`Self::try_drain_pass`] when a frame arrives, or up-front by [`Self::register_channel`]
    /// when a consumer needs a buffer to wait on before any frame has come in.
    pub(super) fn cooperative(receivers: Vec<MppReceiver>) -> Self {
        let wrapped = receivers.into_iter().map(Some).collect();
        Self {
            channel_buffers: Mutex::new(ChannelBufferRegistry::default()),
            coop_receivers: Mutex::new(wrapped),
        }
    }

    /// Register (or look up) the channel buffer for `(sender_proc, stage_id, partition)`.
    /// The returned `Arc<DrainBuffer>` is the canonical destination for frames matching
    /// that key: `try_drain_pass` pushes into the same entry on every `Batch { header, .. }`
    /// whose `header.sender_proc()` / `stage_id` / `partition` matches.
    pub(super) fn register_channel(
        &self,
        sender_proc: u32,
        stage_id: u32,
        partition: u32,
    ) -> Arc<DrainBuffer> {
        let mut guard = self
            .channel_buffers
            .lock()
            .expect("DrainHandle channel_buffers mutex poisoned");
        guard
            .map
            .entry((sender_proc, stage_id, partition))
            .or_insert_with(|| {
                // num_sources stays 1: under mesh-multiplexing each
                // (sender_proc, stage, partition) tuple has exactly one upstream — the
                // named sender — even though the underlying inbox is shared across all
                // senders.
                DrainBuffer::new(1)
            })
            .clone()
    }

    /// Cancel every registered channel buffer. Called from `Drop` to unblock any consumer waiting on
    /// a channel buffer when the handle goes away mid-query.
    ///
    /// Collects buffer handles under the registry lock, then notifies after releasing it —
    /// notifying inline would block any concurrent [`Self::register_channel`] for as long as it
    /// takes to acquire `DrainBuffer::inner` N times. Fine today (single backend thread), but
    /// cheap insurance against the multi-thread variant landing later.
    fn cancel_channel_buffers(&self) {
        let to_cancel = {
            let guard = self
                .channel_buffers
                .lock()
                .expect("DrainHandle channel_buffers mutex poisoned");
            guard.map.values().cloned().collect::<Vec<_>>()
        };
        for buf in to_cancel {
            buf.cancel();
        }
    }

    /// Pull batches from each live receiver and demux them into the per-`(stage_id, partition)`
    /// channel buffer registry. Called from `DrainGatherStream::poll_next` and from
    /// `MppSender::send_batch`'s cooperative spin. Drain work happens on the backend thread
    /// (pgrx-safe). No-op for thread-backed handles.
    ///
    /// Each pass drains *every available* batch from each receiver (up to a safety cap). Pulling
    /// only one batch per source per call would mean that under steady producer pressure the
    /// cooperative sender's spin-loop can't keep up: we'd fall N:1 behind peers' sends and the
    /// mesh would stall once any queue fills. Draining until the receiver reports `Empty` bounds
    /// each pass by queue depth rather than by spin-loop iteration count.
    ///
    /// Returns `Ok(())` once every cooperative receiver has been pulled until `Empty` (or
    /// detached). Errors propagate as `Err` so a transport-level failure surfaces at the call
    /// site rather than getting silently dropped.
    ///
    /// Routing rules per outcome:
    /// - `Batch { header, batch }`: look up (or lazily create) the
    ///   `(header.stage_id, header.partition)` channel buffer and push `batch`.
    /// - `Eof { header }`: per-channel EOF. Resolve the channel buffer and call
    ///   `notify_source_done`. Other channels on the same queue keep flowing,
    ///   so the receiver slot stays live.
    /// - `Detached` / `Error`: queue-wide shutdown. Notify every registered
    ///   channel buffer, mark the handle detached, and drop the slot.
    pub fn try_drain_pass(&self) -> Result<(), DataFusionError> {
        // Bound per-source pulls per call. The upper limit exists to give
        // the caller a chance to re-try its own send between drains —
        // otherwise a proc with a very fast peer could drain
        // indefinitely on one source and starve its own outbound.
        const MAX_BATCHES_PER_SOURCE_PER_PASS: usize = 256;

        let mut slots = self.coop_receivers.lock().unwrap();
        for slot in slots.iter_mut() {
            let Some(rx) = slot.as_ref() else {
                continue;
            };
            for _ in 0..MAX_BATCHES_PER_SOURCE_PER_PASS {
                match rx.try_recv_batch() {
                    RecvBatchOutcome::Batch { header, batch } => {
                        let buf = self.register_channel(
                            header.sender_proc(),
                            header.stage_id,
                            header.partition,
                        );
                        buf.push_batch(batch);
                    }
                    RecvBatchOutcome::Eof { header } => {
                        let buf = self.register_channel(
                            header.sender_proc(),
                            header.stage_id,
                            header.partition,
                        );
                        buf.notify_source_done();
                        // Other channels may still flow on this queue, so the receiver slot
                        // stays live.
                    }
                    RecvBatchOutcome::Empty => break,
                    RecvBatchOutcome::Detached => {
                        // Only THIS receiver is dead. Under mesh-multiplexing the drain holds
                        // multiple receivers (own-inbox MPSC + self-loop in-proc); one going
                        // away does not imply the others have. Do not fire a registry-wide
                        // "all channels EOF" here — channel buffers waiting on a still-live
                        // sibling receiver would falsely terminate. Per-channel EOF flows via
                        // the demuxed Eof frame above; query-teardown unblock flows via
                        // `cancel_channel_buffers` from `DrainHandle::Drop`.
                        *slot = None;
                        break;
                    }
                    RecvBatchOutcome::Error(e) => {
                        // Same reasoning as Detached: drop only this slot. The error itself
                        // still propagates up — caller surfaces it as a query error.
                        *slot = None;
                        return Err(e);
                    }
                }
            }
        }
        Ok(())
    }
}

impl Drop for DrainHandle {
    fn drop(&mut self) {
        // Unblock any consumer blocked on a channel buffer when the handle is torn down before EOF
        // flows naturally (e.g. a query error en route to ExecEndCustomScan).
        self.cancel_channel_buffers();
    }
}
/// SPSC channel pair for two use cases:
/// - Unit tests (bounded capacity, exercising backpressure).
/// - Production self-loop slots: when a worker's fragment emits a partition destined for its OWN
///   proc (e.g. peer-mesh hash routing where consumer task t lands on the same worker as
///   producer task t), the shm_mq grid leaves the `slot(this_proc, this_proc)` diagonal
///   unattached. The dispatcher routes those self-loops through this in-proc channel instead.
///   It shares the same `BatchChannelSender`/`BatchChannelReceiver` abstraction as shm_mq, so the
///   drain and channel buffer registry don't need a special case.
///
/// Production callers pass a very large `capacity` so the channel is effectively unbounded under
/// steady state. The current-thread Tokio runtime interleaves producer and consumer fragments
/// via `yield_now().await`, so backpressure would be benign anyway, but unbounded rules out any
/// chance of self-deadlock if the producer never yields.
pub(super) fn in_proc_channel(capacity: usize) -> (InProcSender, InProcReceiver) {
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(capacity);
    (
        InProcSender {
            tx,
            send_lock: tokio::sync::Mutex::new(()),
        },
        InProcReceiver { rx: Mutex::new(rx) },
    )
}

pub(super) struct InProcSender {
    tx: std::sync::mpsc::SyncSender<Vec<u8>>,
    /// Per-instance lock so the [`BatchChannelSender::send_lock`] contract holds even when an
    /// in-proc channel ends up in a code path that would otherwise need serialization. In-proc
    /// `send_bytes` is already atomic (each call pushes a complete `Vec<u8>`), so the lock is
    /// effectively a no-op here, but keeping it uniform with `ShmMqSender` avoids special-casing
    /// the caller.
    send_lock: tokio::sync::Mutex<()>,
}

pub(super) struct InProcReceiver {
    // The std::sync::mpsc receiver is !Sync; wrap in a Mutex so the drain
    // thread can hold it behind a `Box<dyn BatchChannelReceiver>` (which is
    // `Send + Sync`-relaxed by design, but we only need Send for the thread
    // hand-off). Tests only ever access from one thread so the Mutex is
    // uncontended.
    rx: Mutex<std::sync::mpsc::Receiver<Vec<u8>>>,
}

impl BatchChannelSender for InProcSender {
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), DataFusionError> {
        self.tx.send(bytes.to_vec()).map_err(|_| {
            DataFusionError::Execution("mpp: in-proc channel detached during send".into())
        })
    }

    fn try_send_bytes(&self, bytes: &[u8]) -> Result<bool, DataFusionError> {
        match self.tx.try_send(bytes.to_vec()) {
            Ok(()) => Ok(true),
            Err(std::sync::mpsc::TrySendError::Full(_)) => Ok(false),
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => Err(DataFusionError::Execution(
                "mpp: in-proc channel detached during try_send".into(),
            )),
        }
    }

    fn send_lock(&self) -> &tokio::sync::Mutex<()> {
        &self.send_lock
    }
}

impl BatchChannelReceiver for InProcReceiver {
    fn try_recv(&self) -> RecvOutcome {
        let rx = self.rx.lock().expect("InProcReceiver mutex poisoned");
        match rx.try_recv() {
            Ok(bytes) => RecvOutcome::Bytes(bytes),
            Err(std::sync::mpsc::TryRecvError::Empty) => RecvOutcome::Empty,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => RecvOutcome::Detached,
        }
    }
}

/// Effectively unbounded capacity for self-loop in-proc channels. The
/// `std::sync::mpsc::sync_channel` API requires a numeric capacity; this constant picks one large
/// enough that production workloads won't reach it but small enough that a runaway producer
/// (e.g. infinite-loop bug) won't allocate billions of `Vec<u8>` before OOM.
pub(super) const SELF_LOOP_CAPACITY: usize = 1 << 20;

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::array::{Int32Array, Int64Array, StringArray, UInt64Array};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use std::sync::Arc as StdArc;
    use std::thread;

    use std::thread::JoinHandle;

    impl DrainBuffer {
        /// Block until a batch is available, EOF is reached, or the buffer is cancelled.
        ///
        /// INVARIANT: any already-buffered batch is returned *before* honoring either
        /// cancellation or all-sources-done. Reordering the queue pop ahead of the cancel/eof
        /// check would silently drop buffered data on an otherwise-clean shutdown; the
        /// `drain_buffer_drains_buffered_before_eof` test locks this in.
        fn pop_front(&self) -> DrainItem {
            let mut guard = self.inner.lock().expect("DrainBuffer mutex poisoned");
            loop {
                if let Some(batch) = guard.queue.pop_front() {
                    return DrainItem::Batch(batch);
                }
                if guard.cancelled || guard.sources_done >= guard.num_sources {
                    return DrainItem::Eof;
                }
                guard = self.cond.wait(guard).expect("DrainBuffer mutex poisoned");
            }
        }

        /// True if `cancel` has been called. The local `drain_loop` consults this; the
        /// cooperative production path watches the flag through `notify_source_done` fan-out
        /// instead.
        fn is_cancelled(&self) -> bool {
            self.inner
                .lock()
                .expect("DrainBuffer mutex poisoned")
                .cancelled
        }
    }

    impl MppSender {
        /// Construct a sender with the default `(stage_id=0, partition=0)` header. Used where
        /// the header carries no actionable routing info.
        fn new(channel: Arc<dyn BatchChannelSender>) -> Self {
            Self::with_header(channel, MppFrameHeader::batch(0, 0, 0))
        }

        /// Stats-less wrapper around `send_batch_traced`. Production call sites
        /// (`ShuffleStream::process_batch`) always pass a `SendBatchStats` so per-peer
        /// wall-time shows up in the EOF trace. Wraps the async send in a tiny current-thread
        /// Tokio runtime so `#[test]` functions don't have to be `#[tokio::test]` and the
        /// OS-thread-spawning test harnesses don't have to plumb an async runtime themselves.
        fn send_batch(&self, batch: &RecordBatch) -> Result<(), DataFusionError> {
            let mut stats = SendBatchStats::default();
            let rt = tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("test tokio runtime build");
            rt.block_on(self.send_batch_traced(batch, &mut stats))
        }
    }

    /// Configuration for `spawn_drain_thread`. pgrx panics on any pg FFI call (including
    /// `shm_mq_receive`) from a non-backend thread, so production never spawns a drain thread —
    /// see [`DrainHandle::cooperative`] for the cooperative path.
    struct DrainConfig {
        /// Receivers to drain. Ownership moves into the spawned thread.
        receivers: Vec<MppReceiver>,
        /// Destination buffer.
        buffer: Arc<DrainBuffer>,
        /// How long to sleep when every receiver is empty but some are still attached. Tuning:
        /// small values reduce end-of-batch latency but raise CPU; 1 ms is a safe default until
        /// we integrate with WaitLatch.
        idle_sleep: Duration,
    }

    impl DrainConfig {
        fn new(receivers: Vec<MppReceiver>, buffer: Arc<DrainBuffer>) -> Self {
            Self {
                receivers,
                buffer,
                idle_sleep: Duration::from_millis(1),
            }
        }
    }

    /// Spawn the dedicated drain thread. The thread round-robins through every receiver with
    /// non-blocking `try_recv`, pushes decoded batches into `buffer`, and marks each source done
    /// as soon as it observes a detach or decode error. When every source is done, the thread
    /// exits.
    fn spawn_drain_thread(config: DrainConfig) -> JoinHandle<Result<(), DataFusionError>> {
        thread::spawn(move || drain_loop(config))
    }

    /// RAII wrapper: owns the drain thread's `JoinHandle` and the buffer it writes into.
    /// `Drop` cancels the buffer (unblocking the consumer) and joins the thread, so the thread
    /// can never outlive the test scope even on a panic.
    struct ThreadedDrainHandle {
        buffer: Arc<DrainBuffer>,
        join: Mutex<Option<JoinHandle<Result<(), DataFusionError>>>>,
    }

    impl ThreadedDrainHandle {
        fn spawn(config: DrainConfig) -> Self {
            let buffer = Arc::clone(&config.buffer);
            let join = spawn_drain_thread(config);
            Self {
                buffer,
                join: Mutex::new(Some(join)),
            }
        }
    }

    impl Drop for ThreadedDrainHandle {
        fn drop(&mut self) {
            self.buffer.cancel();
            if let Some(join) = self.join.lock().unwrap().take() {
                let _ = join.join();
            }
        }
    }

    /// Test-only thread-backed drain. Writes every observed frame into a single shared
    /// [`DrainBuffer`] with `num_sources = N`. Per-channel `Eof` frames are treated as "this source
    /// is done" rather than "this logical channel within the source is done"; sufficient for unit
    /// tests that don't exercise per-channel demux. Production drains route through
    /// [`DrainHandle::try_drain_pass`] (cooperative variant), which keys on the frame header. Tests
    /// that need to validate production demux must use [`DrainHandle::cooperative`] and call
    /// `try_drain_pass` directly.
    fn drain_loop(config: DrainConfig) -> Result<(), DataFusionError> {
        let DrainConfig {
            receivers,
            buffer,
            idle_sleep,
        } = config;

        let mut done = vec![false; receivers.len()];
        loop {
            // Observe cancellation before each pass so a `DrainHandle::drop` with
            // live peer senders tears down cleanly. Without this check, the drain
            // thread would spin `try_recv` forever because no source has detached.
            if buffer.is_cancelled() {
                return Ok(());
            }

            let mut got_any = false;
            let mut all_done = true;
            for (i, rx) in receivers.iter().enumerate() {
                if done[i] {
                    continue;
                }
                all_done = false;
                match rx.try_recv_batch() {
                    RecvBatchOutcome::Batch { header: _, batch } => {
                        got_any = true;
                        buffer.push_batch(batch);
                    }
                    RecvBatchOutcome::Eof { header: _ } => {
                        // Per-channel Eof frame: single-channel positional design
                        // treats it as a source-done signal. See `try_drain_pass`.
                        done[i] = true;
                        buffer.notify_source_done();
                    }
                    RecvBatchOutcome::Empty => {}
                    RecvBatchOutcome::Detached => {
                        done[i] = true;
                        buffer.notify_source_done();
                    }
                    RecvBatchOutcome::Error(e) => {
                        // Treat a decode error as a fatal detach for this source
                        // so the consumer can observe EOF and abort the query.
                        done[i] = true;
                        buffer.notify_source_done();
                        return Err(e);
                    }
                }
            }

            if all_done {
                return Ok(());
            }
            if !got_any {
                thread::sleep(idle_sleep);
            }
        }
    }

    fn sample_batch(rows: i32) -> RecordBatch {
        let schema = StdArc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]));
        let ids = Int32Array::from_iter_values(0..rows);
        let names = StringArray::from_iter_values((0..rows).map(|i| format!("n{i}")));
        RecordBatch::try_new(schema, vec![StdArc::new(ids), StdArc::new(names)]).unwrap()
    }

    #[test]
    fn frame_round_trips_a_batch_with_header() {
        let orig = sample_batch(64);
        let header = MppFrameHeader::batch(7, 3, 0);
        let mut buf = Vec::with_capacity(1024);
        encode_frame_into(header, &orig, &mut buf).expect("encode_frame");

        let (parsed, batch_opt) = decode_frame(&buf).expect("decode_frame");
        assert_eq!(parsed, header);
        assert_eq!(parsed.kind().unwrap(), MppFrameKind::Batch);
        let decoded = batch_opt.expect("Batch frame must carry a payload");
        assert_eq!(decoded.num_rows(), 64);
        assert_eq!(decoded.schema(), orig.schema());
        assert_eq!(decoded.num_columns(), orig.num_columns());
        for col in 0..orig.num_columns() {
            assert_eq!(orig.column(col).as_ref(), decoded.column(col).as_ref());
        }
    }

    #[test]
    fn frame_round_trips_eof() {
        let mut buf = Vec::new();
        encode_eof_frame_into(2, 5, 0, &mut buf).expect("encode_eof");
        assert_eq!(buf.len(), MPP_FRAME_HEADER_SIZE);

        let (header, batch_opt) = decode_frame(&buf).expect("decode_frame");
        assert_eq!(header, MppFrameHeader::eof(2, 5, 0));
        assert_eq!(header.kind().unwrap(), MppFrameKind::Eof);
        assert!(batch_opt.is_none());
    }

    #[test]
    fn frame_rejects_short_message() {
        let too_short = vec![0u8; MPP_FRAME_HEADER_SIZE - 1];
        let err = decode_frame(&too_short).expect_err("short frame must fail");
        assert!(format!("{err}").contains("too short"));
    }

    #[test]
    fn frame_rejects_bad_magic() {
        // Explicit non-zero, non-magic prefix. Don't rely on the
        // happenstance that 0u32 != MPP_FRAME_MAGIC.
        let mut bad = vec![0u8; MPP_FRAME_HEADER_SIZE];
        bad[0..4].copy_from_slice(&0xCAFEBABE_u32.to_le_bytes());
        let err = decode_frame(&bad).expect_err("bad magic must fail");
        assert!(format!("{err}").contains("bad frame magic"));
        bad[0..4].copy_from_slice(&0xDEADBEEF_u32.to_le_bytes());
        let err = decode_frame(&bad).expect_err("bad magic must fail");
        assert!(format!("{err}").contains("bad frame magic"));
    }

    #[test]
    fn frame_rejects_unknown_kind() {
        let header = MppFrameHeader {
            magic: MPP_FRAME_MAGIC,
            flags: 0x42, // unknown kind byte, no reserved bits set
            stage_id: 0,
            partition: 0,
        };
        let mut buf = vec![0u8; MPP_FRAME_HEADER_SIZE];
        header.write_to(&mut buf);
        let err = decode_frame(&buf).expect_err("unknown kind must fail");
        assert!(format!("{err}").contains("unknown frame kind"));
    }

    #[test]
    fn frame_rejects_reserved_flag_bits() {
        // Reserved range is bits 8..16. Bits 16..32 are sender_proc and must NOT trip the
        // reserved check. Cover both boundaries of the reserved range.
        for bit in [0x0000_0100u32, 0x0000_8000u32] {
            let header = MppFrameHeader {
                magic: MPP_FRAME_MAGIC,
                flags: bit, // kind byte 0 (Batch), reserved bit set, no sender_proc
                stage_id: 0,
                partition: 0,
            };
            let mut buf = vec![0u8; MPP_FRAME_HEADER_SIZE];
            header.write_to(&mut buf);
            let err = decode_frame(&buf).expect_err(&format!("reserved bit {bit:#x} must fail"));
            assert!(
                format!("{err}").contains("reserved frame flag bits"),
                "bit {bit:#x}: {err}"
            );
        }
    }

    #[test]
    fn frame_kind_coexists_with_max_sender_proc() {
        // Negative-space companion to frame_rejects_reserved_flag_bits: setting every bit in
        // 16..32 (= max sender_proc) along with kind=Eof in bit 0 must parse cleanly without
        // tripping the reserved-bits check, and sender_proc()/kind() must read both back.
        let header = MppFrameHeader {
            magic: MPP_FRAME_MAGIC,
            flags: 0xFFFF_0001, // Eof in low byte, max sender_proc in high half, reserved=0
            stage_id: 0,
            partition: 0,
        };
        assert_eq!(header.kind().unwrap(), MppFrameKind::Eof);
        assert_eq!(header.sender_proc(), MPP_MAX_SENDER_PROC);
    }

    #[test]
    fn frame_sender_proc_round_trip() {
        // sender_proc lives in flags bits 16..32 and shouldn't collide with kind or reserved.
        for &sp in &[0u32, 1, 7, 255, 256, 1023, 65534, MPP_MAX_SENDER_PROC] {
            let header = MppFrameHeader::batch(11, 5, sp);
            assert_eq!(header.sender_proc(), sp, "batch round-trip sp={sp}");
            assert_eq!(header.kind().unwrap(), MppFrameKind::Batch);

            let mut buf = Vec::with_capacity(MPP_FRAME_HEADER_SIZE);
            let payload = sample_batch(8);
            encode_frame_into(header, &payload, &mut buf).expect("encode batch");
            let (parsed, _) = decode_frame(&buf).expect("decode batch");
            assert_eq!(parsed.sender_proc(), sp, "decoded batch sender_proc");

            let mut eof_buf = Vec::new();
            encode_eof_frame_into(11, 5, sp, &mut eof_buf).expect("encode eof");
            let (parsed_eof, _) = decode_frame(&eof_buf).expect("decode eof");
            assert_eq!(parsed_eof.sender_proc(), sp, "decoded eof sender_proc");
            assert_eq!(parsed_eof.kind().unwrap(), MppFrameKind::Eof);
        }
    }

    #[test]
    fn frame_eof_with_payload_is_rejected() {
        let mut buf = Vec::with_capacity(32);
        encode_eof_frame_into(0, 0, 0, &mut buf).expect("encode_eof");
        buf.push(0xAB); // smuggle a payload byte after the Eof header
        let err = decode_frame(&buf).expect_err("Eof+payload must fail");
        assert!(format!("{err}").contains("Eof frame carries payload"));
    }

    #[test]
    fn codec_round_trips_many_batch_sizes() {
        let mut buf = Vec::with_capacity(1024);
        for rows in [0, 1, 7, 64, 1024] {
            let orig = sample_batch(rows);
            encode_frame_into(MppFrameHeader::batch(0, 0, 0), &orig, &mut buf).expect("encode");
            let (_header, decoded) = decode_frame(&buf).expect("decode");
            let decoded = decoded.expect("Batch frame must carry a payload");
            assert_eq!(orig.num_rows(), decoded.num_rows());
        }
    }

    #[test]
    fn drain_buffer_pop_returns_pushed_batches_in_order() {
        let buf = DrainBuffer::new(1);
        buf.push_batch(sample_batch(3));
        buf.push_batch(sample_batch(5));
        buf.notify_source_done();

        match buf.pop_front() {
            DrainItem::Batch(b) => assert_eq!(b.num_rows(), 3),
            DrainItem::Eof => panic!("expected batch"),
        }
        match buf.pop_front() {
            DrainItem::Batch(b) => assert_eq!(b.num_rows(), 5),
            DrainItem::Eof => panic!("expected batch"),
        }
        matches!(buf.pop_front(), DrainItem::Eof);
    }

    #[test]
    fn drain_buffer_pop_blocks_until_push_then_eof() {
        let buf = DrainBuffer::new(2);
        let producer = StdArc::clone(&buf);
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            producer.push_batch(sample_batch(2));
            producer.notify_source_done();
            thread::sleep(Duration::from_millis(20));
            producer.notify_source_done();
        });

        match buf.pop_front() {
            DrainItem::Batch(b) => assert_eq!(b.num_rows(), 2),
            DrainItem::Eof => panic!("expected batch first"),
        }
        assert!(matches!(buf.pop_front(), DrainItem::Eof));
        handle.join().unwrap();
    }

    #[test]
    fn drain_buffer_cancel_unblocks_waiter() {
        let buf = DrainBuffer::new(1);
        let canceller = StdArc::clone(&buf);
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            canceller.cancel();
        });
        assert!(matches!(buf.pop_front(), DrainItem::Eof));
        handle.join().unwrap();
    }

    #[test]
    fn in_proc_channel_round_trips_through_mpp_sender_receiver() {
        let (tx, rx) = in_proc_channel(8);
        let sender = MppSender::new(Arc::new(tx));
        let receiver = MppReceiver::new(Box::new(rx));

        sender.send_batch(&sample_batch(4)).unwrap();
        std::mem::drop(sender);

        match receiver.try_recv_batch() {
            RecvBatchOutcome::Batch { header: _, batch } => assert_eq!(batch.num_rows(), 4),
            other => panic!("expected batch, got {other:?}"),
        }
        assert!(matches!(
            receiver.try_recv_batch(),
            RecvBatchOutcome::Detached
        ));
    }

    #[test]
    fn drain_thread_drains_single_source() {
        let (tx, rx) = in_proc_channel(4);
        let sender = MppSender::new(Arc::new(tx));
        let receiver = MppReceiver::new(Box::new(rx));
        let buffer = DrainBuffer::new(1);

        let join = spawn_drain_thread(DrainConfig::new(vec![receiver], StdArc::clone(&buffer)));

        thread::spawn(move || {
            for rows in [1, 2, 3, 4, 5] {
                sender.send_batch(&sample_batch(rows)).unwrap();
            }
            // Drop sender to signal EOF
        })
        .join()
        .unwrap();

        let mut received = Vec::new();
        while let DrainItem::Batch(b) = buffer.pop_front() {
            received.push(b.num_rows());
        }
        assert_eq!(received, vec![1, 2, 3, 4, 5]);
        join.join().unwrap().unwrap();
    }

    #[test]
    fn drain_handle_shutdown_joins_cleanly() {
        let (tx, rx) = in_proc_channel(4);
        let sender = MppSender::new(Arc::new(tx));
        let receiver = MppReceiver::new(Box::new(rx));
        let buffer = DrainBuffer::new(1);
        let handle =
            ThreadedDrainHandle::spawn(DrainConfig::new(vec![receiver], StdArc::clone(&buffer)));

        sender.send_batch(&sample_batch(2)).unwrap();
        std::mem::drop(sender); // detach
                                // Pop the one batch
        assert!(matches!(buffer.pop_front(), DrainItem::Batch(_)));
        assert!(matches!(buffer.pop_front(), DrainItem::Eof));
        // Drop drives production teardown (cancel + join). Test passes if
        // this returns without hanging.
        std::mem::drop(handle);
    }

    #[test]
    fn drain_handle_drop_cancels_and_joins() {
        // Build a drain that never detaches (we keep the sender alive), then
        // drop the handle. The Drop impl must cancel the buffer and join the
        // thread without hanging.
        let (tx, rx) = in_proc_channel(4);
        let _sender_kept_alive = MppSender::new(Arc::new(tx));
        let receiver = MppReceiver::new(Box::new(rx));
        let buffer = DrainBuffer::new(1);
        let handle =
            ThreadedDrainHandle::spawn(DrainConfig::new(vec![receiver], StdArc::clone(&buffer)));

        // Simulate consumer path error: drop the handle without calling
        // shutdown(). The drain thread must exit before drop returns.
        let start = Instant::now();
        drop(handle);
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_secs(2),
            "ThreadedDrainHandle::drop took too long: {elapsed:?}"
        );
        // Consumer observes EOF because cancel was called.
        assert!(matches!(buffer.pop_front(), DrainItem::Eof));
    }

    #[test]
    fn drain_thread_drains_n2_mesh_100k_batches() {
        // Simulates a 2-proc mesh under load. Each of two producers
        // pushes 50_000 small batches through a bounded channel; the drain
        // thread interleaves and the consumer reads EOF exactly after
        // receiving all 100_000 batches. Exercises backpressure (bounded
        // capacity = 16) without deadlock.
        const PER_SOURCE: usize = 50_000;
        let (tx0, rx0) = in_proc_channel(16);
        let (tx1, rx1) = in_proc_channel(16);
        let receivers = vec![
            MppReceiver::new(Box::new(rx0)),
            MppReceiver::new(Box::new(rx1)),
        ];
        let buffer = DrainBuffer::new(2);
        let drain_join = spawn_drain_thread(DrainConfig::new(receivers, StdArc::clone(&buffer)));

        let tx0_send = MppSender::new(Arc::new(tx0));
        let tx1_send = MppSender::new(Arc::new(tx1));
        let batch_template = sample_batch(1);

        let p0 = {
            let b = batch_template.clone();
            thread::spawn(move || {
                for _ in 0..PER_SOURCE {
                    tx0_send.send_batch(&b).unwrap();
                }
            })
        };
        let p1 = {
            let b = batch_template.clone();
            thread::spawn(move || {
                for _ in 0..PER_SOURCE {
                    tx1_send.send_batch(&b).unwrap();
                }
            })
        };

        let mut total = 0usize;
        while let DrainItem::Batch(_) = buffer.pop_front() {
            total += 1;
        }
        assert_eq!(total, 2 * PER_SOURCE);
        p0.join().unwrap();
        p1.join().unwrap();
        drain_join.join().unwrap().unwrap();
    }

    #[test]
    fn drain_buffer_drains_buffered_before_eof() {
        // Even if all sources have finished and cancel fires, any already-
        // buffered batches must be observed before Eof.
        let buf = DrainBuffer::new(1);
        buf.push_batch(sample_batch(1));
        buf.push_batch(sample_batch(1));
        buf.notify_source_done();
        buf.cancel();

        assert!(matches!(buf.pop_front(), DrainItem::Batch(_)));
        assert!(matches!(buf.pop_front(), DrainItem::Batch(_)));
        assert!(matches!(buf.pop_front(), DrainItem::Eof));
    }

    // ---------------------------------------------------------------------
    // Throughput microbenches.
    //
    // These are `#[ignore]` by default because they spin for seconds and spam stdout. Run with:
    //
    //   cargo test --package pg_search --release \
    //       postgres::customscan::mpp::transport::tests::throughput \
    //       -- --ignored --nocapture
    //
    // They help us bound the transport layer's cost independently of DataFusion/Tantivy. All use
    // the `in_proc_channel` backend (same `MppSender`/`MppReceiver` trait boundary as the shm_mq
    // one), so numbers here are an optimistic ceiling. shm_mq adds the ring-buffer copy +
    // cross-process notification cost on top. If these numbers are already below the row rate
    // the real query needs, we know IPC encode + channel handoff is the bottleneck without
    // needing CI data.
    // ---------------------------------------------------------------------

    /// Row shape matching the post-Partial shuffle in
    /// `aggregate_join_groupby`: a grouping key (title string) plus two
    /// partial-aggregate accumulators (COUNT u64, SUM i64).
    fn postagg_shape_batch(rows: usize) -> RecordBatch {
        let schema = StdArc::new(Schema::new(vec![
            Field::new("title", DataType::Utf8, false),
            Field::new("count_partial", DataType::UInt64, false),
            Field::new("sum_partial", DataType::Int64, false),
        ]));
        // Titles averaging ~30 bytes, typical for the docs dataset.
        let titles = StringArray::from_iter_values(
            (0..rows).map(|i| format!("file_{i:012}_title_with_some_length")),
        );
        let counts = UInt64Array::from_iter_values((0..rows as u64).map(|i| i % 64 + 1));
        let sums = Int64Array::from_iter_values((0..rows as i64).map(|i| i * 1024));
        RecordBatch::try_new(
            schema,
            vec![StdArc::new(titles), StdArc::new(counts), StdArc::new(sums)],
        )
        .unwrap()
    }

    /// Row shape matching the probe-side shuffle in the same query:
    /// `pages.fileId` (u64) plus `pages.sizeInBytes` (i64).
    fn probe_shape_batch(rows: usize) -> RecordBatch {
        let schema = StdArc::new(Schema::new(vec![
            Field::new("fileId", DataType::UInt64, false),
            Field::new("sizeInBytes", DataType::Int64, false),
        ]));
        let ids =
            UInt64Array::from_iter_values((0..rows as u64).map(|i| i.wrapping_mul(2654435761)));
        let sizes = Int64Array::from_iter_values((0..rows as i64).map(|i| i * 37));
        RecordBatch::try_new(schema, vec![StdArc::new(ids), StdArc::new(sizes)]).unwrap()
    }

    fn bench_throughput(
        label: &str,
        make_batch: fn(usize) -> RecordBatch,
        batch_rows: usize,
        total_rows: usize,
    ) {
        let batches = total_rows.div_ceil(batch_rows);
        let template = make_batch(batch_rows);
        // Encode once up front so we also report pure-encode throughput
        // separately. Real queries encode inside the hot path per batch.
        let enc_start = Instant::now();
        let mut enc_bytes = 0usize;
        let mut enc_buf = Vec::with_capacity(1024);
        for _ in 0..batches {
            encode_frame_into(MppFrameHeader::batch(0, 0, 0), &template, &mut enc_buf)
                .expect("encode");
            enc_bytes += enc_buf.len();
        }
        let enc_elapsed = enc_start.elapsed();

        // N=2 mesh: two senders, one drain thread, one consumer. Matches
        // the gb_postagg / gb_right topology in the real query.
        let (tx0, rx0) = in_proc_channel(16);
        let (tx1, rx1) = in_proc_channel(16);
        let receivers = vec![
            MppReceiver::new(Box::new(rx0)),
            MppReceiver::new(Box::new(rx1)),
        ];
        let buffer = DrainBuffer::new(2);
        let drain_join = spawn_drain_thread(DrainConfig::new(receivers, StdArc::clone(&buffer)));
        let tx0_send = MppSender::new(Arc::new(tx0));
        let tx1_send = MppSender::new(Arc::new(tx1));

        let per_source = batches / 2;
        let round_trip_start = Instant::now();
        let p0 = {
            let b = template.clone();
            thread::spawn(move || {
                for _ in 0..per_source {
                    tx0_send.send_batch(&b).unwrap();
                }
            })
        };
        let p1 = {
            let b = template.clone();
            thread::spawn(move || {
                for _ in 0..per_source {
                    tx1_send.send_batch(&b).unwrap();
                }
            })
        };

        let mut got_rows = 0usize;
        let mut got_batches = 0usize;
        while let DrainItem::Batch(b) = buffer.pop_front() {
            got_rows += b.num_rows();
            got_batches += 1;
        }
        p0.join().unwrap();
        p1.join().unwrap();
        drain_join.join().unwrap().unwrap();
        let rt_elapsed = round_trip_start.elapsed();

        let enc_mb_per_s = (enc_bytes as f64 / (1024.0 * 1024.0)) / enc_elapsed.as_secs_f64();
        let enc_rows_per_s = (batches * batch_rows) as f64 / enc_elapsed.as_secs_f64();
        let rt_rows_per_s = got_rows as f64 / rt_elapsed.as_secs_f64();
        let rt_bytes_total_mb = enc_bytes as f64 / (1024.0 * 1024.0);
        let rt_mb_per_s = rt_bytes_total_mb / rt_elapsed.as_secs_f64();
        let per_batch_us = rt_elapsed.as_micros() as f64 / got_batches as f64;

        println!(
            "[throughput] {label:<18} batch_rows={batch_rows:<5} batches={got_batches:<6} rows={got_rows} \
             encode_only: {enc_rows_per_s:>11.0} rows/s {enc_mb_per_s:>7.1} MB/s | \
             round_trip: {rt_rows_per_s:>11.0} rows/s {rt_mb_per_s:>7.1} MB/s ({per_batch_us:.1}us/batch)"
        );
    }

    #[test]
    #[ignore]
    fn throughput_postagg_shape() {
        // Sweeps batch size to show per-batch fixed cost vs per-row cost.
        // 1.25M total rows ≈ what one proc ships through gb_postagg at
        // 25M scale. 625K per proc × 2 = 1.25M.
        for batch_rows in [128, 512, 2048, 8192, 32_768] {
            bench_throughput("postagg", postagg_shape_batch, batch_rows, 1_250_000);
        }
    }

    #[test]
    #[ignore]
    fn throughput_probe_shape() {
        // 12.5M total rows ≈ what one proc ships through gb_right at 25M.
        for batch_rows in [128, 512, 2048, 8192, 32_768] {
            bench_throughput("probe", probe_shape_batch, batch_rows, 12_500_000);
        }
    }

    // ---------------------------------------------------------------------
    // Per-`(stage_id, partition)` channel buffer registry on the cooperative `DrainHandle`.
    //
    // Producers stamp `MppFrameHeader::batch(stage_id, partition)` on every outgoing frame, and
    // the receiver-side cooperative drain demuxes by header into a channel buffer per
    // `(stage_id, partition)`. These tests use the `in_proc_channel` backend to drive
    // `try_drain_pass` from the test thread. That mirrors how the production path runs the drain
    // inline from `DrainGatherStream::poll_next` on the backend thread.
    // ---------------------------------------------------------------------

    /// Drain a `DrainHandle::cooperative` to completion: poll until every receiver returns
    /// `Empty`. With the `in_proc_channel` test backend the drain observes `Detached` once the
    /// producer drops its sender, so a bounded loop of `try_drain_pass` calls is enough to flush
    /// everything the producer wrote.
    fn drain_until_detached(handle: &DrainHandle) {
        for _ in 0..64 {
            handle.try_drain_pass().expect("try_drain_pass");
        }
    }

    #[test]
    fn drain_handle_demuxes_frames_by_header() {
        // One queue carrying two channels: `(0, 0)` and `(0, 1)`. Each
        // channel buffer receives only its own batches. Per-channel EOF is
        // out of scope here — see `drain_handle_eof_frame_closes_one_channel`
        // for explicit-Eof routing and `drain_handle_drop_cancels_registered_channel_buffers`
        // for the teardown-EOF contract.
        let (tx, rx) = in_proc_channel(8);
        let base = MppSender::new(Arc::new(tx));
        let s00 = base.clone_with_header(MppFrameHeader::batch(0, 0, 0));
        let s01 = base.clone_with_header(MppFrameHeader::batch(0, 1, 0));
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver]);

        s00.send_batch(&sample_batch(2)).unwrap();
        s01.send_batch(&sample_batch(7)).unwrap();
        s00.send_batch(&sample_batch(3)).unwrap();
        drop(s00);
        drop(s01);
        drop(base);

        let buf00 = handle.register_channel(0, 0, 0);
        let buf01 = handle.register_channel(0, 0, 1);

        drain_until_detached(&handle);

        let mut p0_rows = Vec::new();
        while let Some(DrainItem::Batch(b)) = buf00.try_pop() {
            p0_rows.push(b.num_rows());
        }
        let mut p1_rows = Vec::new();
        while let Some(DrainItem::Batch(b)) = buf01.try_pop() {
            p1_rows.push(b.num_rows());
        }
        assert_eq!(p0_rows, vec![2, 3]);
        assert_eq!(p1_rows, vec![7]);
    }

    #[test]
    fn drain_handle_eof_frame_closes_one_channel() {
        // An `Eof` frame on `(0, 0)` closes that channel buffer while frames on
        // `(0, 1)` continue to flow on the same queue. `Detached` no longer
        // broadcasts a registry-wide EOF (see commit 1bda01f02), so `(0, 1)`
        // surfaces EOF only when the handle's `Drop` runs `cancel_channel_buffers`.
        let (tx, rx) = in_proc_channel(8);
        let tx_arc: Arc<dyn BatchChannelSender> = Arc::new(tx);
        let s00 = MppSender::with_header(Arc::clone(&tx_arc), MppFrameHeader::batch(0, 0, 0));
        let s01 = MppSender::with_header(Arc::clone(&tx_arc), MppFrameHeader::batch(0, 1, 0));
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver]);

        s00.send_batch(&sample_batch(4)).unwrap();
        let mut eof_buf = Vec::new();
        encode_eof_frame_into(0, 0, 0, &mut eof_buf).unwrap();
        tx_arc.send_bytes(&eof_buf).unwrap();
        s01.send_batch(&sample_batch(6)).unwrap();

        let buf00 = handle.register_channel(0, 0, 0);
        let buf01 = handle.register_channel(0, 0, 1);

        drop(s00);
        drop(s01);
        drop(tx_arc);
        drain_until_detached(&handle);

        match buf00.try_pop() {
            Some(DrainItem::Batch(b)) => assert_eq!(b.num_rows(), 4),
            other => panic!("expected (0,0) batch, got {other:?}"),
        }
        assert!(matches!(buf00.try_pop(), Some(DrainItem::Eof)));

        match buf01.try_pop() {
            Some(DrainItem::Batch(b)) => assert_eq!(b.num_rows(), 6),
            other => panic!("expected (0,1) batch, got {other:?}"),
        }
        assert!(buf01.try_pop().is_none());
        drop(handle);
        assert!(matches!(buf01.try_pop(), Some(DrainItem::Eof)));
    }

    #[test]
    fn drain_handle_register_channel_is_idempotent() {
        // Two calls for the same key return Arcs pointing to the same
        // DrainBuffer instance.
        let (_tx, rx) = in_proc_channel(8);
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver]);

        let first = handle.register_channel(0, 2, 3);
        let second = handle.register_channel(0, 2, 3);
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn drain_handle_demuxes_frames_by_stage_id() {
        // Same partition (0) for two different stage ids on the same queue.
        // The registry's compound key keeps them on separate channel buffers.
        let (tx, rx) = in_proc_channel(8);
        let tx_arc: Arc<dyn BatchChannelSender> = Arc::new(tx);
        let s_stage0 = MppSender::with_header(Arc::clone(&tx_arc), MppFrameHeader::batch(0, 0, 0));
        let s_stage1 = MppSender::with_header(Arc::clone(&tx_arc), MppFrameHeader::batch(1, 0, 0));
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver]);

        s_stage0.send_batch(&sample_batch(2)).unwrap();
        s_stage1.send_batch(&sample_batch(9)).unwrap();
        s_stage0.send_batch(&sample_batch(4)).unwrap();
        drop(s_stage0);
        drop(s_stage1);
        drop(tx_arc);

        let buf0 = handle.register_channel(0, 0, 0);
        let buf1 = handle.register_channel(0, 1, 0);

        drain_until_detached(&handle);

        let mut stage0_rows = Vec::new();
        while let Some(DrainItem::Batch(b)) = buf0.try_pop() {
            stage0_rows.push(b.num_rows());
        }
        let mut stage1_rows = Vec::new();
        while let Some(DrainItem::Batch(b)) = buf1.try_pop() {
            stage1_rows.push(b.num_rows());
        }
        assert_eq!(stage0_rows, vec![2, 4]);
        assert_eq!(stage1_rows, vec![9]);
    }

    #[test]
    fn drain_handle_drop_cancels_registered_channel_buffers() {
        // Dropping a cooperative DrainHandle must wake any consumer holding an Arc<DrainBuffer>
        // from `register_channel`. Otherwise a query error path that tears down the mesh would
        // leave a consumer blocked on a buffer that will never see EOF.
        let (_tx, rx) = in_proc_channel(8);
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver]);

        let buf_a = handle.register_channel(0, 0, 0);
        let buf_b = handle.register_channel(0, 7, 3);
        // No data ever flows; the handle is just dropped.
        drop(handle);

        assert!(matches!(buf_a.try_pop(), Some(DrainItem::Eof)));
        assert!(matches!(buf_b.try_pop(), Some(DrainItem::Eof)));
    }
}

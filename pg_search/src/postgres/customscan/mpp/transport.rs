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

/// Magic bytes "MPPF" (MPP Frame) at the start of every wire message.
/// Lets receivers reject misrouted / corrupt frames before they hit Arrow IPC.
const MPP_FRAME_MAGIC: u32 = 0x4D505046;

/// Sentinel embedded in [`DataFusionError`] messages that the producer-side driver matches on to
/// distinguish "consumer torn down mid-stream" (a clean signal — return `Ok(())`) from any other
/// transport-layer error (which must propagate). Centralising the string here keeps the producers
/// and the predicate in [`crate::postgres::customscan::mpp::worker::is_consumer_detached`] in
/// lockstep. Don't change the literal without grepping for the matching `contains` call.
pub(crate) const CONSUMER_DETACHED_SENTINEL: &str = "mpp:consumer_detached";

/// Build the canonical "consumer torn down" error every shm_mq / in-proc sender returns when its
/// peer queue is gone. The producer side recognises it via [`CONSUMER_DETACHED_SENTINEL`].
/// `detail` is a short, free-form note appended for diagnostics (channel kind, send path).
pub(crate) fn consumer_detached_error(detail: &str) -> DataFusionError {
    DataFusionError::Execution(format!("{CONSUMER_DETACHED_SENTINEL}: {detail}"))
}

/// Wire-format size of [`MppFrameHeader`] in bytes. Asserted at compile time
/// below via `const _: ()`.
const MPP_FRAME_HEADER_SIZE: usize = 16;

/// Kind of payload following [`MppFrameHeader`].
///
/// `Batch` is the common case. The header is followed by an Arrow IPC stream containing one
/// `RecordBatch`. `Eof` carries no payload; it signals the receiver that the named
/// `(stage_id, partition)` channel is done, even though the underlying shm_mq queue may still
/// carry frames for other channels. `Request` also carries no payload; it's the consumer-side
/// "please run task X and stream me partition P" message used by the pull-shape protocol (consumer-side `WorkerConnection::stream_partition` sends one before pulling).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MppFrameKind {
    Batch = 0,
    Eof = 1,
    Request = 2,
}

/// 16-byte prefix on every transport frame.
///
/// The fixed layout `[magic, flags, stage_id, partition]` (4×u32) is what senders prepend before
/// the Arrow IPC stream bytes and what receivers parse before deciding which channel buffer the
/// payload belongs to.
///
/// `flags` packs the [`MppFrameKind`] discriminant in the low byte (mask `0x0000_00FF`). The
/// upper 24 bits are interpreted by kind: for `Batch`/`Eof` they are reserved-must-be-zero (and
/// validated at parse time so a future use can repurpose them without a wire-format break), and
/// for `Request` they carry the producer-task index, keeping the header at 16 bytes without
/// growing the wire format.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MppFrameHeader {
    pub magic: u32,
    pub flags: u32,
    pub stage_id: u32,
    pub partition: u32,
}

/// Bit mask in [`MppFrameHeader::flags`] for the [`MppFrameKind`] discriminant.
const FRAME_KIND_MASK: u32 = 0x0000_00FF;

/// Shift to position `task_idx` above the kind byte in `MppFrameHeader::flags`.
const TASK_IDX_SHIFT: u32 = 8;

/// Maximum task index encodable in a `Request` frame: 24 bits, since the low byte is the kind.
/// Plenty of headroom for realistic stage task counts; the constructor errors above this.
const TASK_IDX_MAX: u32 = (1 << 24) - 1;

const _: () = {
    // shm_mq slot layout calculations depend on this being exact.
    assert!(std::mem::size_of::<MppFrameHeader>() == MPP_FRAME_HEADER_SIZE);
};

impl MppFrameHeader {
    /// Build a `Batch` header for the given `(stage_id, partition)`.
    pub fn batch(stage_id: u32, partition: u32) -> Self {
        Self {
            magic: MPP_FRAME_MAGIC,
            flags: MppFrameKind::Batch as u32,
            stage_id,
            partition,
        }
    }

    /// Build an `Eof` header for the given `(stage_id, partition)`. Carries no payload; receivers
    /// route it to the channel buffer's source-done counter. Emitted by
    /// [`MppSender::send_eof_traced`] after a producer fragment's per-partition stream exhausts
    /// (or errors), and consumed by the matching channel buffer's `notify_source_done`.
    pub fn eof(stage_id: u32, partition: u32) -> Self {
        Self {
            magic: MPP_FRAME_MAGIC,
            flags: MppFrameKind::Eof as u32,
            stage_id,
            partition,
        }
    }

    /// Build a `Request` header asking the producer proc to run `(stage_id, task_idx)` and stream
    /// its output partition `partition` back. Errors if `task_idx` doesn't fit in 24 bits — see
    /// [`TASK_IDX_MAX`].
    ///
    /// Header carries the kind in the low byte and `task_idx` in the upper 24 bits; consumers
    /// pull a partition with `(stage_id, task_idx, partition)` and the producer's service loop
    /// dispatches.
    pub fn request(stage_id: u32, task_idx: u32, partition: u32) -> Result<Self, DataFusionError> {
        if task_idx > TASK_IDX_MAX {
            return Err(DataFusionError::Internal(format!(
                "mpp: Request frame task_idx={task_idx} exceeds 24-bit max ({TASK_IDX_MAX})"
            )));
        }
        Ok(Self {
            magic: MPP_FRAME_MAGIC,
            flags: (MppFrameKind::Request as u32) | (task_idx << TASK_IDX_SHIFT),
            stage_id,
            partition,
        })
    }

    /// Read the kind out of `flags`. Errors on unknown kinds, or on non-zero upper bits for
    /// kinds that don't use them (`Batch`/`Eof`). `Request` interprets the upper 24 bits as
    /// `task_idx`, so reserved-bit validation is skipped there.
    pub(super) fn kind(&self) -> Result<MppFrameKind, DataFusionError> {
        let kind = match self.flags & FRAME_KIND_MASK {
            0 => MppFrameKind::Batch,
            1 => MppFrameKind::Eof,
            2 => MppFrameKind::Request,
            other => {
                return Err(DataFusionError::Internal(format!(
                    "mpp: unknown frame kind {other:#x}"
                )))
            }
        };
        if !matches!(kind, MppFrameKind::Request) {
            let reserved = self.flags & !FRAME_KIND_MASK;
            if reserved != 0 {
                return Err(DataFusionError::Internal(format!(
                    "mpp: reserved frame flag bits set ({reserved:#x})"
                )));
            }
        }
        Ok(kind)
    }

    /// Task index encoded in a `Request` frame's upper 24 bits. Only meaningful when `kind()` is
    /// `Request`; returns the raw upper bits otherwise.
    pub(super) fn task_idx(&self) -> u32 {
        self.flags >> TASK_IDX_SHIFT
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
            // Diagnostic: a recent benchmark run on origin/main produced a 12-byte frame on the
            // leader's inbound queue for the `aggregate_join_groupby` query at 100k scale. No
            // encoder in this file produces sub-16-byte output, so the source is something
            // outside the MPP layer. Hex-dump the bytes so the next benchmark surfaces what's
            // actually arriving and we can chase the producer.
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
    buf: &mut Vec<u8>,
) -> Result<(), DataFusionError> {
    buf.clear();
    buf.resize(MPP_FRAME_HEADER_SIZE, 0);
    MppFrameHeader::eof(stage_id, partition).write_to(&mut buf[..MPP_FRAME_HEADER_SIZE]);
    Ok(())
}

/// Serialize a payload-less [`MppFrameKind::Request`] frame for `(stage_id, task_idx,
/// partition)` into `buf`. Same 16-byte-message shape as [`encode_eof_frame_into`]; the producer
/// proc decodes the header, dispatches `(stage_id, task_idx)`, and streams partition `partition`
/// back.
fn encode_request_frame_into(
    stage_id: u32,
    task_idx: u32,
    partition: u32,
    buf: &mut Vec<u8>,
) -> Result<(), DataFusionError> {
    let header = MppFrameHeader::request(stage_id, task_idx, partition)?;
    buf.clear();
    buf.resize(MPP_FRAME_HEADER_SIZE, 0);
    header.write_to(&mut buf[..MPP_FRAME_HEADER_SIZE]);
    Ok(())
}

/// Inverse of [`encode_frame_into`]. Parses the 16-byte header and, for `Batch` frames, decodes
/// the trailing Arrow IPC stream. `Eof` and `Request` frames return `(header, None)`. Receivers
/// branch on `header.kind()` to decide routing.
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
        MppFrameKind::Request => {
            if bytes.len() != MPP_FRAME_HEADER_SIZE {
                return Err(DataFusionError::Internal(format!(
                    "mpp: Request frame carries payload ({} > {})",
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
    /// after observing `SHM_MQ_DETACHED` on a given inbound queue or from
    /// [`DrainHandle::mark_detached`] when the underlying receiver has died.
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
pub(super) trait BatchChannelSender: Send + Sync {
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), DataFusionError>;

    /// Non-blocking variant. Returns `Ok(true)` on success, `Ok(false)`
    /// when the channel is full (caller should retry), `Err` on detach /
    /// transport error. Default falls back to the blocking send — safe
    /// for in-proc channels used by tests where "full" doesn't arise.
    fn try_send_bytes(&self, bytes: &[u8]) -> Result<bool, DataFusionError> {
        self.send_bytes(bytes).map(|()| true)
    }
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
    channel: Arc<dyn BatchChannelSender>,
    cooperative_drain: Option<Arc<dyn CooperativeDrainSet>>,
    /// Frame header prepended to every outgoing batch. Identifies the logical
    /// `(stage_id, partition)` channel the receiver demultiplexes on. Per-sender rather than
    /// per-call: each partition gets its own `MppSender` via `clone_with_header`, all sharing
    /// the underlying `Arc<dyn BatchChannelSender>` of a single shm_mq queue.
    header: MppFrameHeader,
    /// Scratch buffer reused across every `encode_frame_into` on this sender. Sized by the first
    /// batch; subsequent batches clear and re-fill without reallocating. Interior mutability
    /// lets the caller keep the `&self` signature (senders live inside `ShuffleWiring` behind a
    /// shared borrow during `process_batch`).
    scratch: std::cell::RefCell<Vec<u8>>,
}

// SAFETY: `MppSender` lives inside `ShuffleWiring`, which is owned by a
// single `ShuffleExec` running on a single backend thread. The async
// `send_batch_traced` future captures `&self` and contains a Tokio
// `yield_now().await`; the compiler conservatively requires the future
// to be `Send`, which forces `&MppSender: Send` and therefore
// `MppSender: Sync`. At runtime the future is created and consumed on
// the same thread (DataFusion's current-thread runtime on the backend),
// so there is no actual cross-thread aliasing of the inner `RefCell` or
// of the `Box<dyn BatchChannelSender>`. This mirrors the same
// single-thread-by-construction contract that justifies
// `unsafe impl Send for ShmMqSender` over in `mesh.rs`.
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

    /// The header this sender currently stamps onto every outgoing frame. Exposed so callers
    /// holding a `&MppSender` can build a channel-equivalent clone with a different header via
    /// `clone_with_header(s.header())` — useful for diagnostics and for tests that need to
    /// inspect what header the producer side will write.
    #[allow(dead_code)]
    pub fn header(&self) -> MppFrameHeader {
        self.header
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

    /// `send_batch` variant that accumulates per-call timings and spin
    /// counts into `stats`. Callers that report these at EOF (e.g.,
    /// `ShuffleStream`) use this to diagnose where time goes when the
    /// outbound queue is full.
    ///
    /// Async because the cooperative-spin path needs to surrender the
    /// Tokio runtime back periodically — see the body comment.
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

    /// Consumer-side "please run `(stage_id, task_idx)` and stream partition `partition` back"
    /// message for the pull-shape protocol. Same shape as
    /// [`Self::send_eof_traced`]: header-only, no Arrow IPC body, dispatched through the
    /// cooperative-drain spin so a full outbound queue doesn't stall the backend thread.
    ///
    /// `stage_id` and `partition` come from `self.header` (matching the consumer's request
    /// addressing); `task_idx` is passed explicitly because the same `MppSender` can request
    /// different producer tasks across calls.
    ///
    /// `pub(super)` so callers within `mpp::` pick it up without exposing it to other customscan
    /// code.
    pub(super) async fn send_request_traced(
        &self,
        task_idx: u32,
        stats: &mut SendBatchStats,
    ) -> Result<(), DataFusionError> {
        let mut scratch = self.scratch.replace(Vec::new());
        let result = self
            .send_request_with_scratch(task_idx, &mut scratch, stats)
            .await;
        self.scratch.replace(scratch);
        result
    }

    async fn send_eof_with_scratch(
        &self,
        scratch: &mut Vec<u8>,
        stats: &mut SendBatchStats,
    ) -> Result<(), DataFusionError> {
        encode_eof_frame_into(self.header.stage_id, self.header.partition, scratch)?;
        self.send_header_only(scratch, stats).await
    }

    async fn send_request_with_scratch(
        &self,
        task_idx: u32,
        scratch: &mut Vec<u8>,
        stats: &mut SendBatchStats,
    ) -> Result<(), DataFusionError> {
        encode_request_frame_into(
            self.header.stage_id,
            task_idx,
            self.header.partition,
            scratch,
        )?;
        self.send_header_only(scratch, stats).await
    }

    /// Shared spin used by every header-only send path (`Eof`, `Request`). Without a cooperative
    /// drain attached, falls through to the blocking send (in-proc test channels behave that way
    /// and never block long enough to matter).
    async fn send_header_only(
        &self,
        scratch: &[u8],
        stats: &mut SendBatchStats,
    ) -> Result<(), DataFusionError> {
        let Some(drain) = self.cooperative_drain.as_ref() else {
            return self.channel.send_bytes(scratch);
        };
        // No yield_now inside the spin — that's what makes this safe without a per-handle lock.
        // PG's shm_mq partial-send invariant says: once `try_send_bytes` returns WOULD_BLOCK with
        // partial bytes on the wire, the next call on the same handle must repeat the *same*
        // `(nbytes, data)` or the queue corrupts. With no `.await` between WOULD_BLOCK and the
        // retry, no other tokio task on the current-thread runtime can sneak in a `try_send_bytes`
        // with different bytes — the spin is atomic from the scheduler's POV. Skipping the lock
        // also skips the contention that made multiplexed senders serialize through one mutex.
        // Cross-process deadlock breaking still works: `drain.try_drain_pass` pulls peer batches
        // out of our inbound every iteration, freeing peers' outbound slots so their sends
        // unstall and ours eventually fits.
        let mut first_try = true;
        let t_wait_start = Instant::now();
        loop {
            #[cfg(not(test))]
            pgrx::check_for_interrupts!();
            if self.channel.try_send_bytes(scratch)? {
                if !first_try {
                    stats.send_wait += t_wait_start.elapsed();
                }
                return Ok(());
            }
            first_try = false;
            stats.spin_iters += 1;
            let t_drain = Instant::now();
            drain.try_drain_pass()?;
            stats.coop_drain_in_spin += t_drain.elapsed();
        }
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
        // Why there's no yield_now in this spin (and no send_lock either):
        //
        // PG's `shm_mq_send` invariant: once a call returns SHM_MQ_WOULD_BLOCK with partial bytes
        // on the wire, the next call on the same handle has to repeat the *same* `(nbytes, data)`
        // or the queue corrupts. Two `MppSender` clones can share one underlying
        // `Arc<dyn BatchChannelSender>` (that's how `(stage_id, partition)` channels multiplex
        // onto one shm_mq), so a yield between the WOULD_BLOCK and the retry used to let another
        // task slip in a `try_send_bytes` with different bytes.
        //
        // The earlier fix gated the spin behind a per-handle `tokio::sync::Mutex`, which was
        // correct but serialized every multiplexed sender through one lock and tanked the
        // aggregate_join benchmarks by ~2×. This version keeps correctness by removing the only
        // `.await` inside the partial-send window. On the current-thread tokio runtime, with no
        // await between WOULD_BLOCK and retry, no other tokio task can interleave a
        // `try_send_bytes` — the spin is atomic from the scheduler's POV, so a lock is redundant.
        //
        // Cross-process deadlock breaking still works the same way. The two procs each spin on
        // full outbounds; each one's `drain.try_drain_pass` pulls peer batches out of its own
        // inbound queues, which frees the peer's outbound-to-us slots, which lets the peer's
        // sender advance, which eventually drains our outbound and lets us advance too. Peer
        // progress happens in a different OS process — it doesn't need us to yield to make it.
        //
        // Trade-off: while the spin is running, other tokio tasks on this thread (sibling
        // producers, the local consumer draining the `DrainBuffer`) don't get a turn. In
        // practice each spin completes within a few iterations once the peer drains its end, so
        // the latency hit is bounded and a lot smaller than the lock contention it replaces.
        let mut first_try = true;
        let t_wait_start = Instant::now();
        loop {
            // `pgrx::check_for_interrupts!()` pulls in PG symbols
            // (`ProcessInterrupts`, `PG_exception_stack`,
            // `CopyErrorData`, …) that aren't linked into the crate's
            // `--tests` / llvm-cov build. This fn is reached from
            // `#[cfg(test)]` code in this file and `shuffle.rs`, so
            // gate the check out of test builds; `InProc` channels
            // used in tests never block, so the send side of the loop
            // returns on the first iteration anyway.
            #[cfg(not(test))]
            pgrx::check_for_interrupts!();
            if self.channel.try_send_bytes(scratch)? {
                if !first_try {
                    stats.send_wait += t_wait_start.elapsed();
                }
                return Ok(());
            }
            first_try = false;
            stats.spin_iters += 1;
            // Would-block: pull from our own mesh's inbound so peers'
            // sends to us unblock. Without this interleave two
            // procs blocking on symmetric sends deadlock —
            // neither gets to drain. Errors propagate so a peer
            // detaching mid-spin doesn't leave the sender looping
            // forever on a closed mesh.
            let t_drain = Instant::now();
            drain.try_drain_pass()?;
            stats.coop_drain_in_spin += t_drain.elapsed();
        }
    }
}

/// Per-call timing + spin metrics for [`MppSender::send_batch_traced`].
/// All fields accumulate; callers zero or reuse as needed.
#[derive(Default, Debug, Clone)]
pub(crate) struct SendBatchStats {
    /// Cumulative time spent inside `encode_frame_into` (header + Arrow IPC serialization).
    pub(crate) encode: Duration,
    /// Cumulative wall time in the send-retry spin after the first failed
    /// `try_send_bytes`. Zero if the first try succeeded.
    pub(crate) send_wait: Duration,
    /// Cumulative time spent in `try_drain_pass` while spinning on a
    /// full outbound. A subset of `send_wait`; the remainder is the
    /// `tokio::task::yield_now()` await + the (small) cost of
    /// `try_send_bytes` itself.
    pub(crate) coop_drain_in_spin: Duration,
    /// Count of `try_send_bytes` calls that returned `Ok(false)` (full).
    pub(crate) spin_iters: u64,
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
                Ok((header, None)) => match header.kind() {
                    Ok(MppFrameKind::Request) => RecvBatchOutcome::Request { header },
                    Ok(MppFrameKind::Eof) => RecvBatchOutcome::Eof { header },
                    Ok(MppFrameKind::Batch) => RecvBatchOutcome::Error(DataFusionError::Internal(
                        "mpp: decode_frame returned Batch kind with no payload".into(),
                    )),
                    Err(e) => RecvBatchOutcome::Error(e),
                },
                Err(e) => RecvBatchOutcome::Error(e),
            },
            RecvOutcome::Empty => RecvBatchOutcome::Empty,
            RecvOutcome::Detached => RecvBatchOutcome::Detached,
        }
    }
}

/// Producer-side dispatcher for incoming [`MppFrameKind::Request`] frames. The cooperative drain
/// calls `on_request` whenever a peer asks for a partition; the implementation looks up the named
/// `(stage_id, task_idx)`, builds (or reuses) a task driver, and spawns a per-partition future
/// that streams batches back through `mesh.outbound_sender(sender_proc)`.
///
/// `sender_proc` is the proc that issued the Request, observed by the receiving drain (it's the
/// `sender_proc` the drain was constructed with). Implementations route the response over the
/// matching outbound queue.
pub trait RequestHandler: Send + Sync {
    fn on_request(
        &self,
        sender_proc: u32,
        stage_id: u32,
        task_idx: u32,
        partition: u32,
    ) -> Result<(), DataFusionError>;
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
    /// A payload-less `Request` frame asking the producer side to run
    /// `(header.stage_id, header.task_idx())` and stream partition
    /// `header.partition` back through the matching outbound queue. The
    /// cooperative drain dispatches to its installed [`RequestHandler`].
    Request {
        header: MppFrameHeader,
    },
    Empty,
    Detached,
    Error(DataFusionError),
}

/// Per-`(stage_id, partition)` channel buffer registry owned by a cooperative [`DrainHandle`]. The
/// handle serves one sender_proc, whose shm_mq queue carries frames for many logical channels,
/// each tagged by the [`MppFrameHeader`] prefix. `try_drain_pass` looks up the right channel buffer
/// on every frame and pushes the payload into it. Consumers waiting on
/// `(stage_id=s, partition=p)` only see frames matching that key.
///
/// Each entry is a `DrainBuffer::new(1)` because exactly one source (the sender_proc this handle
/// serves) emits frames for any given channel via this drain. When the sender_proc detaches
/// (`Detached` outcome on the underlying receiver), `detached` flips to `true` and every existing
/// channel buffer is notified, so any consumer blocked on `try_pop` unblocks with `DrainItem::Eof`.
/// Channel buffers registered *after* detach come back already EOF'd so a late consumer doesn't hang.
#[derive(Default)]
struct ChannelBufferRegistry {
    map: HashMap<(u32, u32), Arc<DrainBuffer>>,
    detached: bool,
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
    /// Which proc this drain pulls from. `Some(p)` in production (set during DSM attach so the
    /// dispatcher can route Request responses back to `p`'s outbound queue). `None` for in-proc
    /// test harnesses that don't care about the proc identity. A `Request` frame arriving on a
    /// drain with `sender_proc = None` is a hard error: the handler can't pick an outbound queue
    /// without it.
    sender_proc: Option<u32>,
    /// Installed by the producer's service loop via [`MppMesh::install_request_handler`]. The
    /// drain dispatches every [`RecvBatchOutcome::Request`] to this handler so the producer can
    /// spawn a task driver. Held behind `Mutex` (not `OnceLock`) so the loop can also clear it
    /// at teardown — needed to break the `MppMesh ↔ DrainHandle ↔ Arc<dyn RequestHandler> ↔
    /// Weak<MppMesh>` chain on shutdown.
    request_handler: Mutex<Option<Arc<dyn RequestHandler>>>,
}

impl DrainHandle {
    /// Construct a cooperative drain handle. Channel buffers are populated lazily by
    /// [`Self::try_drain_pass`] when a frame arrives, or up-front by [`Self::register_channel`]
    /// when a consumer needs a buffer to wait on before any frame has come in.
    ///
    /// `sender_proc` is the proc this drain pulls from in the production mesh, or `None` for
    /// in-proc test harnesses that don't care. It's the value the installed [`RequestHandler`]
    /// receives as `sender_proc` on every `on_request` callback, so the dispatcher knows which
    /// outbound queue to reply on.
    pub(super) fn cooperative(receivers: Vec<MppReceiver>, sender_proc: Option<u32>) -> Self {
        let wrapped = receivers.into_iter().map(Some).collect();
        Self {
            channel_buffers: Mutex::new(ChannelBufferRegistry::default()),
            coop_receivers: Mutex::new(wrapped),
            sender_proc,
            request_handler: Mutex::new(None),
        }
    }

    /// Install a request handler. Idempotent overwrite — installing twice replaces the previous
    /// handler. Called by [`MppMesh::install_request_handler`] for every inbound drain so the
    /// producer service loop's registry is reached by Request frames from any peer.
    pub fn set_request_handler(&self, handler: Arc<dyn RequestHandler>) {
        *self
            .request_handler
            .lock()
            .expect("DrainHandle request_handler mutex poisoned") = Some(handler);
    }

    /// Drop the installed request handler so the `Arc<dyn RequestHandler>` ref count releases.
    /// The service loop calls this at teardown to break the `MppMesh → DrainHandle → Registry`
    /// cycle (registry holds `Weak<MppMesh>`, but the drain holds a strong `Arc<Registry>`).
    pub fn clear_request_handler(&self) {
        *self
            .request_handler
            .lock()
            .expect("DrainHandle request_handler mutex poisoned") = None;
    }

    /// True once [`Self::try_drain_pass`] has observed `Detached` from its underlying receiver.
    /// Polled by the worker service loop to detect that the leader (or any monitored peer) has
    /// torn down and no more frames will arrive.
    pub fn is_detached(&self) -> bool {
        self.channel_buffers
            .lock()
            .expect("DrainHandle channel_buffers mutex poisoned")
            .detached
    }

    /// Drop the underlying receivers and mark every registered channel detached. Called by the
    /// leader through [`crate::postgres::customscan::mpp::runtime::MppMesh::detach_inbound_receivers`]
    /// when its plan finishes — once the inbound shm_mq handle drops, the peer-side sender's
    /// next `shm_mq_send` returns `SHM_MQ_DETACHED`, which surfaces as a `send_*_traced` error
    /// that the producer driver treats as "consumer torn down" and exits cleanly.
    pub fn force_detach(&self) {
        self.coop_receivers
            .lock()
            .expect("DrainHandle coop_receivers mutex poisoned")
            .clear();
        self.mark_detached();
        self.cancel_channel_buffers();
    }

    /// Register (or look up) the channel buffer for `(stage_id, partition)`. The returned
    /// `Arc<DrainBuffer>` is the canonical destination for frames matching that key:
    /// `try_drain_pass` pushes into the same entry on every `Batch { header, .. }` whose header
    /// matches.
    ///
    /// If the drain has already observed `Detached` from its underlying receiver, the
    /// newly-created buffer comes back with `notify_source_done` already called so a consumer
    /// registering after detach sees `Eof` on the first `try_pop` instead of hanging forever.
    pub(super) fn register_channel(&self, stage_id: u32, partition: u32) -> Arc<DrainBuffer> {
        let mut guard = self
            .channel_buffers
            .lock()
            .expect("DrainHandle channel_buffers mutex poisoned");
        let detached = guard.detached;
        guard
            .map
            .entry((stage_id, partition))
            .or_insert_with(|| {
                let buf = DrainBuffer::new(1);
                if detached {
                    buf.notify_source_done();
                }
                buf
            })
            .clone()
    }

    /// Mark the drain as detached and `notify_source_done` every registered channel buffer.
    /// Idempotent. Used by `try_drain_pass` after `Detached` / `Error` outcomes; the
    /// cooperative-path equivalent fires from `Drop` via [`Self::cancel_channel_buffers`] so any
    /// consumer blocked on `try_pop` unblocks with `Eof` even if the query is torn down before
    /// EOF frames flow.
    ///
    /// Collects buffer handles under the registry lock, then notifies after releasing it.
    /// Notifying inline would block any concurrent [`Self::register_channel`] for as long as it
    /// takes to acquire `DrainBuffer::inner` N times. Fine today (single backend thread), but
    /// cheap insurance against the multi-thread variant landing later.
    fn mark_detached(&self) {
        let to_notify = {
            let mut guard = self
                .channel_buffers
                .lock()
                .expect("DrainHandle channel_buffers mutex poisoned");
            if guard.detached {
                return;
            }
            guard.detached = true;
            guard.map.values().cloned().collect::<Vec<_>>()
        };
        for buf in to_notify {
            buf.notify_source_done();
        }
    }

    /// Cancel every registered channel buffer. Called from `Drop` to unblock any consumer waiting on
    /// a channel buffer when the handle goes away mid-query. Same collect-then-notify pattern as
    /// [`Self::mark_detached`].
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

        // Collect Request frames during the receiver-locked pass and dispatch them AFTER the
        // lock is released. The dispatcher's `on_request` may run expensive work
        // (`DistributedExec::prepare_in_process_plan` on a cache miss, several Arc clones, a
        // `tokio::spawn`) — holding the `coop_receivers` lock across it would stall any
        // concurrent `force_detach` and serialise drain progress on the slow-path of a single
        // Request. The two halves don't have to be atomic: Request dispatch only spawns a future
        // and updates registry-local state, neither of which depends on which Batch/Eof frames
        // got drained before or after.
        let mut pending_requests: Vec<MppFrameHeader> = Vec::new();
        {
            let mut slots = self.coop_receivers.lock().unwrap();
            for slot in slots.iter_mut() {
                let Some(rx) = slot.as_ref() else {
                    continue;
                };
                for _ in 0..MAX_BATCHES_PER_SOURCE_PER_PASS {
                    match rx.try_recv_batch() {
                        RecvBatchOutcome::Batch { header, batch } => {
                            let buf = self.register_channel(header.stage_id, header.partition);
                            buf.push_batch(batch);
                        }
                        RecvBatchOutcome::Eof { header } => {
                            let buf = self.register_channel(header.stage_id, header.partition);
                            buf.notify_source_done();
                            // Other channels may still flow on this queue, so the receiver slot
                            // stays live.
                        }
                        RecvBatchOutcome::Request { header } => {
                            pending_requests.push(header);
                        }
                        RecvBatchOutcome::Empty => break,
                        RecvBatchOutcome::Detached => {
                            *slot = None;
                            self.mark_detached();
                            break;
                        }
                        RecvBatchOutcome::Error(e) => {
                            *slot = None;
                            self.mark_detached();
                            return Err(e);
                        }
                    }
                }
            }
            // `slots` guard drops here, releasing `coop_receivers` before dispatch.
        }

        if !pending_requests.is_empty() {
            self.dispatch_requests(pending_requests)?;
        }
        Ok(())
    }

    /// Forward `Request` frames to the installed handler. Two important corners:
    ///
    /// 1. The handler may not be installed (teardown race: `clear_request_handler` ran while a
    ///    Request was already buffered on the shm_mq side). Drop the requests silently — the
    ///    next drain pass will see `Detached` and the consumer that issued these Requests has
    ///    already given up.
    /// 2. `sender_proc` must be set on production drains, because the handler needs to know
    ///    which outbound queue to respond on. A `None` here is a configuration bug — the
    ///    in-proc test drains build with `None`, but those never receive Request frames.
    fn dispatch_requests(&self, headers: Vec<MppFrameHeader>) -> Result<(), DataFusionError> {
        let handler = self
            .request_handler
            .lock()
            .expect("DrainHandle request_handler mutex poisoned")
            .as_ref()
            .map(Arc::clone);
        let Some(handler) = handler else {
            crate::mpp_log!(
                "mpp drain: {} Request frame(s) dropped, no handler installed (teardown race)",
                headers.len()
            );
            return Ok(());
        };
        let Some(sender_proc) = self.sender_proc else {
            return Err(DataFusionError::Internal(format!(
                "mpp: {} Request frame(s) received on drain with sender_proc=None; production \
                 drains must carry their peer proc index",
                headers.len()
            )));
        };
        for header in headers {
            handler.on_request(
                sender_proc,
                header.stage_id,
                header.task_idx(),
                header.partition,
            )?;
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
    (InProcSender { tx }, InProcReceiver { rx: Mutex::new(rx) })
}

pub(super) struct InProcSender {
    tx: std::sync::mpsc::SyncSender<Vec<u8>>,
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
        self.tx
            .send(bytes.to_vec())
            .map_err(|_| consumer_detached_error("in-proc channel send"))
    }

    fn try_send_bytes(&self, bytes: &[u8]) -> Result<bool, DataFusionError> {
        match self.tx.try_send(bytes.to_vec()) {
            Ok(()) => Ok(true),
            Err(std::sync::mpsc::TrySendError::Full(_)) => Ok(false),
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                Err(consumer_detached_error("in-proc channel try_send"))
            }
        }
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
            Self::with_header(channel, MppFrameHeader::batch(0, 0))
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

        /// Stats-less wrapper around `send_request_traced`. Same runtime-on-demand pattern as
        /// `send_batch`; covers the phase-1 wire-format tests until phase 2 wires real callers.
        fn send_request(&self, task_idx: u32) -> Result<(), DataFusionError> {
            let mut stats = SendBatchStats::default();
            let rt = tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("test tokio runtime build");
            rt.block_on(self.send_request_traced(task_idx, &mut stats))
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
                    RecvBatchOutcome::Request { header } => {
                        // Tests using `spawn_drain_thread` don't exercise the
                        // pull-shape request/response loop; treat a Request the
                        // same as a malformed frame so the failure mode surfaces
                        // explicitly instead of silently being lost.
                        done[i] = true;
                        buffer.notify_source_done();
                        return Err(DataFusionError::Internal(format!(
                            "mpp test drain_loop: unexpected Request frame \
                             (stage_id={}, task_idx={}, partition={})",
                            header.stage_id,
                            header.task_idx(),
                            header.partition,
                        )));
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
        let header = MppFrameHeader::batch(7, 3);
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
        encode_eof_frame_into(2, 5, &mut buf).expect("encode_eof");
        assert_eq!(buf.len(), MPP_FRAME_HEADER_SIZE);

        let (header, batch_opt) = decode_frame(&buf).expect("decode_frame");
        assert_eq!(header, MppFrameHeader::eof(2, 5));
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
        // Any bit above the low byte of `flags` is reserved-must-be-zero;
        // setting one should trip `kind()` before the kind byte is consulted.
        let header = MppFrameHeader {
            magic: MPP_FRAME_MAGIC,
            flags: 0x0000_0100, // bit 8 set, kind byte 0 (would be Batch)
            stage_id: 0,
            partition: 0,
        };
        let mut buf = vec![0u8; MPP_FRAME_HEADER_SIZE];
        header.write_to(&mut buf);
        let err = decode_frame(&buf).expect_err("reserved bit must fail");
        assert!(format!("{err}").contains("reserved frame flag bits"));
    }

    #[test]
    fn frame_eof_with_payload_is_rejected() {
        let mut buf = Vec::with_capacity(32);
        encode_eof_frame_into(0, 0, &mut buf).expect("encode_eof");
        buf.push(0xAB); // smuggle a payload byte after the Eof header
        let err = decode_frame(&buf).expect_err("Eof+payload must fail");
        assert!(format!("{err}").contains("Eof frame carries payload"));
    }

    #[test]
    fn frame_round_trips_request() {
        let mut buf = Vec::with_capacity(MPP_FRAME_HEADER_SIZE);
        encode_request_frame_into(11, 4, 9, &mut buf).expect("encode_request");
        assert_eq!(buf.len(), MPP_FRAME_HEADER_SIZE);

        let (header, batch_opt) = decode_frame(&buf).expect("decode_frame");
        assert_eq!(header.kind().unwrap(), MppFrameKind::Request);
        assert_eq!(header.stage_id, 11);
        assert_eq!(header.partition, 9);
        assert_eq!(header.task_idx(), 4);
        assert!(batch_opt.is_none());
    }

    #[test]
    fn frame_request_with_payload_is_rejected() {
        let mut buf = Vec::with_capacity(32);
        encode_request_frame_into(0, 0, 0, &mut buf).expect("encode_request");
        buf.push(0xCD); // smuggle a payload byte after the header
        let err = decode_frame(&buf).expect_err("Request+payload must fail");
        assert!(format!("{err}").contains("Request frame carries payload"));
    }

    #[test]
    fn frame_request_rejects_bad_magic() {
        // Reuses the encode helper, then clobbers the magic. Magic check sits before the kind
        // dispatch, so this catches both Request and any other kind that gets corrupted on the
        // wire.
        let mut buf = Vec::with_capacity(MPP_FRAME_HEADER_SIZE);
        encode_request_frame_into(1, 2, 3, &mut buf).expect("encode_request");
        buf[0..4].copy_from_slice(&0xDEADBEEF_u32.to_le_bytes());
        let err = decode_frame(&buf).expect_err("bad magic must fail");
        assert!(format!("{err}").contains("bad frame magic"));
    }

    #[test]
    fn request_frame_packs_max_task_idx() {
        // 24-bit max round-trips cleanly: the kind byte stays Request and `task_idx()` returns
        // the same value.
        let header = MppFrameHeader::request(0, TASK_IDX_MAX, 0).expect("max task_idx must fit");
        let mut buf = vec![0u8; MPP_FRAME_HEADER_SIZE];
        header.write_to(&mut buf);
        let (parsed, _) = decode_frame(&buf).expect("decode_frame");
        assert_eq!(parsed.kind().unwrap(), MppFrameKind::Request);
        assert_eq!(parsed.task_idx(), TASK_IDX_MAX);
    }

    #[test]
    fn request_frame_rejects_overflowing_task_idx() {
        // 25th bit set is one past the encodable range; the constructor must surface the
        // overflow rather than silently truncating into the kind byte.
        let err = MppFrameHeader::request(0, TASK_IDX_MAX + 1, 0)
            .expect_err("overflow must surface as error");
        assert!(format!("{err}").contains("exceeds 24-bit max"));
    }

    #[test]
    fn frame_rejects_request_with_zero_task_idx_still_decodes() {
        // Pin the "task_idx = 0" case so a future change that conflates "no upper bits" with
        // "wrong kind" surfaces in the test, not in production.
        let header = MppFrameHeader::request(5, 0, 7).expect("task_idx=0 must construct");
        assert_eq!(header.kind().unwrap(), MppFrameKind::Request);
        assert_eq!(header.task_idx(), 0);
    }

    #[test]
    fn codec_round_trips_many_batch_sizes() {
        let mut buf = Vec::with_capacity(1024);
        for rows in [0, 1, 7, 64, 1024] {
            let orig = sample_batch(rows);
            encode_frame_into(MppFrameHeader::batch(0, 0), &orig, &mut buf).expect("encode");
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
    fn send_request_round_trips_through_in_proc_channel() {
        // Drives `send_request_traced` through the real encode path, then decodes the bytes
        // back at the channel level. The receiver's `try_recv_batch` doesn't yet know about
        // Request frames (that lands with the phase-2 dispatcher), so we go straight through
        // `decode_frame` to keep this test scoped to the wire format.
        let (tx, rx) = in_proc_channel(4);
        let sender = MppSender::with_header(Arc::new(tx), MppFrameHeader::batch(13, 6));

        sender.send_request(2).expect("send_request");
        std::mem::drop(sender);

        let bytes = match rx.try_recv() {
            RecvOutcome::Bytes(b) => b,
            other => panic!("expected bytes, got {other:?}"),
        };
        let (header, batch_opt) = decode_frame(&bytes).expect("decode_frame");
        assert_eq!(header.kind().unwrap(), MppFrameKind::Request);
        assert_eq!(header.stage_id, 13);
        assert_eq!(header.partition, 6);
        assert_eq!(header.task_idx(), 2);
        assert!(batch_opt.is_none());
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
            encode_frame_into(MppFrameHeader::batch(0, 0), &template, &mut enc_buf)
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
            // After enough passes the in-proc backend reports `Detached`, which flips
            // `mark_detached` and notifies every channel buffer. We keep polling so any queued
            // frames flow through first.
        }
    }

    #[test]
    fn drain_handle_demuxes_frames_by_header() {
        // One queue carrying two channels: `(0, 0)` and `(0, 1)`. Each
        // channel buffer receives only its own batches.
        let (tx, rx) = in_proc_channel(8);
        let base = MppSender::new(Arc::new(tx));
        let s00 = base.clone_with_header(MppFrameHeader::batch(0, 0));
        let s01 = base.clone_with_header(MppFrameHeader::batch(0, 1));
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver], None);

        s00.send_batch(&sample_batch(2)).unwrap();
        s01.send_batch(&sample_batch(7)).unwrap();
        s00.send_batch(&sample_batch(3)).unwrap();
        drop(s00);
        drop(s01);
        drop(base); // Last sender dropped. Receiver will report Detached.

        let buf00 = handle.register_channel(0, 0);
        let buf01 = handle.register_channel(0, 1);

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
        assert!(matches!(buf00.try_pop(), Some(DrainItem::Eof)));
        assert!(matches!(buf01.try_pop(), Some(DrainItem::Eof)));
    }

    #[test]
    fn drain_handle_eof_frame_closes_one_channel() {
        // An `Eof` frame on `(0, 0)` closes that channel buffer while frames on
        // `(0, 1)` continue to flow on the same queue.
        let (tx, rx) = in_proc_channel(8);
        let tx_arc: Arc<dyn BatchChannelSender> = Arc::new(tx);
        let s00 = MppSender::with_header(Arc::clone(&tx_arc), MppFrameHeader::batch(0, 0));
        let s01 = MppSender::with_header(Arc::clone(&tx_arc), MppFrameHeader::batch(0, 1));
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver], None);

        s00.send_batch(&sample_batch(4)).unwrap();
        // Hand-roll an Eof frame for (0, 0) onto the same shared channel.
        let mut eof_buf = Vec::new();
        encode_eof_frame_into(0, 0, &mut eof_buf).unwrap();
        tx_arc.send_bytes(&eof_buf).unwrap();
        // (0, 1) is still live and ships another batch after the EOF.
        s01.send_batch(&sample_batch(6)).unwrap();

        let buf00 = handle.register_channel(0, 0);
        let buf01 = handle.register_channel(0, 1);

        // Drop senders so the receiver eventually observes Detached.
        drop(s00);
        drop(s01);
        drop(tx_arc);
        drain_until_detached(&handle);

        match buf00.try_pop() {
            Some(DrainItem::Batch(b)) => assert_eq!(b.num_rows(), 4),
            other => panic!("expected (0,0) batch, got {other:?}"),
        }
        // The Eof frame closed (0, 0) before the queue detached.
        assert!(matches!(buf00.try_pop(), Some(DrainItem::Eof)));

        match buf01.try_pop() {
            Some(DrainItem::Batch(b)) => assert_eq!(b.num_rows(), 6),
            other => panic!("expected (0,1) batch, got {other:?}"),
        }
        // (0, 1) sees EOF at receiver-detach time.
        assert!(matches!(buf01.try_pop(), Some(DrainItem::Eof)));
    }

    #[test]
    fn drain_handle_detach_eofs_all_registered_channel_buffers() {
        // No frames flow; consumer pre-registers two channels. When the sender drops and
        // `try_drain_pass` observes `Detached`, both channel buffers immediately surface `Eof`.
        // Without that, a consumer blocked on `try_pop` would hang past the producer's death.
        let (tx, rx) = in_proc_channel(8);
        drop(tx); // detach immediately
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver], None);

        let buf00 = handle.register_channel(0, 0);
        let buf01 = handle.register_channel(0, 1);
        // No frames ever land; the next poll observes Detached and notifies.
        handle.try_drain_pass().unwrap();

        assert!(matches!(buf00.try_pop(), Some(DrainItem::Eof)));
        assert!(matches!(buf01.try_pop(), Some(DrainItem::Eof)));
    }

    #[test]
    fn drain_handle_register_after_detach_returns_eof_buffer() {
        // Detach first, then register. The returned buffer is already
        // EOF'd so a late consumer doesn't block forever waiting on a
        // channel whose source is gone.
        let (tx, rx) = in_proc_channel(8);
        drop(tx);
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver], None);

        handle.try_drain_pass().unwrap(); // observes Detached, marks handle

        let late_buf = handle.register_channel(5, 9);
        assert!(matches!(late_buf.try_pop(), Some(DrainItem::Eof)));
    }

    #[test]
    fn drain_handle_register_channel_is_idempotent() {
        // Two calls for the same key return Arcs pointing to the same
        // DrainBuffer instance.
        let (_tx, rx) = in_proc_channel(8);
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver], None);

        let first = handle.register_channel(2, 3);
        let second = handle.register_channel(2, 3);
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn drain_handle_demuxes_frames_by_stage_id() {
        // Same partition (0) for two different stage ids on the same queue.
        // The registry's compound key keeps them on separate channel buffers.
        let (tx, rx) = in_proc_channel(8);
        let tx_arc: Arc<dyn BatchChannelSender> = Arc::new(tx);
        let s_stage0 = MppSender::with_header(Arc::clone(&tx_arc), MppFrameHeader::batch(0, 0));
        let s_stage1 = MppSender::with_header(Arc::clone(&tx_arc), MppFrameHeader::batch(1, 0));
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver], None);

        s_stage0.send_batch(&sample_batch(2)).unwrap();
        s_stage1.send_batch(&sample_batch(9)).unwrap();
        s_stage0.send_batch(&sample_batch(4)).unwrap();
        drop(s_stage0);
        drop(s_stage1);
        drop(tx_arc);

        let buf0 = handle.register_channel(0, 0);
        let buf1 = handle.register_channel(1, 0);

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
        let handle = DrainHandle::cooperative(vec![receiver], None);

        let buf_a = handle.register_channel(0, 0);
        let buf_b = handle.register_channel(7, 3);
        // No data ever flows; the handle is just dropped.
        drop(handle);

        assert!(matches!(buf_a.try_pop(), Some(DrainItem::Eof)));
        assert!(matches!(buf_b.try_pop(), Some(DrainItem::Eof)));
    }

    // ---------------------------------------------------------------------
    // Pull-shape Request→handler dispatch tests.
    //
    // The cooperative drain demuxes inbound frames by kind: `Batch`/`Eof` go to channel buffers,
    // `Request` goes to the installed `RequestHandler`. These tests exercise the handler-dispatch
    // half — the load-bearing piece of the pull-shape protocol — through a mock handler so the
    // production producer-service code path stays out of scope.
    // ---------------------------------------------------------------------

    /// `(sender_proc, stage_id, task_idx, partition)` captured by [`CapturingHandler`].
    type CapturedCall = (u32, u32, u32, u32);

    /// Mock handler that captures `(sender_proc, stage_id, task_idx, partition)` tuples into an
    /// std-mpsc channel. Tests assert on the captured calls.
    struct CapturingHandler {
        calls: std::sync::Mutex<std::sync::mpsc::Sender<CapturedCall>>,
        /// Optional Err to return on every call (defaults to Ok). Used by the error-propagation
        /// test to verify the drain surfaces handler errors instead of silently swallowing.
        force_err: std::sync::Mutex<Option<String>>,
    }

    impl CapturingHandler {
        fn new() -> (Arc<Self>, std::sync::mpsc::Receiver<CapturedCall>) {
            let (tx, rx) = std::sync::mpsc::channel();
            (
                Arc::new(Self {
                    calls: std::sync::Mutex::new(tx),
                    force_err: std::sync::Mutex::new(None),
                }),
                rx,
            )
        }

        fn set_force_err(&self, msg: &str) {
            *self.force_err.lock().unwrap() = Some(msg.into());
        }
    }

    impl RequestHandler for CapturingHandler {
        fn on_request(
            &self,
            sender_proc: u32,
            stage_id: u32,
            task_idx: u32,
            partition: u32,
        ) -> Result<(), DataFusionError> {
            self.calls
                .lock()
                .unwrap()
                .send((sender_proc, stage_id, task_idx, partition))
                .ok();
            if let Some(msg) = self.force_err.lock().unwrap().clone() {
                return Err(DataFusionError::Internal(msg));
            }
            Ok(())
        }
    }

    /// Push a Request frame straight into the wire bytes of an in-proc channel. Bypasses the
    /// MppSender encoder so the test stays scoped to the drain side.
    fn push_request_frame(
        tx: &dyn BatchChannelSender,
        stage_id: u32,
        task_idx: u32,
        partition: u32,
    ) {
        let header = MppFrameHeader::request(stage_id, task_idx, partition)
            .expect("test request frame must construct");
        let mut buf = vec![0u8; MPP_FRAME_HEADER_SIZE];
        header.write_to(&mut buf);
        tx.send_bytes(&buf)
            .expect("in-proc channel should accept the frame");
    }

    #[test]
    fn drain_handle_dispatches_single_request_to_handler() {
        let (tx, rx) = in_proc_channel(8);
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver], Some(7));
        let (handler, calls) = CapturingHandler::new();
        handle.set_request_handler(Arc::clone(&handler) as Arc<dyn RequestHandler>);

        push_request_frame(
            &tx, /*stage*/ 3, /*task*/ 5, /*partition*/ 11,
        );
        drop(tx);

        handle.try_drain_pass().expect("dispatch should succeed");

        let captured = calls
            .recv_timeout(Duration::from_millis(100))
            .expect("handler must have been invoked");
        // sender_proc comes from the drain's stored `sender_proc`, not the frame.
        assert_eq!(captured, (7, 3, 5, 11));
        // No additional calls.
        assert!(calls.try_recv().is_err());
    }

    #[test]
    fn drain_handle_dispatches_multiple_requests_in_send_order() {
        // Two Request frames pumped in one drain pass land at the handler in the same order.
        // Locks in the "collect-then-dispatch" ordering invariant — if a future refactor moves
        // Request handling back inline with the receiver pump, this test catches order
        // shuffling.
        let (tx, rx) = in_proc_channel(8);
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver], Some(2));
        let (handler, calls) = CapturingHandler::new();
        handle.set_request_handler(Arc::clone(&handler) as Arc<dyn RequestHandler>);

        push_request_frame(&tx, 1, 0, 0);
        push_request_frame(&tx, 1, 0, 1);
        push_request_frame(&tx, 1, 0, 2);
        drop(tx);

        handle.try_drain_pass().expect("dispatch should succeed");

        let first = calls.recv_timeout(Duration::from_millis(100)).unwrap();
        let second = calls.recv_timeout(Duration::from_millis(100)).unwrap();
        let third = calls.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(first, (2, 1, 0, 0));
        assert_eq!(second, (2, 1, 0, 1));
        assert_eq!(third, (2, 1, 0, 2));
    }

    #[test]
    fn drain_handle_drops_request_silently_when_no_handler_installed() {
        // Production case: the service loop calls `uninstall_request_handler` at teardown but a
        // Request frame is already in the shm_mq queue. The drain must drop the Request silently
        // (return `Ok`) rather than raising — raising here would abort the transaction after the
        // query completed successfully.
        let (tx, rx) = in_proc_channel(8);
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver], Some(1));
        // Deliberately do NOT install a handler.

        push_request_frame(&tx, 2, 3, 4);
        drop(tx);

        handle
            .try_drain_pass()
            .expect("Request with no handler must drop silently, not error");
    }

    #[test]
    fn drain_handle_errors_on_request_when_sender_proc_unknown() {
        // Production drains carry `sender_proc = Some(_)`. A `None` here is a configuration
        // bug: in-proc test drains use `None`, but those should never receive Request frames.
        // Make sure a Request slipping into such a drain surfaces loudly.
        let (tx, rx) = in_proc_channel(8);
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver], None);
        let (handler, _calls) = CapturingHandler::new();
        handle.set_request_handler(Arc::clone(&handler) as Arc<dyn RequestHandler>);

        push_request_frame(&tx, 0, 0, 0);
        drop(tx);

        let err = handle
            .try_drain_pass()
            .expect_err("Request on sender_proc=None drain must error");
        assert!(
            err.to_string().contains("sender_proc=None"),
            "expected sender_proc=None mention, got {err}"
        );
    }

    #[test]
    fn drain_handle_propagates_handler_error() {
        // If the handler's `on_request` returns `Err`, `try_drain_pass` must surface it. The
        // service loop checks the registry's `first_error` between drain passes, but a transport
        // error from the handler itself (e.g., dispatcher saw an unsupported plan shape) should
        // not be silently swallowed.
        let (tx, rx) = in_proc_channel(8);
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver], Some(1));
        let (handler, _calls) = CapturingHandler::new();
        handler.set_force_err("synthetic handler failure");
        handle.set_request_handler(Arc::clone(&handler) as Arc<dyn RequestHandler>);

        push_request_frame(&tx, 0, 0, 0);
        drop(tx);

        let err = handle
            .try_drain_pass()
            .expect_err("handler Err must propagate");
        assert!(
            err.to_string().contains("synthetic handler failure"),
            "expected synthetic message, got {err}"
        );
    }

    #[test]
    fn drain_handle_demuxes_request_among_batch_and_eof_frames() {
        // A drain pass that sees a mix of Batch, Eof, and Request frames must route each one
        // correctly: Batch → channel buffer, Eof → channel buffer source-done, Request →
        // handler. Locks in the "collect-then-dispatch" structure of `try_drain_pass`: Batch and
        // Eof frames land in their buffers regardless of where the Request sits in the queue.
        let (tx, rx) = in_proc_channel(16);
        let tx_arc: Arc<dyn BatchChannelSender> = Arc::new(tx);
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver], Some(4));
        let (handler, calls) = CapturingHandler::new();
        handle.set_request_handler(Arc::clone(&handler) as Arc<dyn RequestHandler>);

        // Interleave: Batch on (s=0,p=0), Request(s=1,t=2,p=3), Batch on (s=0,p=0), Eof(s=0,p=0).
        let s00 = MppSender::with_header(Arc::clone(&tx_arc), MppFrameHeader::batch(0, 0));
        s00.send_batch(&sample_batch(2)).unwrap();
        push_request_frame(tx_arc.as_ref(), 1, 2, 3);
        s00.send_batch(&sample_batch(3)).unwrap();
        let mut eof_buf = Vec::new();
        encode_eof_frame_into(0, 0, &mut eof_buf).unwrap();
        tx_arc.send_bytes(&eof_buf).unwrap();

        let buf = handle.register_channel(0, 0);

        drop(s00);
        drop(tx_arc);
        drain_until_detached(&handle);

        // Two Batches then EOF on (0, 0).
        match buf.try_pop() {
            Some(DrainItem::Batch(b)) => assert_eq!(b.num_rows(), 2),
            other => panic!("expected first batch, got {other:?}"),
        }
        match buf.try_pop() {
            Some(DrainItem::Batch(b)) => assert_eq!(b.num_rows(), 3),
            other => panic!("expected second batch, got {other:?}"),
        }
        assert!(matches!(buf.try_pop(), Some(DrainItem::Eof)));

        // Exactly one Request dispatched.
        let captured = calls.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(captured, (4, 1, 2, 3));
        assert!(calls.try_recv().is_err());
    }

    // ---------------------------------------------------------------------
    // Consumer-detach sentinel contract.
    //
    // The producer-side driver in `worker.rs` treats "consumer torn down mid-stream" as a clean
    // signal — returns `Ok(())` so the registry's `first_error` slot stays empty. The signal is
    // recognised via [`CONSUMER_DETACHED_SENTINEL`] embedded in the sender's error message.
    // These tests lock the contract for both transport backends (shm_mq's behavior is exercised
    // by the integration tests; here we cover the in-proc backend which is shared between unit
    // tests and worker self-loop routing).
    // ---------------------------------------------------------------------

    #[test]
    fn consumer_detached_helper_round_trips_the_sentinel() {
        let err = consumer_detached_error("unit test");
        assert!(
            err.to_string().contains(CONSUMER_DETACHED_SENTINEL),
            "consumer_detached_error must embed CONSUMER_DETACHED_SENTINEL; got {err}"
        );
    }

    #[test]
    fn in_proc_send_after_receiver_drop_returns_consumer_detached() {
        // The in-proc backend is the self-loop sender on every worker. If a consumer drops mid
        // -stream, the next try_send / send must produce the sentinel error so `run_partition
        // _driver`'s `is_consumer_detached` predicate returns `true` and the driver exits Ok.
        let (tx, rx) = in_proc_channel(4);
        drop(rx);

        let err = tx
            .try_send_bytes(b"frame after detach")
            .expect_err("try_send after receiver drop must error");
        assert!(
            err.to_string().contains(CONSUMER_DETACHED_SENTINEL),
            "try_send sentinel missing: {err}"
        );

        let err = tx
            .send_bytes(b"frame after detach")
            .expect_err("blocking send after receiver drop must error");
        assert!(
            err.to_string().contains(CONSUMER_DETACHED_SENTINEL),
            "send sentinel missing: {err}"
        );
    }
}

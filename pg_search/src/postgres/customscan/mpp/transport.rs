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
//! - [`MppFrameHeader`] is a fixed 16-byte prefix every wire message carries.
//!   It tags the payload with `(stage_id, partition)` so a single underlying
//!   queue can multiplex frames for many logical channels — the foundation
//!   the multi-stage natural-shape path needs.
//! - [`encode_frame_into`] / [`decode_frame`] serialize a `RecordBatch` with a
//!   header prefix via Arrow IPC. [`encode_batch`] / [`decode_batch`] are
//!   test-only header-less wrappers retained for codec round-trip tests.
//! - [`DrainBuffer`] is the local per-participant queue that the drain thread
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
use std::task::Waker;
#[cfg(test)]
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::ipc::reader::StreamReader;
use datafusion::arrow::ipc::writer::StreamWriter;
use datafusion::common::DataFusionError;

/// Magic bytes "MPPF" (MPP Frame) at the start of every wire message.
/// Lets receivers reject misrouted / corrupt frames before they hit Arrow IPC.
pub const MPP_FRAME_MAGIC: u32 = 0x4D505046;

/// Wire-format size of [`MppFrameHeader`] in bytes. Asserted at compile time
/// below via `const _: ()`.
pub const MPP_FRAME_HEADER_SIZE: usize = 16;

/// Kind of payload following [`MppFrameHeader`].
///
/// `Batch` is the common case — header is followed by an Arrow IPC stream
/// containing one `RecordBatch`. `Eof` carries no payload and signals the
/// receiver that the named `(stage_id, partition)` channel is finished, even
/// though the underlying shm_mq queue may still carry frames for other
/// channels.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MppFrameKind {
    Batch = 0,
    Eof = 1,
}

/// 16-byte prefix on every transport frame.
///
/// The fixed layout `[magic, flags, stage_id, partition]` (4×u32) is what
/// senders prepend before the Arrow IPC stream bytes and what receivers
/// parse before deciding which sub-buffer the payload belongs to.
///
/// The `flags` word currently encodes `MppFrameKind` in its low byte (mask
/// `0x0000_00FF`); the upper 24 bits are reserved-must-be-zero and are
/// validated at parse time so a future use can repurpose them without a
/// wire-format break.
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

    /// Build an `Eof` header for the given `(stage_id, partition)`. Carries no
    /// payload; receivers route it to the sub-buffer's source-done counter.
    /// Consumed by M2's per-channel EOF signalling; exercised in tests today.
    #[allow(dead_code)]
    pub fn eof(stage_id: u32, partition: u32) -> Self {
        Self {
            magic: MPP_FRAME_MAGIC,
            flags: MppFrameKind::Eof as u32,
            stage_id,
            partition,
        }
    }

    /// Read the kind out of `flags`. Returns an error if the kind byte is
    /// unknown or if any reserved upper bit is set, which catches wire-format
    /// drift early.
    pub fn kind(&self) -> Result<MppFrameKind, DataFusionError> {
        let reserved = self.flags & !FRAME_KIND_MASK;
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
            return Err(DataFusionError::Internal(format!(
                "mpp: frame too short for header ({} < {})",
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

/// Serialize one `RecordBatch` as a self-contained Arrow IPC Stream message,
/// header-less. Test-only allocating wrapper retained for codec round-trip
/// tests; production code paths go through [`encode_frame_into`] so the wire
/// format always carries an [`MppFrameHeader`].
#[cfg(test)]
pub fn encode_batch(batch: &RecordBatch) -> Result<Vec<u8>, DataFusionError> {
    let mut buf = Vec::with_capacity(1024);
    encode_batch_into(batch, &mut buf)?;
    Ok(buf)
}

/// Serialize `batch` into `buf`, clearing `buf` first so the caller's
/// already-allocated capacity is reused. Header-less; production senders call
/// [`encode_frame_into`] which inlines the same IPC stream after a header.
/// Retained as a public helper for codec round-trip tests and external
/// (header-less) consumers.
#[allow(dead_code)]
pub fn encode_batch_into(batch: &RecordBatch, buf: &mut Vec<u8>) -> Result<(), DataFusionError> {
    buf.clear();
    let mut writer = StreamWriter::try_new(&mut *buf, batch.schema_ref())?;
    writer.write(batch)?;
    writer.finish()?;
    Ok(())
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
pub fn encode_frame_into(
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
/// the sub-buffer's source-done counter without touching Arrow IPC.
/// Consumed by M2's per-channel EOF signalling; exercised in tests today.
#[allow(dead_code)]
pub fn encode_eof_frame_into(
    stage_id: u32,
    partition: u32,
    buf: &mut Vec<u8>,
) -> Result<(), DataFusionError> {
    buf.clear();
    buf.resize(MPP_FRAME_HEADER_SIZE, 0);
    MppFrameHeader::eof(stage_id, partition).write_to(&mut buf[..MPP_FRAME_HEADER_SIZE]);
    Ok(())
}

/// Inverse of [`encode_batch`]. Expects exactly one batch per message,
/// header-less. Test-only.
#[cfg(test)]
pub fn decode_batch(bytes: &[u8]) -> Result<RecordBatch, DataFusionError> {
    let mut reader = StreamReader::try_new(bytes, None)?;
    let batch = reader.next().ok_or_else(|| {
        DataFusionError::Execution("mpp: empty arrow-ipc stream in decode_batch".into())
    })??;
    Ok(batch)
}

/// Inverse of [`encode_frame_into`]. Parses the 16-byte header and, for
/// `Batch` frames, decodes the trailing Arrow IPC stream. `Eof` frames return
/// `(header, None)` — receivers branch on `header.kind()` to decide routing.
pub fn decode_frame(
    bytes: &[u8],
) -> Result<(MppFrameHeader, Option<RecordBatch>), DataFusionError> {
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

/// Local queue that sits between the drain thread and the DataFusion consumer.
///
/// Push side: the drain thread appends fully-deserialized batches as they arrive
/// from inbound shm_mqs. When a source queue detaches, [`DrainBuffer::notify_source_done`]
/// is called; once all sources are done AND the queue is empty, `pop_front` returns
/// [`DrainItem::Eof`].
///
/// Pop side: the consumer calls `pop_front` which blocks on the condvar until
/// either a batch is available or EOF has been reached.
#[derive(Debug)]
pub struct DrainBuffer {
    inner: Mutex<DrainBufferInner>,
    cond: Condvar,
}

#[derive(Debug)]
struct DrainBufferInner {
    queue: VecDeque<RecordBatch>,
    num_sources: u32,
    sources_done: u32,
    /// Consumer-side cancel flag. When set (e.g., query cancelled), the drain
    /// thread should stop pushing and unblock waiting consumers with EOF.
    cancelled: bool,
    /// Most recently registered async waker from a `poll_pop_front` caller.
    /// Woken on push / source-done / cancel so a stream running inside a
    /// DataFusion `poll_next` returns `Poll::Pending` without blocking the
    /// executor thread. `Option` so we don't allocate one if the buffer is
    /// only consumed synchronously via `pop_front`.
    waker: Option<Waker>,
}

/// Yielded by [`DrainBuffer::pop_front`].
#[derive(Debug)]
pub enum DrainItem {
    /// A batch produced by one of the inbound shm_mqs.
    Batch(RecordBatch),
    /// All source queues have detached and the local queue is drained.
    Eof,
}

impl DrainBuffer {
    /// Create a drain buffer expecting `num_sources` inbound queues. For a
    /// participant in an N-way mesh, `num_sources == N - 1` (all peers
    /// excluding self — the self-partition bypasses the buffer).
    pub fn new(num_sources: u32) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(DrainBufferInner {
                queue: VecDeque::new(),
                num_sources,
                sources_done: 0,
                cancelled: false,
                waker: None,
            }),
            cond: Condvar::new(),
        })
    }

    /// Push a freshly-received batch into the local queue.
    pub fn push_batch(&self, batch: RecordBatch) {
        let mut guard = self.inner.lock().expect("DrainBuffer mutex poisoned");
        guard.queue.push_back(batch);
        self.cond.notify_one();
        if let Some(w) = guard.waker.take() {
            w.wake();
        }
    }

    /// Mark one source queue as detached. Safe to call from the drain thread
    /// after observing `SHM_MQ_DETACHED` on a given inbound queue.
    pub fn notify_source_done(&self) {
        let mut guard = self.inner.lock().expect("DrainBuffer mutex poisoned");
        guard.sources_done = guard.sources_done.saturating_add(1);
        if guard.sources_done >= guard.num_sources {
            self.cond.notify_all();
            if let Some(w) = guard.waker.take() {
                w.wake();
            }
        }
    }

    /// Cancel all further pushes and wake all consumers with EOF.
    pub fn cancel(&self) {
        let mut guard = self.inner.lock().expect("DrainBuffer mutex poisoned");
        guard.cancelled = true;
        self.cond.notify_all();
        if let Some(w) = guard.waker.take() {
            w.wake();
        }
    }

    /// Block until a batch is available, EOF is reached, or the buffer is
    /// cancelled.
    ///
    /// INVARIANT: any already-buffered batch is returned *before* honoring
    /// either cancellation or all-sources-done. Reordering the queue pop ahead
    /// of the cancel/eof check would silently drop buffered data on an
    /// otherwise-clean shutdown; the
    /// `drain_buffer_drains_buffered_before_eof` test locks this in.
    #[cfg(test)]
    pub fn pop_front(&self) -> DrainItem {
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

    /// True if `cancel` has been called. The test-only `drain_loop` and its
    /// tests consult this; the cooperative production path watches the flag
    /// through `notify_source_done` fan-out instead.
    #[cfg(test)]
    pub fn is_cancelled(&self) -> bool {
        self.inner
            .lock()
            .expect("DrainBuffer mutex poisoned")
            .cancelled
    }

    /// Non-blocking, non-waker variant. Returns the
    /// front item or `DrainItem::Eof` if all sources have detached and
    /// the queue is drained; returns `None` only when more data may yet
    /// arrive. Cooperative consumers loop on `try_drain_pass` + `try_pop`,
    /// yielding to the executor between iterations.
    pub fn try_pop(&self) -> Option<DrainItem> {
        let mut guard = self.inner.lock().expect("DrainBuffer mutex poisoned");
        Self::try_pop_locked(&mut guard)
    }

    /// Shared body of [`try_pop`] and [`poll_pop_front`]. Returns
    /// `Some(Batch)` if the queue has data, `Some(Eof)` if all sources
    /// have detached or the buffer is cancelled, and `None` otherwise.
    /// Lets the two public entry points stay in lockstep on the
    /// "buffered data wins over cancellation/EOF" invariant locked in
    /// by `drain_buffer_drains_buffered_before_eof`.
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
pub enum RecvOutcome {
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
pub trait BatchChannelReceiver: Send {
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
pub trait BatchChannelSender: Send + Sync {
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), DataFusionError>;

    /// Non-blocking variant. Returns `Ok(true)` on success, `Ok(false)`
    /// when the channel is full (caller should retry), `Err` on detach /
    /// transport error. Default falls back to the blocking send — safe
    /// for in-proc channels used by tests where "full" doesn't arise.
    fn try_send_bytes(&self, bytes: &[u8]) -> Result<bool, DataFusionError> {
        self.send_bytes(bytes).map(|()| true)
    }
}

/// High-level sender: encodes a `RecordBatch` then pushes bytes through the
/// underlying channel.
///
/// With `cooperative_drain` set, `send_batch` breaks the symmetric-send
/// deadlock on a single-threaded tokio runtime by interleaving send-retries
/// with `DrainHandle::try_drain_pass` on the same mesh's inbound side.
/// Each participant's sender doing the same guarantees mutual progress:
/// our drain pulls peer-shipped rows out of our inbound queues, which
/// frees peers' outbound-to-us send space, which lets their sends un-stall.
pub struct MppSender {
    /// Underlying byte channel. Held behind `Arc` so multiple `MppSender`s
    /// can share one `shm_mq` queue while tagging frames with different
    /// `(stage_id, partition)` headers — the multiplexed path's natural
    /// pattern. Clone the Arc, build a new `MppSender` with a different
    /// header, both write into the same queue.
    channel: Arc<dyn BatchChannelSender>,
    cooperative_drain: Option<Arc<DrainHandle>>,
    /// Frame header prepended to every outgoing batch. Identifies the logical
    /// `(stage_id, partition)` channel the receiver demultiplexes on. For the
    /// current single-stage architecture this is `(stage_id=0, partition=p)`
    /// where `p` is the consumer-side partition this sender feeds. The header
    /// is per-sender for now so existing call sites don't have to thread
    /// `(stage_id, partition)` through every `send_batch_traced`; once
    /// multiplexed senders carry multiple `(stage_id, partition)` channels
    /// over a single shm_mq queue, the header moves to a per-call argument.
    header: MppFrameHeader,
    /// Scratch buffer reused across every `encode_frame_into` on this
    /// sender. Sized by the first batch; subsequent batches clear and
    /// re-fill without reallocating. Interior mutability lets the caller
    /// keep the `&self` signature (senders live inside `ShuffleWiring`
    /// behind a shared borrow during `process_batch`).
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
    /// Construct a sender that tags every outgoing batch with `header`.
    /// Production call sites clone one shared `Arc<dyn BatchChannelSender>`
    /// across N senders, each with a different `MppFrameHeader::batch(stage, p)`
    /// — the multiplexed pattern for fanning multiple partitions over one
    /// shm_mq queue.
    pub fn with_header(channel: Arc<dyn BatchChannelSender>, header: MppFrameHeader) -> Self {
        Self {
            channel,
            cooperative_drain: None,
            header,
            scratch: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Construct a sender with the default `(stage_id=0, partition=0)` header.
    /// Used by tests where the header carries no actionable routing info.
    #[cfg(test)]
    pub fn new(channel: Arc<dyn BatchChannelSender>) -> Self {
        Self::with_header(channel, MppFrameHeader::batch(0, 0))
    }

    /// Frame header this sender stamps onto every outgoing batch.
    /// Consumed by M2's per-channel sender pool when sender per-call
    /// re-tagging lands; today this getter is for diagnostics only.
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

    /// Test-only stats-less wrapper around [`Self::send_batch_traced`].
    /// Production call sites (`ShuffleStream::process_batch`) always pass
    /// a `SendBatchStats` so per-peer wall-time shows up in the EOF trace.
    /// Wraps the async send in a tiny current-thread Tokio runtime so test
    /// `#[test]` functions don't have to be `#[tokio::test]` and the
    /// existing OS-thread-spawning test harnesses don't have to plumb an
    /// async runtime themselves.
    #[cfg(test)]
    pub fn send_batch(&self, batch: &RecordBatch) -> Result<(), DataFusionError> {
        let mut stats = SendBatchStats::default();
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("test tokio runtime build");
        rt.block_on(self.send_batch_traced(batch, &mut stats))
    }

    /// `send_batch` variant that accumulates per-call timings and spin
    /// counts into `stats`. Callers that report these at EOF (e.g.,
    /// `ShuffleStream`) use this to diagnose where time goes when the
    /// outbound queue is full.
    ///
    /// Async because the cooperative-spin path needs to surrender the
    /// Tokio runtime back periodically — see the body comment.
    pub async fn send_batch_traced(
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
        let mut first_try = true;
        let t_wait_start = Instant::now();
        // Mental model: a current-thread Tokio runtime lives on the
        // backend thread (DataFusion needs one to drive `Stream`s).
        // This spin runs *inside* a Tokio task — specifically the body
        // of `ShuffleStream::poll_next`. The deadlock the cooperative
        // drain prevents is *cross-participant*, not same-runtime: two
        // peers each blocking on a full outbound and never reading the
        // other side. We break that by driving our own inbound on this
        // same OS thread via `try_drain_pass`, which pulls peer
        // batches that have already arrived and frees their slots so
        // peers' writers can advance.
        //
        // The `tokio::task::yield_now().await` between iterations
        // hands the runtime back to the executor each spin. Today's
        // MPP topology is linear (`ChainExec` polls inline, no
        // `RepartitionExec` / `CoalescePartitions` driver above the
        // shuffle), so there are no sibling Tokio tasks ready to run
        // and yielding is effectively a no-op cost. Once the planner
        // grows a parallel operator above us, those siblings start
        // making forward progress during this yield instead of
        // starving.
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
            // participants blocking on symmetric sends deadlock —
            // neither gets to drain. Errors propagate so a peer
            // detaching mid-spin doesn't leave the sender looping
            // forever on a closed mesh.
            let t_drain = Instant::now();
            drain.try_drain_pass()?;
            stats.coop_drain_in_spin += t_drain.elapsed();
            tokio::task::yield_now().await;
        }
    }
}

/// Per-call timing + spin metrics for [`MppSender::send_batch_traced`].
/// All fields accumulate; callers zero or reuse as needed.
#[derive(Default, Debug, Clone)]
pub struct SendBatchStats {
    /// Cumulative time spent inside `encode_batch` (Arrow IPC serialization).
    pub encode: Duration,
    /// Cumulative wall time in the send-retry spin after the first failed
    /// `try_send_bytes`. Zero if the first try succeeded.
    pub send_wait: Duration,
    /// Cumulative time spent in `try_drain_pass` while spinning on a
    /// full outbound. A subset of `send_wait`; the remainder is the
    /// `tokio::task::yield_now()` await + the (small) cost of
    /// `try_send_bytes` itself.
    pub coop_drain_in_spin: Duration,
    /// Count of `try_send_bytes` calls that returned `Ok(false)` (full).
    pub spin_iters: u64,
}

/// High-level receiver: pulls bytes via the underlying channel and decodes them
/// into `RecordBatch`. Used by the drain thread.
pub struct MppReceiver {
    channel: Box<dyn BatchChannelReceiver>,
}

impl MppReceiver {
    pub fn new(channel: Box<dyn BatchChannelReceiver>) -> Self {
        Self { channel }
    }

    pub fn try_recv_batch(&self) -> RecvBatchOutcome {
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

/// Decoded result of an [`MppReceiver::try_recv_batch`]. Carries the parsed
/// [`MppFrameHeader`] so the drain thread can route the payload to the right
/// `(stage_id, partition)` sub-buffer once multi-stage multiplexing lands.
/// Today's positional design ignores `header` because there is exactly one
/// channel per queue; M1.c starts consuming the field for routing.
#[derive(Debug)]
pub enum RecvBatchOutcome {
    Batch {
        // Consumed by M1.c's per-(stage_id, partition) demux.
        #[allow(dead_code)]
        header: MppFrameHeader,
        batch: RecordBatch,
    },
    /// A payload-less `Eof` frame for `header.(stage_id, partition)`. The
    /// underlying shm_mq queue is still attached; the sender is announcing
    /// that this logical channel is done. Used by the multiplexed design to
    /// per-channel-EOF without dropping the whole queue.
    Eof {
        // Consumed by M1.c's per-(stage_id, partition) demux.
        #[allow(dead_code)]
        header: MppFrameHeader,
    },
    Empty,
    Detached,
    Error(DataFusionError),
}

/// Configuration for [`spawn_drain_thread`].
///
/// Only used by the thread-backed drain path, which is test-only: pgrx panics
/// on any pg FFI call (including `shm_mq_receive`) from a non-backend thread,
/// so production uses [`DrainHandle::cooperative`] — see the notes on
/// `DrainHandle::spawn` for details.
#[cfg(test)]
pub struct DrainConfig {
    /// Receivers to drain. Ownership moves into the spawned thread.
    pub receivers: Vec<MppReceiver>,
    /// Destination buffer.
    pub buffer: Arc<DrainBuffer>,
    /// How long to sleep when every receiver is empty but some are still
    /// attached. Tuning: small values reduce end-of-batch latency but raise
    /// CPU; 1 ms is a safe default until we integrate with WaitLatch.
    pub idle_sleep: Duration,
}

#[cfg(test)]
impl DrainConfig {
    pub fn new(receivers: Vec<MppReceiver>, buffer: Arc<DrainBuffer>) -> Self {
        Self {
            receivers,
            buffer,
            idle_sleep: Duration::from_millis(1),
        }
    }
}

/// Spawn the dedicated drain thread.
///
/// Test-only: the thread round-robins through every receiver with non-blocking
/// `try_recv`, pushes decoded batches into `buffer`, and marks each source
/// done as soon as it observes a detach or decode error. When every source is
/// done, the thread exits.
#[cfg(test)]
pub fn spawn_drain_thread(config: DrainConfig) -> JoinHandle<Result<(), DataFusionError>> {
    thread::spawn(move || drain_loop(config))
}

/// Per-`(stage_id, partition)` sub-buffer registry owned by a cooperative
/// [`DrainHandle`]. The handle serves one sender_proc — that proc's shm_mq
/// queue carries frames for many logical channels, each tagged by the
/// [`MppFrameHeader`] prefix. `try_drain_pass` looks up the right sub-buffer
/// on every frame and pushes the payload into it, so consumers waiting on
/// `(stage_id=s, partition=p)` see only frames matching that key.
///
/// Each entry is a `DrainBuffer::new(1)` because exactly one source (the
/// sender_proc this handle serves) emits frames for any given channel via
/// this drain. When the sender_proc detaches (`Detached` outcome on the
/// underlying receiver) `detached` flips to `true` and every existing
/// sub-buffer is notified — any consumer blocked on `try_pop` unblocks with
/// `DrainItem::Eof`. Sub-buffers registered *after* detach come back
/// already EOF'd so a late consumer doesn't hang.
#[derive(Default)]
struct SubBufferRegistry {
    map: HashMap<(u32, u32), Arc<DrainBuffer>>,
    detached: bool,
}

/// RAII wrapper around a drain thread's `JoinHandle` and a
/// per-`(stage_id, partition)` sub-buffer registry.
///
/// On drop, the handle cancels every sub-buffer (unblocking any waiting
/// consumer) and joins the test-only thread if one is attached. This
/// guarantees the drain thread never outlives the query's DSM segment — if
/// `ExecEndCustomScan` panics after dropping the handle, the thread has
/// already been torn down and cannot touch the freed shm_mq memory.
///
/// Review finding: the prior implementation's `JoinHandle` was hanging off
/// free-form execution state, so an error path that skipped manual cleanup
/// left a zombie drain thread alive with dangling DSM pointers. Enforcing
/// cancel+join via Drop closes that window.
pub struct DrainHandle {
    /// Cooperative variant's per-(stage_id, partition) sub-buffer registry.
    /// Populated lazily on first frame for a channel, or up-front by callers
    /// (e.g. `WorkerConnection::stream_partition`) that need a buffer to
    /// wait on before any frame arrives.
    sub_buffers: Mutex<SubBufferRegistry>,
    /// Thread-backed (test-only) variant's single shared buffer. The
    /// `drain_loop` writes to it directly; tests read via `Arc::clone` of
    /// the same buffer they constructed in `DrainConfig`. The cooperative
    /// path keeps this `None` and routes everything through `sub_buffers`.
    legacy_buffer: Option<Arc<DrainBuffer>>,
    /// Background-thread variant: `Some(JoinHandle)`. In-proc tests still use
    /// this path — their `InProcReceiver` is an `std::sync::mpsc` wrapper, not
    /// a pg FFI call, so the drain thread is safe. Wrapped in `Mutex` so
    /// `shutdown(&self)` can take the handle without needing `&mut self` —
    /// this lets cooperative senders hold `Arc<DrainHandle>` shares.
    join: Mutex<Option<JoinHandle<Result<(), DataFusionError>>>>,
    /// Cooperative variant: the receivers are owned by the handle and polled
    /// inline from `DrainGatherStream::poll_next` via [`Self::try_drain_pass`].
    /// Production uses this variant because any pg FFI call
    /// (`shm_mq_receive` included) from a non-backend thread panics pgrx's
    /// `check_active_thread` guard. `None` when the handle was spawned
    /// instead of constructed cooperatively.
    ///
    /// `Send` bound on `MppReceiver` is preserved — the receivers move
    /// thread-once at construction then are only accessed from the backend
    /// thread; the `Mutex` is just for interior mutability, not
    /// cross-thread coordination.
    coop_receivers: Mutex<Option<Vec<Option<MppReceiver>>>>,
}

impl DrainHandle {
    /// Spawn a drain thread and wrap the join handle together with the buffer
    /// it drains into. Test-only: pgrx's `check_active_thread` panics on any
    /// pg FFI call (including `shm_mq_receive`) from a non-backend thread, so
    /// a real mesh receiver can never drain from a std::thread worker.
    /// Production paths must use [`Self::cooperative`] instead.
    #[cfg(test)]
    pub fn spawn(config: DrainConfig) -> Self {
        let buffer = Arc::clone(&config.buffer);
        let join = spawn_drain_thread(config);
        Self {
            sub_buffers: Mutex::new(SubBufferRegistry::default()),
            legacy_buffer: Some(buffer),
            join: Mutex::new(Some(join)),
            coop_receivers: Mutex::new(None),
        }
    }

    /// Construct a cooperative drain handle: the receivers are stashed in the
    /// handle and drained inline from `DrainGatherStream::poll_next` (see
    /// [`Self::try_drain_pass`]). No background thread. This is the correct
    /// variant for production pg backend workers — the drain work runs on
    /// the backend thread, so any pg FFI inside `shm_mq_receive` is safe.
    ///
    /// Sub-buffers are populated lazily by `try_drain_pass` when a frame
    /// arrives, or up-front by [`Self::register_channel`] when a consumer
    /// needs a buffer to wait on before any frame has come in.
    pub fn cooperative(receivers: Vec<MppReceiver>) -> Self {
        let wrapped = receivers.into_iter().map(Some).collect();
        Self {
            sub_buffers: Mutex::new(SubBufferRegistry::default()),
            legacy_buffer: None,
            join: Mutex::new(None),
            coop_receivers: Mutex::new(Some(wrapped)),
        }
    }

    /// Register (or look up) the sub-buffer for `(stage_id, partition)`. The
    /// returned `Arc<DrainBuffer>` is the canonical destination for frames
    /// matching that key: `try_drain_pass` pushes into the same entry on
    /// every `Batch { header, .. }` whose header matches.
    ///
    /// If the drain has already observed `Detached` from its underlying
    /// receiver, the newly-created buffer comes back with `notify_source_done`
    /// already called so a consumer registering after detach sees `Eof` on
    /// the first `try_pop` instead of hanging forever.
    pub fn register_channel(&self, stage_id: u32, partition: u32) -> Arc<DrainBuffer> {
        let mut guard = self
            .sub_buffers
            .lock()
            .expect("DrainHandle sub_buffers mutex poisoned");
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

    /// Mark the drain as detached and `notify_source_done` every registered
    /// sub-buffer. Idempotent. Used by `try_drain_pass` after `Detached` /
    /// `Error` outcomes; also called from `Drop` for the cooperative path so
    /// any consumer blocked on `try_pop` unblocks with `Eof` even if the
    /// query is torn down before EOF frames flow.
    fn mark_detached(&self) {
        let mut guard = self
            .sub_buffers
            .lock()
            .expect("DrainHandle sub_buffers mutex poisoned");
        if guard.detached {
            return;
        }
        guard.detached = true;
        for buf in guard.map.values() {
            buf.notify_source_done();
        }
    }

    /// Cancel every registered sub-buffer. Called from `Drop` to unblock any
    /// consumer waiting on a sub-buffer when the handle goes away mid-query.
    fn cancel_sub_buffers(&self) {
        let guard = self
            .sub_buffers
            .lock()
            .expect("DrainHandle sub_buffers mutex poisoned");
        for buf in guard.map.values() {
            buf.cancel();
        }
    }

    /// Pull batches from each live receiver and demux them into the
    /// per-`(stage_id, partition)` sub-buffer registry. Called from
    /// `DrainGatherStream::poll_next` and from `MppSender::send_batch`'s
    /// cooperative spin — drain work happens on the backend thread
    /// (pgrx-safe). No-op for thread-backed handles.
    ///
    /// Each pass drains *every available* batch from each receiver (up to
    /// a safety cap). Pulling only one batch per source per call means
    /// that under steady producer pressure the cooperative sender's
    /// spin-loop cannot keep up — we'd fall N:1 behind peers' sends and
    /// the mesh stalls once any queue fills. Draining until the receiver
    /// reports `Empty` bounds each pass by queue depth rather than by
    /// spin-loop iteration count.
    ///
    /// Returns `Ok(())` once every cooperative receiver has been pulled until
    /// `Empty` (or detached). A previous version returned a `bool` indicating
    /// whether any progress had been made; no caller used it (the
    /// cooperative-spin loop in [`MppSender::send_batch`] retries on its own
    /// `try_send_bytes` regardless of drain progress), so the return is now
    /// just `Result<()>` so transport errors propagate instead of being
    /// silently dropped at the call site.
    ///
    /// Routing rules per outcome:
    /// - `Batch { header, batch }`: look up (or lazily create) the
    ///   `(header.stage_id, header.partition)` sub-buffer and push `batch`.
    /// - `Eof { header }`: per-channel EOF. Resolve the sub-buffer and call
    ///   `notify_source_done`. Other channels on the same queue keep flowing,
    ///   so the receiver slot stays live.
    /// - `Detached` / `Error`: queue-wide shutdown. Notify every registered
    ///   sub-buffer, mark the handle detached, and drop the slot.
    pub fn try_drain_pass(&self) -> Result<(), DataFusionError> {
        // Bound per-source pulls per call. The upper limit exists to give
        // the caller a chance to re-try its own send between drains —
        // otherwise a participant with a very fast peer could drain
        // indefinitely on one source and starve its own outbound.
        const MAX_BATCHES_PER_SOURCE_PER_PASS: usize = 256;

        let mut guard = self.coop_receivers.lock().unwrap();
        let Some(slots) = guard.as_mut() else {
            // Thread-backed handle — caller should read from buffer directly.
            return Ok(());
        };
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
                        // Other channels may still flow on this queue, so
                        // the receiver slot stays live.
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
        Ok(())
    }

    fn join_inner(&self) -> Result<(), DataFusionError> {
        let join_opt = self.join.lock().unwrap().take();
        if let Some(join) = join_opt {
            match join.join() {
                Ok(res) => res,
                Err(panic) => Err(DataFusionError::Execution(format!(
                    "mpp: drain thread panicked: {panic:?}"
                ))),
            }
        } else {
            Ok(())
        }
    }
}

impl Drop for DrainHandle {
    fn drop(&mut self) {
        let has_join = self.join.lock().unwrap().is_some();
        if has_join {
            // Cancel and join even on the panic path. We swallow any error
            // here because Drop cannot fail; callers who care should use
            // `shutdown()` to observe the thread's result.
            if let Some(buf) = &self.legacy_buffer {
                buf.cancel();
            }
            let _ = self.join_inner();
        }
        // Cooperative path: unblock any consumer blocked on a sub-buffer
        // when the handle is torn down before EOF flows naturally (e.g. a
        // query error en route to ExecEndCustomScan).
        self.cancel_sub_buffers();
    }
}

#[cfg(test)]
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

/// SPSC channel pair for unit tests and in-process single-worker validation.
/// Bounded capacity via `std::sync::mpsc::sync_channel` so tests can exercise
/// backpressure (`send` blocks when full).
#[cfg(test)]
pub fn in_proc_channel(capacity: usize) -> (InProcSender, InProcReceiver) {
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(capacity);
    (InProcSender { tx }, InProcReceiver { rx: Mutex::new(rx) })
}

#[cfg(test)]
pub struct InProcSender {
    tx: std::sync::mpsc::SyncSender<Vec<u8>>,
}

#[cfg(test)]
pub struct InProcReceiver {
    // The std::sync::mpsc receiver is !Sync; wrap in a Mutex so the drain
    // thread can hold it behind a `Box<dyn BatchChannelReceiver>` (which is
    // `Send + Sync`-relaxed by design, but we only need Send for the thread
    // hand-off). Tests only ever access from one thread so the Mutex is
    // uncontended.
    rx: Mutex<std::sync::mpsc::Receiver<Vec<u8>>>,
}

#[cfg(test)]
impl BatchChannelSender for InProcSender {
    fn send_bytes(&self, bytes: &[u8]) -> Result<(), DataFusionError> {
        self.tx.send(bytes.to_vec()).map_err(|_| {
            DataFusionError::Execution("mpp: in-proc channel detached during send".into())
        })
    }
}

#[cfg(test)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::array::{Int32Array, Int64Array, StringArray, UInt64Array};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use std::sync::Arc as StdArc;
    use std::thread;

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
    fn codec_round_trips_a_batch() {
        let orig = sample_batch(128);
        let bytes = encode_batch(&orig).expect("encode");
        let decoded = decode_batch(&bytes).expect("decode");
        assert_eq!(orig.schema(), decoded.schema());
        assert_eq!(orig.num_rows(), decoded.num_rows());
        assert_eq!(orig.num_columns(), decoded.num_columns());
        // Equality across the whole batch
        for col in 0..orig.num_columns() {
            assert_eq!(orig.column(col).as_ref(), decoded.column(col).as_ref());
        }
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
        let batch = batch_opt.expect("Batch frame must carry a payload");
        assert_eq!(batch.num_rows(), 64);
        assert_eq!(batch.schema(), orig.schema());
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
        let mut bad = vec![0u8; MPP_FRAME_HEADER_SIZE];
        // Magic is the first 4 bytes; zeroing them is enough.
        let err = decode_frame(&bad).expect_err("bad magic must fail");
        assert!(format!("{err}").contains("bad frame magic"));
        // And a non-zero garbage prefix also fails.
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
    fn codec_round_trips_many_batch_sizes() {
        for rows in [0, 1, 7, 64, 1024] {
            let orig = sample_batch(rows);
            let bytes = encode_batch(&orig).expect("encode");
            let decoded = decode_batch(&bytes).expect("decode");
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
        let handle = DrainHandle::spawn(DrainConfig::new(vec![receiver], StdArc::clone(&buffer)));

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
        let handle = DrainHandle::spawn(DrainConfig::new(vec![receiver], StdArc::clone(&buffer)));

        // Simulate consumer path error: drop the handle without calling
        // shutdown(). The drain thread must exit before drop returns.
        let start = Instant::now();
        drop(handle);
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_secs(2),
            "DrainHandle::drop took too long: {elapsed:?}"
        );
        // Consumer observes EOF because cancel was called.
        assert!(matches!(buffer.pop_front(), DrainItem::Eof));
    }

    #[test]
    fn drain_thread_drains_n2_mesh_100k_batches() {
        // Simulates a 2-participant mesh under load. Each of two producers
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
    // These are `#[ignore]` by default because they spin for seconds and
    // spam stdout. Run with:
    //
    //   cargo test --package pg_search --release \
    //       postgres::customscan::mpp::transport::tests::throughput \
    //       -- --ignored --nocapture
    //
    // They help us bound the transport layer's cost independently of
    // DataFusion/Tantivy. All use the `in_proc_channel` backend (same
    // `MppSender`/`MppReceiver` trait boundary as the shm_mq one), so
    // numbers here are an optimistic ceiling — shm_mq adds the ring-buffer
    // copy + cross-process notification cost on top. If these numbers are
    // already below the row rate the real query needs, we know IPC encode
    // + channel handoff is the bottleneck without needing CI data.
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
        // Titles averaging ~30 bytes — typical for the docs dataset.
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
        for _ in 0..batches {
            enc_bytes += encode_batch(&template).expect("encode").len();
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
        // 1.25M total rows ≈ what one participant ships through gb_postagg at
        // 25M scale. 625K per participant × 2 = 1.25M.
        for batch_rows in [128, 512, 2048, 8192, 32_768] {
            bench_throughput("postagg", postagg_shape_batch, batch_rows, 1_250_000);
        }
    }

    #[test]
    #[ignore]
    fn throughput_probe_shape() {
        // 12.5M total rows ≈ what one participant ships through gb_right at 25M.
        for batch_rows in [128, 512, 2048, 8192, 32_768] {
            bench_throughput("probe", probe_shape_batch, batch_rows, 12_500_000);
        }
    }

    // ---------------------------------------------------------------------
    // M2.b — per-`(stage_id, partition)` sub-buffer registry on the
    // cooperative `DrainHandle`.
    //
    // Producers stamp `MppFrameHeader::batch(stage_id, partition)` on every
    // outgoing frame; the receiver-side cooperative drain demuxes by header
    // into a sub-buffer per `(stage_id, partition)`. These tests use the
    // `in_proc_channel` backend to drive `try_drain_pass` from the test
    // thread, mirroring how the production path runs the drain inline from
    // `DrainGatherStream::poll_next` on the backend thread.
    // ---------------------------------------------------------------------

    /// Drain a `DrainHandle::cooperative` to completion: poll until every
    /// receiver returns `Empty`. With the `in_proc_channel` test backend the
    /// drain observes `Detached` once the producer drops its sender, so a
    /// bounded loop of `try_drain_pass` calls is enough to flush everything
    /// the producer wrote.
    fn drain_until_detached(handle: &DrainHandle) {
        for _ in 0..64 {
            handle.try_drain_pass().expect("try_drain_pass");
            // After enough passes the in-proc backend reports `Detached`,
            // which flips `mark_detached` and notifies every sub-buffer. We
            // keep polling so any queued frames flow through first.
        }
    }

    #[test]
    fn drain_handle_demuxes_frames_by_header() {
        // One queue carrying two channels — `(0, 0)` and `(0, 1)`. Each
        // sub-buffer receives only its own batches.
        let (tx, rx) = in_proc_channel(8);
        let base = MppSender::new(Arc::new(tx));
        let s00 = base.clone_with_header(MppFrameHeader::batch(0, 0));
        let s01 = base.clone_with_header(MppFrameHeader::batch(0, 1));
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver]);

        s00.send_batch(&sample_batch(2)).unwrap();
        s01.send_batch(&sample_batch(7)).unwrap();
        s00.send_batch(&sample_batch(3)).unwrap();
        drop(s00);
        drop(s01);
        drop(base); // last sender dropped — receiver will report Detached.

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
        // An `Eof` frame on `(0, 0)` closes that sub-buffer while frames on
        // `(0, 1)` continue to flow on the same queue.
        let (tx, rx) = in_proc_channel(8);
        let tx_arc: Arc<dyn BatchChannelSender> = Arc::new(tx);
        let s00 = MppSender::with_header(Arc::clone(&tx_arc), MppFrameHeader::batch(0, 0));
        let s01 = MppSender::with_header(Arc::clone(&tx_arc), MppFrameHeader::batch(0, 1));
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver]);

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
    fn drain_handle_detach_eofs_all_registered_sub_buffers() {
        // No frames flow; consumer pre-registers two channels. When the
        // sender drops and `try_drain_pass` observes `Detached`, both
        // sub-buffers immediately surface `Eof` — without this, a consumer
        // blocked on `try_pop` would hang past the producer's death.
        let (tx, rx) = in_proc_channel(8);
        drop(tx); // detach immediately
        let receiver = MppReceiver::new(Box::new(rx));
        let handle = DrainHandle::cooperative(vec![receiver]);

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
        let handle = DrainHandle::cooperative(vec![receiver]);

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
        let handle = DrainHandle::cooperative(vec![receiver]);

        let first = handle.register_channel(2, 3);
        let second = handle.register_channel(2, 3);
        assert!(Arc::ptr_eq(&first, &second));
    }
}

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
//! - [`encode_batch`] / [`decode_batch`] serialize `RecordBatch` via Arrow IPC.
//! - [`DrainBuffer`] is the local per-participant queue that the drain thread
//!   writes into and the DataFusion consumer reads from. It decouples
//!   consumer-side backpressure from producer-side backpressure: the drain thread
//!   always makes forward progress on the inbound shm_mqs, so a stalled consumer
//!   cannot propagate backpressure to remote producers and cause the N×N cycle
//!   that deadlocked the prior attempt.
//!
//! The shm_mq-backed sender/receiver and drain thread spawn logic build on
//! top of these primitives.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
#[cfg(test)]
use std::time::Duration;

use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::ipc::reader::StreamReader;
use datafusion::arrow::ipc::writer::StreamWriter;
use datafusion::common::DataFusionError;

use crate::postgres::customscan::mpp::stage::MppTaskKey;

/// Four-byte magic prefix that marks an MPP-framed message. Receivers that
/// find this at the start of an incoming byte buffer strip the fixed-size
/// [`MppFrameHeader`] before handing the remainder to the Arrow IPC reader.
///
/// The magic exists so test paths and production paths can share
/// [`decode_batch`]: unit tests keep sending raw Arrow IPC (no header), the
/// bridges in `exec_bridge.rs` opt in via [`MppSender::with_frame_id`], and
/// the receiver auto-detects which flavor it has.
const FRAME_MAGIC: [u8; 4] = *b"MPPF";

/// On-wire header that prefixes every framed batch. 24 bytes, little-endian.
/// Mirrors the routing tuple that datafusion-distributed's `FlightAppMetadata`
/// protobuf carries, minus the transport-specific fields (URL / worker addr)
/// we don't need when every seat lives in the same DSM segment.
///
/// Fields:
/// - `query_id` (8 B) — `MppExecutionState::query_id()` at plan time.
/// - `stage_id` (4 B) — boundary's [`MppStage::stage_id`]; disambiguates
///   multiple cuts in the same plan.
/// - `task_number` (4 B) — the *sender's* participant index; tells the
///   receiver which peer produced this batch.
/// - `partition` (4 B) — the destination partition inside the stage's task.
///   Today `partition == dest_seat_index` 1:1, but that decoupling is what
///   P5b (channel flattening) needs to multiplex multiple logical streams
///   across one shm_mq per peer.
///
/// Not `#[repr(C)]`: we hand-encode little-endian via `to_le_bytes` to avoid
/// unaligned reads on architectures where `u64` needs 8-byte alignment — the
/// header may start at any offset within the shm_mq payload buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MppFrameHeader {
    pub query_id: u64,
    pub stage_id: u32,
    pub task_number: u32,
    pub partition: u32,
}

/// Size in bytes of [`MppFrameHeader`] on the wire, magic included.
pub const FRAME_HEADER_LEN: usize = 4 /* magic */ + 8 + 4 + 4 + 4;

impl MppFrameHeader {
    /// Append the framed header (magic + fields) to `buf`. Caller is expected
    /// to follow this with the Arrow IPC-encoded payload bytes.
    fn write_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&FRAME_MAGIC);
        buf.extend_from_slice(&self.query_id.to_le_bytes());
        buf.extend_from_slice(&self.stage_id.to_le_bytes());
        buf.extend_from_slice(&self.task_number.to_le_bytes());
        buf.extend_from_slice(&self.partition.to_le_bytes());
    }

    /// Parse a framed header from the start of `bytes`. Returns `Some(hdr)`
    /// on a valid magic match, `None` if the buffer is too short or the
    /// magic is missing (unframed legacy payload — pass through to Arrow IPC
    /// as-is).
    fn read_from(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < FRAME_HEADER_LEN || bytes[..4] != FRAME_MAGIC {
            return None;
        }
        let query_id = u64::from_le_bytes(bytes[4..12].try_into().ok()?);
        let stage_id = u32::from_le_bytes(bytes[12..16].try_into().ok()?);
        let task_number = u32::from_le_bytes(bytes[16..20].try_into().ok()?);
        let partition = u32::from_le_bytes(bytes[20..24].try_into().ok()?);
        Some(Self {
            query_id,
            stage_id,
            task_number,
            partition,
        })
    }
}

/// Routing tag stamped on every outgoing batch when a sender has opted in via
/// [`MppSender::with_frame_id`]. `task_key` locates the logical stream
/// `(query, stage, producing-task)`; `partition` addresses a lane within
/// that stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameId {
    pub task_key: MppTaskKey,
    pub partition: u32,
}

impl FrameId {
    fn to_header(self) -> MppFrameHeader {
        MppFrameHeader {
            query_id: self.task_key.query_id,
            stage_id: self.task_key.stage_id,
            task_number: self.task_key.task_number,
            partition: self.partition,
        }
    }
}

/// Serialize one `RecordBatch` as a self-contained Arrow IPC Stream message.
///
/// Test-only allocating wrapper for [`encode_batch_into`]; production hot paths
/// reuse a scratch `Vec` so the ~500 KB/batch allocator traffic the 25M GROUP BY
/// benchmark once spent 19 s on stays out of the critical loop.
#[cfg(test)]
pub fn encode_batch(batch: &RecordBatch) -> Result<Vec<u8>, DataFusionError> {
    let mut buf = Vec::with_capacity(1024);
    encode_batch_into(batch, &mut buf, None)?;
    Ok(buf)
}

/// Serialize `batch` into `buf`, clearing `buf` first so the caller's
/// already-allocated capacity is reused. Caller is expected to hold `buf`
/// alive across many encode calls (one per sender) so the peak-sized
/// allocation amortizes.
///
/// When `frame` is `Some`, the 24-byte [`MppFrameHeader`] is prepended before
/// the Arrow IPC bytes so the receiver can route without inspecting the
/// Arrow schema. Senders that haven't been stamped with a `FrameId` (tests,
/// in-proc smoke harnesses) pass `None` and write unframed Arrow IPC — the
/// receiver's `decode_batch` auto-detects either flavor.
pub fn encode_batch_into(
    batch: &RecordBatch,
    buf: &mut Vec<u8>,
    frame: Option<FrameId>,
) -> Result<(), DataFusionError> {
    buf.clear();
    if let Some(frame) = frame {
        frame.to_header().write_to(buf);
    }
    let mut writer = StreamWriter::try_new(&mut *buf, batch.schema_ref())?;
    writer.write(batch)?;
    writer.finish()?;
    Ok(())
}

/// Inverse of [`encode_batch`]. Expects exactly one batch per message.
///
/// Auto-detects the framed wire format: a leading `FRAME_MAGIC` triggers a
/// 24-byte strip before Arrow IPC decode. Unframed legacy payloads (the
/// in-proc test path) pass straight through. The parsed [`MppFrameHeader`]
/// is discarded today — P5b's channel multiplexer will extend this function
/// (or pair it with `decode_batch_with_frame`) to surface the header to the
/// receiver-side multiplexer.
pub fn decode_batch(bytes: &[u8]) -> Result<RecordBatch, DataFusionError> {
    let (_frame, payload) = peek_frame(bytes);
    let mut reader = StreamReader::try_new(payload, None)?;
    let batch = reader.next().ok_or_else(|| {
        DataFusionError::Execution("mpp: empty arrow-ipc stream in decode_batch".into())
    })??;
    Ok(batch)
}

/// Split an incoming byte buffer into `(frame_header, arrow_ipc_payload)`.
/// `None` header means the buffer is unframed legacy bytes; `payload` is then
/// the entire input. Used internally by [`decode_batch`] and available for
/// future per-channel multiplexers that need the routing tag.
fn peek_frame(bytes: &[u8]) -> (Option<MppFrameHeader>, &[u8]) {
    if let Some(hdr) = MppFrameHeader::read_from(bytes) {
        (Some(hdr), &bytes[FRAME_HEADER_LEN..])
    } else {
        (None, bytes)
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
    waker: Option<std::task::Waker>,
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

    /// Async-friendly variant of `pop_front`. Returns `DrainItem` when a
    /// batch or EOF is immediately available; otherwise registers `waker`
    /// to be woken on the next `push_batch` / `notify_source_done` / `cancel`
    /// and returns `None`.
    ///
    /// Using this from a `Stream::poll_next` lets `DrainGatherStream` return
    /// `Poll::Pending` instead of blocking the executor thread — critical
    /// under peer-to-peer backpressure, where a blocking wait could deadlock
    /// with this worker's own outbound pump.
    pub fn poll_pop_front(&self, waker: &std::task::Waker) -> Option<DrainItem> {
        let mut guard = self.inner.lock().expect("DrainBuffer mutex poisoned");
        if let Some(batch) = guard.queue.pop_front() {
            return Some(DrainItem::Batch(batch));
        }
        if guard.cancelled || guard.sources_done >= guard.num_sources {
            return Some(DrainItem::Eof);
        }
        // Register (or replace) the waker. Only one consumer at a time is
        // expected — DrainGatherStream — so simple replacement is fine.
        guard.waker = Some(waker.clone());
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
pub trait BatchChannelSender: Send {
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
/// with `DrainHandle::poll_drain_pass` on the same mesh's inbound side.
/// Each participant's sender doing the same guarantees mutual progress:
/// our drain pulls peer-shipped rows out of our inbound queues, which
/// frees peers' outbound-to-us send space, which lets their sends un-stall.
pub struct MppSender {
    channel: Box<dyn BatchChannelSender>,
    cooperative_drain: Option<Arc<DrainHandle>>,
    /// Routing header stamped on every outgoing batch. Set via
    /// [`Self::with_frame_id`] at bridge-construction time so peers know
    /// which `(query, stage, task, partition)` each batch belongs to once
    /// P5b multiplexes multiple logical streams over one shm_mq. `None`
    /// for test paths and pre-P5 callers: [`decode_batch`] auto-handles
    /// both flavors by sniffing the magic prefix.
    frame_id: Option<FrameId>,
    /// Scratch buffer reused across every `encode_batch_into` on this
    /// sender. Sized by the first batch; subsequent batches clear and
    /// re-fill without reallocating. Interior mutability lets the caller
    /// keep the `&self` signature (senders live inside `ShuffleWiring`
    /// behind a shared borrow during `process_batch`).
    scratch: std::cell::RefCell<Vec<u8>>,
}

impl MppSender {
    pub fn new(channel: Box<dyn BatchChannelSender>) -> Self {
        Self {
            channel,
            cooperative_drain: None,
            frame_id: None,
            scratch: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Attach a drain handle whose `poll_drain_pass` is called between
    /// send retries when the channel is full. Required on production
    /// shm_mq backends (see struct docs); tests without pressure can
    /// omit it.
    pub fn with_cooperative_drain(mut self, drain: Arc<DrainHandle>) -> Self {
        self.cooperative_drain = Some(drain);
        self
    }

    /// Stamp every outgoing batch with `task_key` + `partition`. Called by
    /// the bridges in `exec_bridge.rs` so the wire format carries enough
    /// routing information to multiplex multiple logical streams across one
    /// shm_mq (groundwork for P5b's N×(N−1) channel flattening). Today the
    /// receiver accepts framed bytes, discards the header, and returns the
    /// decoded batch; P5b will plug in a per-channel dispatcher that uses
    /// `(stage_id, task_number, partition)` to route batches to the right
    /// `DrainBuffer`.
    pub fn with_frame_id(mut self, task_key: MppTaskKey, partition: u32) -> Self {
        self.frame_id = Some(FrameId {
            task_key,
            partition,
        });
        self
    }

    /// Inspect the stamped routing tag. `None` until `with_frame_id` is
    /// called; the bridges always stamp in production. Test-only in the
    /// current tree — exposed so unit tests can assert the bridge plumbing
    /// wired the right tag to the right sender.
    #[cfg(test)]
    pub fn frame_id(&self) -> Option<FrameId> {
        self.frame_id
    }

    /// Test-only stats-less wrapper around [`Self::send_batch_traced`].
    /// Production call sites (`ShuffleStream::process_batch`) always pass a
    /// `SendBatchStats` so per-peer wall-time shows up in the EOF trace.
    #[cfg(test)]
    pub fn send_batch(&self, batch: &RecordBatch) -> Result<(), DataFusionError> {
        let mut stats = SendBatchStats::default();
        self.send_batch_traced(batch, &mut stats)
    }

    /// `send_batch` variant that accumulates per-call timings and spin counts
    /// into `stats`. Callers that report these at EOF (e.g., `ShuffleStream`)
    /// use this to diagnose where time goes when the outbound queue is full.
    pub fn send_batch_traced(
        &self,
        batch: &RecordBatch,
        stats: &mut SendBatchStats,
    ) -> Result<(), DataFusionError> {
        let mut scratch = self.scratch.borrow_mut();
        let t_enc = std::time::Instant::now();
        encode_batch_into(batch, &mut scratch, self.frame_id)?;
        stats.encode += t_enc.elapsed();
        let Some(drain) = self.cooperative_drain.as_ref() else {
            // No drain attached (unit tests, in-proc channels): fall back
            // to the blocking send path.
            return self.channel.send_bytes(&scratch);
        };
        let mut first_try = true;
        let t_wait_start = std::time::Instant::now();
        loop {
            // `pgrx::check_for_interrupts!()` pulls in PG symbols
            // (`ProcessInterrupts`, `PG_exception_stack`, `CopyErrorData`, …)
            // that aren't linked into the crate's `--tests` / llvm-cov build.
            // This fn is reached from `#[cfg(test)]` code in this file and
            // `shuffle.rs`, so gate the check out of test builds; `InProc`
            // channels used in tests never block, so the send side of the
            // loop returns on the first iteration anyway.
            #[cfg(not(test))]
            pgrx::check_for_interrupts!();
            if self.channel.try_send_bytes(&scratch)? {
                if !first_try {
                    stats.send_wait += t_wait_start.elapsed();
                }
                return Ok(());
            }
            first_try = false;
            stats.spin_iters += 1;
            // Would-block: pull from our own mesh's inbound so peers' sends
            // to us unblock. Without this interleave two participants
            // blocking on symmetric sends deadlock — neither gets to drain.
            let t_drain = std::time::Instant::now();
            let _ = drain.poll_drain_pass();
            stats.coop_drain_in_spin += t_drain.elapsed();
            std::thread::yield_now();
        }
    }
}

/// Per-call timing + spin metrics for [`MppSender::send_batch_traced`].
/// All fields accumulate; callers zero or reuse as needed.
#[derive(Default, Debug, Clone)]
pub struct SendBatchStats {
    /// Cumulative time spent inside `encode_batch` (Arrow IPC serialization).
    pub encode: std::time::Duration,
    /// Cumulative wall time in the send-retry spin after the first failed
    /// `try_send_bytes`. Zero if the first try succeeded.
    pub send_wait: std::time::Duration,
    /// Cumulative time spent in `poll_drain_pass` while spinning on a full
    /// outbound. A subset of `send_wait`; the remainder is `yield_now` +
    /// the (small) cost of `try_send_bytes` itself.
    pub coop_drain_in_spin: std::time::Duration,
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
            RecvOutcome::Bytes(bytes) => match decode_batch(&bytes) {
                Ok(batch) => RecvBatchOutcome::Batch(batch),
                Err(e) => RecvBatchOutcome::Error(e),
            },
            RecvOutcome::Empty => RecvBatchOutcome::Empty,
            RecvOutcome::Detached => RecvBatchOutcome::Detached,
        }
    }
}

/// Decoded result of an [`MppReceiver::try_recv_batch`].
#[derive(Debug)]
pub enum RecvBatchOutcome {
    Batch(RecordBatch),
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
    std::thread::spawn(move || drain_loop(config))
}

/// RAII wrapper around a drain thread's `JoinHandle` and its `DrainBuffer`.
///
/// On drop, the handle cancels the buffer (unblocking the drain thread if it
/// is sleeping on an empty cycle) and joins the thread. This guarantees the
/// drain thread never outlives the query's DSM segment — if `ExecEndCustomScan`
/// panics after dropping the handle, the thread has already been torn down
/// and cannot touch the freed shm_mq memory.
///
/// Review finding: the prior implementation's `JoinHandle` was hanging off
/// free-form execution state, so an error path that skipped manual cleanup
/// left a zombie drain thread alive with dangling DSM pointers. Enforcing
/// cancel+join via Drop closes that window.
pub struct DrainHandle {
    buffer: Arc<DrainBuffer>,
    /// Background-thread variant: `Some(JoinHandle)`. In-proc tests still use
    /// this path — their `InProcReceiver` is an `std::sync::mpsc` wrapper, not
    /// a pg FFI call, so the drain thread is safe. Wrapped in `Mutex` so
    /// `shutdown(&self)` can take the handle without needing `&mut self` —
    /// this lets cooperative senders hold `Arc<DrainHandle>` shares.
    join: Mutex<Option<JoinHandle<Result<(), DataFusionError>>>>,
    /// Cooperative variant: the receivers are owned by the handle and polled
    /// inline from `DrainGatherStream::poll_next` via [`Self::poll_drain_pass`].
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
            buffer,
            join: Mutex::new(Some(join)),
            coop_receivers: Mutex::new(None),
        }
    }

    /// Construct a cooperative drain handle: the receivers are stashed in the
    /// handle and drained inline from `DrainGatherStream::poll_next` (see
    /// [`Self::poll_drain_pass`]). No background thread. This is the correct
    /// variant for production pg backend workers — the drain work runs on
    /// the backend thread, so any pg FFI inside `shm_mq_receive` is safe.
    pub fn cooperative(receivers: Vec<MppReceiver>, buffer: Arc<DrainBuffer>) -> Self {
        let wrapped = receivers.into_iter().map(Some).collect();
        Self {
            buffer,
            join: Mutex::new(None),
            coop_receivers: Mutex::new(Some(wrapped)),
        }
    }

    /// Pull batches from each live receiver into the buffer. Called from
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
    /// Returns `Ok(true)` if anything changed (a batch pushed or a source
    /// marked done), `Ok(false)` if nothing changed in this pass.
    pub fn poll_drain_pass(&self) -> Result<bool, DataFusionError> {
        // Bound per-source pulls per call. The upper limit exists to give
        // the caller a chance to re-try its own send between drains —
        // otherwise a participant with a very fast peer could drain
        // indefinitely on one source and starve its own outbound.
        const MAX_BATCHES_PER_SOURCE_PER_PASS: usize = 256;

        let mut guard = self.coop_receivers.lock().unwrap();
        let Some(slots) = guard.as_mut() else {
            // Thread-backed handle — caller should read from buffer directly.
            return Ok(false);
        };
        let mut progress = false;
        for slot in slots.iter_mut() {
            let Some(rx) = slot.as_ref() else {
                continue;
            };
            for _ in 0..MAX_BATCHES_PER_SOURCE_PER_PASS {
                match rx.try_recv_batch() {
                    RecvBatchOutcome::Batch(b) => {
                        self.buffer.push_batch(b);
                        progress = true;
                    }
                    RecvBatchOutcome::Empty => break,
                    RecvBatchOutcome::Detached => {
                        *slot = None;
                        self.buffer.notify_source_done();
                        progress = true;
                        break;
                    }
                    RecvBatchOutcome::Error(e) => {
                        *slot = None;
                        self.buffer.notify_source_done();
                        return Err(e);
                    }
                }
            }
        }
        Ok(progress)
    }

    /// True if this handle drains cooperatively (no background thread).
    pub fn is_cooperative(&self) -> bool {
        self.coop_receivers.lock().unwrap().is_some()
    }

    /// Access the shared buffer so consumers (typically `GatherExec`) can
    /// `pop_front` without holding the handle itself.
    pub fn buffer(&self) -> &Arc<DrainBuffer> {
        &self.buffer
    }

    /// Cancel the drain explicitly and join the thread (thread-backed only).
    /// For cooperative handles, simply drops the receivers (signalling detach
    /// to peer senders) and returns. Called by the consumer at natural
    /// shutdown; the Drop impl handles panic paths.
    pub fn shutdown(&self) -> Result<(), DataFusionError> {
        self.buffer.cancel();
        // Drop receivers (if any) so peer shm_mq senders observe detach.
        let _ = self.coop_receivers.lock().unwrap().take();
        self.join_inner()
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
            self.buffer.cancel();
            let _ = self.join_inner();
        }
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
                RecvBatchOutcome::Batch(batch) => {
                    got_any = true;
                    buffer.push_batch(batch);
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
            std::thread::sleep(idle_sleep);
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
    use datafusion::arrow::array::{Int32Array, StringArray};
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
    fn codec_round_trips_many_batch_sizes() {
        for rows in [0, 1, 7, 64, 1024] {
            let orig = sample_batch(rows);
            let bytes = encode_batch(&orig).expect("encode");
            let decoded = decode_batch(&bytes).expect("decode");
            assert_eq!(orig.num_rows(), decoded.num_rows());
        }
    }

    #[test]
    fn framed_round_trip_preserves_payload_and_strips_header() {
        // Explicit frame round-trip: encode with a stamped header, then decode
        // through the auto-detecting `decode_batch`. The magic prefix must
        // be recognized and stripped before the Arrow IPC reader sees the
        // bytes — otherwise StreamReader would either error or misread
        // whatever happened to follow the magic.
        let orig = sample_batch(17);
        let frame = FrameId {
            task_key: MppTaskKey {
                query_id: 0xdead_beef_cafe_babe,
                stage_id: 2,
                task_number: 1,
            },
            partition: 3,
        };
        let mut buf = Vec::new();
        encode_batch_into(&orig, &mut buf, Some(frame)).expect("encode framed");

        // Wire invariant: magic at offset 0, then 20 bytes of fields, then
        // the arrow stream begins. If any of these break, the field layout
        // must be bumped and peers must agree on the new shape.
        assert_eq!(&buf[..4], b"MPPF");
        let hdr = MppFrameHeader::read_from(&buf).expect("header readable");
        assert_eq!(hdr.query_id, 0xdead_beef_cafe_babe);
        assert_eq!(hdr.stage_id, 2);
        assert_eq!(hdr.task_number, 1);
        assert_eq!(hdr.partition, 3);

        let decoded = decode_batch(&buf).expect("decode");
        assert_eq!(orig.num_rows(), decoded.num_rows());
        assert_eq!(orig.schema(), decoded.schema());
    }

    #[test]
    fn unframed_bytes_still_decode() {
        // Regression-proof: raw Arrow IPC (no header) decodes via the same
        // public entry point. Every existing #[cfg(test)] caller relies on
        // this — they never opt into `with_frame_id`.
        let orig = sample_batch(5);
        let bytes = encode_batch(&orig).expect("encode unframed");
        assert_ne!(&bytes[..4], b"MPPF");
        let decoded = decode_batch(&bytes).expect("decode");
        assert_eq!(orig.num_rows(), decoded.num_rows());
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
            thread::sleep(std::time::Duration::from_millis(20));
            producer.push_batch(sample_batch(2));
            producer.notify_source_done();
            thread::sleep(std::time::Duration::from_millis(20));
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
            thread::sleep(std::time::Duration::from_millis(20));
            canceller.cancel();
        });
        assert!(matches!(buf.pop_front(), DrainItem::Eof));
        handle.join().unwrap();
    }

    #[test]
    fn in_proc_channel_round_trips_through_mpp_sender_receiver() {
        let (tx, rx) = in_proc_channel(8);
        let sender = MppSender::new(Box::new(tx));
        let receiver = MppReceiver::new(Box::new(rx));

        sender.send_batch(&sample_batch(4)).unwrap();
        std::mem::drop(sender);

        match receiver.try_recv_batch() {
            RecvBatchOutcome::Batch(b) => assert_eq!(b.num_rows(), 4),
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
        let sender = MppSender::new(Box::new(tx));
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
        let sender = MppSender::new(Box::new(tx));
        let receiver = MppReceiver::new(Box::new(rx));
        let buffer = DrainBuffer::new(1);
        let handle = DrainHandle::spawn(DrainConfig::new(vec![receiver], StdArc::clone(&buffer)));

        sender.send_batch(&sample_batch(2)).unwrap();
        std::mem::drop(sender); // detach
                                // Pop the one batch
        assert!(matches!(buffer.pop_front(), DrainItem::Batch(_)));
        assert!(matches!(buffer.pop_front(), DrainItem::Eof));
        handle.shutdown().unwrap();
    }

    #[test]
    fn drain_handle_drop_cancels_and_joins() {
        // Build a drain that never detaches (we keep the sender alive), then
        // drop the handle. The Drop impl must cancel the buffer and join the
        // thread without hanging.
        let (tx, rx) = in_proc_channel(4);
        let _sender_kept_alive = MppSender::new(Box::new(tx));
        let receiver = MppReceiver::new(Box::new(rx));
        let buffer = DrainBuffer::new(1);
        let handle = DrainHandle::spawn(DrainConfig::new(vec![receiver], StdArc::clone(&buffer)));

        // Simulate consumer path error: drop the handle without calling
        // shutdown(). The drain thread must exit before drop returns.
        let start = std::time::Instant::now();
        drop(handle);
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "DrainHandle::drop took too long: {elapsed:?}"
        );
        // Consumer observes EOF because cancel was called.
        assert!(matches!(buffer.pop_front(), DrainItem::Eof));
    }

    #[test]
    fn drain_thread_drains_n2_mesh_100k_batches() {
        // Milestone-1 gate: simulate the 2-participant mesh. Each of two
        // producers pushes 50_000 small batches through a bounded channel;
        // the drain thread interleaves and the consumer reads EOF exactly
        // after receiving all 100_000 batches. Exercises backpressure
        // (bounded capacity = 16) without deadlock.
        const PER_SOURCE: usize = 50_000;
        let (tx0, rx0) = in_proc_channel(16);
        let (tx1, rx1) = in_proc_channel(16);
        let receivers = vec![
            MppReceiver::new(Box::new(rx0)),
            MppReceiver::new(Box::new(rx1)),
        ];
        let buffer = DrainBuffer::new(2);
        let drain_join = spawn_drain_thread(DrainConfig::new(receivers, StdArc::clone(&buffer)));

        let tx0_send = MppSender::new(Box::new(tx0));
        let tx1_send = MppSender::new(Box::new(tx1));
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
        use datafusion::arrow::array::{Int64Array, UInt64Array};
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
        use datafusion::arrow::array::{Int64Array, UInt64Array};
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
        let enc_start = std::time::Instant::now();
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
        let tx0_send = MppSender::new(Box::new(tx0));
        let tx1_send = MppSender::new(Box::new(tx1));

        let per_source = batches / 2;
        let round_trip_start = std::time::Instant::now();
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
        // 1.25M total rows ≈ what one seat ships through gb_postagg at
        // 25M scale. 625K per seat × 2 = 1.25M.
        for batch_rows in [128, 512, 2048, 8192, 32_768] {
            bench_throughput("postagg", postagg_shape_batch, batch_rows, 1_250_000);
        }
    }

    #[test]
    #[ignore]
    fn throughput_probe_shape() {
        // 12.5M total rows ≈ what one seat ships through gb_right at 25M.
        for batch_rows in [128, 512, 2048, 8192, 32_768] {
            bench_throughput("probe", probe_shape_batch, batch_rows, 12_500_000);
        }
    }
}

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

#![allow(dead_code)]
//! Transport layer for MPP shuffle.
//!
//! Layout:
//! - [`encode_batch`] / [`decode_batch`] serialize `RecordBatch` via Arrow IPC.
//! - [`DrainBuffer`] is the local per-participant queue that the drain thread
//!   writes into and the DataFusion consumer reads from. It decouples
//!   consumer-side backpressure from producer-side backpressure: the drain thread
//!   always makes forward progress on the inbound shm_mqs, so a stalled consumer
//!   cannot propagate backpressure to remote producers and cause an N×N
//!   peer-stall cycle.
//!
//! The shm_mq-backed sender/receiver and drain thread spawn logic build on
//! top of these primitives.

use std::collections::VecDeque;
use std::future::poll_fn;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::task::{Poll, Waker};
#[cfg(test)]
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::ipc::reader::StreamReader;
use datafusion::arrow::ipc::writer::StreamWriter;
use datafusion::common::DataFusionError;

/// Serialize one `RecordBatch` as a self-contained Arrow IPC Stream message.
///
/// Test-only allocating wrapper for [`encode_batch_into`]; production hot paths
/// reuse a scratch `Vec` so the ~500 KB/batch allocator traffic the 25M GROUP BY
/// benchmark once spent 19 s on stays out of the critical loop.
#[cfg(test)]
pub fn encode_batch(batch: &RecordBatch) -> Result<Vec<u8>, DataFusionError> {
    let mut buf = Vec::with_capacity(1024);
    encode_batch_into(batch, &mut buf)?;
    Ok(buf)
}

/// Serialize `batch` into `buf`, clearing `buf` first so the caller's
/// already-allocated capacity is reused. Caller is expected to hold `buf`
/// alive across many encode calls (one per sender) so the peak-sized
/// allocation amortizes.
pub fn encode_batch_into(batch: &RecordBatch, buf: &mut Vec<u8>) -> Result<(), DataFusionError> {
    buf.clear();
    let mut writer = StreamWriter::try_new(&mut *buf, batch.schema_ref())?;
    writer.write(batch)?;
    writer.finish()?;
    Ok(())
}

/// Inverse of [`encode_batch`]. Expects exactly one batch per message.
pub fn decode_batch(bytes: &[u8]) -> Result<RecordBatch, DataFusionError> {
    let mut reader = StreamReader::try_new(bytes, None)?;
    let batch = reader.next().ok_or_else(|| {
        DataFusionError::Execution("mpp: empty arrow-ipc stream in decode_batch".into())
    })??;
    Ok(batch)
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

    /// Async-friendly variant of `pop_front`. Returns immediately with a
    /// batch or EOF if available; otherwise registers `waker` (to be woken
    /// on the next `push_batch` / `notify_source_done` / `cancel`) and
    /// returns `None`. Lets `DrainGatherStream::poll_next` return
    /// `Poll::Pending` instead of blocking the executor thread.
    pub fn poll_pop_front(&self, waker: &Waker) -> Option<DrainItem> {
        let mut guard = self.inner.lock().expect("DrainBuffer mutex poisoned");
        if let Some(item) = Self::try_pop_locked(&mut guard) {
            return Some(item);
        }
        // Single-consumer invariant; switch to `Vec<Waker>` if ever lifted.
        debug_assert!(
            guard.waker.is_none() || guard.waker.as_ref().unwrap().will_wake(waker),
            "DrainBuffer::poll_pop_front: second consumer registered a different waker — \
             only one consumer is supported per buffer"
        );
        guard.waker = Some(waker.clone());
        None
    }

    /// Await-able wrapper over [`poll_pop_front`]. Use only for thread-backed
    /// drains; cooperative handles must use [`try_pop`](Self::try_pop) +
    /// executor yield (otherwise the await suspends with no one to wake it).
    pub async fn recv(self: &Arc<Self>) -> DrainItem {
        let buf = Arc::clone(self);
        poll_fn(move |cx| match buf.poll_pop_front(cx.waker()) {
            Some(item) => Poll::Ready(item),
            None => Poll::Pending,
        })
        .await
    }

    /// Non-blocking, non-waker variant of [`poll_pop_front`]. Returns the
    /// front item or `DrainItem::Eof` if all sources have detached and
    /// the queue is drained; returns `None` only when more data may yet
    /// arrive. Cooperative consumers loop on `poll_drain_pass` + `try_pop`,
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
    /// Scratch buffer reused across every `encode_batch_into` on this
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
    pub fn new(channel: Box<dyn BatchChannelSender>) -> Self {
        Self {
            channel,
            cooperative_drain: None,
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
        encode_batch_into(batch, scratch)?;
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
        // same OS thread via `poll_drain_pass`, which pulls peer
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
            drain.poll_drain_pass()?;
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
    /// Cumulative time spent in `poll_drain_pass` while spinning on a
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
    thread::spawn(move || drain_loop(config))
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
    /// Returns `Ok(())` once every cooperative receiver has been pulled until
    /// `Empty` (or detached). A previous version returned a `bool` indicating
    /// whether any progress had been made; no caller used it (the
    /// cooperative-spin loop in [`MppSender::send_batch`] retries on its own
    /// `try_send_bytes` regardless of drain progress), so the return is now
    /// just `Result<()>` so transport errors propagate instead of being
    /// silently dropped at the call site.
    pub fn poll_drain_pass(&self) -> Result<(), DataFusionError> {
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
                    RecvBatchOutcome::Batch(b) => {
                        self.buffer.push_batch(b);
                    }
                    RecvBatchOutcome::Empty => break,
                    RecvBatchOutcome::Detached => {
                        *slot = None;
                        self.buffer.notify_source_done();
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
        Ok(())
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
        let tx0_send = MppSender::new(Box::new(tx0));
        let tx1_send = MppSender::new(Box::new(tx1));

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
}

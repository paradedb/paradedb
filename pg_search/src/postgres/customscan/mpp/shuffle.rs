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
//! Shuffle operator primitives for MPP.
//!
//! [`MppShuffleExec`] declares `Partitioning::Hash(keys, N)` natively (or
//! `UnknownPartitioning(1)` for the scalar-final gather) and, on the
//! participant's `drive_partition`, merges its producer-side stream (drives
//! upstream, hash-routes via [`RowPartitioner`], ships peer rows through
//! [`MppSender`]) with its consumer-side stream (drains peer-shipped rows
//! from `shm_mq`). Other partitions return empty.
//!
//! # Why a separate operator instead of `datafusion_distributed::NetworkShuffleExec`
//!
//! Both implement [`NetworkBoundary`] and declare `Partitioning::Hash(keys, N)`,
//! so DataFusion's optimizer sees the same plan-shape vocabulary either way.
//! The split is only in `execute()`, because the fork's transport doesn't fit
//! ParadeDB's runtime:
//!
//! * **Transport** — `NetworkShuffleExec` dials a URL-addressed worker via
//!   Tonic/gRPC and consumes a `FlightRecordBatchStream`. ParadeDB has no
//!   URLs and no HTTP server; participants are parallel workers in one
//!   Postgres backend, talking through DSM `shm_mq` ring buffers.
//! * **Consumer concurrency** — the fork demuxes per-partition
//!   `tokio::sync::mpsc` channels under a `MemoryReservation` budget,
//!   pulled by async tasks. `shm_mq` is a fixed-size ring, so a stalled
//!   DataFusion consumer would propagate backpressure to remote producers
//!   and deadlock the mesh. We instead run a drain thread per participant
//!   into a spillable [`DrainBuffer`](super::transport::DrainBuffer).
//! * **Cancellation** — Flight uses Tonic tokens / HTTP/2 stream resets;
//!   ParadeDB uses Postgres-style `check_for_interrupts!()` and DSM
//!   teardown. The cooperative-spin path in `MppSender::send_batch_traced`
//!   is shaped around that and has no Flight analogue.
//!
//! [`BoundaryFactory`](datafusion_distributed::BoundaryFactory) is the
//! extension point that lets us slot `MppShuffleExec` into the same walker
//! without forking the planner. Generalizing `NetworkShuffleExec` itself
//! behind a `WorkerTransport` trait (URL resolution, demux task, memory
//! budget, cancellation) belongs upstream in
//! `datafusion-contrib/datafusion-distributed`, not in our fork.
//!
//! [`NetworkShuffleExec`]: https://github.com/datafusion-contrib/datafusion-distributed

use std::any::Any;
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use datafusion::arrow::array::{RecordBatch, UInt64Array};
use datafusion::arrow::compute::take;
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::common::hash_utils::create_hashes;
use datafusion::common::{DataFusionError, Result as DFResult};
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_expr::PhysicalExpr;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::stream::{EmptyRecordBatchStream, RecordBatchStreamAdapter};
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties,
};
use futures::Stream;
use tokio::task::yield_now;

#[cfg(not(test))]
use crate::gucs::mpp_trace;
use crate::postgres::customscan::mpp::transport::{
    DrainHandle, DrainItem, MppSender, SendBatchStats,
};
use datafusion_distributed::{NetworkBoundary, Stage};
use uuid::Uuid;

/// GUC read wrapper that is inert under `cfg(test)`.
///
/// `pgrx::guc::GucSetting::get()` calls `check_active_thread`, which panics
/// if invoked from a thread other than the one that first touched the
/// pgrx FFI layer. `cargo test` (no `--test-threads=1`) runs each test on
/// a fresh worker thread, so a GUC read inside a stream poller blows up
/// across threads. The production path still reads the GUC; tests just see
/// `false` (no trace instrumentation needed for correctness).
#[inline]
fn mpp_trace_flag() -> bool {
    #[cfg(not(test))]
    {
        mpp_trace()
    }
    #[cfg(test)]
    {
        false
    }
}

/// Assigns each row of a `RecordBatch` to one of N destination participants.
///
/// Implementations must return exactly `batch.num_rows()` values, each in
/// `0..total_participants`. `ShuffleExec` uses this at the top of its
/// producer loop to fan rows out across the mesh.
///
/// TODO(future): align this with DataFusion's own `Partitioning` so the
/// routing decision rides on the optimizer's existing partition concept
/// rather than a parallel one. The destination model
/// (`datafusion-distributed`'s `PartitionIsolatorExec`) is to express
/// participant identity as a `Partitioning::Hash` partition that the
/// transport peels off, which would let us delete this trait. Out of
/// scope for the MPP chain — flagged for a follow-up ticket.
pub trait RowPartitioner: Send + Sync {
    /// Return a destination index in `0..total_participants` for every row.
    fn partition_for_each_row(&self, batch: &RecordBatch) -> Result<Vec<u32>, DataFusionError>;

    /// Total destinations this partitioner will target. Used by
    /// `ShuffleExec` to size per-destination scratch arrays.
    fn total_participants(&self) -> u32;
}

/// Production partitioner: hashes one or more key columns and assigns each
/// row to `hash % total_participants`.
///
/// Uses `create_hashes` with `ahash::RandomState::with_seeds(0, 0, 0, 0)` —
/// the same seed DataFusion uses for `REPARTITION_RANDOM_STATE`, so routing
/// is stable across workers.
///
/// NULL keys hash to a single sentinel, so a heavily-NULL key column skews
/// destinations. Correct (the receiving HashJoin/Aggregate clusters NULL keys
/// the same way), but worth checking `mpp_trace`'s per-peer `rows_sent` if
/// a hot peer shows up.
///
/// TODO: accept `Vec<Arc<dyn PhysicalExpr>>` instead of column indices so the
/// routing stays byte-compatible if a planner pushes `CAST(col)` or similar
/// into the key list. Today we only accept column refs.
pub struct HashPartitioner {
    /// Indices into the input schema for the key columns to hash.
    key_columns: Vec<usize>,
    total_participants: u32,
}

impl HashPartitioner {
    pub fn new(key_columns: Vec<usize>, total_participants: u32) -> Self {
        assert!(
            total_participants > 0,
            "HashPartitioner requires total_participants >= 1"
        );
        Self {
            key_columns,
            total_participants,
        }
    }
}

impl RowPartitioner for HashPartitioner {
    fn partition_for_each_row(&self, batch: &RecordBatch) -> Result<Vec<u32>, DataFusionError> {
        if self.total_participants == 1 {
            // Single participant degenerate case — everyone is self.
            return Ok(vec![0u32; batch.num_rows()]);
        }
        if self.key_columns.is_empty() {
            return Err(DataFusionError::Internal(
                "HashPartitioner: no key columns provided".into(),
            ));
        }
        let columns: Vec<_> = self
            .key_columns
            .iter()
            .map(|&idx| batch.column(idx).clone())
            .collect();
        let mut hashes_buf = vec![0u64; batch.num_rows()];
        // Use a fixed, non-zero seed so hashes stay stable across calls.
        let random_state = ahash::RandomState::with_seeds(0, 0, 0, 0);
        create_hashes(&columns, &random_state, &mut hashes_buf)?;
        let n = self.total_participants as u64;
        Ok(hashes_buf.into_iter().map(|h| (h % n) as u32).collect())
    }

    fn total_participants(&self) -> u32 {
        self.total_participants
    }
}

/// Route every row to one fixed destination. Used by the scalar-aggregate
/// final-gather: workers ship Partial rows to the leader's participant for
/// `FinalPartitioned`.
pub struct FixedTargetPartitioner {
    target: u32,
    total_participants: u32,
}

impl FixedTargetPartitioner {
    pub fn new(target: u32, total_participants: u32) -> Self {
        assert!(total_participants > 0);
        assert!(
            target < total_participants,
            "FixedTargetPartitioner: target {target} >= total_participants {total_participants}"
        );
        Self {
            target,
            total_participants,
        }
    }
}

impl RowPartitioner for FixedTargetPartitioner {
    fn partition_for_each_row(&self, batch: &RecordBatch) -> Result<Vec<u32>, DataFusionError> {
        Ok(vec![self.target; batch.num_rows()])
    }

    fn total_participants(&self) -> u32 {
        self.total_participants
    }
}

/// Scatter a `RecordBatch` into one sub-batch per destination participant.
///
/// Returns a `Vec` of length `total_participants`. Each entry is either:
/// - `Some(sub_batch)` if one or more rows of `batch` routed to that participant; or
/// - `None` if no rows routed to that participant (skip a send round-trip).
///
/// Row order within each sub-batch matches the original batch. Uses
/// `arrow::compute::take` so the implementation is zero-copy per-column for
/// fixed-width arrays and slice-copy for variable-width.
pub fn split_batch_by_partition(
    batch: &RecordBatch,
    destinations: &[u32],
    total_participants: u32,
) -> Result<Vec<Option<RecordBatch>>, DataFusionError> {
    if destinations.len() != batch.num_rows() {
        return Err(DataFusionError::Internal(format!(
            "split_batch_by_partition: destinations.len()={} != batch.num_rows()={}",
            destinations.len(),
            batch.num_rows()
        )));
    }
    if total_participants == 0 {
        return Err(DataFusionError::Internal(
            "split_batch_by_partition: total_participants must be > 0".into(),
        ));
    }

    // Bucket row indices per destination. Allocate lazily so single-destination
    // inputs don't pay for N unused Vecs.
    let n = total_participants as usize;
    let mut buckets: Vec<Option<Vec<u64>>> = (0..n).map(|_| None).collect();
    for (row_idx, &dest) in destinations.iter().enumerate() {
        if dest as usize >= n {
            return Err(DataFusionError::Internal(format!(
                "split_batch_by_partition: destination {dest} >= total_participants {total_participants}"
            )));
        }
        buckets[dest as usize]
            .get_or_insert_with(Vec::new)
            .push(row_idx as u64);
    }

    let schema = batch.schema();
    let mut out: Vec<Option<RecordBatch>> = Vec::with_capacity(n);
    for bucket in buckets {
        match bucket {
            None => out.push(None),
            Some(indices) => {
                let idx_array = UInt64Array::from(indices);
                let taken_cols: Result<Vec<_>, DataFusionError> = batch
                    .columns()
                    .iter()
                    .map(|c| {
                        take(c.as_ref(), &idx_array, None)
                            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
                    })
                    .collect();
                let taken_cols = taken_cols?;
                let sub = RecordBatch::try_new(schema.clone(), taken_cols)
                    .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
                out.push(Some(sub));
            }
        }
    }
    Ok(out)
}

/// How this participant connects to the mesh. Moved into `ShuffleExec`
/// at construction; the operator owns the senders and drops them at stream
/// EOF / abort so peers observe detach.
pub struct ShuffleWiring {
    pub partitioner: Arc<dyn RowPartitioner>,
    /// One slot per participant. Slot at `participant_index` must be `None`;
    /// the rest must be `Some(MppSender)`. Asserted by `ShuffleExec::new`.
    pub outbound_senders: Vec<Option<MppSender>>,
    pub participant_index: u32,
    /// Inbound drain handle for the same mesh. When set,
    /// `build_shuffle_stream` calls `poll_drain_pass` every iteration so
    /// inbound queues keep draining even when outbound sends aren't
    /// blocking — otherwise a fast-sending participant never reads its
    /// inbound, peers stall trying to ship to it, and the mesh deadlocks.
    pub cooperative_drain: Option<Arc<DrainHandle>>,
}

/// Mutable per-stream state for [`build_shuffle_stream`]. Trace fields are
/// only read by [`log_shuffle_eof`] which is `cfg(not(test))`-gated; the
/// `cfg_attr` silences `dead_code` under `cfg(test)`.
#[cfg_attr(test, allow(dead_code))]
struct ShuffleState {
    /// Taken to `None` once the child is exhausted so peer senders detach
    /// and signal EOF.
    wiring: Option<ShuffleWiring>,
    /// At most one self-share queued for the next outer-loop yield.
    self_queue: Option<RecordBatch>,
    participant_index: u32,
    rows_in: u64,
    rows_self: u64,
    /// Per-destination counters for the EOF trace; entry at
    /// `participant_index` stays 0.
    rows_sent: Vec<u64>,
    batches_in: u64,
    time_in_partition: Duration,
    time_in_split: Duration,
    time_in_encode: Duration,
    time_in_send_wait: Duration,
    time_in_coop_drain_in_spin: Duration,
    send_spin_iters: u64,
}

impl ShuffleState {
    fn new(wiring: ShuffleWiring) -> Self {
        let total = wiring.partitioner.total_participants() as usize;
        let participant_index = wiring.participant_index;
        Self {
            wiring: Some(wiring),
            self_queue: None,
            participant_index,
            rows_in: 0,
            rows_self: 0,
            rows_sent: vec![0u64; total],
            batches_in: 0,
            time_in_partition: Duration::ZERO,
            time_in_split: Duration::ZERO,
            time_in_encode: Duration::ZERO,
            time_in_send_wait: Duration::ZERO,
            time_in_coop_drain_in_spin: Duration::ZERO,
            send_spin_iters: 0,
        }
    }

    /// Process one input batch: partition rows, queue our self-share,
    /// ship peer-shares via `MppSender`. Mutates row/timing counters.
    /// `async` because `MppSender::send_batch_traced` yields to the Tokio
    /// runtime in its cooperative-spin loop — see that fn's body comment.
    async fn process_batch(&mut self, batch: RecordBatch, trace_on: bool) -> DFResult<()> {
        let rows = batch.num_rows() as u64;
        if rows == 0 {
            return Ok(());
        }
        self.rows_in += rows;
        self.batches_in += 1;
        let wiring = self.wiring.as_ref().ok_or_else(|| {
            DataFusionError::Internal("ShuffleStream: wiring missing mid-stream".into())
        })?;
        let t_part = trace_on.then(Instant::now);
        let dests = wiring.partitioner.partition_for_each_row(&batch)?;
        if let Some(t0) = t_part {
            self.time_in_partition += t0.elapsed();
        }
        let n = wiring.partitioner.total_participants();
        let t_split = trace_on.then(Instant::now);
        let mut subs = split_batch_by_partition(&batch, &dests, n)?;
        if let Some(t0) = t_split {
            self.time_in_split += t0.elapsed();
        }

        // Self first (see run_shuffle_pump docstring for rationale).
        if let Some(self_sub) = subs[wiring.participant_index as usize].take() {
            self.rows_self += self_sub.num_rows() as u64;
            // Slot is always free here: process_batch only runs when the
            // outer pump has just yielded any prior self-share, and each
            // input batch produces at most one self-share.
            debug_assert!(self.self_queue.is_none(), "self_queue overwrite");
            self.self_queue = Some(self_sub);
        }
        let mut send_stats = SendBatchStats::default();
        for (dest_idx, sub) in subs.into_iter().enumerate() {
            if dest_idx as u32 == wiring.participant_index {
                debug_assert!(sub.is_none());
                continue;
            }
            let Some(sub) = sub else { continue };
            let sender = wiring.outbound_senders[dest_idx].as_ref().ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "ShuffleStream: no outbound sender for participant {dest_idx}"
                ))
            })?;
            let sub_rows = sub.num_rows() as u64;
            sender.send_batch_traced(&sub, &mut send_stats).await?;
            self.rows_sent[dest_idx] += sub_rows;
        }
        if trace_on {
            self.time_in_encode += send_stats.encode;
            self.time_in_send_wait += send_stats.send_wait;
            self.time_in_coop_drain_in_spin += send_stats.coop_drain_in_spin;
            self.send_spin_iters += send_stats.spin_iters;
        }
        Ok(())
    }
}

/// Async stream powering [`ShuffleExec`]: pulls from `child`, hash-routes
/// peer rows out through `wiring`'s senders, yields the self-share locally,
/// and pumps the cooperative inbound drain between iterations.
fn build_shuffle_stream(
    mut child: SendableRecordBatchStream,
    wiring: ShuffleWiring,
    tag: &'static str,
) -> impl Stream<Item = DFResult<RecordBatch>> + Send {
    use futures::StreamExt;

    async_stream::stream! {
        let trace_on = mpp_trace_flag();
        let first_poll_at = trace_on.then(Instant::now);
        let mut time_in_child = Duration::ZERO;
        let mut time_in_process = Duration::ZERO;
        let mut time_in_coop_drain = Duration::ZERO;
        let mut state = ShuffleState::new(wiring);

        let outcome: DFResult<()> = 'pump: loop {
            // Yield queued self-shares first so each re-enters the loop and
            // re-runs the inbound drain below before pulling more.
            if let Some(b) = state.self_queue.take() {
                yield Ok(b);
                continue 'pump;
            }

            // Cooperative inbound drain (see `ShuffleWiring::cooperative_drain`)
            // — keeps peer sends un-stalled even when our outbound is fast.
            if let Some(w) = state.wiring.as_ref() {
                if let Some(drain) = w.cooperative_drain.as_ref() {
                    let t0 = trace_on.then(Instant::now);
                    let _ = drain.poll_drain_pass();
                    if let Some(t0) = t0 {
                        time_in_coop_drain += t0.elapsed();
                    }
                }
            }

            // Pull until a batch produces a self-row, or the child exhausts.
            'pull: loop {
                let t_child = trace_on.then(Instant::now);
                let next = child.next().await;
                if let Some(t0) = t_child {
                    time_in_child += t0.elapsed();
                }
                match next {
                    None => break 'pump Ok(()),
                    Some(Err(e)) => break 'pump Err(e),
                    Some(Ok(batch)) => {
                        let t_proc = trace_on.then(Instant::now);
                        let result = state.process_batch(batch, trace_on).await;
                        if let Some(t0) = t_proc {
                            time_in_process += t0.elapsed();
                        }
                        if let Err(e) = result {
                            break 'pump Err(e);
                        }
                        if state.self_queue.is_some() {
                            continue 'pump;
                        }
                        continue 'pull;
                    }
                }
            }
        };

        // EOF/error tail: drop wiring so peers detach. A queued self-share
        // is intentionally discarded if outcome is `Err` — DataFusion's
        // contract is no items after a yielded error.
        state.wiring.take();
        log_shuffle_eof(
            tag,
            &state,
            ShuffleTimings {
                first_poll_at,
                time_in_child,
                time_in_process,
                time_in_coop_drain,
            },
        );
        if let Err(e) = outcome {
            yield Err(e);
        }
    }
}

/// Wall-clock timings emitted at EOF by [`log_shuffle_eof`]. `cfg_attr`
/// silences `dead_code` under `cfg(test)` (same reason as [`ShuffleState`]).
#[cfg_attr(test, allow(dead_code))]
struct ShuffleTimings {
    /// `Some` iff `mpp_trace_flag()` was on at first poll.
    first_poll_at: Option<Instant>,
    time_in_child: Duration,
    time_in_process: Duration,
    /// Top-of-iteration cooperative drain time; excludes the in-spin drain
    /// inside `send_batch_traced`.
    time_in_coop_drain: Duration,
}

/// EOF trace emitter. `cfg(not(test))`-gated because the unit-test target
/// can't link the FFI symbols `pgrx::{warning,debug1}!` expand into; the
/// runtime EOF path itself is still exercised by tests.
fn log_shuffle_eof(tag: &'static str, state: &ShuffleState, t: ShuffleTimings) {
    #[cfg(not(test))]
    {
        let sent_summary = state
            .rows_sent
            .iter()
            .enumerate()
            .map(|(i, n)| format!("->{i}={n}"))
            .collect::<Vec<_>>()
            .join(",");
        let wall_ms = t
            .first_poll_at
            .map(|s| s.elapsed().as_secs_f64() * 1000.0)
            .unwrap_or(0.0);
        let child_ms = t.time_in_child.as_secs_f64() * 1000.0;
        let process_ms = t.time_in_process.as_secs_f64() * 1000.0;
        let coop_ms = t.time_in_coop_drain.as_secs_f64() * 1000.0;
        let part_ms = state.time_in_partition.as_secs_f64() * 1000.0;
        let split_ms = state.time_in_split.as_secs_f64() * 1000.0;
        let encode_ms = state.time_in_encode.as_secs_f64() * 1000.0;
        let send_wait_ms = state.time_in_send_wait.as_secs_f64() * 1000.0;
        let drain_in_spin_ms = state.time_in_coop_drain_in_spin.as_secs_f64() * 1000.0;
        if mpp_trace() {
            pgrx::warning!(
                "mpp: ShuffleStream[{}] participant={} EOF rows_in={} batches_in={} self={} sent=[{}] wall_ms={:.1} child_ms={:.1} process_ms={:.1} coop_drain_ms={:.1} part_ms={:.1} split_ms={:.1} encode_ms={:.1} send_wait_ms={:.1} drain_in_spin_ms={:.1} spin_iters={}",
                tag,
                state.participant_index,
                state.rows_in,
                state.batches_in,
                state.rows_self,
                sent_summary,
                wall_ms,
                child_ms,
                process_ms,
                coop_ms,
                part_ms,
                split_ms,
                encode_ms,
                send_wait_ms,
                drain_in_spin_ms,
                state.send_spin_iters,
            );
        } else {
            pgrx::debug1!(
                "mpp: ShuffleStream[{}] participant={} EOF rows_in={} self={} sent=[{}]",
                tag,
                state.participant_index,
                state.rows_in,
                state.rows_self,
                sent_summary
            );
        }
    }
    #[cfg(test)]
    {
        let _ = (tag, state, t);
    }
}

/// RAII guard: cancels the drain on stream drop so peer senders observe
/// detach and stop pushing into a buffer no one reads (which would otherwise
/// grow unbounded on `LIMIT` shortcuts, query cancel, or panic). Shutdown
/// is idempotent, so the natural EOF/Err tail can call it too.
struct ShutdownOnDrop {
    handle: Arc<DrainHandle>,
}

impl Drop for ShutdownOnDrop {
    fn drop(&mut self) {
        let _ = self.handle.shutdown();
    }
}

/// Build the async stream that powers [`DrainGatherExec`]. Yields decoded
/// peer batches in arrival order; cleanly returns once every inbound
/// source has detached and the buffer is drained.
///
/// Cooperative drain: for pg-backed handles the drain work happens on
/// the consumer's own task (not a background thread, which would panic
/// on pgrx's `check_active_thread` the moment it touched any pg FFI via
/// `shm_mq_receive`). Each loop iteration runs one drain pass and then
/// `try_pop`s the buffer; on empty we yield to the executor so sibling
/// tasks (peer `ShuffleExec`s shipping us rows) can make progress before
/// the next pass.
fn build_drain_gather_stream(
    handle: Arc<DrainHandle>,
    schema: SchemaRef,
    tag: &'static str,
    participant_index: u32,
) -> impl Stream<Item = DFResult<RecordBatch>> + Send {
    async_stream::stream! {
        // Held across the whole body so any early-drop of the stream
        // (LIMIT shortcut, cancel, panic) still cancels the buffer and
        // releases the receivers. Peer senders observe detach on their
        // next outbound `try_send_bytes` instead of looping forever.
        let _shutdown_guard = ShutdownOnDrop { handle: handle.clone() };

        let trace_on = mpp_trace_flag();
        let first_poll_at = trace_on.then(Instant::now);
        let mut time_in_drain_pass = Duration::ZERO;
        let mut time_in_pop = Duration::ZERO;
        let mut pending_polls: u64 = 0;
        let mut rows_received: u64 = 0;
        let mut batches_received: u64 = 0;

        let coop = handle.is_cooperative();
        let buffer = handle.buffer().clone();

        // Outcome of the loop body. Both break paths run the same
        // shutdown + log_eof tail below; using a local `Result` keeps the
        // tail in one place rather than duplicating it at every exit.
        let outcome: DFResult<()> = 'drain: loop {
            // A peer that crashes before sending its detach leaves us
            // pinned in the cooperative loop until statement_timeout;
            // honor query cancel so the consumer can exit promptly.
            // Gated out of `cargo test --lib` for the same reason as
            // the `MppSender::send_batch_traced` spin: the macro pulls
            // `ProcessInterrupts` / `PG_exception_stack` / `CopyErrorData`
            // FFI symbols that aren't linked into the unit-test binary.
            #[cfg(not(test))]
            pgrx::check_for_interrupts!();

            if coop {
                let t0 = trace_on.then(Instant::now);
                let res = handle.poll_drain_pass();
                if let Some(t0) = t0 {
                    time_in_drain_pass += t0.elapsed();
                }
                if let Err(e) = res {
                    // Any batches already pushed into the buffer by an
                    // earlier successful pass are intentionally discarded:
                    // DataFusion's contract for a yielded `Err` is that no
                    // subsequent items follow, so partial results would
                    // mislead aggregators above us.
                    break 'drain Err(e);
                }
            }

            let t_pop = trace_on.then(Instant::now);
            let item = if coop {
                buffer.try_pop()
            } else {
                Some(buffer.recv().await)
            };
            if let Some(t0) = t_pop {
                time_in_pop += t0.elapsed();
            }

            match item {
                Some(DrainItem::Batch(b)) => {
                    // Peer-shipped batches went through encode_batch /
                    // decode_batch, so their schema comes from IPC metadata
                    // rather than from what we expected locally. A peer
                    // running a drifted schema would otherwise silently
                    // feed garbage into the Union above us.
                    if b.schema() != schema {
                        break 'drain Err(DataFusionError::Internal(format!(
                            "DrainGatherExec: peer batch schema {:?} disagrees with expected {:?}",
                            b.schema(),
                            schema
                        )));
                    }
                    rows_received += b.num_rows() as u64;
                    batches_received += 1;
                    yield Ok(b);
                }
                Some(DrainItem::Eof) => break 'drain Ok(()),
                None => {
                    // Cooperative path only: buffer empty + sources still
                    // alive. Yield so peer `ShuffleExec`s can produce, then
                    // loop back into another drain pass.
                    if trace_on {
                        pending_polls += 1;
                    }
                    yield_now().await;
                }
            }
        };

        // Single shutdown + log_eof tail. Joins the drain thread
        // deterministically so plan teardown doesn't leave a zombie
        // holding DSM pointers; logs trailing trace metrics.
        let shutdown_err = handle.shutdown().err();
        log_drain_gather_eof(
            tag,
            participant_index,
            DrainGatherTrace {
                rows_received,
                batches_received,
                first_poll_at,
                time_in_drain_pass,
                time_in_pop,
                pending_polls,
            },
        );
        if let Err(e) = outcome {
            yield Err(e);
        } else if let Some(e) = shutdown_err {
            yield Err(e);
        }
    }
}

/// EOF metrics for [`build_drain_gather_stream`]; see [`ShuffleTimings`]
/// for the same `cfg_attr` rationale.
#[cfg_attr(test, allow(dead_code))]
struct DrainGatherTrace {
    rows_received: u64,
    batches_received: u64,
    /// `Some` iff `mpp_trace_flag()` was on at first poll.
    first_poll_at: Option<Instant>,
    time_in_drain_pass: Duration,
    time_in_pop: Duration,
    /// `try_pop` returned `None`; subset of cooperative-only iterations.
    pending_polls: u64,
}

/// EOF trace emitter; `cfg(not(test))`-gated for the same FFI-link reason
/// as [`log_shuffle_eof`].
fn log_drain_gather_eof(tag: &'static str, participant_index: u32, t: DrainGatherTrace) {
    #[cfg(not(test))]
    {
        let wall_ms = t
            .first_poll_at
            .map(|s| s.elapsed().as_secs_f64() * 1000.0)
            .unwrap_or(0.0);
        let drain_ms = t.time_in_drain_pass.as_secs_f64() * 1000.0;
        let pop_ms = t.time_in_pop.as_secs_f64() * 1000.0;
        if mpp_trace() {
            pgrx::warning!(
                "mpp: DrainGatherStream[{}] participant={} EOF rows_received={} batches_received={} wall_ms={:.1} drain_ms={:.1} pop_ms={:.1} pending_polls={}",
                tag,
                participant_index,
                t.rows_received,
                t.batches_received,
                wall_ms,
                drain_ms,
                pop_ms,
                t.pending_polls,
            );
        } else {
            pgrx::debug1!(
                "mpp: DrainGatherStream[{}] participant={} EOF rows_received={} batches_received={}",
                tag,
                participant_index,
                t.rows_received,
                t.batches_received
            );
        }
    }
    #[cfg(test)]
    {
        let _ = (tag, participant_index, t);
    }
}

/// Single-operator MPP shuffle. Shape-aligned with
/// `datafusion_distributed::NetworkShuffleExec`: declares
/// `Partitioning::Hash(keys, N)` natively (or `UnknownPartitioning(1)` for
/// the scalar-final gather). The walker emits one per cut.
///
/// Only `execute(drive_partition)` does work — it returns the merged
/// producer + drain stream. Other partitions return an empty stream:
/// for hash boundaries those rows live on peers; for fixed-target gathers
/// non-target participants still drive upstream so their rows ship out,
/// then yield empty.
///
/// `wiring` and `drain_handle` are one-shot. The first `execute(p)` with
/// `p == drive_partition` consumes both; later calls error rather than
/// silently re-running.
pub struct MppShuffleExec {
    input: Arc<dyn ExecutionPlan>,
    wiring: Mutex<Option<ShuffleWiring>>,
    drain_handle: Mutex<Option<Arc<DrainHandle>>>,
    plan_properties: Arc<PlanProperties>,
    tag: &'static str,
    participant_index: u32,
    drive_partition: u32,
    /// Stage this boundary consumes from. Stamped by the walker via
    /// [`NetworkBoundary::with_input_stage`]; `None` only on un-stamped
    /// test fixtures that construct `MppShuffleExec` directly. The fork's
    /// [`NetworkBoundary`] trait returns `&Stage`, so we keep an owned
    /// field rather than synthesize one on demand. ParadeDB's u64
    /// `query_id` round-trips as the low 64 bits of a UUID
    /// ([`Uuid::from_u128`] of `query_id as u128`).
    input_stage: Option<Stage>,
}

impl fmt::Debug for MppShuffleExec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MppShuffleExec")
            .field("tag", &self.tag)
            .field("drive_partition", &self.drive_partition)
            .field("participant_index", &self.participant_index)
            .finish_non_exhaustive()
    }
}

impl MppShuffleExec {
    /// Construct an `MppShuffleExec`.
    ///
    /// - `hash_keys`: when `Some`, output partitioning is
    ///   `Partitioning::Hash(keys, total_participants)` — the hash-shuffle
    ///   case. When `None`, output is `UnknownPartitioning(1)` — the
    ///   scalar-final gather case where every row routes to one target.
    /// - `drive_partition`: the output partition that triggers the
    ///   participant's work. For hash shuffles this is `participant_index`;
    ///   for fixed-target gathers it is `target` on every participant.
    pub fn new(
        input: Arc<dyn ExecutionPlan>,
        wiring: ShuffleWiring,
        drain_handle: Arc<DrainHandle>,
        hash_keys: Option<Vec<Arc<dyn PhysicalExpr>>>,
        drive_partition: u32,
        tag: &'static str,
    ) -> Self {
        let n = wiring.partitioner.total_participants();
        let participant_index = wiring.participant_index;
        assert_eq!(
            wiring.outbound_senders.len(),
            n as usize,
            "MppShuffleExec: outbound_senders.len() != total_participants"
        );
        assert!(
            participant_index < n,
            "MppShuffleExec: participant_index >= total_participants"
        );
        assert!(
            wiring.outbound_senders[participant_index as usize].is_none(),
            "MppShuffleExec: self participant sender slot must be None"
        );
        assert!(
            drive_partition < n,
            "MppShuffleExec: drive_partition ({drive_partition}) >= total_participants ({n})"
        );

        let partitioning = match &hash_keys {
            Some(keys) => Partitioning::Hash(keys.clone(), n as usize),
            None => Partitioning::UnknownPartitioning(1),
        };
        let eq_properties = EquivalenceProperties::new(input.schema());
        let plan_properties = Arc::new(PlanProperties::new(
            eq_properties,
            partitioning,
            EmissionType::Incremental,
            Boundedness::Bounded,
        ));

        Self {
            input,
            wiring: Mutex::new(Some(wiring)),
            drain_handle: Mutex::new(Some(drain_handle)),
            plan_properties,
            tag,
            participant_index,
            drive_partition,
            input_stage: None,
        }
    }
}

impl NetworkBoundary for MppShuffleExec {
    fn input_stage(&self) -> &Stage {
        // Walker-emitted boundaries always have `input_stage = Some(_)` from
        // `with_input_stage`; the placeholder only fires for un-stamped test
        // fixtures.
        self.input_stage.as_ref().unwrap_or_else(|| {
            static PLACEHOLDER: OnceLock<Stage> = OnceLock::new();
            PLACEHOLDER.get_or_init(|| Stage::new_unaddressed(Uuid::nil(), 0, 0))
        })
    }

    fn with_input_stage(&self, stage: Stage) -> DFResult<Arc<dyn ExecutionPlan>> {
        let wiring = self.wiring.lock().unwrap().take().ok_or_else(|| {
            DataFusionError::Internal(
                "MppShuffleExec::with_input_stage: wiring already consumed".into(),
            )
        })?;
        let drain_handle = self.drain_handle.lock().unwrap().take().ok_or_else(|| {
            DataFusionError::Internal(
                "MppShuffleExec::with_input_stage: drain handle already consumed".into(),
            )
        })?;
        let hash_keys = match &self.plan_properties.partitioning {
            Partitioning::Hash(exprs, _) => Some(exprs.clone()),
            _ => None,
        };
        let mut node = MppShuffleExec::new(
            self.input.clone(),
            wiring,
            drain_handle,
            hash_keys,
            self.drive_partition,
            self.tag,
        );
        node.input_stage = Some(stage);
        Ok(Arc::new(node))
    }
}

impl DisplayAs for MppShuffleExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MppShuffleExec: tag={} drive_partition={} participant={}",
            self.tag, self.drive_partition, self.participant_index
        )
    }
}

impl ExecutionPlan for MppShuffleExec {
    fn name(&self) -> &str {
        "MppShuffleExec"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn properties(&self) -> &Arc<PlanProperties> {
        &self.plan_properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.input]
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> DFResult<Arc<dyn ExecutionPlan>> {
        if children.len() != 1 {
            return Err(DataFusionError::Internal(format!(
                "MppShuffleExec expects exactly one child, got {}",
                children.len()
            )));
        }
        let wiring = self.wiring.lock().unwrap().take().ok_or_else(|| {
            DataFusionError::Internal(
                "MppShuffleExec: with_new_children called after wiring consumed".into(),
            )
        })?;
        let drain_handle = self.drain_handle.lock().unwrap().take().ok_or_else(|| {
            DataFusionError::Internal(
                "MppShuffleExec: with_new_children called after drain handle consumed".into(),
            )
        })?;
        let hash_keys = match &self.plan_properties.partitioning {
            Partitioning::Hash(exprs, _) => Some(exprs.clone()),
            _ => None,
        };
        Ok(Arc::new(MppShuffleExec::new(
            children.into_iter().next().unwrap(),
            wiring,
            drain_handle,
            hash_keys,
            self.drive_partition,
            self.tag,
        )))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> DFResult<SendableRecordBatchStream> {
        let n = self.plan_properties.partitioning.partition_count();
        if partition >= n {
            return Err(DataFusionError::Internal(format!(
                "MppShuffleExec: partition {partition} >= partition_count {n}"
            )));
        }
        if partition as u32 != self.drive_partition {
            // Other partitions: empty stream. The drive_partition's stream
            // performs all the work (drives upstream, ships peers, drains
            // inbound). DataFusion will iterate execute(p) for every p in
            // the declared partition count; only the drive partition is
            // load-bearing.
            return Ok(Box::pin(EmptyRecordBatchStream::new(self.input.schema())));
        }

        let wiring =
            self.wiring.lock().unwrap().take().ok_or_else(|| {
                DataFusionError::Internal("MppShuffleExec: already executed".into())
            })?;
        let drain_handle = self.drain_handle.lock().unwrap().take().ok_or_else(|| {
            DataFusionError::Internal(
                "MppShuffleExec: drain handle already consumed before execute".into(),
            )
        })?;

        let child_stream = self.input.execute(0, context)?;
        let producer = build_shuffle_stream(child_stream, wiring, self.tag);
        let consumer = build_drain_gather_stream(
            drain_handle,
            self.input.schema(),
            self.tag,
            self.participant_index,
        );

        // `select` polls producer and consumer on the same task and yields
        // whichever has a batch ready first; the cooperative-drain pattern
        // in `build_shuffle_stream` keeps both halves making progress.
        let merged = futures::stream::select(producer, consumer);
        Ok(Box::pin(RecordBatchStreamAdapter::new(
            self.input.schema(),
            merged,
        )))
    }
}

// `pub` (in test builds only, since the whole module is gated `#[cfg(test)]`)
// so `plan_build.rs::tests` can reuse `ModuloPartitioner`. Keeping the
// shared test helpers here rather than in a separate `test_helpers` module
// avoids introducing a second test-utility surface for one type.
#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::postgres::customscan::mpp::transport::{
        in_proc_channel, DrainBuffer, DrainConfig, DrainItem, MppReceiver, MppSender,
    };
    use datafusion::arrow::array::{Int32Array, StringArray};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};

    // ---- shared test helpers ----

    /// Test-only partitioner: row `i` -> destination `i % total_participants`.
    ///
    /// Not suitable for production because adjacent rows land on different
    /// participants regardless of their join key, which would break
    /// HashJoin/Aggregate correctness. Useful in unit tests because the
    /// routing is trivially predictable without committing to a specific
    /// hash output.
    pub struct ModuloPartitioner {
        total_participants: u32,
    }

    impl ModuloPartitioner {
        pub fn new(total_participants: u32) -> Self {
            assert!(total_participants > 0);
            Self { total_participants }
        }
    }

    impl RowPartitioner for ModuloPartitioner {
        fn partition_for_each_row(&self, batch: &RecordBatch) -> Result<Vec<u32>, DataFusionError> {
            let n = self.total_participants;
            Ok((0..batch.num_rows() as u32).map(|i| i % n).collect())
        }

        fn total_participants(&self) -> u32 {
            self.total_participants
        }
    }

    /// Used by tests to verify round-trip: `split_batch_by_partition` followed
    /// by `concat_batches` (over the same ordering) produces a permutation of
    /// the original batch rows grouped by destination. In production, the
    /// consumer side doesn't need this — the DrainBuffer just yields
    /// sub-batches directly.
    fn concat_batches(
        schema: &SchemaRef,
        batches: impl IntoIterator<Item = RecordBatch>,
    ) -> Result<RecordBatch, DataFusionError> {
        let collected: Vec<RecordBatch> = batches.into_iter().collect();
        if collected.is_empty() {
            return RecordBatch::try_new(schema.clone(), vec![])
                .map_err(|e| DataFusionError::ArrowError(Box::new(e), None));
        }
        let refs: Vec<&RecordBatch> = collected.iter().collect();
        datafusion::arrow::compute::concat_batches(schema, refs)
            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
    }

    /// Synchronous producer-side shuffle pump.
    ///
    /// Consumes an iterator of input batches, partitions each batch by the
    /// supplied [`RowPartitioner`], ships the non-self sub-batches through
    /// `outbound_senders[j]` (for `j != participant_index`), and delivers
    /// the self-partition sub-batch via `push_self`.
    ///
    /// ## Ownership contract
    ///
    /// `outbound_senders` is passed **by value** and dropped when the pump
    /// returns (Ok or Err). Dropping the senders is what signals clean EOF
    /// to peer `MppReceiver`s on the other end of the channel — the peer's
    /// drain thread observes `Detached` and marks its corresponding source
    /// done. Taking this by `&mut` would leave senders alive in the
    /// caller's scope and let peers block forever; moving ownership into
    /// this function makes the end-of-stream signal automatic.
    ///
    /// ## Error semantics
    ///
    /// On any error (partition compute, scatter, `push_self`, or `send`)
    /// the pump returns `Err(DataFusionError)` immediately, dropping all
    /// remaining senders. Peers cannot distinguish a truncated shuffle
    /// from a clean EOF — the shm_mq / channel protocol doesn't carry an
    /// explicit "abort" tag. It is therefore the caller's responsibility
    /// to propagate the error up through the DataFusion `ExecutionPlan`
    /// so every worker aborts the whole query, not just this operator. A
    /// silent drop would make peers' downstream aggregate nodes happily
    /// finalize over incomplete input and return wrong answers.
    /// `build_shuffle_stream` honors this by surfacing the error via the
    /// stream's `Err` item.
    ///
    /// ## Producer order
    ///
    /// Inside each input batch we push the self-partition to `push_self`
    /// *first*, then the peer sub-batches. That means a local downstream
    /// operator (e.g., FinalAgg on our participant) can start consuming
    /// the self-partition even while some peer sender is blocked on
    /// back-pressure. If we pushed peers first, a full-sender stall on
    /// participant 0 would indirectly stall our own consumer because it
    /// never sees any self rows until the stall unblocks.
    ///
    /// This is the non-streaming core of `ShuffleExec`: the DataFusion
    /// operator composes this same logic inside a `Stream::poll_next`,
    /// but the synchronous form is unit-testable without setting up a
    /// DataFusion runtime.
    fn run_shuffle_pump<I>(
        input: I,
        partitioner: &dyn RowPartitioner,
        outbound_senders: Vec<Option<MppSender>>,
        participant_index: u32,
        mut push_self: impl FnMut(RecordBatch) -> Result<(), DataFusionError>,
    ) -> Result<(), DataFusionError>
    where
        I: IntoIterator<Item = Result<RecordBatch, DataFusionError>>,
    {
        let n = partitioner.total_participants();
        if outbound_senders.len() != n as usize {
            return Err(DataFusionError::Internal(format!(
                "run_shuffle_pump: outbound_senders.len()={} != total_participants={}",
                outbound_senders.len(),
                n
            )));
        }
        if participant_index >= n {
            return Err(DataFusionError::Internal(format!(
                "run_shuffle_pump: participant_index {participant_index} >= total_participants {n}"
            )));
        }

        for batch_result in input {
            let batch = batch_result?;
            if batch.num_rows() == 0 {
                continue;
            }
            let dests = partitioner.partition_for_each_row(&batch)?;
            let mut subs = split_batch_by_partition(&batch, &dests, n)?;

            // Push self first so a blocked peer send doesn't starve the
            // local downstream consumer — see module docstring.
            if let Some(self_sub) = subs[participant_index as usize].take() {
                push_self(self_sub)?;
            }

            for (dest_idx, sub) in subs.into_iter().enumerate() {
                if dest_idx as u32 == participant_index {
                    // Already handled above (taken() cleared the slot).
                    debug_assert!(sub.is_none());
                    continue;
                }
                let Some(sub) = sub else { continue };
                let sender = outbound_senders[dest_idx].as_ref().ok_or_else(|| {
                    DataFusionError::Internal(format!(
                        "run_shuffle_pump: no outbound sender for participant {dest_idx}"
                    ))
                })?;
                sender.send_batch(&sub)?;
            }
        }

        // `outbound_senders` drops here: peer MppReceivers observe
        // `Detached` and mark their source done, producing clean EOF on
        // the remote drain.
        drop(outbound_senders);
        Ok(())
    }

    // ---- tests ----

    fn sample_batch(rows: i32) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]));
        let ids = Int32Array::from_iter_values(0..rows);
        let names = StringArray::from_iter_values((0..rows).map(|i| format!("n{i}")));
        RecordBatch::try_new(schema, vec![Arc::new(ids), Arc::new(names)]).unwrap()
    }

    #[test]
    fn modulo_partitioner_round_robins() {
        let batch = sample_batch(7);
        let p = ModuloPartitioner::new(3);
        let dests = p.partition_for_each_row(&batch).unwrap();
        assert_eq!(dests, vec![0, 1, 2, 0, 1, 2, 0]);
    }

    #[test]
    fn split_batch_by_partition_preserves_order() {
        let batch = sample_batch(6);
        let dests = vec![0, 1, 0, 1, 0, 1]; // N=2
        let out = split_batch_by_partition(&batch, &dests, 2).unwrap();
        assert_eq!(out.len(), 2);
        let p0 = out[0].as_ref().unwrap();
        let p1 = out[1].as_ref().unwrap();
        assert_eq!(p0.num_rows(), 3);
        assert_eq!(p1.num_rows(), 3);
        // Within each destination, original order is preserved.
        let ids_p0 = p0.column(0).as_any().downcast_ref::<Int32Array>().unwrap();
        assert_eq!(ids_p0.values(), &[0, 2, 4]);
        let ids_p1 = p1.column(0).as_any().downcast_ref::<Int32Array>().unwrap();
        assert_eq!(ids_p1.values(), &[1, 3, 5]);
    }

    #[test]
    fn split_batch_by_partition_returns_none_for_empty_destinations() {
        let batch = sample_batch(4);
        let dests = vec![0, 0, 0, 0]; // All to 0
        let out = split_batch_by_partition(&batch, &dests, 3).unwrap();
        assert!(out[0].is_some());
        assert!(out[1].is_none());
        assert!(out[2].is_none());
    }

    #[test]
    fn split_batch_by_partition_rejects_length_mismatch() {
        let batch = sample_batch(4);
        let dests = vec![0, 1]; // Wrong length
        assert!(split_batch_by_partition(&batch, &dests, 2).is_err());
    }

    #[test]
    fn split_batch_by_partition_rejects_out_of_range_destination() {
        let batch = sample_batch(3);
        let dests = vec![0, 5, 0]; // Destination 5 > total=2
        assert!(split_batch_by_partition(&batch, &dests, 2).is_err());
    }

    #[test]
    fn split_roundtrips_via_concat() {
        // Full round-trip: split then concat in destination order; the
        // result is a permutation of the original batch by destination.
        let batch = sample_batch(8);
        let p = ModuloPartitioner::new(3);
        let dests = p.partition_for_each_row(&batch).unwrap();
        let split = split_batch_by_partition(&batch, &dests, 3).unwrap();
        let schema = batch.schema();
        let concatenated = concat_batches(&schema, split.into_iter().flatten()).unwrap();
        assert_eq!(concatenated.num_rows(), batch.num_rows());
        // IDs should appear grouped by destination (0,3,6 then 1,4,7 then 2,5).
        let ids = concatenated
            .column(0)
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        assert_eq!(ids.values(), &[0, 3, 6, 1, 4, 7, 2, 5]);
    }

    #[test]
    fn hash_partitioner_is_deterministic() {
        let batch = sample_batch(100);
        let p = HashPartitioner::new(vec![0], 4);
        let dests_a = p.partition_for_each_row(&batch).unwrap();
        let dests_b = p.partition_for_each_row(&batch).unwrap();
        assert_eq!(dests_a, dests_b);
        assert_eq!(dests_a.len(), 100);
        for d in &dests_a {
            assert!(*d < 4);
        }
    }

    #[test]
    fn hash_partitioner_is_deterministic_with_multi_column_keys() {
        // Production HashJoin keys are typically composite. Verify that
        // multi-column routing is deterministic across repeated calls
        // and that all destinations stay within `[0, total_participants)`.
        // (DataFusion's `create_hashes` combines per-column hashes
        // commutatively, so swapping `vec![0, 1]` for `vec![1, 0]` does
        // not change the destination — that's not a property worth
        // asserting here.)
        let batch = sample_batch(64);
        let p = HashPartitioner::new(vec![0, 1], 4);
        let dests_a = p.partition_for_each_row(&batch).unwrap();
        let dests_b = p.partition_for_each_row(&batch).unwrap();
        assert_eq!(dests_a, dests_b);
        assert_eq!(dests_a.len(), 64);
        for d in &dests_a {
            assert!(*d < 4);
        }
        // Distribution sanity: with 64 rows over 4 destinations, every
        // bucket should have at least one row. A multi-column key that
        // accidentally collapsed to a single hash would drop most of
        // these to zero.
        let mut hits = [false; 4];
        for d in &dests_a {
            hits[*d as usize] = true;
        }
        assert!(hits.iter().all(|h| *h));
    }

    #[test]
    fn hash_partitioner_degenerate_n1_routes_all_to_zero() {
        let batch = sample_batch(50);
        let p = HashPartitioner::new(vec![0], 1);
        let dests = p.partition_for_each_row(&batch).unwrap();
        assert!(dests.iter().all(|&d| d == 0));
    }

    #[test]
    fn hash_partitioner_requires_key_columns() {
        let batch = sample_batch(10);
        let p = HashPartitioner::new(vec![], 2);
        assert!(p.partition_for_each_row(&batch).is_err());
    }

    /// Harness: wire N=2 in-proc channels and feed `batches` through the
    /// pump as participant 0. Returns (self_batches, peer_batches).
    fn drive_pump_n2(
        batches: Vec<RecordBatch>,
        partitioner: &dyn RowPartitioner,
    ) -> (Vec<RecordBatch>, Vec<RecordBatch>) {
        let (tx, rx) = in_proc_channel(16);
        let senders: Vec<Option<MppSender>> = vec![
            None,                               // participant 0 = self, no sender
            Some(MppSender::new(Box::new(tx))), // participant 1 = peer
        ];

        let mut self_batches = Vec::new();

        let input_iter = batches.into_iter().map(Ok::<_, DataFusionError>);
        run_shuffle_pump(input_iter, partitioner, senders, 0, |b| {
            self_batches.push(b);
            Ok(())
        })
        .unwrap();
        // `senders` was moved into the pump and dropped on return; peer
        // receiver now observes `Detached` on its next `try_recv`.

        // Drain the peer channel into a DrainBuffer via the drain thread so
        // we exercise the actual drain path end-to-end.
        let receiver = MppReceiver::new(Box::new(rx));
        let buffer = DrainBuffer::new(1);
        let drain_handle =
            DrainHandle::spawn(DrainConfig::new(vec![receiver], Arc::clone(&buffer)));

        let mut peer_batches = Vec::new();
        while let DrainItem::Batch(b) = buffer.pop_front() {
            peer_batches.push(b);
        }
        drain_handle.shutdown().unwrap();

        (self_batches, peer_batches)
    }

    #[test]
    fn pump_routes_self_and_peer_partitions_end_to_end() {
        // Row i -> participant (i % 2). With 6 input rows: self gets 0,2,4; peer
        // gets 1,3,5. Verifies the full stack: partition → scatter → send
        // over in-proc channel → drain thread → drain buffer.
        let input = vec![sample_batch(6)];
        let partitioner = ModuloPartitioner::new(2);
        let (self_b, peer_b) = drive_pump_n2(input, &partitioner);

        assert_eq!(self_b.len(), 1);
        assert_eq!(peer_b.len(), 1);

        let self_ids = self_b[0]
            .column(0)
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        assert_eq!(self_ids.values(), &[0, 2, 4]);

        let peer_ids = peer_b[0]
            .column(0)
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        assert_eq!(peer_ids.values(), &[1, 3, 5]);
    }

    #[test]
    fn pump_handles_multiple_batches() {
        let input = vec![sample_batch(4), sample_batch(4), sample_batch(4)];
        let partitioner = ModuloPartitioner::new(2);
        let (self_b, peer_b) = drive_pump_n2(input, &partitioner);
        // 3 batches × 2 rows each per destination
        let self_total: usize = self_b.iter().map(|b| b.num_rows()).sum();
        let peer_total: usize = peer_b.iter().map(|b| b.num_rows()).sum();
        assert_eq!(self_total, 6);
        assert_eq!(peer_total, 6);
    }

    #[test]
    fn pump_skips_empty_batches() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]));
        let empty = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(Int32Array::from_iter_values([])),
                Arc::new(StringArray::from_iter_values::<String, _>([])),
            ],
        )
        .unwrap();
        let input = vec![empty, sample_batch(2)];
        let partitioner = ModuloPartitioner::new(2);
        let (self_b, peer_b) = drive_pump_n2(input, &partitioner);
        let self_total: usize = self_b.iter().map(|b| b.num_rows()).sum();
        let peer_total: usize = peer_b.iter().map(|b| b.num_rows()).sum();
        assert_eq!(self_total, 1);
        assert_eq!(peer_total, 1);
    }

    #[test]
    fn pump_propagates_input_errors() {
        let err: Result<RecordBatch, DataFusionError> =
            Err(DataFusionError::Execution("synthetic".into()));
        let (tx, _rx) = in_proc_channel(1);
        let senders: Vec<Option<MppSender>> = vec![None, Some(MppSender::new(Box::new(tx)))];
        let partitioner = ModuloPartitioner::new(2);

        let result = run_shuffle_pump(vec![err], &partitioner, senders, 0, |_| Ok(()));
        assert!(result.is_err());
    }

    #[test]
    fn pump_errors_when_peer_slot_is_none() {
        // Participant 1 should route rows to a peer, but there's no MppSender.
        // Must fail with a meaningful error rather than silently drop data.
        let partitioner = ModuloPartitioner::new(2);
        let senders: Vec<Option<MppSender>> = vec![None, None];
        let result = run_shuffle_pump(
            vec![Ok(sample_batch(2))], // row 0 -> participant 0 (self), row 1 -> participant 1 (peer)
            &partitioner,
            senders,
            0,
            |_| Ok(()),
        );
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("no outbound sender for participant 1"),
            "got: {msg}"
        );
    }

    #[test]
    fn pump_rejects_wrong_sender_count() {
        let partitioner = ModuloPartitioner::new(2);
        // Only 1 sender slot for N=2.
        let senders: Vec<Option<MppSender>> = vec![None];
        let result = run_shuffle_pump(vec![Ok(sample_batch(4))], &partitioner, senders, 0, |_| {
            Ok(())
        });
        assert!(result.is_err());
    }

    #[test]
    fn hash_partitioner_distributes_across_participants() {
        // Over 1000 rows with 4 participants, every participant should get at least some.
        // This is a statistical claim that should hold for any non-adversarial
        // hash.
        let batch = sample_batch(1000);
        let p = HashPartitioner::new(vec![0], 4);
        let dests = p.partition_for_each_row(&batch).unwrap();
        let mut counts = [0usize; 4];
        for d in &dests {
            counts[*d as usize] += 1;
        }
        for (participant, count) in counts.iter().enumerate() {
            assert!(
                *count > 100,
                "participant {participant} only received {count}/1000 rows — partitioner is skewed"
            );
        }
    }
}

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

//! Shuffle operator primitives for MPP.
//!
//! Today this file carries the pure-function primitives that the forthcoming
//! `ShuffleExec` DataFusion operator composes:
//!
//! - [`RowPartitioner`] is the trait `ShuffleExec` calls per batch to decide
//!   which destination seat each row belongs to.
//! - [`HashPartitioner`] is the production impl — a hash over join key columns
//!   modulo N participants. Uses DataFusion's `create_hashes` helper with a
//!   seed that matches DataFusion's own `REPARTITION_RANDOM_STATE`
//!   (`datafusion::physical_plan::repartition` uses `with_seeds(0,0,0,0)`).
//!   Routing is therefore stable across workers — every worker that sees the
//!   same input rows places them on the same seat. Downstream `HashJoinExec`
//!   and `AggregateExec` intentionally use *different* seeds internally (see
//!   their `HASH_JOIN_SEED` / `AGGREGATION_HASH_SEED` constants); that is by
//!   design to avoid hash collisions between routing and internal hash
//!   tables, and has no bearing on our shuffle correctness.
//! - [`ModuloPartitioner`] is the test-only variant that routes row `i` to
//!   destination `i % N`, so tests can verify routing without committing to
//!   a specific hash output.
//! - [`split_batch_by_partition`] is the row-scatter step: given a batch and
//!   its per-row destination vector, return one sub-batch per destination,
//!   preserving row order within each destination. `ShuffleExec`'s producer
//!   loop calls this once per input batch, then feeds each sub-batch to its
//!   corresponding `MppSender` (peers) or local channel (self).

#![allow(dead_code)]

use datafusion::arrow::array::{RecordBatch, UInt64Array};
use datafusion::arrow::compute::take;
use datafusion::common::hash_utils::create_hashes;
use datafusion::common::DataFusionError;

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
        crate::gucs::mpp_trace()
    }
    #[cfg(test)]
    {
        false
    }
}

/// Assigns each row of a `RecordBatch` to one of N destination seats.
///
/// Implementations must return exactly `batch.num_rows()` values, each in
/// `0..total_participants`. `ShuffleExec` uses this at the top of its
/// producer loop to fan rows out across the mesh.
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
/// Uses `datafusion::common::hash_utils::create_hashes` with
/// `ahash::RandomState::with_seeds(0, 0, 0, 0)` — the same seed DataFusion
/// itself uses for `REPARTITION_RANDOM_STATE`. That gives stable routing
/// across workers and across planner reruns. Downstream HashJoin / Aggregate
/// use different seeds internally (by design, to avoid collisions with
/// routing); that is not a problem for our shuffle because only the routing
/// hash needs agreement across workers, not the internal-table hash.
///
/// TODO: accept `Vec<Arc<dyn PhysicalExpr>>` instead of column indices so a
/// planner that pushes `CAST(col)` or `col + 0` into the key list stays
/// byte-compatible with DataFusion's own routing. Today we accept only
/// column refs, which is sufficient for the expected planner output but
/// will silently diverge if that changes.
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

/// Route every row to one fixed destination seat. Used by the MPP scalar-
/// aggregate final-gather step: workers ship their Partial row to seat 0 so
/// the leader can run `AggregateExec(FinalPartitioned)` on the combined
/// stream. Leader's self-partition is the target seat, so nothing is shipped
/// out; its drain receives N-1 partials and it emits exactly one final row.
/// Workers' self-partition is empty (target != self), so their
/// `FinalPartitioned` sees 0 rows and emits 0 rows — PG's Gather on top
/// therefore concatenates exactly one row per query, not one per worker.
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

/// Test-only partitioner: row `i` -> destination `i % total_participants`.
///
/// Not suitable for production because adjacent rows land on different seats
/// regardless of their join key, which would break HashJoin/Aggregate
/// correctness. Useful in unit tests because the routing is trivially
/// predictable without committing to a specific hash output.
#[cfg(test)]
pub struct ModuloPartitioner {
    total_participants: u32,
}

#[cfg(test)]
impl ModuloPartitioner {
    pub fn new(total_participants: u32) -> Self {
        assert!(total_participants > 0);
        Self { total_participants }
    }
}

#[cfg(test)]
impl RowPartitioner for ModuloPartitioner {
    fn partition_for_each_row(&self, batch: &RecordBatch) -> Result<Vec<u32>, DataFusionError> {
        let n = self.total_participants;
        Ok((0..batch.num_rows() as u32).map(|i| i % n).collect())
    }

    fn total_participants(&self) -> u32 {
        self.total_participants
    }
}

/// Scatter a `RecordBatch` into one sub-batch per destination seat.
///
/// Returns a `Vec` of length `total_participants`. Each entry is either:
/// - `Some(sub_batch)` if one or more rows of `batch` routed to that seat; or
/// - `None` if no rows routed to that seat (skip a send round-trip).
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

/// Concatenate destination sub-batches back into one RecordBatch.
///
/// Used by tests to verify round-trip: `split_batch_by_partition` followed by
/// `concat_batches` (over the same ordering) produces a permutation of the
/// original batch rows grouped by destination. In production, the consumer
/// side doesn't need this — the DrainBuffer just yields sub-batches directly.
#[cfg(test)]
fn concat_batches(
    schema: &datafusion::arrow::datatypes::SchemaRef,
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
/// `outbound_senders[j]` (for `j != participant_index`), and delivers the
/// self-partition sub-batch via `push_self`.
///
/// ## Ownership contract
///
/// `outbound_senders` is passed **by value** and dropped when the pump
/// returns (Ok or Err). Dropping the senders is what signals clean EOF to
/// peer `MppReceiver`s on the other end of the channel — the peer's drain
/// thread observes `Detached` and marks its corresponding source done.
/// Taking this by `&mut` would leave senders alive in the caller's scope
/// and let peers block forever; moving ownership into this function makes
/// the end-of-stream signal automatic.
///
/// ## Error semantics
///
/// On any error (partition compute, scatter, `push_self`, or `send`) the
/// pump returns `Err(DataFusionError)` immediately, dropping all remaining
/// senders. Peers cannot distinguish a truncated shuffle from a clean EOF
/// — the shm_mq / channel protocol doesn't carry an explicit "abort" tag.
/// It is therefore the caller's responsibility to propagate the error up
/// through the DataFusion `ExecutionPlan` so every worker aborts the whole
/// query, not just this operator. A silent drop would make peers' downstream
/// aggregate nodes happily finalize over incomplete input and return wrong
/// answers. `ShuffleStream::poll_next` honors this by surfacing the error
/// via its `Stream::Err` item.
///
/// ## Producer order
///
/// Inside each input batch we push the self-partition to `push_self` *first*,
/// then the peer sub-batches. That means a local downstream operator
/// (e.g., FinalAgg on our seat) can start consuming the self-partition even
/// while some peer sender is blocked on back-pressure. If we pushed peers
/// first, a full-sender stall on seat 0 would indirectly stall our own
/// consumer because it never sees any self rows until the stall unblocks.
///
/// This is the non-streaming core of `ShuffleExec`: the DataFusion operator
/// composes this same logic inside a `Stream::poll_next`, but the synchronous
/// form is unit-testable without setting up a DataFusion runtime.
#[cfg(test)]
pub fn run_shuffle_pump<I>(
    input: I,
    partitioner: &dyn RowPartitioner,
    outbound_senders: Vec<Option<crate::postgres::customscan::mpp::transport::MppSender>>,
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

        // Push self first so a blocked peer send doesn't starve the local
        // downstream consumer — see module docstring.
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
                    "run_shuffle_pump: no outbound sender for seat {dest_idx}"
                ))
            })?;
            sender.send_batch(&sub)?;
        }
    }

    // `outbound_senders` drops here: peer MppReceivers observe `Detached`
    // and mark their source done, producing clean EOF on the remote drain.
    drop(outbound_senders);
    Ok(())
}

/// Wiring passed to `ShuffleExec::new` that describes how this participant
/// connects to the mesh. Moved into the operator at construction time; the
/// operator owns the senders for the query's lifetime and drops them at
/// stream EOF / abort so peer receivers cleanly observe detach.
pub struct ShuffleWiring {
    pub partitioner: std::sync::Arc<dyn RowPartitioner>,
    /// One slot per seat, length = `partitioner.total_participants()`.
    /// Seat `participant_index` must be `None`; all other seats must be
    /// `Some(MppSender)`. `ShuffleExec::new` asserts this invariant.
    pub outbound_senders: Vec<Option<crate::postgres::customscan::mpp::transport::MppSender>>,
    pub participant_index: u32,
    /// Our inbound-side drain handle for the same mesh. When set,
    /// `ShuffleStream::poll_next` proactively calls
    /// `DrainHandle::poll_drain_pass` every iteration so our inbound
    /// queues keep draining even when our outbound sends aren't
    /// blocking. Without this, a participant whose sends succeed fast
    /// never drains, peers can't ship to it, and the mesh deadlocks on
    /// one-sided pressure (observed as a 120s timeout on
    /// `aggregate_join_groupby - alternative 2` at 25M rows).
    pub cooperative_drain:
        Option<std::sync::Arc<crate::postgres::customscan::mpp::transport::DrainHandle>>,
}

/// DataFusion `ExecutionPlan` that hashes every input row and ships non-self
/// rows to peer participants via `MppSender`s, emitting only the
/// self-partition rows as its output stream.
///
/// Peer-received rows are NOT merged in here — the plan builder layers a
/// gather-style operator above `ShuffleExec` that unions the self-partition
/// output with a `DrainBuffer` populated by the drain thread reading inbound
/// receivers. Keeping the two sides separate at this layer makes the operator
/// composable and individually testable.
///
/// The wiring (partitioner + senders) is taken on `new` and handed to the
/// first `execute()` call via a `Mutex<Option<_>>`. A second `execute()` call
/// would require re-planning the query and re-attaching a fresh mesh — not
/// supported today; the `Option` returns an error on re-execute.
pub struct ShuffleExec {
    input: std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>,
    wiring: std::sync::Mutex<Option<ShuffleWiring>>,
    plan_properties: std::sync::Arc<datafusion::physical_plan::PlanProperties>,
    /// Diagnostic label — e.g. "left", "right", "postagg", "final". Echoed
    /// in the row-count `mpp_log!` line emitted at stream EOF. Purely for
    /// tracing; no control flow depends on it.
    tag: &'static str,
}

impl std::fmt::Debug for ShuffleExec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShuffleExec")
            .field("input", &self.input)
            .field("tag", &self.tag)
            .finish_non_exhaustive()
    }
}

impl ShuffleExec {
    pub fn new(
        input: std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>,
        wiring: ShuffleWiring,
        tag: &'static str,
    ) -> Self {
        let n = wiring.partitioner.total_participants();
        assert_eq!(
            wiring.outbound_senders.len(),
            n as usize,
            "ShuffleExec: outbound_senders.len() != total_participants"
        );
        assert!(
            wiring.participant_index < n,
            "ShuffleExec: participant_index >= total_participants"
        );
        assert!(
            wiring.outbound_senders[wiring.participant_index as usize].is_none(),
            "ShuffleExec: self seat's sender slot must be None"
        );

        use datafusion::physical_expr::EquivalenceProperties;
        use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
        use datafusion::physical_plan::{Partitioning, PlanProperties};

        let eq_properties = EquivalenceProperties::new(input.schema());
        let plan_properties = std::sync::Arc::new(PlanProperties::new(
            eq_properties,
            Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        ));

        Self {
            input,
            wiring: std::sync::Mutex::new(Some(wiring)),
            plan_properties,
            tag,
        }
    }
}

impl datafusion::physical_plan::DisplayAs for ShuffleExec {
    fn fmt_as(
        &self,
        _t: datafusion::physical_plan::DisplayFormatType,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "ShuffleExec")
    }
}

impl datafusion::physical_plan::ExecutionPlan for ShuffleExec {
    fn name(&self) -> &str {
        "ShuffleExec"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn properties(&self) -> &std::sync::Arc<datafusion::physical_plan::PlanProperties> {
        &self.plan_properties
    }

    fn children(&self) -> Vec<&std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
        vec![&self.input]
    }

    fn with_new_children(
        self: std::sync::Arc<Self>,
        children: Vec<std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>>,
    ) -> datafusion::common::Result<std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>>
    {
        if children.len() != 1 {
            return Err(DataFusionError::Internal(format!(
                "ShuffleExec expects exactly one child, got {}",
                children.len()
            )));
        }
        // Cloning the wiring isn't possible (MppSender isn't Clone). This path
        // is hit by some optimizer rules that re-wire children; those rules
        // fire before we attach the wiring, so `None` means re-planning.
        let wiring = self.wiring.lock().unwrap().take();
        let Some(wiring) = wiring else {
            return Err(DataFusionError::Internal(
                "ShuffleExec: with_new_children called after wiring was consumed".into(),
            ));
        };
        Ok(std::sync::Arc::new(ShuffleExec::new(
            children.into_iter().next().unwrap(),
            wiring,
            self.tag,
        )))
    }

    fn execute(
        &self,
        _partition: usize,
        context: std::sync::Arc<datafusion::execution::TaskContext>,
    ) -> datafusion::common::Result<datafusion::execution::SendableRecordBatchStream> {
        let wiring = self
            .wiring
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| DataFusionError::Internal("ShuffleExec: already executed".into()))?;
        let child_stream = self.input.execute(0, context)?;
        let schema = self.input.schema();
        let stream = ShuffleStream::new(child_stream, wiring, schema.clone(), self.tag);
        Ok(Box::pin(stream))
    }
}

/// Internal stream wrapper for `ShuffleExec::execute`.
struct ShuffleStream {
    child: datafusion::execution::SendableRecordBatchStream,
    wiring: Option<ShuffleWiring>,
    self_queue: std::collections::VecDeque<RecordBatch>,
    done: bool,
    schema: datafusion::arrow::datatypes::SchemaRef,
    /// Diagnostic label for the `mpp_log!` row-count report at EOF.
    tag: &'static str,
    /// Remembered `participant_index` for logging after `wiring` is dropped.
    participant_index: u32,
    /// Total rows read from `child`.
    rows_in: u64,
    /// Rows kept for this participant's self seat (queued to `self_queue`).
    rows_self: u64,
    /// Rows sent to each peer seat. Indexed by destination seat. Entry at
    /// `participant_index` stays 0 (no self-send).
    rows_sent: Vec<u64>,
    /// Number of input batches pulled from child. Average batch size at EOF
    /// is `rows_in / batches_in` — useful for spotting small-batch overhead.
    batches_in: u64,
    /// Set when the EOF `mpp_log!` line has been emitted so we don't double-log.
    logged_eof: bool,
    /// Wall-clock instant of the first poll — used to report total stream
    /// lifetime at EOF. Gated behind `mpp_trace`; set only when the GUC is on
    /// so overhead is zero on the hot path in non-traced runs.
    first_poll_at: Option<std::time::Instant>,
    /// Cumulative time spent inside the child's `poll_next`. Approximates
    /// "upstream wait" for this shuffle.
    time_in_child: std::time::Duration,
    /// Cumulative time spent in `process_batch` (partition compute + split +
    /// peer sends). Approximates this shuffle's own CPU cost.
    time_in_process: std::time::Duration,
    /// Cumulative time inside the cooperative inbound-drain polls at the top
    /// of `poll_next`. Approximates cost paid on the sender side to un-stall
    /// peers when local outbound is backed up.
    time_in_coop_drain: std::time::Duration,
    /// Cumulative time inside `HashPartitioner::partition_for_each_row` — the
    /// per-row hash that picks each row's destination seat.
    time_in_partition: std::time::Duration,
    /// Cumulative time inside `split_batch_by_partition` — the Arrow `take`
    /// kernel that materializes per-destination sub-batches.
    time_in_split: std::time::Duration,
    /// Cumulative time inside Arrow-IPC `encode_batch` (from `send_batch_traced`).
    time_in_encode: std::time::Duration,
    /// Cumulative wall time waiting on a full outbound queue (retry-spin
    /// after the first failed `try_send_bytes`). This is the MPP equivalent
    /// of "send-side blocked on peer".
    time_in_send_wait: std::time::Duration,
    /// Cumulative `poll_drain_pass` time while stuck in the send-retry spin.
    /// A subset of `time_in_send_wait`; separating it tells us whether the
    /// wait is spent draining inbound (productive) or yielding (not productive).
    time_in_coop_drain_in_spin: std::time::Duration,
    /// Count of failed `try_send_bytes` attempts across all outbound sends.
    /// Divide by `sum(rows_sent to peers / rows_per_batch)` to get avg spin
    /// iters per send; large values → outbound is chronically full.
    send_spin_iters: u64,
}

impl ShuffleStream {
    fn new(
        child: datafusion::execution::SendableRecordBatchStream,
        wiring: ShuffleWiring,
        schema: datafusion::arrow::datatypes::SchemaRef,
        tag: &'static str,
    ) -> Self {
        let total = wiring.partitioner.total_participants() as usize;
        let participant_index = wiring.participant_index;
        Self {
            child,
            wiring: Some(wiring),
            self_queue: std::collections::VecDeque::new(),
            done: false,
            schema,
            tag,
            participant_index,
            rows_in: 0,
            rows_self: 0,
            rows_sent: vec![0u64; total],
            batches_in: 0,
            logged_eof: false,
            first_poll_at: None,
            time_in_child: std::time::Duration::ZERO,
            time_in_process: std::time::Duration::ZERO,
            time_in_coop_drain: std::time::Duration::ZERO,
            time_in_partition: std::time::Duration::ZERO,
            time_in_split: std::time::Duration::ZERO,
            time_in_encode: std::time::Duration::ZERO,
            time_in_send_wait: std::time::Duration::ZERO,
            time_in_coop_drain_in_spin: std::time::Duration::ZERO,
            send_spin_iters: 0,
        }
    }

    fn log_eof(&mut self) {
        if self.logged_eof {
            return;
        }
        self.logged_eof = true;
        // `pgrx::warning!` / `pgrx::debug1!` expand to `ereport` which pulls
        // PG FFI symbols (`errstart`, `errfinish`, `palloc0`, …) that aren't
        // linked into the crate's `--tests` / llvm-cov build. Under
        // `--instrument-coverage` DCE is disabled, so gate the whole body in
        // non-test builds. Tests don't exercise ShuffleStream poll paths.
        #[cfg(not(test))]
        {
            let sent_summary = self
                .rows_sent
                .iter()
                .enumerate()
                .map(|(i, n)| format!("->{i}={n}"))
                .collect::<Vec<_>>()
                .join(",");
            // Per-participant EOF trace. Kept off `mpp_debug` because these lines
            // emit concurrently from every participant and reorder run-to-run —
            // at WARNING under mpp_debug they flaked regress expected files.
            // Gated on the dedicated `mpp_trace` GUC so benchmarks can capture
            // row counts in CI (WARNING stream) without affecting regress.
            let wall_ms = self
                .first_poll_at
                .map(|t| t.elapsed().as_secs_f64() * 1000.0)
                .unwrap_or(0.0);
            let child_ms = self.time_in_child.as_secs_f64() * 1000.0;
            let process_ms = self.time_in_process.as_secs_f64() * 1000.0;
            let coop_ms = self.time_in_coop_drain.as_secs_f64() * 1000.0;
            let part_ms = self.time_in_partition.as_secs_f64() * 1000.0;
            let split_ms = self.time_in_split.as_secs_f64() * 1000.0;
            let encode_ms = self.time_in_encode.as_secs_f64() * 1000.0;
            let send_wait_ms = self.time_in_send_wait.as_secs_f64() * 1000.0;
            let drain_in_spin_ms = self.time_in_coop_drain_in_spin.as_secs_f64() * 1000.0;
            if crate::gucs::mpp_trace() {
                pgrx::warning!(
                    "mpp: ShuffleStream[{}] seat={} EOF rows_in={} batches_in={} self={} sent=[{}] wall_ms={:.1} child_ms={:.1} process_ms={:.1} coop_drain_ms={:.1} part_ms={:.1} split_ms={:.1} encode_ms={:.1} send_wait_ms={:.1} drain_in_spin_ms={:.1} spin_iters={}",
                    self.tag,
                    self.participant_index,
                    self.rows_in,
                    self.batches_in,
                    self.rows_self,
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
                    self.send_spin_iters,
                );
            } else {
                pgrx::debug1!(
                    "mpp: ShuffleStream[{}] seat={} EOF rows_in={} self={} sent=[{}]",
                    self.tag,
                    self.participant_index,
                    self.rows_in,
                    self.rows_self,
                    sent_summary
                );
            }
        }
    }

    /// Process one input batch: partition, push self-sub to queue, send peer
    /// subs. Returns Ok(()) on success, Err otherwise.
    fn process_batch(&mut self, batch: RecordBatch) -> Result<(), DataFusionError> {
        let rows = batch.num_rows() as u64;
        if rows == 0 {
            return Ok(());
        }
        self.rows_in += rows;
        self.batches_in += 1;
        let trace_on = mpp_trace_flag();
        let wiring = self.wiring.as_ref().ok_or_else(|| {
            DataFusionError::Internal("ShuffleStream: wiring missing mid-stream".into())
        })?;
        let t_part = trace_on.then(std::time::Instant::now);
        let dests = wiring.partitioner.partition_for_each_row(&batch)?;
        if let Some(t0) = t_part {
            self.time_in_partition += t0.elapsed();
        }
        let n = wiring.partitioner.total_participants();
        let t_split = trace_on.then(std::time::Instant::now);
        let mut subs = split_batch_by_partition(&batch, &dests, n)?;
        if let Some(t0) = t_split {
            self.time_in_split += t0.elapsed();
        }

        // Self first (see run_shuffle_pump docstring for rationale).
        if let Some(self_sub) = subs[wiring.participant_index as usize].take() {
            self.rows_self += self_sub.num_rows() as u64;
            self.self_queue.push_back(self_sub);
        }
        let mut send_stats = crate::postgres::customscan::mpp::transport::SendBatchStats::default();
        for (dest_idx, sub) in subs.into_iter().enumerate() {
            if dest_idx as u32 == wiring.participant_index {
                debug_assert!(sub.is_none());
                continue;
            }
            let Some(sub) = sub else { continue };
            let sender = wiring.outbound_senders[dest_idx].as_ref().ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "ShuffleStream: no outbound sender for seat {dest_idx}"
                ))
            })?;
            let sub_rows = sub.num_rows() as u64;
            sender.send_batch_traced(&sub, &mut send_stats)?;
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

impl futures::Stream for ShuffleStream {
    type Item = datafusion::common::Result<RecordBatch>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        // Timing: record first-poll instant so EOF can report total lifetime.
        // Gated on the GUC so a non-traced run pays nothing beyond a branch.
        let trace_on = mpp_trace_flag();
        if trace_on && self.first_poll_at.is_none() {
            self.first_poll_at = Some(std::time::Instant::now());
        }
        // 1. Drain any queued self-partition batches first.
        if let Some(batch) = self.self_queue.pop_front() {
            return std::task::Poll::Ready(Some(Ok(batch)));
        }
        if self.done {
            return std::task::Poll::Ready(None);
        }

        // Proactive inbound drain: while we're producing + shipping, keep
        // consuming peer-shipped batches into our DrainBuffer. Without
        // this, a participant whose outbound sends succeed fast never
        // drains its inbound, peers' sends to us can't un-stall, and the
        // mesh deadlocks on asymmetric pressure (observed as a 120s
        // statement timeout on `aggregate_join_groupby - alternative 2`
        // at 25M rows). The DrainBuffer is consumed later by
        // `DrainGatherStream::poll_next` — this just keeps the pipe
        // moving in the meantime.
        if let Some(wiring) = self.wiring.as_ref() {
            if let Some(drain) = wiring.cooperative_drain.as_ref() {
                let t0 = trace_on.then(std::time::Instant::now);
                let _ = drain.poll_drain_pass();
                if let Some(t0) = t0 {
                    self.time_in_coop_drain += t0.elapsed();
                }
            }
        }

        loop {
            let t_child = trace_on.then(std::time::Instant::now);
            let res = futures::Stream::poll_next(std::pin::Pin::new(&mut self.child), cx);
            if let Some(t0) = t_child {
                self.time_in_child += t0.elapsed();
            }
            match res {
                std::task::Poll::Pending => return std::task::Poll::Pending,
                std::task::Poll::Ready(None) => {
                    // Child exhausted — drop senders to signal peer EOF.
                    self.wiring.take();
                    self.done = true;
                    self.log_eof();
                    return std::task::Poll::Ready(None);
                }
                std::task::Poll::Ready(Some(Err(e))) => {
                    self.wiring.take();
                    self.done = true;
                    self.log_eof();
                    return std::task::Poll::Ready(Some(Err(e)));
                }
                std::task::Poll::Ready(Some(Ok(batch))) => {
                    let t_proc = trace_on.then(std::time::Instant::now);
                    let result = self.process_batch(batch);
                    if let Some(t0) = t_proc {
                        self.time_in_process += t0.elapsed();
                    }
                    match result {
                        Ok(()) => {
                            if let Some(self_batch) = self.self_queue.pop_front() {
                                return std::task::Poll::Ready(Some(Ok(self_batch)));
                            }
                            // No self-partition in this batch; pull the next one.
                            continue;
                        }
                        Err(e) => {
                            self.wiring.take();
                            self.done = true;
                            self.log_eof();
                            return std::task::Poll::Ready(Some(Err(e)));
                        }
                    }
                }
            }
        }
    }
}

impl datafusion::execution::RecordBatchStream for ShuffleStream {
    fn schema(&self) -> datafusion::arrow::datatypes::SchemaRef {
        self.schema.clone()
    }
}

/// DataFusion `ExecutionPlan` that drains a
/// [`DrainBuffer`](crate::postgres::customscan::mpp::transport::DrainBuffer)
/// as a `SendableRecordBatchStream`. In the MPP topology, the drain buffer is
/// populated by the worker's drain thread reading peer-sent rows from inbound
/// `shm_mq` queues; `DrainGatherExec` hands those rows to a `UnionExec` above
/// it so they can merge with the local `ShuffleExec`'s self-partition output.
///
/// No child. Keeps the [`DrainHandle`] alive inside the operator so the
/// drain thread is cancelled + joined on plan tear-down (whether clean or
/// panic), closing the zombie-thread class of bugs the first review round
/// flagged.
pub struct DrainGatherExec {
    drain_handle: std::sync::Mutex<
        Option<std::sync::Arc<crate::postgres::customscan::mpp::transport::DrainHandle>>,
    >,
    schema: datafusion::arrow::datatypes::SchemaRef,
    plan_properties: std::sync::Arc<datafusion::physical_plan::PlanProperties>,
    /// Diagnostic label — same value the sibling `ShuffleExec` uses so a
    /// mesh's send-side and receive-side logs line up by tag.
    tag: &'static str,
    /// Remembered so `DrainGatherStream` can log which seat received.
    participant_index: u32,
}

impl std::fmt::Debug for DrainGatherExec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DrainGatherExec")
            .field("tag", &self.tag)
            .finish_non_exhaustive()
    }
}

impl DrainGatherExec {
    pub fn new(
        drain_handle: std::sync::Arc<crate::postgres::customscan::mpp::transport::DrainHandle>,
        schema: datafusion::arrow::datatypes::SchemaRef,
        tag: &'static str,
        participant_index: u32,
    ) -> Self {
        use datafusion::physical_expr::EquivalenceProperties;
        use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
        use datafusion::physical_plan::{Partitioning, PlanProperties};
        let eq = EquivalenceProperties::new(schema.clone());
        let plan_properties = std::sync::Arc::new(PlanProperties::new(
            eq,
            Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        ));
        Self {
            drain_handle: std::sync::Mutex::new(Some(drain_handle)),
            schema,
            plan_properties,
            tag,
            participant_index,
        }
    }
}

impl datafusion::physical_plan::DisplayAs for DrainGatherExec {
    fn fmt_as(
        &self,
        _t: datafusion::physical_plan::DisplayFormatType,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "DrainGatherExec")
    }
}

impl datafusion::physical_plan::ExecutionPlan for DrainGatherExec {
    fn name(&self) -> &str {
        "DrainGatherExec"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn properties(&self) -> &std::sync::Arc<datafusion::physical_plan::PlanProperties> {
        &self.plan_properties
    }

    fn children(&self) -> Vec<&std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
        vec![]
    }

    fn with_new_children(
        self: std::sync::Arc<Self>,
        children: Vec<std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>>,
    ) -> datafusion::common::Result<std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>>
    {
        if !children.is_empty() {
            return Err(DataFusionError::Internal(format!(
                "DrainGatherExec expects zero children, got {}",
                children.len()
            )));
        }
        Ok(self)
    }

    fn execute(
        &self,
        _partition: usize,
        _context: std::sync::Arc<datafusion::execution::TaskContext>,
    ) -> datafusion::common::Result<datafusion::execution::SendableRecordBatchStream> {
        let handle =
            self.drain_handle.lock().unwrap().take().ok_or_else(|| {
                DataFusionError::Internal("DrainGatherExec: already executed".into())
            })?;
        let stream = DrainGatherStream {
            handle: Some(handle),
            done: false,
            schema: self.schema.clone(),
            tag: self.tag,
            participant_index: self.participant_index,
            rows_received: 0,
            batches_received: 0,
            logged_eof: false,
            first_poll_at: None,
            time_in_drain_pass: std::time::Duration::ZERO,
            time_in_pop: std::time::Duration::ZERO,
            pending_polls: 0,
        };
        Ok(Box::pin(stream))
    }
}

struct DrainGatherStream {
    handle: Option<std::sync::Arc<crate::postgres::customscan::mpp::transport::DrainHandle>>,
    done: bool,
    schema: datafusion::arrow::datatypes::SchemaRef,
    tag: &'static str,
    participant_index: u32,
    rows_received: u64,
    batches_received: u64,
    logged_eof: bool,
    /// First-poll instant (gated on `mpp_trace`). Total stream lifetime at EOF.
    first_poll_at: Option<std::time::Instant>,
    /// Cumulative time in `poll_drain_pass` (shm_mq receive + IPC decode).
    time_in_drain_pass: std::time::Duration,
    /// Cumulative time in `poll_pop_front` on the drain buffer.
    time_in_pop: std::time::Duration,
    /// Count of times `poll_next` returned `Pending` (buffer empty). High
    /// counts with small throughput means we're re-waking without producing.
    pending_polls: u64,
}

impl DrainGatherStream {
    fn log_eof(&mut self) {
        if self.logged_eof {
            return;
        }
        self.logged_eof = true;
        // See sibling ShuffleStream::log_eof comment: `pgrx::{warning,debug1}!`
        // pull PG FFI symbols that aren't linked in the `--tests`/llvm-cov
        // build. Under `--instrument-coverage` DCE is disabled so gate the
        // whole body out of test builds. Tests never exercise this path.
        #[cfg(not(test))]
        {
            let wall_ms = self
                .first_poll_at
                .map(|t| t.elapsed().as_secs_f64() * 1000.0)
                .unwrap_or(0.0);
            let drain_ms = self.time_in_drain_pass.as_secs_f64() * 1000.0;
            let pop_ms = self.time_in_pop.as_secs_f64() * 1000.0;
            if crate::gucs::mpp_trace() {
                pgrx::warning!(
                    "mpp: DrainGatherStream[{}] seat={} EOF rows_received={} batches_received={} wall_ms={:.1} drain_ms={:.1} pop_ms={:.1} pending_polls={}",
                    self.tag,
                    self.participant_index,
                    self.rows_received,
                    self.batches_received,
                    wall_ms,
                    drain_ms,
                    pop_ms,
                    self.pending_polls,
                );
            } else {
                pgrx::debug1!(
                    "mpp: DrainGatherStream[{}] seat={} EOF rows_received={} batches_received={}",
                    self.tag,
                    self.participant_index,
                    self.rows_received,
                    self.batches_received
                );
            }
        }
    }
}

impl futures::Stream for DrainGatherStream {
    type Item = datafusion::common::Result<RecordBatch>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let trace_on = mpp_trace_flag();
        if trace_on && self.first_poll_at.is_none() {
            self.first_poll_at = Some(std::time::Instant::now());
        }
        if self.done {
            return std::task::Poll::Ready(None);
        }
        let handle = self
            .handle
            .as_ref()
            .expect("DrainGatherStream: handle missing before done");
        let buffer = handle.buffer().clone();

        // Cooperative drain: for pg-backed handles, the drain work happens
        // here on the backend thread (not a background thread, which would
        // panic on pgrx's `check_active_thread` the moment it touched pg
        // FFI via `shm_mq_receive`). One pass per poll pulls at most one
        // item from each live receiver into `buffer`; if nothing became
        // available this pass, we re-wake ourselves below so the tokio
        // runtime interleaves us with sibling tasks that might be producing
        // (e.g., the peer `ShuffleExec`s shipping rows via `shm_mq_send`).
        let coop = handle.is_cooperative();
        if coop {
            let t0 = trace_on.then(std::time::Instant::now);
            let res = handle.poll_drain_pass();
            if let Some(t0) = t0 {
                self.time_in_drain_pass += t0.elapsed();
            }
            match res {
                Ok(_) => {}
                Err(e) => {
                    self.done = true;
                    if let Some(h) = self.handle.take() {
                        let _ = h.shutdown();
                    }
                    return std::task::Poll::Ready(Some(Err(e)));
                }
            }
        }

        // Async-friendly pop: returns `None` and registers our waker if the
        // buffer is empty. For cooperative handles we also self-wake so the
        // next poll runs another drain pass — the buffer's waker only fires
        // when *this* pass produced data, which doesn't happen when every
        // receiver is still `Empty`.
        let t_pop = trace_on.then(std::time::Instant::now);
        let item_opt = buffer.poll_pop_front(cx.waker());
        if let Some(t0) = t_pop {
            self.time_in_pop += t0.elapsed();
        }
        let Some(item) = item_opt else {
            if trace_on {
                self.pending_polls += 1;
            }
            if coop {
                cx.waker().wake_by_ref();
            }
            return std::task::Poll::Pending;
        };

        match item {
            crate::postgres::customscan::mpp::transport::DrainItem::Batch(b) => {
                // Validate schema: peer-shipped batches went through
                // `encode_batch` → `decode_batch`, whose reconstructed
                // schema comes from IPC metadata, not from what we expected
                // locally. A peer running a drifted schema would otherwise
                // silently feed garbage into the Union above us.
                if b.schema() != self.schema {
                    self.done = true;
                    self.log_eof();
                    if let Some(h) = self.handle.take() {
                        let _ = h.shutdown();
                    }
                    return std::task::Poll::Ready(Some(Err(DataFusionError::Internal(format!(
                        "DrainGatherExec: peer batch schema {:?} disagrees with expected {:?}",
                        b.schema(),
                        self.schema
                    )))));
                }
                self.rows_received += b.num_rows() as u64;
                self.batches_received += 1;
                std::task::Poll::Ready(Some(Ok(b)))
            }
            crate::postgres::customscan::mpp::transport::DrainItem::Eof => {
                self.done = true;
                self.log_eof();
                // Join the drain thread deterministically so the plan's
                // teardown path never leaves a zombie thread holding DSM
                // pointers.
                if let Some(h) = self.handle.take() {
                    if let Err(e) = h.shutdown() {
                        return std::task::Poll::Ready(Some(Err(e)));
                    }
                }
                std::task::Poll::Ready(None)
            }
        }
    }
}

impl datafusion::execution::RecordBatchStream for DrainGatherStream {
    fn schema(&self) -> datafusion::arrow::datatypes::SchemaRef {
        self.schema.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::customscan::mpp::transport::{
        in_proc_channel, DrainBuffer, DrainItem, MppReceiver, MppSender,
    };
    use datafusion::arrow::array::{Int32Array, StringArray};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use std::sync::Arc;

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
            None,                               // seat 0 = self, no sender
            Some(MppSender::new(Box::new(tx))), // seat 1 = peer
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
        let drain_handle = crate::postgres::customscan::mpp::transport::DrainHandle::spawn(
            crate::postgres::customscan::mpp::transport::DrainConfig::new(
                vec![receiver],
                Arc::clone(&buffer),
            ),
        );

        let mut peer_batches = Vec::new();
        while let DrainItem::Batch(b) = buffer.pop_front() {
            peer_batches.push(b);
        }
        drain_handle.shutdown().unwrap();

        (self_batches, peer_batches)
    }

    #[test]
    fn pump_routes_self_and_peer_partitions_end_to_end() {
        // Row i -> seat (i % 2). With 6 input rows: self gets 0,2,4; peer
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
        // Seat 1 should route rows to a peer, but there's no MppSender.
        // Must fail with a meaningful error rather than silently drop data.
        let partitioner = ModuloPartitioner::new(2);
        let senders: Vec<Option<MppSender>> = vec![None, None];
        let result = run_shuffle_pump(
            vec![Ok(sample_batch(2))], // row 0 -> seat 0 (self), row 1 -> seat 1 (peer)
            &partitioner,
            senders,
            0,
            |_| Ok(()),
        );
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("no outbound sender for seat 1"), "got: {msg}");
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
    fn shuffle_exec_emits_only_self_partition() {
        use datafusion::datasource::memory::MemorySourceConfig;
        use datafusion::prelude::SessionContext;
        use futures::StreamExt;

        // N=2 mesh as participant 0. Row i -> seat i % 2: self gets
        // 0,2,4,6,8; peer gets 1,3,5,7,9.
        let batch = sample_batch(10);
        let schema = batch.schema();
        let input = MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch]).unwrap();

        let (tx, rx) = in_proc_channel(32);
        let wiring = ShuffleWiring {
            partitioner: Arc::new(ModuloPartitioner::new(2)),
            outbound_senders: vec![None, Some(MppSender::new(Box::new(tx)))],
            participant_index: 0,
            cooperative_drain: None,
        };
        let shuffle: Arc<dyn datafusion::physical_plan::ExecutionPlan> =
            Arc::new(ShuffleExec::new(input, wiring, "test"));

        // Run the output stream to completion.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ctx = SessionContext::new();
        let self_batches: Vec<RecordBatch> = rt.block_on(async {
            let mut stream = shuffle.execute(0, ctx.task_ctx()).unwrap();
            let mut out = Vec::new();
            while let Some(item) = stream.next().await {
                out.push(item.unwrap());
            }
            out
        });

        // Self-partition output: row IDs should be 0, 2, 4, 6, 8.
        let self_ids: Vec<i32> = self_batches
            .iter()
            .flat_map(|b| {
                b.column(0)
                    .as_any()
                    .downcast_ref::<Int32Array>()
                    .unwrap()
                    .values()
                    .to_vec()
            })
            .collect();
        assert_eq!(self_ids, vec![0, 2, 4, 6, 8]);

        // Peer side: drain the channel and confirm we got 1, 3, 5, 7, 9.
        let receiver = MppReceiver::new(Box::new(rx));
        let buf = DrainBuffer::new(1);
        let handle = crate::postgres::customscan::mpp::transport::DrainHandle::spawn(
            crate::postgres::customscan::mpp::transport::DrainConfig::new(
                vec![receiver],
                Arc::clone(&buf),
            ),
        );
        let mut peer_ids: Vec<i32> = Vec::new();
        while let DrainItem::Batch(b) = buf.pop_front() {
            peer_ids.extend(
                b.column(0)
                    .as_any()
                    .downcast_ref::<Int32Array>()
                    .unwrap()
                    .values()
                    .iter()
                    .copied(),
            );
        }
        handle.shutdown().unwrap();
        assert_eq!(peer_ids, vec![1, 3, 5, 7, 9]);
    }

    #[test]
    fn mpp_data_path_end_to_end_via_union() {
        // Full milestone-1 MPP data path in one process, no PG:
        //
        //   MemorySourceConfig (10 rows)
        //     -> ShuffleExec (self-partition output)
        //                                   \
        //   simulated-peer-sender           UnionExec -> collected output
        //     -> DrainHandle                /
        //     -> DrainGatherExec (peer-partition output)
        //
        // At participant 0 with ModuloPartitioner(N=2), ShuffleExec yields
        // IDs {0,2,4,6,8}. A simulated peer pushes synthetic batches into
        // the inbound drain (IDs 100, 200) before dropping the sender.
        // UnionExec emits 5 + 2 = 7 rows. We assert on the exact ID
        // multiset, with the peer side explicitly arriving *after* self
        // because Union preserves child order.
        use datafusion::datasource::memory::MemorySourceConfig;
        use datafusion::physical_plan::union::UnionExec;
        use datafusion::physical_plan::ExecutionPlanProperties;
        use datafusion::prelude::SessionContext;
        use futures::StreamExt;

        let batch = sample_batch(10);
        let schema = batch.schema();
        let input = MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch]).unwrap();

        let (out_tx, out_rx) = in_proc_channel(16); // outbound to peer
        let (in_tx, in_rx) = in_proc_channel(16); // inbound from peer

        // Our ShuffleExec at seat 0 in an N=2 mesh. Peer = seat 1.
        let wiring = ShuffleWiring {
            partitioner: Arc::new(ModuloPartitioner::new(2)),
            outbound_senders: vec![None, Some(MppSender::new(Box::new(out_tx)))],
            participant_index: 0,
            cooperative_drain: None,
        };
        let shuffle: Arc<dyn datafusion::physical_plan::ExecutionPlan> =
            Arc::new(ShuffleExec::new(input, wiring, "test"));

        // Simulated peer: spawn a thread that pushes two synthetic batches
        // into `in_tx` with a small delay between them so the buffer is
        // *empty* when the main thread first polls — exercising the
        // waker-based `Poll::Pending` path. If we pre-joined this thread
        // (as an earlier version did), batches would all be buffered
        // before the stream ran and the async contract would be untested.
        let peer_schema = schema.clone();
        let peer_thread = std::thread::spawn(move || {
            let sender = MppSender::new(Box::new(in_tx));
            for (id, name) in [(100i32, "peer100"), (200i32, "peer200")] {
                std::thread::sleep(std::time::Duration::from_millis(10));
                let batch = RecordBatch::try_new(
                    peer_schema.clone(),
                    vec![
                        Arc::new(Int32Array::from_iter_values([id])),
                        Arc::new(StringArray::from_iter_values([name.to_string()])),
                    ],
                )
                .unwrap();
                sender.send_batch(&batch).unwrap();
            }
            // `sender` drops here → peer receiver observes detach → EOF.
        });

        // DrainHandle draining the inbound channel into a DrainBuffer.
        let receiver = MppReceiver::new(Box::new(in_rx));
        let drain_buffer = DrainBuffer::new(1);
        let drain_handle = crate::postgres::customscan::mpp::transport::DrainHandle::spawn(
            crate::postgres::customscan::mpp::transport::DrainConfig::new(
                vec![receiver],
                Arc::clone(&drain_buffer),
            ),
        );
        let gather: Arc<dyn datafusion::physical_plan::ExecutionPlan> = Arc::new(
            DrainGatherExec::new(Arc::new(drain_handle), schema, "test", 0),
        );

        // Union: ShuffleExec emits first, then DrainGatherExec (Union
        // preserves child order).
        let union: Arc<dyn datafusion::physical_plan::ExecutionPlan> =
            UnionExec::try_new(vec![shuffle, gather]).unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ctx = SessionContext::new();

        let mut all_ids = Vec::new();
        rt.block_on(async {
            // UnionExec reports output partitioning = N children; iterate
            // each partition's stream in order.
            let n_partitions = union.output_partitioning().partition_count();
            for p in 0..n_partitions {
                let mut s = union.execute(p, ctx.task_ctx()).unwrap();
                while let Some(b) = s.next().await {
                    let b = b.unwrap();
                    let ids = b.column(0).as_any().downcast_ref::<Int32Array>().unwrap();
                    all_ids.extend(ids.values().iter().copied());
                }
            }
        });

        // Collect the outbound side into a second DrainBuffer just so we
        // verify the ShuffleExec did, in fact, ship its peer rows.
        let receiver = MppReceiver::new(Box::new(out_rx));
        let out_buffer = DrainBuffer::new(1);
        let out_handle = crate::postgres::customscan::mpp::transport::DrainHandle::spawn(
            crate::postgres::customscan::mpp::transport::DrainConfig::new(
                vec![receiver],
                Arc::clone(&out_buffer),
            ),
        );
        let mut outbound_ids = Vec::new();
        while let DrainItem::Batch(b) = out_buffer.pop_front() {
            outbound_ids.extend(
                b.column(0)
                    .as_any()
                    .downcast_ref::<Int32Array>()
                    .unwrap()
                    .values()
                    .iter()
                    .copied(),
            );
        }
        out_handle.shutdown().unwrap();

        peer_thread.join().unwrap();

        // Assertions:
        // 1. Union yielded self-partition (0,2,4,6,8) + peer-simulated (100, 200)
        all_ids.sort();
        assert_eq!(all_ids, vec![0, 2, 4, 6, 8, 100, 200]);
        // 2. ShuffleExec correctly shipped odd IDs to peer
        outbound_ids.sort();
        assert_eq!(outbound_ids, vec![1, 3, 5, 7, 9]);
    }

    #[test]
    fn drain_gather_exec_yields_eof_on_empty_peer() {
        // Peer immediately drops the sender without shipping anything.
        // DrainGatherExec must yield `None` cleanly via the waker path.
        use datafusion::prelude::SessionContext;
        use futures::StreamExt;

        let (tx, rx) = in_proc_channel(4);
        drop(MppSender::new(Box::new(tx))); // immediate EOF

        let receiver = MppReceiver::new(Box::new(rx));
        let buffer = DrainBuffer::new(1);
        let handle = crate::postgres::customscan::mpp::transport::DrainHandle::spawn(
            crate::postgres::customscan::mpp::transport::DrainConfig::new(
                vec![receiver],
                Arc::clone(&buffer),
            ),
        );

        let schema = sample_batch(1).schema();
        let gather: Arc<dyn datafusion::physical_plan::ExecutionPlan> =
            Arc::new(DrainGatherExec::new(Arc::new(handle), schema, "test", 0));

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ctx = SessionContext::new();
        let count: usize = rt.block_on(async {
            let mut stream = gather.execute(0, ctx.task_ctx()).unwrap();
            let mut c = 0;
            while let Some(_b) = stream.next().await {
                c += 1;
            }
            c
        });
        assert_eq!(count, 0, "empty peer should yield zero batches");
    }

    #[test]
    fn drain_gather_exec_rejects_schema_mismatch() {
        use datafusion::arrow::array::Int64Array;
        use datafusion::prelude::SessionContext;
        use futures::StreamExt;

        // Peer ships a batch with an Int64 id column; the DrainGatherExec
        // was told the schema has Int32. The stream must surface an error
        // rather than silently emit the mismatched batch.
        let peer_schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));
        let (tx, rx) = in_proc_channel(4);
        let sender = MppSender::new(Box::new(tx));
        let batch = RecordBatch::try_new(
            peer_schema,
            vec![Arc::new(Int64Array::from_iter_values([42i64]))],
        )
        .unwrap();
        sender.send_batch(&batch).unwrap();
        drop(sender);

        let receiver = MppReceiver::new(Box::new(rx));
        let buffer = DrainBuffer::new(1);
        let handle = crate::postgres::customscan::mpp::transport::DrainHandle::spawn(
            crate::postgres::customscan::mpp::transport::DrainConfig::new(
                vec![receiver],
                Arc::clone(&buffer),
            ),
        );

        let expected_schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let gather: Arc<dyn datafusion::physical_plan::ExecutionPlan> = Arc::new(
            DrainGatherExec::new(Arc::new(handle), expected_schema, "test", 0),
        );

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ctx = SessionContext::new();
        let got_error = rt.block_on(async {
            let mut stream = gather.execute(0, ctx.task_ctx()).unwrap();
            matches!(stream.next().await, Some(Err(_)))
        });
        assert!(
            got_error,
            "DrainGatherExec must reject schema-mismatched batches"
        );
    }

    #[test]
    fn drain_gather_exec_yields_all_buffered_batches() {
        use datafusion::prelude::SessionContext;
        use futures::StreamExt;

        // Set up a drain handle whose drain thread reads from an in-proc
        // channel. Push 3 batches through the channel, then drop the sender
        // to signal EOF. DrainGatherExec should yield exactly those 3
        // batches.
        let (tx, rx) = in_proc_channel(8);
        let receiver = MppReceiver::new(Box::new(rx));
        let buffer = DrainBuffer::new(1);
        let handle = crate::postgres::customscan::mpp::transport::DrainHandle::spawn(
            crate::postgres::customscan::mpp::transport::DrainConfig::new(
                vec![receiver],
                Arc::clone(&buffer),
            ),
        );

        // Sender feeds 3 sample batches then drops (→ EOF).
        let sender = MppSender::new(Box::new(tx));
        for rows in [2, 3, 5] {
            sender.send_batch(&sample_batch(rows)).unwrap();
        }
        drop(sender);

        let schema = sample_batch(1).schema();
        let gather: Arc<dyn datafusion::physical_plan::ExecutionPlan> =
            Arc::new(DrainGatherExec::new(Arc::new(handle), schema, "test", 0));

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ctx = SessionContext::new();
        let totals: Vec<usize> = rt.block_on(async {
            let mut stream = gather.execute(0, ctx.task_ctx()).unwrap();
            let mut v = Vec::new();
            while let Some(b) = stream.next().await {
                v.push(b.unwrap().num_rows());
            }
            v
        });
        assert_eq!(totals, vec![2, 3, 5]);
    }

    #[test]
    fn shuffle_exec_propagates_child_errors() {
        use datafusion::arrow::array::RecordBatch as ArrowBatch;
        use datafusion::prelude::SessionContext;
        use futures::StreamExt;

        // Custom child that errors on first poll.
        #[derive(Debug)]
        struct ErroringExec {
            schema: datafusion::arrow::datatypes::SchemaRef,
            properties: std::sync::Arc<datafusion::physical_plan::PlanProperties>,
        }

        impl datafusion::physical_plan::DisplayAs for ErroringExec {
            fn fmt_as(
                &self,
                _t: datafusion::physical_plan::DisplayFormatType,
                f: &mut std::fmt::Formatter<'_>,
            ) -> std::fmt::Result {
                write!(f, "ErroringExec")
            }
        }

        impl datafusion::physical_plan::ExecutionPlan for ErroringExec {
            fn name(&self) -> &str {
                "ErroringExec"
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn properties(&self) -> &std::sync::Arc<datafusion::physical_plan::PlanProperties> {
                &self.properties
            }
            fn children(
                &self,
            ) -> Vec<&std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
                vec![]
            }
            fn with_new_children(
                self: std::sync::Arc<Self>,
                _children: Vec<std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>>,
            ) -> datafusion::common::Result<
                std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>,
            > {
                Ok(self)
            }
            fn execute(
                &self,
                _partition: usize,
                _context: std::sync::Arc<datafusion::execution::TaskContext>,
            ) -> datafusion::common::Result<datafusion::execution::SendableRecordBatchStream>
            {
                let schema = self.schema.clone();
                let stream = futures::stream::once(async move {
                    Err::<ArrowBatch, _>(DataFusionError::Execution("synthetic".into()))
                });
                Ok(Box::pin(
                    datafusion::physical_plan::stream::RecordBatchStreamAdapter::new(
                        schema, stream,
                    ),
                ))
            }
        }

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let input_exec: Arc<dyn datafusion::physical_plan::ExecutionPlan> = {
            use datafusion::physical_expr::EquivalenceProperties;
            use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
            use datafusion::physical_plan::{Partitioning, PlanProperties};
            let props = std::sync::Arc::new(PlanProperties::new(
                EquivalenceProperties::new(schema.clone()),
                Partitioning::UnknownPartitioning(1),
                EmissionType::Incremental,
                Boundedness::Bounded,
            ));
            Arc::new(ErroringExec {
                schema: schema.clone(),
                properties: props,
            })
        };

        let (tx, _rx) = in_proc_channel(4);
        let wiring = ShuffleWiring {
            partitioner: Arc::new(ModuloPartitioner::new(2)),
            outbound_senders: vec![None, Some(MppSender::new(Box::new(tx)))],
            participant_index: 0,
            cooperative_drain: None,
        };
        let shuffle: Arc<dyn datafusion::physical_plan::ExecutionPlan> =
            Arc::new(ShuffleExec::new(input_exec, wiring, "test"));

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ctx = SessionContext::new();
        let got_error = rt.block_on(async {
            let mut stream = shuffle.execute(0, ctx.task_ctx()).unwrap();
            matches!(stream.next().await, Some(Err(_)))
        });
        assert!(got_error, "ShuffleExec must propagate child errors");
    }

    #[test]
    fn hash_partitioner_distributes_across_seats() {
        // Over 1000 rows with 4 seats, every seat should get at least some.
        // This is a statistical claim that should hold for any non-adversarial
        // hash.
        let batch = sample_batch(1000);
        let p = HashPartitioner::new(vec![0], 4);
        let dests = p.partition_for_each_row(&batch).unwrap();
        let mut counts = [0usize; 4];
        for d in &dests {
            counts[*d as usize] += 1;
        }
        for (seat, count) in counts.iter().enumerate() {
            assert!(
                *count > 100,
                "seat {seat} only received {count}/1000 rows — partitioner is skewed"
            );
        }
    }
}

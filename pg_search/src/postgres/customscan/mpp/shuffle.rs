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
//! Carries both the pure-function primitives and the DataFusion operator
//! that composes them. [`MppRepartitionExec`] is the analogue of DF's
//! `RepartitionExec` for cross-participant shuffles: per execute() it
//! reads the local child, hash-routes rows across the mesh via
//! `MppSender`s, and yields the local participant's slice — including
//! rows shipped to us by peers via the inbound `DrainHandle`. Earlier
//! drafts split this into `MppRepartitionExec` (self-route) +
//! `DrainGatherExec` (peer drain) joined under
//! `CoalescePartitionsExec(UnionExec(...))`; PR #4828's DF-partition
//! alignment folded both halves into a single operator so the output
//! is a single stream that already contains both, matching DF's
//! `RepartitionExec` shape.
//!
//! Primitives:
//!
//! - [`RowPartitioner`] is the trait `MppRepartitionExec` calls per batch to decide
//!   which destination participant each row belongs to.
//! - [`HashPartitioner`] is the production impl — a hash over join key columns
//!   modulo N participants. Uses DataFusion's `create_hashes` helper with a
//!   seed that matches DataFusion's own `REPARTITION_RANDOM_STATE`
//!   (`datafusion::physical_plan::repartition` uses `with_seeds(0,0,0,0)`).
//!   Routing is therefore stable across workers — every worker that sees the
//!   same input rows places them on the same participant. Downstream `HashJoinExec`
//!   and `AggregateExec` intentionally use *different* seeds internally (see
//!   their `HASH_JOIN_SEED` / `AGGREGATION_HASH_SEED` constants) — that
//!   keeps routing distribution and internal hash-table distribution
//!   decoupled, and has no bearing on our shuffle correctness.
//! - `tests::ModuloPartitioner` is the test-only variant that routes row
//!   `i` to destination `i % N`, so tests can verify routing without
//!   committing to a specific hash output. Lives in this file's `tests`
//!   submodule and is reused by `plan_build.rs`'s tests via the same
//!   path.
//! - [`split_batch_by_partition`] is the row-scatter step: given a batch and
//!   its per-row destination vector, return one sub-batch per destination,
//!   preserving row order within each destination. `MppRepartitionExec`'s producer
//!   loop calls this once per input batch, then feeds each sub-batch to its
//!   corresponding `MppSender` (peers) or local channel (self).
//!
//! TODO(arch): the "destination participant" routing here is parallel to,
//! but distinct from, DataFusion's own `partition` concept.
//! `datafusion-distributed` takes a different approach: it lets each
//! `MppRepartitionExec` expose N output partitions (one per participant) and
//! uses `PartitionIsolatorExec` to select which partition a worker reads.
//! That alignment with native DF partitions has real upside (every DF
//! optimizer rule that reasons about partitioning would Just Work). The
//! current code prefers the row-routing model because it keeps
//! `MppRepartitionExec` shaped like a single-output operator (matches
//! ParadeDB's pre-existing custom-scan single-partition assumption) and
//! avoids needing a `PartitionIsolatorExec` analogue everywhere we
//! reference a participant index. Re-evaluating once the rest of the
//! MPP stack settles is on the table; tracked separately so a future
//! reviewer can pick it up without re-deriving the trade-off.

use std::any::Any;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use datafusion::arrow::array::{RecordBatch, UInt64Array};
use datafusion::arrow::compute::take;
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::common::hash_utils::create_hashes;
use datafusion::common::{DataFusionError, Result as DFResult};
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties,
};
use futures::Stream;
use tokio::task::yield_now;

#[cfg(not(test))]
use crate::gucs::mpp_trace;
use crate::postgres::customscan::mpp::stage::{MppNetworkBoundary, MppStage};
use crate::postgres::customscan::mpp::transport::{
    DrainHandle, DrainItem, MppSender, SendBatchStats,
};

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
/// `0..total_participants`. `MppRepartitionExec` uses this at the top of its
/// producer loop to fan rows out across the mesh.
pub trait RowPartitioner: Send + Sync {
    /// Return a destination index in `0..total_participants` for every row.
    fn partition_for_each_row(&self, batch: &RecordBatch) -> Result<Vec<u32>, DataFusionError>;

    /// Total destinations this partitioner will target. Used by
    /// `MppRepartitionExec` to size per-destination scratch arrays.
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
/// NULL keys hash to a fixed sentinel inside `create_hashes`, so every row
/// with a NULL in any key column routes to the same destination. That is
/// correct for join/aggregate purposes — the receiving HashJoin/Aggregate
/// also clusters NULL-key rows together and applies SQL NULL semantics
/// from there — but it means a heavily-NULL key column produces skewed
/// destination distribution. Diagnose with `mpp_trace`'s per-peer
/// `rows_sent` counters if a hot peer shows up.
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

/// Route every row to one fixed destination participant. Used by the MPP scalar-
/// aggregate final-gather step: workers ship their Partial row to participant 0 so
/// the leader can run `AggregateExec(FinalPartitioned)` on the combined
/// stream. Leader's self-partition is the target participant, so nothing is shipped
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

/// Concatenate destination sub-batches back into one RecordBatch.
///
/// Wiring passed to `MppRepartitionExec::new` that describes how this participant
/// connects to the mesh. Moved into the operator at construction time; the
/// operator owns the senders for the query's lifetime and drops them at
/// stream EOF / abort so peer receivers cleanly observe detach.
pub struct ShuffleWiring {
    pub partitioner: Arc<dyn RowPartitioner>,
    /// One slot per participant, length = `partitioner.total_participants()`.
    /// Participant `participant_index` must be `None`; all other participants must be
    /// `Some(MppSender)`. `MppRepartitionExec::new` asserts this invariant.
    pub outbound_senders: Vec<Option<MppSender>>,
    pub participant_index: u32,
    /// Our inbound-side drain handle for the same mesh. When set,
    /// `build_mpp_repartition_stream` proactively calls
    /// `DrainHandle::poll_drain_pass` every iteration so our inbound
    /// queues keep draining even when our outbound sends aren't
    /// blocking. Without this, a participant whose sends succeed fast
    /// never drains, peers can't ship to it, and the mesh deadlocks on
    /// one-sided pressure (observed as a 120s timeout on
    /// `aggregate_join_groupby - alternative 2` at 25M rows).
    pub cooperative_drain: Option<Arc<DrainHandle>>,
}

/// DataFusion `ExecutionPlan` that hash-routes every input row across an
/// MPP mesh and yields the local participant's slice. Output is the
/// union of:
///
///  * rows of `child` whose hash lands on this participant
///    (self-routed via `wiring.partitioner`), and
///  * rows shipped to us by peers, decoded by the drain thread into the
///    `DrainBuffer` referenced by `wiring.cooperative_drain`.
///
/// Both streams are merged inside the operator's `execute()` body — the
/// caller sees a single `SendableRecordBatchStream`. This is the
/// MPP-side analogue of DF's `RepartitionExec`: same N-way routing
/// semantics, same single-stream output, with the cross-process
/// transport handled by the `MppSender` / `DrainHandle` pair attached
/// via [`ShuffleWiring`].
///
/// The wiring (partitioner + senders) is taken on `new` and handed to the
/// first `execute()` call via a `Mutex<Option<_>>`. A second `execute()` call
/// would require re-planning the query and re-attaching a fresh mesh — not
/// supported today; the `Option` returns an error on re-execute.
///
/// The wiring is also one-shot with respect to `with_new_children`: an
/// optimizer rule that calls `with_new_children` *moves* the wiring into
/// the freshly-built `MppRepartitionExec`, leaving the original instance in a
/// "wiring-consumed" state where any subsequent `execute()` returns an
/// error. `MppSender` is not `Clone`, so duplicating the wiring is not an
/// option. In practice DataFusion's optimizer rewrites the plan top-down
/// and drops the old node before any caller could `execute` it; this
/// note exists so a future planner change that retains references to the
/// old plan fails loudly with the right error rather than reaching some
/// later "already executed" branch.
pub struct MppRepartitionExec {
    input: Arc<dyn ExecutionPlan>,
    wiring: Mutex<Option<ShuffleWiring>>,
    plan_properties: Arc<PlanProperties>,
    /// Diagnostic label — e.g. "left", "right", "postagg", "final". Echoed
    /// in the row-count `mpp_log!` line emitted at stream EOF. Purely for
    /// tracing; no control flow depends on it.
    tag: &'static str,
    /// Copy of `wiring.participant_index` so `execute(partition)` can decide
    /// whether to return the real stream (when `partition == participant_index`)
    /// or an empty stream (when another partition is asked for) without
    /// taking the one-shot wiring out of its slot.
    participant_index: u32,
    /// Stage this boundary consumes from. `None` on construction; stamped by
    /// [`MppNetworkBoundary::with_input_stage`].
    /// P1 seam only — read only by tests today.
    #[allow(dead_code)]
    input_stage: Option<MppStage>,
}

impl fmt::Debug for MppRepartitionExec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MppRepartitionExec")
            .field("input", &self.input)
            .field("tag", &self.tag)
            .finish_non_exhaustive()
    }
}

impl MppRepartitionExec {
    pub fn new(input: Arc<dyn ExecutionPlan>, wiring: ShuffleWiring, tag: &'static str) -> Self {
        let n = wiring.partitioner.total_participants();
        assert_eq!(
            wiring.outbound_senders.len(),
            n as usize,
            "MppRepartitionExec: outbound_senders.len() != total_participants"
        );
        assert!(
            wiring.participant_index < n,
            "MppRepartitionExec: participant_index >= total_participants"
        );
        assert!(
            wiring.outbound_senders[wiring.participant_index as usize].is_none(),
            "MppRepartitionExec: self participant sender slot must be None"
        );

        let eq_properties = EquivalenceProperties::new(input.schema());
        // `UnknownPartitioning(1)` is intentional even though the rows
        // emerging from the shuffle *are* hash-partitioned by
        // `wiring.partitioner.key_columns`. MPP plans are hand-built —
        // there is no `EnforceDistribution` rule running over them — so
        // declaring `Partitioning::Hash(...)` would buy us nothing and
        // would force us to materialise `Arc<dyn PhysicalExpr>`s for the
        // key columns just to satisfy the API. If the planner ever runs
        // `EnforceDistribution` over an MPP plan, revisit this so a
        // downstream `HashJoinExec(Partitioned)` doesn't insert a
        // redundant `RepartitionExec` on top of us.
        let plan_properties = Arc::new(PlanProperties::new(
            eq_properties,
            Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        ));

        Self {
            input,
            wiring: Mutex::new(Some(wiring)),
            plan_properties,
            tag,
            participant_index,
            input_stage: None,
        }
    }
}

impl MppNetworkBoundary for MppRepartitionExec {
    fn input_stage(&self) -> Option<&MppStage> {
        self.input_stage.as_ref()
    }

    fn with_input_stage(&self, stage: MppStage) -> DFResult<Arc<dyn ExecutionPlan>> {
        let wiring = self.wiring.lock().unwrap().take().ok_or_else(|| {
            DataFusionError::Internal(
                "MppRepartitionExec::with_input_stage: wiring already consumed".into(),
            )
        })?;
        let mut node = MppRepartitionExec::new(self.input.clone(), wiring, self.tag);
        node.input_stage = Some(stage);
        Ok(Arc::new(node))
    }
}

impl DisplayAs for MppRepartitionExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MppRepartitionExec")
    }
}

impl ExecutionPlan for MppRepartitionExec {
    fn name(&self) -> &str {
        "MppRepartitionExec"
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
                "MppRepartitionExec expects exactly one child, got {}",
                children.len()
            )));
        }
        // Cloning the wiring isn't possible (MppSender isn't Clone). This path
        // is hit by some optimizer rules that re-wire children; those rules
        // fire before we attach the wiring, so `None` means re-planning.
        let wiring = self.wiring.lock().unwrap().take();
        let Some(wiring) = wiring else {
            return Err(DataFusionError::Internal(
                "MppRepartitionExec: with_new_children called after wiring was consumed".into(),
            ));
        };
        let mut node = MppRepartitionExec::new(children.into_iter().next().unwrap(), wiring, self.tag);
        node.input_stage = self.input_stage;
        Ok(Arc::new(node))
    }

    fn execute(
        &self,
        _partition: usize,
        context: Arc<TaskContext>,
    ) -> DFResult<SendableRecordBatchStream> {
        let wiring = self.wiring.lock().unwrap().take().ok_or_else(|| {
            DataFusionError::Internal("MppRepartitionExec: already executed".into())
        })?;
        let child_stream = self.input.execute(0, context)?;
        let schema = self.input.schema();
        let stream = build_mpp_repartition_stream(child_stream, wiring, self.tag, schema.clone());
        Ok(Box::pin(RecordBatchStreamAdapter::new(schema, stream)))
    }
}

/// Mutable per-stream state for [`build_mpp_repartition_stream`]. Kept as a
/// struct (rather than scattered locals) so [`process_batch`] can mutate
/// row counters, push to the self-queue, and accumulate trace metrics
/// in one place.
///
/// Trace fields are only consumed by [`log_shuffle_eof`], whose body
/// is gated to `cfg(not(test))`. Under `cfg(test)` rustc sees no reader
/// for the fields and trips `dead_code`; the `cfg_attr` silences that
/// for the unit-test build only.
#[cfg_attr(test, allow(dead_code))]
struct ShuffleState {
    /// Set on entry; taken to `None` once the child is exhausted (or
    /// errors mid-stream) so peer senders detach and signal EOF.
    wiring: Option<ShuffleWiring>,
    /// Sub-batch destined for this participant, queued for the next
    /// `yield` from the outer loop. At most one entry: each input batch
    /// produces a single self-share (or none), drained before pulling
    /// another child batch.
    self_queue: Option<RecordBatch>,
    participant_index: u32,
    rows_in: u64,
    rows_self: u64,
    /// Per-destination row counts for the EOF trace. Indexed by
    /// destination participant; entry at `participant_index` stays 0.
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

/// Build the async stream that powers [`MppRepartitionExec`]. Pulls batches from
/// `child`, partitions each by row, ships peer-shares via the `MppSender`s
/// in `wiring`, and yields the local participant's share back up the plan.
///
/// The body interleaves four concerns on a single task: pulling from
/// `child`, shipping outbound via `shm_mq_send`, pumping the cooperative
/// inbound drain so peer `MppSender`s keep flushing into our `DrainBuffer`,
/// and yielding peer-shipped batches that the drain has already
/// delivered. The drain is synchronous because `shm_mq` has no async
/// readiness signal; an `async-stream` body shaped around it (rather
/// than a `select!` over async sources) keeps the cooperative step
/// explicit.
///
/// Output is the union of:
///
///   * rows of `child` whose hash lands on this participant
///     (self-routed via `partitioner.partition_for_each_row`), and
///   * rows shipped to us by peers, decoded by the drain thread into
///     `DrainBuffer` and pulled out here in arrival order.
///
/// The two streams used to live in separate operators
/// (`MppRepartitionExec` + `DrainGatherExec`) joined by
/// `CoalescePartitionsExec(UnionExec(...))`. They were folded into one
/// body so the operator declares a single stream that already contains
/// both halves — the natural shape for a DataFusion `RepartitionExec`
/// analogue.
fn build_mpp_repartition_stream(
    mut child: SendableRecordBatchStream,
    wiring: ShuffleWiring,
    tag: &'static str,
    schema: SchemaRef,
) -> impl Stream<Item = DFResult<RecordBatch>> + Send {
    use futures::StreamExt;

    let drain_handle = wiring.cooperative_drain.clone();
    let participant_index = wiring.participant_index;

    async_stream::stream! {
        // RAII guard: shutdown the drain handle if the consumer drops the
        // stream early (LIMIT shortcut, parent cancel, panic). Without
        // this, peer `MppRepartitionExec`s holding `Arc<DrainHandle>` clones
        // via their `MppSender::cooperative_drain` would keep calling
        // `poll_drain_pass`, pushing peer batches into a buffer no one
        // reads — an unbounded memory-growth path on cancel.
        let _shutdown_guard = drain_handle.as_ref().map(|h| ShutdownOnDrop { handle: h.clone() });

        let trace_on = mpp_trace_flag();
        let first_poll_at = trace_on.then(Instant::now);
        let mut time_in_child = Duration::ZERO;
        let mut time_in_process = Duration::ZERO;
        let mut time_in_coop_drain = Duration::ZERO;
        let mut time_in_drain_pop = Duration::ZERO;
        let mut rows_received_from_peers: u64 = 0;
        let mut batches_received_from_peers: u64 = 0;
        let mut state = ShuffleState::new(wiring);
        let mut child_done = false;
        // No drain handle ⇒ no peers to receive from (single-participant
        // case, or tests that skipped the inbound side).
        let mut drain_eof = drain_handle.is_none();

        let outcome: DFResult<()> = 'pump: loop {
            // 1. Yield any queued self-partition batch from a previous
            //    `process_batch`. Each yield re-enters the loop, so the
            //    drain pop + drain pass below run again before we pull
            //    another child batch.
            if let Some(b) = state.self_queue.take() {
                yield Ok(b);
                continue 'pump;
            }

            // 2. Yield a peer-shipped batch if the drain has one queued.
            //    Schema-check it: a peer running with a drifted schema
            //    would otherwise silently feed garbage to the operator
            //    above us.
            if let Some(handle) = drain_handle.as_ref() {
                if !drain_eof {
                    let t0 = trace_on.then(Instant::now);
                    let item = handle.buffer().try_pop();
                    if let Some(t0) = t0 {
                        time_in_drain_pop += t0.elapsed();
                    }
                    match item {
                        Some(DrainItem::Batch(b)) => {
                            if b.schema() != schema {
                                break 'pump Err(DataFusionError::Internal(format!(
                                    "MppRepartitionExec[{tag}]: peer batch schema {:?} disagrees with expected {:?}",
                                    b.schema(),
                                    schema
                                )));
                            }
                            rows_received_from_peers += b.num_rows() as u64;
                            batches_received_from_peers += 1;
                            yield Ok(b);
                            continue 'pump;
                        }
                        Some(DrainItem::Eof) => {
                            drain_eof = true;
                        }
                        None => {
                            // Buffer empty but sources still alive — fall
                            // through to drain-pass + child-pull below.
                        }
                    }
                }
            }

            // 3. Cooperative drain pass: synchronously read whatever
            //    peer-shipped bytes are currently sitting in `shm_mq` and
            //    push them into our `DrainBuffer`. Without this, a
            //    participant whose outbound sends succeed fast never
            //    drains its inbound, peers' sends to us can't un-stall,
            //    and the mesh deadlocks on asymmetric pressure (observed
            //    as a 120s statement timeout on
            //    `aggregate_join_groupby - alternative 2` at 25M rows).
            //    Pumping here also feeds step 2 on the next iteration.
            if !drain_eof {
                if let Some(w) = state.wiring.as_ref() {
                    if let Some(drain) = w.cooperative_drain.as_ref() {
                        let t0 = trace_on.then(Instant::now);
                        let _ = drain.poll_drain_pass();
                        if let Some(t0) = t0 {
                            time_in_coop_drain += t0.elapsed();
                        }
                    }
                }
            }

            // 4. Pull a child batch (if not exhausted) and route. The
            //    inner pull loop runs until either the child surfaces a
            //    self-share (handed off to the outer loop) or the child
            //    exhausts.
            if !child_done {
                'pull: loop {
                    let t_child = trace_on.then(Instant::now);
                    let next = child.next().await;
                    if let Some(t0) = t_child {
                        time_in_child += t0.elapsed();
                    }
                    match next {
                        None => {
                            child_done = true;
                            // Drop wiring so peer senders detach and our
                            // peers see EOF on their inbound. We may still
                            // be reading peer-shipped rows in subsequent
                            // iterations.
                            state.wiring.take();
                            break 'pull;
                        }
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
                            // No self-partition in this batch — keep pulling.
                            continue 'pull;
                        }
                    }
                }
            }

            // 5. Exit when both producer sides are done.
            if child_done && drain_eof {
                break 'pump Ok(());
            }

            // 6. Child is exhausted but the drain still has live sources;
            //    yield to the executor so peer `MppRepartitionExec`s can
            //    make progress on their outbound shipping before our next
            //    drain pass.
            if child_done && !drain_eof {
                yield_now().await;
            }
        };

        // Tail: deterministic drain shutdown (joins the drain thread so
        // plan teardown doesn't leave a zombie holding DSM pointers) plus
        // the trace line. A self-share that landed in the queue right
        // before an error is intentionally discarded — DataFusion's
        // contract for a yielded `Err` is that no subsequent items
        // follow, so emitting the queued batch first would mislead
        // aggregators above us.
        state.wiring.take();
        let shutdown_err = drain_handle.as_ref().and_then(|h| h.shutdown().err());
        log_shuffle_eof(
            tag,
            participant_index,
            &state,
            ShuffleTimings {
                first_poll_at,
                time_in_child,
                time_in_process,
                time_in_coop_drain,
                time_in_drain_pop,
                rows_received_from_peers,
                batches_received_from_peers,
            },
        );
        if let Err(e) = outcome {
            yield Err(e);
        } else if let Some(e) = shutdown_err {
            yield Err(e);
        }
    }
}

/// Wall-clock timings carried alongside [`ShuffleState`] in
/// [`build_mpp_repartition_stream`]; emitted at EOF by [`log_shuffle_eof`].
/// `cfg_attr` rationale: same as [`ShuffleState`].
#[cfg_attr(test, allow(dead_code))]
struct ShuffleTimings {
    /// Set only when `mpp_trace_flag()` was on at first poll.
    first_poll_at: Option<Instant>,
    time_in_child: Duration,
    time_in_process: Duration,
    /// Cooperative inbound-drain time at the top of each loop iteration
    /// (excludes the in-spin drain accounted for inside `send_batch_traced`).
    time_in_coop_drain: Duration,
    /// Time spent in `DrainBuffer::try_pop` at the top of each loop
    /// iteration. Subset of the cooperative drain accounting that used
    /// to live in `DrainGatherStream` before the fold.
    time_in_drain_pop: Duration,
    /// Rows received from peers (the half of the output that used to come
    /// out of `DrainGatherExec`).
    rows_received_from_peers: u64,
    /// Batches received from peers (the half of the output that used to
    /// come out of `DrainGatherExec`).
    batches_received_from_peers: u64,
}

/// Trace-line emitter for [`build_mpp_repartition_stream`]'s EOF path. Gated out
/// of `cargo test --lib` because the unit-test target is not a `pg_test`
/// target and can't link the FFI symbols `pgrx::{warning,debug1}!` expand
/// into via `ereport`. The runtime EOF path itself *is* exercised by tests.
fn log_shuffle_eof(
    tag: &'static str,
    participant_index: u32,
    state: &ShuffleState,
    t: ShuffleTimings,
) {
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
        let drain_pop_ms = t.time_in_drain_pop.as_secs_f64() * 1000.0;
        let part_ms = state.time_in_partition.as_secs_f64() * 1000.0;
        let split_ms = state.time_in_split.as_secs_f64() * 1000.0;
        let encode_ms = state.time_in_encode.as_secs_f64() * 1000.0;
        let send_wait_ms = state.time_in_send_wait.as_secs_f64() * 1000.0;
        let drain_in_spin_ms = state.time_in_coop_drain_in_spin.as_secs_f64() * 1000.0;
        if mpp_trace() {
            pgrx::warning!(
                "mpp: MppRepartitionStream[{}] participant={} EOF rows_in={} batches_in={} self={} sent=[{}] rx_rows={} rx_batches={} wall_ms={:.1} child_ms={:.1} process_ms={:.1} coop_drain_ms={:.1} drain_pop_ms={:.1} part_ms={:.1} split_ms={:.1} encode_ms={:.1} send_wait_ms={:.1} drain_in_spin_ms={:.1} spin_iters={}",
                tag,
                participant_index,
                state.rows_in,
                state.batches_in,
                state.rows_self,
                sent_summary,
                t.rows_received_from_peers,
                t.batches_received_from_peers,
                wall_ms,
                child_ms,
                process_ms,
                coop_ms,
                drain_pop_ms,
                part_ms,
                split_ms,
                encode_ms,
                send_wait_ms,
                drain_in_spin_ms,
                state.send_spin_iters,
            );
        } else {
            pgrx::debug1!(
                "mpp: MppRepartitionStream[{}] participant={} EOF rows_in={} self={} sent=[{}] rx_rows={}",
                tag,
                participant_index,
                state.rows_in,
                state.rows_self,
                sent_summary,
                t.rows_received_from_peers,
            );
        }
    }
    #[cfg(test)]
    {
        let _ = (tag, participant_index, state, t);
    }
}

/// RAII guard ensuring `handle.shutdown()` runs even when the consumer
/// drops the stream before the natural EOF/Err tail (e.g., a parent
/// `LIMIT` shortcut, query cancel during the loop, panic). Without this,
/// peer `MppRepartitionExec`s holding `Arc<DrainHandle>` clones via their
/// `MppSender::cooperative_drain` would keep calling `poll_drain_pass`,
/// pushing peer batches into a buffer no one reads — an unbounded
/// memory-growth path on cancel. Shutdown is idempotent (cancel is
/// idempotent; receivers are taken via `Option::take`), so the natural
/// tail can also call it without consequence.
struct ShutdownOnDrop {
    handle: Arc<DrainHandle>,
}

impl Drop for ShutdownOnDrop {
    fn drop(&mut self) {
        let _ = self.handle.shutdown();
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
    use datafusion::arrow::array::{Int32Array, RecordBatch as ArrowBatch, StringArray};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::datasource::memory::MemorySourceConfig;
    use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
    use datafusion::prelude::SessionContext;
    use futures::StreamExt;
    use std::thread;

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
    /// `build_mpp_repartition_stream` honors this by surfacing the error via the
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
    /// This is the non-streaming core of `MppRepartitionExec`: the DataFusion
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
    fn shuffle_exec_emits_only_self_partition() {
        // N=2 mesh as participant 0. Row i -> participant i % 2: self gets
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
        let shuffle: Arc<dyn ExecutionPlan> =
            Arc::new(MppRepartitionExec::new(input, wiring, "test"));

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
        let handle = DrainHandle::spawn(DrainConfig::new(vec![receiver], Arc::clone(&buf)));
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
    fn mpp_repartition_yields_self_and_peer_in_one_stream() {
        // Full milestone-1 MPP data path in one process, no PG:
        //
        //   MemorySourceConfig (10 rows)
        //     ─→ MppRepartitionExec (folds self-routing + peer drain
        //                            into one stream)
        //   simulated-peer-sender ─→ DrainHandle (cooperative)
        //
        // At participant 0 with ModuloPartitioner(N=2), MppRepartitionExec
        // yields self-routed IDs {0,2,4,6,8} and peer-shipped IDs
        // {100, 200} in one stream. A simulated peer pushes the two
        // synthetic batches into the inbound drain after a small delay so
        // the buffer is empty on first poll — exercising the
        // wait-then-pop interleaving inside the fold.
        //
        // Replaced the earlier `UnionExec(MppRepartitionExec,
        // DrainGatherExec)` topology after PR #4828 folded the drain
        // logic into `MppRepartitionExec.execute()`.

        let batch = sample_batch(10);
        let schema = batch.schema();
        let input = MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch]).unwrap();

        let (out_tx, out_rx) = in_proc_channel(16); // outbound to peer
        let (in_tx, in_rx) = in_proc_channel(16); // inbound from peer

        // DrainHandle reads peer-shipped batches from `in_rx` into
        // `drain_buffer`. Wired into MppRepartitionExec via
        // `cooperative_drain` so the fold loop picks them up alongside
        // self-routed batches.
        let receiver = MppReceiver::new(Box::new(in_rx));
        let drain_buffer = DrainBuffer::new(1);
        let drain_handle = Arc::new(DrainHandle::spawn(DrainConfig::new(
            vec![receiver],
            Arc::clone(&drain_buffer),
        )));

        let wiring = ShuffleWiring {
            partitioner: Arc::new(ModuloPartitioner::new(2)),
            outbound_senders: vec![None, Some(MppSender::new(Box::new(out_tx)))],
            participant_index: 0,
            cooperative_drain: Some(Arc::clone(&drain_handle)),
        };
        let repartition: Arc<dyn ExecutionPlan> =
            Arc::new(MppRepartitionExec::new(input, wiring, "test"));

        // Simulated peer: spawn a thread that pushes two synthetic batches
        // into `in_tx` with a small delay between them so the buffer is
        // *empty* when the main thread first polls — exercising the
        // wait-then-pop path inside the fold loop. If we pre-joined this
        // thread, batches would all be buffered before the stream ran
        // and the cooperative interleaving would be untested.
        let peer_schema = schema.clone();
        let peer_thread = thread::spawn(move || {
            let sender = MppSender::new(Box::new(in_tx));
            for (id, name) in [(100i32, "peer100"), (200i32, "peer200")] {
                thread::sleep(Duration::from_millis(10));
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

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ctx = SessionContext::new();

        let mut all_ids = Vec::new();
        rt.block_on(async {
            let mut s = repartition.execute(0, ctx.task_ctx()).unwrap();
            while let Some(b) = s.next().await {
                let b = b.unwrap();
                let ids = b.column(0).as_any().downcast_ref::<Int32Array>().unwrap();
                all_ids.extend(ids.values().iter().copied());
            }
        });

        // Collect the outbound side into a second DrainBuffer just so we
        // verify MppRepartitionExec did, in fact, ship its peer rows.
        let receiver = MppReceiver::new(Box::new(out_rx));
        let out_buffer = DrainBuffer::new(1);
        let out_handle =
            DrainHandle::spawn(DrainConfig::new(vec![receiver], Arc::clone(&out_buffer)));
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
        // 1. The single stream yielded self-partition (0,2,4,6,8) plus
        //    peer-simulated (100, 200) interleaved as one sequence. Sort
        //    to make the multiset deterministic.
        all_ids.sort();
        assert_eq!(all_ids, vec![0, 2, 4, 6, 8, 100, 200]);
        // 2. MppRepartitionExec correctly shipped odd IDs to peer.
        outbound_ids.sort();
        assert_eq!(outbound_ids, vec![1, 3, 5, 7, 9]);
        let _ = schema;
    }

    #[test]
    fn shuffle_exec_propagates_child_errors() {
        // Custom child that errors on first poll.
        #[derive(Debug)]
        struct ErroringExec {
            schema: SchemaRef,
            properties: Arc<PlanProperties>,
        }

        impl DisplayAs for ErroringExec {
            fn fmt_as(&self, _t: DisplayFormatType, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "ErroringExec")
            }
        }

        impl ExecutionPlan for ErroringExec {
            fn name(&self) -> &str {
                "ErroringExec"
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn properties(&self) -> &Arc<PlanProperties> {
                &self.properties
            }
            fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
                vec![]
            }
            fn with_new_children(
                self: Arc<Self>,
                _children: Vec<Arc<dyn ExecutionPlan>>,
            ) -> DFResult<Arc<dyn ExecutionPlan>> {
                Ok(self)
            }
            fn execute(
                &self,
                _partition: usize,
                _context: Arc<TaskContext>,
            ) -> DFResult<SendableRecordBatchStream> {
                let schema = self.schema.clone();
                let stream = futures::stream::once(async move {
                    Err::<ArrowBatch, _>(DataFusionError::Execution("synthetic".into()))
                });
                Ok(Box::pin(RecordBatchStreamAdapter::new(schema, stream)))
            }
        }

        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let input_exec: Arc<dyn ExecutionPlan> = {
            let props = Arc::new(PlanProperties::new(
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
        let shuffle: Arc<dyn ExecutionPlan> =
            Arc::new(MppRepartitionExec::new(input_exec, wiring, "test"));

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ctx = SessionContext::new();
        let got_error = rt.block_on(async {
            let mut stream = shuffle.execute(0, ctx.task_ctx()).unwrap();
            matches!(stream.next().await, Some(Err(_)))
        });
        assert!(got_error, "MppRepartitionExec must propagate child errors");
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

    #[test]
    fn hash_partitioner_declares_hash_partitioning() {
        // The HashPartitioner exposes its key columns to DF as
        // `Partitioning::Hash(key_exprs, N)` so a downstream
        // `HashJoinExec(Partitioned)` recognises the alignment.
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]);
        let p = HashPartitioner::new(vec![0], 4);
        let part = p.output_partitioning(&schema);
        match part {
            Partitioning::Hash(exprs, n) => {
                assert_eq!(n, 4);
                assert_eq!(exprs.len(), 1);
                let col = exprs[0]
                    .as_any()
                    .downcast_ref::<Column>()
                    .expect("HashPartitioner key expr should be a Column");
                assert_eq!(col.name(), "id");
                assert_eq!(col.index(), 0);
            }
            other => panic!("expected Partitioning::Hash, got {other:?}"),
        }
    }

    #[test]
    fn fixed_target_partitioner_declares_single_output_partition() {
        // FixedTarget collapses N participants' rows into one stream on
        // the target; from DF's perspective the operator has a single
        // output partition. Declaring `UnknownPartitioning(1)` matches
        // that operational reality.
        let schema = Schema::new(vec![Field::new("id", DataType::Int32, false)]);
        let p = FixedTargetPartitioner::new(0, 3);
        match p.output_partitioning(&schema) {
            Partitioning::UnknownPartitioning(1) => {}
            other => panic!("expected UnknownPartitioning(1), got {other:?}"),
        }
    }

    #[test]
    fn mpp_repartition_exec_declares_partitioner_partitioning() {
        // The operator should adopt whatever the partitioner declares,
        // so a downstream `HashJoinExec` reasoning about partitioning
        // sees the same shape DF would see for a native `RepartitionExec`.
        let batch = sample_batch(2);
        let schema = batch.schema();
        let input = MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch]).unwrap();
        let (tx, _rx) = in_proc_channel(4);
        let wiring = ShuffleWiring {
            partitioner: Arc::new(HashPartitioner::new(vec![0], 2)),
            outbound_senders: vec![None, Some(MppSender::new(Box::new(tx)))],
            participant_index: 0,
            cooperative_drain: None,
        };
        let exec = MppRepartitionExec::new(input, wiring, "test");
        match &exec.plan_properties.partitioning {
            Partitioning::Hash(exprs, n) => {
                assert_eq!(*n, 2);
                assert_eq!(exprs.len(), 1);
            }
            other => panic!("expected Hash partitioning, got {other:?}"),
        }
    }

    #[test]
    fn mpp_repartition_exec_returns_empty_for_non_self_partition() {
        // execute(i) on a participant whose `participant_index != i` must
        // hand back an empty stream — the rows for partition `i` belong
        // to peer participant `i`, who reads them via its own
        // `MppRepartitionExec.execute(participant_index)` call.
        let batch = sample_batch(4);
        let schema = batch.schema();
        let input = MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch]).unwrap();
        let (tx_a, _rx_a) = in_proc_channel(4);
        let (tx_b, _rx_b) = in_proc_channel(4);
        let wiring = ShuffleWiring {
            partitioner: Arc::new(ModuloPartitioner::new(3)),
            outbound_senders: vec![
                None,
                Some(MppSender::new(Box::new(tx_a))),
                Some(MppSender::new(Box::new(tx_b))),
            ],
            participant_index: 0,
            cooperative_drain: None,
        };
        let exec: Arc<dyn ExecutionPlan> = Arc::new(MppRepartitionExec::new(input, wiring, "test"));

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ctx = SessionContext::new();
        rt.block_on(async {
            let mut s = exec.execute(2, ctx.task_ctx()).unwrap();
            let n_batches = std::iter::from_fn(|| {
                futures::executor::block_on(async { s.next().await.map(|_| ()) })
            })
            .count();
            assert_eq!(n_batches, 0, "execute(non-self) must yield an empty stream");
        });
        let _ = schema;
    }

    #[test]
    fn mpp_participant_isolator_forwards_to_self_partition() {
        // The isolator's contract: execute(0) calls
        // child.execute(participant_index). Verify by wiring an
        // MppRepartitionExec(N=3, participant_index=1) under the
        // isolator and confirming the rows yielded match what
        // partition 1 would route locally.
        let batch = sample_batch(6);
        let schema = batch.schema();
        let input = MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch]).unwrap();
        let (tx_to_0, _rx_0) = in_proc_channel(4);
        let (tx_to_2, _rx_2) = in_proc_channel(4);
        let wiring = ShuffleWiring {
            partitioner: Arc::new(ModuloPartitioner::new(3)),
            outbound_senders: vec![
                Some(MppSender::new(Box::new(tx_to_0))),
                None,
                Some(MppSender::new(Box::new(tx_to_2))),
            ],
            participant_index: 1,
            cooperative_drain: None,
        };
        let inner: Arc<dyn ExecutionPlan> =
            Arc::new(MppRepartitionExec::new(input, wiring, "test"));
        let isolator: Arc<dyn ExecutionPlan> = Arc::new(MppParticipantIsolatorExec::new(inner, 1));

        match &isolator.properties().partitioning {
            Partitioning::UnknownPartitioning(1) => {}
            other => panic!("isolator should declare 1 output partition, got {other:?}"),
        }

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ctx = SessionContext::new();
        rt.block_on(async {
            let mut s = isolator.execute(0, ctx.task_ctx()).unwrap();
            let mut all_ids = Vec::new();
            while let Some(b) = s.next().await {
                let b = b.unwrap();
                let ids = b.column(0).as_any().downcast_ref::<Int32Array>().unwrap();
                all_ids.extend(ids.values().iter().copied());
            }
            all_ids.sort();
            // ModuloPartitioner(N=3) on rows 0..6 routes (0,3) → 0,
            // (1,4) → 1, (2,5) → 2. participant_index=1 yields ids 1, 4.
            assert_eq!(all_ids, vec![1, 4]);
        });
        let _ = schema;
    }

    /// P1 seam smoke test: `MppRepartitionExec` starts with
    /// `input_stage() == None`, and `with_input_stage` returns a fresh
    /// node whose `input_stage()` reports the stamped [`MppStage`].
    /// Verifies the boundary trait is wired up so the walker can rely
    /// on it. The DrainGatherExec half of this test went away with the
    /// fold; its responsibilities are now covered by the same
    /// MppRepartitionExec that this test exercises.
    #[test]
    fn mpp_network_boundary_round_trips_stage() {
        let batch = sample_batch(4);
        let schema = batch.schema();
        let input = MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch]).unwrap();
        let (tx, _rx) = in_proc_channel(4);
        let wiring = ShuffleWiring {
            partitioner: Arc::new(ModuloPartitioner::new(2)),
            outbound_senders: vec![None, Some(MppSender::new(Box::new(tx)))],
            participant_index: 0,
            cooperative_drain: None,
        };
        let shuffle = MppRepartitionExec::new(input, wiring, "test");
        assert!(shuffle.input_stage().is_none());

        let stamped = shuffle
            .with_input_stage(MppStage::new(1, 0, 2))
            .expect("with_input_stage should succeed on fresh node");
        let as_boundary = stamped
            .as_any()
            .downcast_ref::<MppRepartitionExec>()
            .expect("stamped node is still a MppRepartitionExec");
        assert_eq!(
            as_boundary.input_stage().copied(),
            Some(MppStage::new(1, 0, 2)),
        );

        // Re-stamping must fail because the wiring was consumed by the
        // first call — enforces "P1 seam, walker stamps once" invariant.
        let re_stamp_err = shuffle.with_input_stage(MppStage::new(1, 0, 2));
        assert!(re_stamp_err.is_err());
        let _ = schema;
    }
}

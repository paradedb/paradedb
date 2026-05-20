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

//! Producer-side dispatcher for the pull-shape protocol.
//!
//! On every worker, a single [`ProducerTaskRegistry`] is installed as the [`RequestHandler`] on
//! every inbound drain. When a peer's `Request(stage_id, task_idx, partition)` lands on a drain,
//! the cooperative drain forwards it to the registry; the registry:
//!
//! 1. Idempotently dedupes against `(stage_id, task_idx, partition)` — a repeat Request is a
//!    no-op (the previous one already spawned the driver, or it's still running, or it already
//!    finished and the consumer has the EOF).
//! 2. Builds (or reuses) the cached `(prepared_plan, TaskContext)` for `(stage_id, task_idx)`.
//!    Multiple partitions of the same task share the prepared plan and context. `DistributedExec
//!    ::prepare_in_process_plan` converts any nested boundaries' input stages from `Stage::Local`
//!    to `Stage::Remote`, so they dispatch through `ShmMqWorkerTransport` exactly like outer
//!    boundaries.
//! 3. Clones an outbound sender to `sender_proc`, stamped with `(stage_id, partition)` so the
//!    consumer-side channel-buffer registry routes each batch correctly.
//! 4. `tokio::spawn`s a future that runs the partition stream through
//!    [`run_partition_driver`](crate::postgres::customscan::mpp::worker::run_partition_driver),
//!    bracketed by an active-driver counter that the service loop watches to know when
//!    in-flight work is done.
//!
//! ## Concurrency
//!
//! Everything runs on the worker's single-threaded tokio runtime. `tokio::spawn` registers the
//! driver future on the same runtime; it interleaves with the service loop and the cooperative
//! drain via `yield_now().await`. The shm_mq FFI invariant (must be called on the backend
//! thread) is preserved because the current-thread runtime never moves work off the worker's PG
//! backend thread.
//!
//! ## Lifetime
//!
//! Two Arc cycles touch the mesh, each with its own release path.
//!
//! 1. **Handler leg**: `MppMesh → Vec<DrainHandle> → Arc<dyn RequestHandler> →
//!    ProducerTaskRegistry → Arc<MppMesh>`. `mesh: Weak<MppMesh>` breaks the back-edge, so by
//!    refcount it isn't a true cycle. We still call [`MppMesh::uninstall_request_handler`] at
//!    teardown anyway, so the `Arc<dyn RequestHandler>` held by each drain releases promptly.
//!    Once that's done, the local `Arc<Registry>` in `run_mpp_worker` is the last strong ref;
//!    dropping it ends the registry.
//!
//! 2. **Cooperative-drain leg**: every spawned per-partition driver future captures an
//!    `MppSender` whose `with_cooperative_drain` field holds a strong `Arc<MppMesh>`. While the
//!    future is alive, the mesh has a transient extra refcount through it. No explicit hook
//!    here. The refs release when the future completes and the runtime drops it (or when the
//!    runtime itself drops). The `active_drivers == 0` check in the service loop's termination
//!    is what guarantees no driver futures outlive the loop, so no mesh refs leak past
//!    `run_mpp_worker`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};

use datafusion::common::DataFusionError;
use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::execution::TaskContext;
use datafusion::physical_plan::ExecutionPlan;
use datafusion::prelude::SessionContext;
use datafusion_distributed::{DistributedExec, DistributedTaskContext, NetworkBoundaryExt};
use datafusion_proto::bytes::physical_plan_from_bytes_with_extension_codec;
use tantivy::index::SegmentId;

use crate::api::HashSet;
use crate::postgres::customscan::datafusion::memory::create_memory_pool;
use crate::postgres::customscan::mpp::runtime::MppMesh;
use crate::postgres::customscan::mpp::transport::{
    CooperativeDrainSet, MppFrameHeader, RequestHandler, SubplanHandler,
};
use crate::postgres::customscan::mpp::worker::run_partition_driver;
use crate::postgres::ParallelScanState;
use crate::scan::physical_codec::MppReconstructionContext;

/// Per-`stage_id` snapshot of `(local_plan, task_count)` walked out of the worker's distributed
/// physical plan at startup. The producer service loop consults this on every Request to find
/// the plan to run.
///
/// The walk recurses through every [`NetworkBoundaryExt`] hit, so nested stages are recorded too
/// — needed because [`DistributedExec::prepare_in_process_plan`] retags nested boundaries to
/// dispatch through `ShmMqWorkerTransport`, which issues fresh Requests against the nested
/// stage_ids. If a nested stage_id weren't recorded, those nested Requests would fail to
/// dispatch.
pub(super) struct StagePlans {
    stages: HashMap<u32, (Arc<dyn ExecutionPlan>, usize)>,
}

impl StagePlans {
    pub(super) fn build(root: &Arc<dyn ExecutionPlan>) -> Self {
        let mut stages = HashMap::new();
        collect(root, &mut stages);
        Self { stages }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    pub(super) fn len(&self) -> usize {
        self.stages.len()
    }

    pub(super) fn lookup(&self, stage_id: u32) -> Option<(Arc<dyn ExecutionPlan>, usize)> {
        self.stages.get(&stage_id).cloned()
    }

    /// `(stage_id, task_count)` pairs in some order. Used by `run_mpp_worker` to enumerate every
    /// `(stage, task)` this proc owns so they can be pre-warmed.
    pub(super) fn iter_task_counts(&self) -> impl Iterator<Item = (u32, usize)> + '_ {
        self.stages.iter().map(|(s, (_, tc))| (*s, *tc))
    }
}

fn collect(plan: &Arc<dyn ExecutionPlan>, out: &mut HashMap<u32, (Arc<dyn ExecutionPlan>, usize)>) {
    if let Some(nb) = plan.as_ref().as_network_boundary() {
        let stage = nb.input_stage();
        let stage_id = stage.num() as u32;
        let task_count = stage.task_count();
        if let Some(stage_plan) = stage.local_plan() {
            out.entry(stage_id)
                .or_insert_with(|| (Arc::clone(stage_plan), task_count));
            collect(stage_plan, out);
            return;
        }
    }
    for child in plan.children() {
        collect(child, out);
    }
}

/// Cached prepared plan + task context for a single `(stage_id, task_idx)`. Built once on the
/// first Request for the task and shared across all subsequent per-partition Requests.
struct PreparedTask {
    plan: Arc<dyn ExecutionPlan>,
    ctx: Arc<TaskContext>,
}

/// Producer-side request dispatcher. Sole [`RequestHandler`] installed across every inbound
/// drain on a worker; spawns one driver future per `(stage_id, task_idx, partition)` Request.
///
/// `mesh: Weak<MppMesh>` is intentional — see the module-level lifetime note.
pub(super) struct ProducerTaskRegistry {
    stage_plans: StagePlans,
    session: Arc<SessionContext>,
    mesh: Weak<MppMesh>,
    work_mem_bytes: usize,
    hash_mem_multiplier: f64,
    /// Per-source canonical segment ID sets, indexed by `plan_position`. The dispatch-flip
    /// Reconstruction context layered onto each `TaskContext` built in [`build_task_ctx`].
    /// Carries the per-source canonical segment IDs (indexed by absolute `plan_position`) and
    /// the worker's `ParallelScanState` pointer. Read by the physical codec's
    /// `decode_pgsearch_scan` to rebuild `Vec<ScanState>` on shipped subplans. Empty on test
    /// paths that don't go through `run_mpp_worker`.
    reconstruction_context: Arc<MppReconstructionContext>,
    active_drivers: Arc<AtomicUsize>,
    /// First driver error observed since startup. The service loop polls this between
    /// `try_drain_pass` iterations and bails out so the worker surfaces a concrete failure
    /// instead of silently hanging on a partition that won't EOF.
    first_error: Arc<Mutex<Option<DataFusionError>>>,
    /// `(stage_id, task_idx) → prepared plan + TaskContext`. Lazy: first Request for a `(stage,
    /// task)` builds it; subsequent partitions of the same `(stage, task)` reuse it.
    prepared: Mutex<HashMap<(u32, u32), PreparedTask>>,
    /// `(stage_id, task_idx) → decoded subplan` populated by [`Self::on_subplan`] when the
    /// leader ships a per-task subplan via a [`MppFrameKind::Subplan`](crate::postgres::customscan::mpp::transport::MppFrameKind::Subplan)
    /// frame. `prepare_task` consults this first; when a hit lands the worker uses the
    /// leader-prepared plan directly instead of re-running `prepare_in_process_plan` locally.
    ///
    /// The fallback path (`stage_plans` → local `prepare_in_process_plan`) is what fires
    /// during the brief startup window between worker launch and the first drain pass that
    /// delivers the Subplan frames. It's also where queries land if the codec hits a gap —
    /// today's `decode_pgsearch_scan` emits an empty `Vec<ScanState>` placeholder, so a
    /// shipped subplan with a `PgSearchScan` leaf would return zero rows. The followup PR
    /// fixes that with per-`PgSearchScanPlan` state reconstruction.
    ///
    /// **Ordering hazard.** `run_mpp_worker` calls `prewarm` for every `(stage_id, task_idx)`
    /// this proc owns *before* installing the SubplanHandler and pumping the drain. The
    /// prewarm path goes through `prepare_task`, which sees an empty `shipped_subplans` and
    /// falls back to local-prepare — caching the locally-built plan in `prepared`. When real
    /// Subplan frames arrive later, they land in `shipped_subplans`, but `on_request` reads
    /// from `prepared` first via the per-`(stage, task, partition)` dedupe path. The followup
    /// PR reorders prewarm vs. handler-install (and pumps the drain once before prewarm) so
    /// shipped subplans are visible to the prep path.
    shipped_subplans: Mutex<HashMap<(u32, u32), Arc<dyn ExecutionPlan>>>,
    /// `(stage_id, task_idx, partition)` set of already-dispatched drivers. Repeat Requests are
    /// dropped. Without this, a consumer that re-issued `stream_partition` would cause the
    /// producer to spawn a second driver pushing duplicate frames onto the channel.
    spawned: Mutex<HashSet<(u32, u32, u32)>>,
}

// SAFETY: `parallel_state: *mut ParallelScanState` makes the auto-derived `Send + Sync` go away,
// but both bounds are required (`RequestHandler` and `SubplanHandler` are both `: Send + Sync`).
//
// Provenance: the pointer is the worker's view of PG's DSM-attached shared state — set up in the
// custom-scan executor state (see `MppWorkerInputs::parallel_state` and the customscan
// `parallel_state` field), passed into `run_mpp_worker` for the lifetime of that call. The
// `ParallelScanState` outlives `run_mpp_worker` because PG won't tear down the executor state
// (and DSM mapping) while the worker is still inside `ExecutorRun`.
//
// Threading: the only runtime the registry's handlers run on is the worker's `current_thread`
// tokio runtime (pinned to the PG backend thread; see `aggregatescan/mpp.rs` and `joinscan/mpp.rs`).
// `on_request`/`on_subplan` are invoked from the cooperative-drain spin on that same thread, and
// the per-partition driver futures `tokio::spawn`ed from `on_request` poll on it too. Cross-thread
// access is therefore impossible by construction. Same pattern that covers `ShmMqSender` and
// `MppSender` in `mesh.rs` / `transport.rs`.
unsafe impl Send for ProducerTaskRegistry {}
unsafe impl Sync for ProducerTaskRegistry {}

impl ProducerTaskRegistry {
    pub(super) fn new(
        stage_plans: StagePlans,
        session: Arc<SessionContext>,
        mesh: &Arc<MppMesh>,
        work_mem_bytes: usize,
        hash_mem_multiplier: f64,
        index_segment_ids: Vec<HashSet<SegmentId>>,
        parallel_state: Option<*mut ParallelScanState>,
    ) -> Self {
        let reconstruction_context = Arc::new(MppReconstructionContext {
            index_segment_ids,
            parallel_state,
        });
        Self {
            stage_plans,
            session,
            mesh: Arc::downgrade(mesh),
            work_mem_bytes,
            hash_mem_multiplier,
            reconstruction_context,
            active_drivers: Arc::new(AtomicUsize::new(0)),
            first_error: Arc::new(Mutex::new(None)),
            prepared: Mutex::new(HashMap::new()),
            shipped_subplans: Mutex::new(HashMap::new()),
            spawned: Mutex::new(HashSet::default()),
        }
    }

    /// Number of subplans the registry has received from the leader via
    /// [`SubplanHandler::on_subplan`](crate::postgres::customscan::mpp::transport::SubplanHandler).
    /// Used by lib tests to verify receive-side regression coverage.
    #[allow(dead_code)] // exercised by lib tests; production reads of `shipped_subplans` go
                        // through `prepare_task` directly, not through this accessor.
    pub(super) fn shipped_subplan_count(&self) -> usize {
        self.shipped_subplans
            .lock()
            .expect("ProducerTaskRegistry shipped_subplans mutex poisoned")
            .len()
    }

    /// Number of currently-running driver futures. Service loop reads this to know when
    /// in-flight work is done.
    ///
    /// The counter only gates termination, it doesn't synchronise data. `Acquire` here pairs
    /// with the drivers' `Release` decrement so the loop gets a clean happens-before for "this
    /// driver is gone". The increment on dispatch (below) is `Relaxed`; it sequences with the
    /// decrement via the registry's `Mutex`es, not the atomic itself.
    pub(super) fn active_drivers(&self) -> usize {
        self.active_drivers.load(Ordering::Acquire)
    }

    /// Take any error captured by a driver future. Service loop calls this between drain passes
    /// and propagates the error out so the worker fails the query instead of hanging.
    pub(super) fn take_error(&self) -> Option<DataFusionError> {
        self.first_error
            .lock()
            .expect("ProducerTaskRegistry first_error mutex poisoned")
            .take()
    }

    /// Eagerly build the cached `(prepared_plan, TaskContext)` for `(stage_id, task_idx)` so
    /// the first Request for this task doesn't pay the `prepare_in_process_plan` cost on the
    /// drain dispatch path. `run_mpp_worker` calls this at startup for every `(stage, task)`
    /// this proc owns.
    ///
    /// Idempotent. Re-warming an already-prepared key is a no-op. Returns the underlying error
    /// if `prepare_task` fails (DF planner refused the shape, memory pool builder failed, etc.).
    /// The caller decides whether to fail worker startup or let the failure resurface lazily on
    /// the first Request.
    pub(super) fn prewarm(&self, stage_id: u32, task_idx: u32) -> Result<(), DataFusionError> {
        let mut map = self
            .prepared
            .lock()
            .expect("ProducerTaskRegistry prepared mutex poisoned");
        if map.contains_key(&(stage_id, task_idx)) {
            return Ok(());
        }
        let prepared = self.prepare_task(stage_id, task_idx)?;
        map.insert((stage_id, task_idx), prepared);
        Ok(())
    }

    /// `(stage_id, task_count)` pairs for every stage this worker's distributed plan touches.
    /// Used by `run_mpp_worker` to drive [`Self::prewarm`].
    pub(super) fn iter_task_counts(&self) -> impl Iterator<Item = (u32, usize)> + '_ {
        self.stage_plans.iter_task_counts()
    }

    fn prepare_task(&self, stage_id: u32, task_idx: u32) -> Result<PreparedTask, DataFusionError> {
        // Prefer a leader-shipped subplan when one is present in `shipped_subplans` — that's
        // the dispatch-flip's "build once on the leader, ship many" path. Fall back to local
        // re-plan if nothing has been shipped for this `(stage_id, task_idx)` yet (only
        // possible during the brief window between worker startup and the first drain pass
        // that delivers the subplan).
        if let Some(shipped) = self
            .shipped_subplans
            .lock()
            .expect("ProducerTaskRegistry shipped_subplans mutex poisoned")
            .get(&(stage_id, task_idx))
            .cloned()
        {
            let task_count = self
                .stage_plans
                .lookup(stage_id)
                .map(|(_, tc)| tc)
                .unwrap_or(1);
            let task_ctx = build_task_ctx(
                &self.session,
                task_idx,
                task_count,
                self.work_mem_bytes,
                self.hash_mem_multiplier,
                &shipped,
                Some(Arc::clone(&self.reconstruction_context)),
            )?;
            return Ok(PreparedTask {
                plan: shipped,
                ctx: task_ctx,
            });
        }
        // Fallback: build the prepared plan locally from `stage_plans`, same recipe as
        // pre-dispatch-flip. Used during startup races (subplan hasn't arrived yet) and as a
        // safety net while the codec's PgSearchScan state-reconstruction story matures.
        let (plan, task_count) = self.stage_plans.lookup(stage_id).ok_or_else(|| {
            DataFusionError::Internal(format!(
                "mpp producer: no plan registered for stage_id={stage_id}"
            ))
        })?;
        prepare_stage_task(
            &plan,
            stage_id,
            task_idx,
            task_count,
            &self.session,
            self.work_mem_bytes,
            self.hash_mem_multiplier,
            Some(Arc::clone(&self.reconstruction_context)),
        )
    }
}

/// Build a per-`(stage_id, task_idx)` `TaskContext` matching what `prepare_stage_task` produces.
/// Used by the shipped-subplan path: the decoded plan already has its `DistributedTaskContext`
/// baked into nested boundary nodes from leader-side preparation, but the executing
/// `TaskContext` still needs the extension layered on so any operator that re-reads it at
/// runtime sees the right `(task_index, task_count)` shape.
///
/// `reconstruction_context` is layered on only when the caller is a worker (the leader's
/// `ship_subplans_to_workers` passes `None` because it never decodes — it only encodes).
fn build_task_ctx(
    session: &SessionContext,
    task_idx: u32,
    task_count: usize,
    work_mem_bytes: usize,
    hash_mem_multiplier: f64,
    plan: &Arc<dyn ExecutionPlan>,
    reconstruction_context: Option<Arc<MppReconstructionContext>>,
) -> Result<Arc<TaskContext>, DataFusionError> {
    let mut cfg =
        session
            .state()
            .config()
            .clone()
            .with_extension(Arc::new(DistributedTaskContext {
                task_index: task_idx as usize,
                task_count,
            }));
    if let Some(recon) = reconstruction_context {
        cfg = cfg.with_extension(recon);
    }
    let memory_pool = create_memory_pool(plan, work_mem_bytes, hash_mem_multiplier);
    let runtime_env = RuntimeEnvBuilder::new()
        .with_memory_pool(memory_pool)
        .build()
        .map_err(|e| DataFusionError::Internal(format!("mpp producer: build RuntimeEnv: {e}")))?;
    Ok(Arc::new(
        TaskContext::default()
            .with_session_config(cfg)
            .with_runtime(Arc::new(runtime_env)),
    ))
}

/// Shared `(stage_id, task_idx) -> PreparedTask` builder. Called by `ProducerTaskRegistry` on
/// the worker side and by [`ship_subplans_to_workers`] on the leader side. Both sites need the
/// same TaskContext + `prepare_in_process_plan` recipe so the prepared plan they produce is
/// bit-identical — that's the whole point of the dispatch-flip's "build once, ship many" model.
///
/// **Bit-identical-config-or-bust.** `session`, `work_mem_bytes`, and `hash_mem_multiplier` MUST
/// match what the worker would supply for the same `(stage_id, task_idx)`. The dispatch-flip
/// relies on this in two places: (1) the leader's encoded subplan has to match the worker's
/// would-be locally-planned subplan for plan equivalence; (2) memory pools sized off
/// `work_mem_bytes` need to match so a memory-bounded operator doesn't behave differently when
/// the worker executes the shipped plan. Both sides feed `pg_sys::work_mem` and
/// `pg_sys::hash_mem_multiplier`; both sides build the session through
/// `crate::postgres::customscan::mpp::exec_worker::build_mpp_session_context`. Any future
/// session-config knob added on only one side breaks this invariant silently — bench-shape
/// regressions then surface as "leader and worker disagree on the plan."
#[allow(clippy::too_many_arguments)]
fn prepare_stage_task(
    plan: &Arc<dyn ExecutionPlan>,
    stage_id: u32,
    task_idx: u32,
    task_count: usize,
    session: &SessionContext,
    work_mem_bytes: usize,
    hash_mem_multiplier: f64,
    reconstruction_context: Option<Arc<MppReconstructionContext>>,
) -> Result<PreparedTask, DataFusionError> {
    let task_ctx = build_task_ctx(
        session,
        task_idx,
        task_count,
        work_mem_bytes,
        hash_mem_multiplier,
        plan,
        reconstruction_context,
    )?;

    // Wrap the stage's local plan in a fresh `DistributedExec` and `prepare_in_process_plan` so
    // nested NetworkShuffleExec / NetworkBroadcastExec / NetworkCoalesceExec dispatch through
    // `ShmMqWorkerTransport` (Stage::Remote) instead of the LocalStage path that errors when
    // task_count > 1.
    let dist = Arc::new(DistributedExec::new(Arc::clone(plan)));
    let prepared = dist.prepare_in_process_plan(&task_ctx).map_err(|e| {
        DataFusionError::Internal(format!(
            "mpp producer: prepare_in_process_plan failed for stage_id={stage_id} \
             task_idx={task_idx}: {e}"
        ))
    })?;
    Ok(PreparedTask {
        plan: prepared,
        ctx: task_ctx,
    })
}

/// Leader-side: walk the distributed physical plan, prepare each `(stage_id, task_idx)`, encode
/// it via [`crate::scan::physical_codec::PgSearchPhysicalCodec`], and ship the bytes to the
/// owning worker via a [`MppFrameKind::Subplan`](crate::postgres::customscan::mpp::transport::MppFrameKind::Subplan)
/// frame. The worker side stashes the decoded plan in its [`ProducerTaskRegistry`] so subsequent
/// `Request` frames for the same `(stage, task)` skip the re-plan path.
///
/// Called once during the leader's setup, after `build_mpp_session_context` produces the
/// distributed physical plan and before the leader starts executing its own plan
/// (`NetworkBoundaryExec` consumers issue `Request` frames once execution kicks in — the
/// subplans must already be at the workers by then).
///
/// `physical_plan` is the LEADER's distributed plan. Walking it for `NetworkBoundary` nodes
/// gives every nested `(stage_id, local_plan, task_count)` we need to ship; the leader and
/// workers run the same `build_mpp_session_context`, so the StagePlans the leader walks here
/// matches the StagePlans each worker would build from its own copy.
///
/// `n_workers` is the producer-worker count (not total procs). `proc_for_task` maps
/// `task_idx -> 1..=n_workers`, so we ship to procs 1..=n_workers and never to ourselves
/// (proc 0).
///
/// **Why the leader's cooperative-drain spin is safe during ship.** The spin (inside
/// `send_subplan_traced` / `send_pre_encoded`) calls `drain.try_drain_pass()` between
/// `try_send` attempts. The leader's inbound drains have no `RequestHandler` or
/// `SubplanHandler` installed at ship time — the handlers go in later in `exec_custom_scan`.
/// If a worker happened to send a Request to the leader during this window, the
/// `dispatch_requests` no-handler branch would silently drop it (`mpp_log!` for diag) and
/// data would be lost. We currently rely on the invariant that workers never send Requests to
/// the leader: `proc_for_task(n_workers, _)` always returns `1..=n_workers`, never 0, so no
/// worker addresses a producer task to the leader. If a future planner change ever places a
/// producer task on proc 0, this comment is wrong and the ship-time drain dispatch needs a
/// proper handler installed before this call runs.
pub(crate) fn ship_subplans_to_workers(
    physical_plan: &Arc<dyn ExecutionPlan>,
    leader_mesh: &Arc<MppMesh>,
    session: &SessionContext,
    work_mem_bytes: usize,
    hash_mem_multiplier: f64,
    runtime: &tokio::runtime::Runtime,
) -> Result<(), DataFusionError> {
    use crate::postgres::customscan::mpp::runtime::proc_for_task;
    use crate::postgres::customscan::mpp::transport::{MppFrameHeader, SendBatchStats};
    use datafusion_distributed::DistributedCodec;
    use datafusion_proto::bytes::physical_plan_to_bytes_with_extension_codec;

    let stage_plans = StagePlans::build(physical_plan);
    if stage_plans.is_empty() {
        // No NetworkBoundary nodes -> no subplans to ship. Single-proc plans hit this path.
        return Ok(());
    }
    let n_workers = leader_mesh.n_procs.saturating_sub(1).max(1);

    // Use DF-D's `DistributedCodec::new_combined_with_user` so DF-D's wrapper nodes
    // (`NetworkShuffleExec`, `NetworkBroadcastExec`, `BroadcastExec`, etc.) serialize through
    // DF-D's own codec, and our `PgSearchPhysicalCodec` only handles the leaves it knows about
    // (`VisibilityFilterExec`, `TantivyLookupExec`, `SegmentedTopKExec`, `PgSearchScan`). The
    // user codec is picked up from the session's distributed config; we registered it earlier
    // via `with_distributed_user_codec`.
    let codec = DistributedCodec::new_combined_with_user(session.state().config());
    let mut tasks: Vec<(u32, u32, usize, Arc<dyn ExecutionPlan>)> = Vec::new();
    for (stage_id, task_count) in stage_plans.iter_task_counts() {
        let (local_plan, _) = stage_plans.lookup(stage_id).expect("stage just walked");
        for task_idx in 0..task_count {
            tasks.push((
                stage_id,
                task_idx as u32,
                task_count,
                Arc::clone(&local_plan),
            ));
        }
    }

    runtime.block_on(async move {
        let mut stats = SendBatchStats::default();
        for (stage_id, task_idx, task_count, local_plan) in tasks {
            let owner_proc = proc_for_task(n_workers, task_idx);

            let prepared = prepare_stage_task(
                &local_plan,
                stage_id,
                task_idx,
                task_count,
                session,
                work_mem_bytes,
                hash_mem_multiplier,
                None, // leader never decodes — only encodes; no reconstruction context.
            )?;

            let bytes = physical_plan_to_bytes_with_extension_codec(prepared.plan, &codec)
                .map_err(|e| {
                    DataFusionError::Internal(format!(
                        "mpp leader: encode subplan stage_id={stage_id} task_idx={task_idx}: {e}"
                    ))
                })?;

            let header = MppFrameHeader::subplan(stage_id, task_idx)?;
            let sender = leader_mesh
                .outbound_sender(owner_proc, header)
                .ok_or_else(|| {
                    DataFusionError::Internal(format!(
                    "mpp leader: no outbound sender to proc {owner_proc} for stage_id={stage_id} \
                     task_idx={task_idx} (mesh detached?)"
                ))
                })?;
            // Attach the leader's mesh as the cooperative drain so the spin can pull any
            // queued-up leader-bound frames (rare during setup) without deadlocking. Even if no
            // drain were attached the blocking send would still complete eventually — workers'
            // own drains consume the bytes regardless of whether their SubplanHandler is
            // installed (Phase 3) — but the explicit drain keeps this path consistent with
            // other producer sends.
            let sender = sender
                .with_cooperative_drain(Arc::clone(leader_mesh) as Arc<dyn CooperativeDrainSet>);
            sender
                .send_subplan_traced(task_idx, bytes.as_ref(), &mut stats)
                .await?;
        }
        Ok::<(), DataFusionError>(())
    })?;
    Ok(())
}

impl RequestHandler for ProducerTaskRegistry {
    fn on_request(
        &self,
        sender_proc: u32,
        stage_id: u32,
        task_idx: u32,
        partition: u32,
    ) -> Result<(), DataFusionError> {
        // Dedupe: one Request per (stage, task, partition) per query is the contract. A second
        // one means the consumer restarted its stream or there's a bug somewhere. Either way
        // re-running would double-send.
        //
        // Insert into `spawned` first so two simultaneous on_request calls for the same key
        // race and only one wins. Roll back on any failure path before we spawn, otherwise a
        // transient error (say `prepare_task` returns Err) leaves the key in the set and any
        // retry silently no-ops. Once we hit `tokio::spawn` the slot belongs to the driver
        // future and we stop touching it.
        let rollback_key = (stage_id, task_idx, partition);
        {
            let mut spawned = self
                .spawned
                .lock()
                .expect("ProducerTaskRegistry spawned mutex poisoned");
            if !spawned.insert(rollback_key) {
                crate::mpp_log!(
                    "mpp producer dispatch: dropping duplicate Request \
                     stage_id={stage_id} task_idx={task_idx} partition={partition}"
                );
                return Ok(());
            }
        }

        // Undo the `spawned` insert if anything below fails. Saves a long ladder of
        // `.map_err(|e| { remove; e })` calls.
        let rollback = || {
            self.spawned
                .lock()
                .expect("ProducerTaskRegistry spawned mutex poisoned")
                .remove(&rollback_key);
        };

        let mesh = match self.mesh.upgrade() {
            Some(m) => m,
            None => {
                rollback();
                return Err(DataFusionError::Internal(
                    "mpp producer dispatch: mesh dropped before Request handled".into(),
                ));
            }
        };

        // Look up the prepared plan + context for this (stage, task). Different partitions of
        // the same task share one entry, so prep runs at most once per task. `run_mpp_worker`
        // pre-warms every task this proc owns at startup, so the lookup here is a cache hit on
        // the steady-state path and the expensive prep doesn't run on the drain dispatch.
        let prepared = {
            let mut map = self
                .prepared
                .lock()
                .expect("ProducerTaskRegistry prepared mutex poisoned");
            match map.get(&(stage_id, task_idx)) {
                Some(p) => PreparedTask {
                    plan: Arc::clone(&p.plan),
                    ctx: Arc::clone(&p.ctx),
                },
                None => match self.prepare_task(stage_id, task_idx) {
                    Ok(prepared) => {
                        map.insert(
                            (stage_id, task_idx),
                            PreparedTask {
                                plan: Arc::clone(&prepared.plan),
                                ctx: Arc::clone(&prepared.ctx),
                            },
                        );
                        prepared
                    }
                    Err(e) => {
                        drop(map);
                        rollback();
                        return Err(e);
                    }
                },
            }
        };

        // Build the per-partition response sender on the outbound queue to `sender_proc`. Header
        // `(stage_id, partition)` so the consumer's drain registry routes batches to the right
        // channel buffer. Cooperative drain so the producer doesn't deadlock on a full outbound
        // queue — the spin pumps every inbound, which is what frees space symmetrically.
        let sender = match mesh
            .outbound_sender(sender_proc, MppFrameHeader::batch(stage_id, partition))
        {
            Some(s) => s.with_cooperative_drain(Arc::clone(&mesh) as Arc<dyn CooperativeDrainSet>),
            None => {
                // No outbound. Two cases to tell apart:
                //   - Detached: the leader (or a peer) called `detach_outbound_senders` between
                //     the Request being enqueued and us dispatching it. Clean teardown. Drop
                //     the Request and roll back the `spawned` insert so any retry path stays
                //     honest (we don't expect a retry, but the bookkeeping shouldn't lie).
                //   - Never present: out-of-range `sender_proc`, or the self-loop slot on the
                //     leader. That's a real config bug, surface as `Internal`.
                if mesh.outbound_detached() {
                    crate::mpp_log!(
                        "mpp producer dispatch: outbound detached after Request enqueue, \
                         dropping stage_id={stage_id} task_idx={task_idx} partition={partition}"
                    );
                    rollback();
                    return Ok(());
                }
                rollback();
                return Err(DataFusionError::Internal(format!(
                    "mpp producer dispatch: no outbound sender for sender_proc={sender_proc} \
                     (this_proc={}, stage_id={stage_id}, task_idx={task_idx}, partition={partition})",
                    mesh.this_proc
                )));
            }
        };

        // Bump the counter BEFORE spawning so the service loop's `active_drivers() == 0` check
        // doesn't race past a freshly-dispatched task. `Relaxed` is enough here; the counter
        // doesn't synchronise data, just gates termination. The `Acquire` load up in
        // `active_drivers()` pairs with the `Release` decrement below for the "driver is gone"
        // happens-before.
        self.active_drivers.fetch_add(1, Ordering::Relaxed);
        let counter = Arc::clone(&self.active_drivers);
        let err_slot = Arc::clone(&self.first_error);
        let plan = prepared.plan;
        let ctx = prepared.ctx;
        tokio::spawn(async move {
            let result = run_partition_driver(plan, partition as usize, sender, ctx).await;
            counter.fetch_sub(1, Ordering::Release);
            if let Err(e) = result {
                let mut guard = err_slot
                    .lock()
                    .expect("ProducerTaskRegistry first_error mutex poisoned");
                if guard.is_none() {
                    crate::mpp_log!(
                        "mpp producer driver failed stage_id={stage_id} task_idx={task_idx} \
                         partition={partition}: {e}"
                    );
                    *guard = Some(e);
                }
            }
        });
        Ok(())
    }
}

impl SubplanHandler for ProducerTaskRegistry {
    /// Receive a leader-shipped subplan for `(stage_id, task_idx)`. Decodes the bytes with
    /// [`crate::scan::physical_codec::PgSearchPhysicalCodec`] and stores the result in
    /// `shipped_subplans` keyed by `(stage_id, task_idx)`. `prepare_task` then consumes from
    /// that map ahead of any local re-plan. The codec's `PgSearchScan` arm still has an empty
    /// `Vec<ScanState>` gap that the followup PR fills in — until then, plans containing a
    /// `PgSearchScan` may emit zero rows when sourced from the shipped path.
    fn on_subplan(
        &self,
        stage_id: u32,
        task_idx: u32,
        payload: Vec<u8>,
    ) -> Result<(), DataFusionError> {
        let key = (stage_id, task_idx);
        // Repeat ship (e.g. leader retried after a transient error, or a future shape where
        // the same subplan is broadcast to multiple drains) lands in the same slot. Last write
        // wins; same `(stage_id, task_idx)` from the same leader session always carries the
        // same bytes today.
        //
        // Decode through `DistributedCodec::new_combined_with_user` so DF-D wrapper nodes
        // round-trip via DF-D's codec. The leader encodes through the same combined codec, so
        // sender and receiver must match.
        let codec = datafusion_distributed::DistributedCodec::new_combined_with_user(
            self.session.state().config(),
        );
        let task_ctx = self.session.task_ctx();
        let decoded = physical_plan_from_bytes_with_extension_codec(
            payload.as_slice(),
            &task_ctx,
            &codec,
        )
        .map_err(|e| {
            DataFusionError::Internal(format!(
                "mpp producer on_subplan: decode failed stage_id={stage_id} task_idx={task_idx}: {e}"
            ))
        })?;
        let total = {
            let mut map = self
                .shipped_subplans
                .lock()
                .expect("ProducerTaskRegistry shipped_subplans mutex poisoned");
            map.insert(key, decoded);
            map.len()
        };
        // Lock dropped above before the log call. `mpp_log!` expands to `pgrx::warning!` under
        // the debug GUC and ereport's; cheap to keep off the hot path.
        crate::mpp_log!(
            "mpp producer on_subplan: received stage_id={stage_id} task_idx={task_idx} \
             (total shipped: {total})"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::fast_fields_helper::FFHelper;
    use crate::scan::tantivy_lookup_exec::TantivyLookupExec;
    use arrow_schema::{DataType, Field, Schema};
    use datafusion::physical_plan::empty::EmptyExec;
    use datafusion_proto::bytes::physical_plan_to_bytes_with_extension_codec;
    use std::sync::Arc;

    /// Build a `ProducerTaskRegistry` with empty mesh/stage-plans, just enough to exercise
    /// `on_subplan` end-to-end. The session has `PgSearchPhysicalCodec` registered as the
    /// distributed user codec so `DistributedCodec::new_combined_with_user` resolves our
    /// custom execs (`TantivyLookupExec`, etc.) when it falls through past the DF-D types it
    /// knows about. Without that registration, decode of our exec variants would miss the
    /// user codec and surface as a decode error.
    fn test_registry() -> Arc<ProducerTaskRegistry> {
        use crate::scan::physical_codec::PgSearchPhysicalCodec;
        use datafusion::execution::SessionStateBuilder;
        use datafusion_distributed::DistributedExt;

        let mesh = Arc::new(MppMesh::new(0, 1, Vec::new(), Vec::new()));
        let stage_plans = StagePlans {
            stages: HashMap::new(),
        };
        let state = SessionStateBuilder::new()
            .with_default_features()
            .with_distributed_user_codec(PgSearchPhysicalCodec)
            .build();
        let session = Arc::new(SessionContext::new_with_state(state));
        // index_segment_ids + parallel_state stay empty/None in lib tests; the reconstruction
        // walker (Phase 2+) is exercised by integration tests instead.
        Arc::new(ProducerTaskRegistry::new(
            stage_plans,
            session,
            &mesh,
            64 * 1024 * 1024,
            2.0,
            Vec::new(),
            None,
        ))
    }

    /// Encode a minimal `TantivyLookupExec` through `DistributedCodec::new_combined_with_user`
    /// — same encode path the leader's `ship_subplans_to_workers` uses, so the bytes are
    /// shape-identical to what production produces. Picking `TantivyLookupExec` because its
    /// codec is fully runnable in `cargo test` (the `VisibilityFilterExec` codec is gated out
    /// of test builds, see the corresponding comment in `physical_codec.rs`).
    fn encode_test_subplan(session: &SessionContext) -> Vec<u8> {
        use datafusion_distributed::DistributedCodec;
        let input_schema = Arc::new(Schema::new(vec![Field::new(
            "ctid",
            DataType::UInt64,
            false,
        )]));
        let input: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(input_schema));
        // `TantivyLookupExec` with an empty deferred-field list is the minimum that still
        // exercises the codec end-to-end. The plan's runtime behavior isn't relevant here —
        // we only check the bytes round-trip.
        let plan = Arc::new(
            TantivyLookupExec::new(input, Vec::new(), crate::api::HashMap::default())
                .expect("TantivyLookupExec::new"),
        ) as Arc<dyn ExecutionPlan>;
        let _ = FFHelper::empty(); // keep FFHelper import live until the codec consumes it.
        let codec = DistributedCodec::new_combined_with_user(session.state().config());
        physical_plan_to_bytes_with_extension_codec(plan, &codec)
            .expect("encode test subplan")
            .to_vec()
    }

    #[test]
    fn on_subplan_decodes_and_stashes_in_shipped_map() {
        let registry = test_registry();
        assert_eq!(registry.shipped_subplan_count(), 0);

        let payload = encode_test_subplan(&registry.session);
        registry
            .on_subplan(7, 3, payload.clone())
            .expect("on_subplan must accept a well-formed payload");

        assert_eq!(registry.shipped_subplan_count(), 1);

        // The decoded plan is keyed by (stage_id, task_idx); not visible through a public
        // accessor (Phase 4 will surface it through the Request path), but we can verify the
        // key exists by re-shipping the same key and confirming the count stays at 1.
        registry
            .on_subplan(7, 3, payload)
            .expect("re-ship of same key must succeed");
        assert_eq!(registry.shipped_subplan_count(), 1);
    }

    #[test]
    fn on_subplan_multiple_keys_accumulate() {
        let registry = test_registry();
        let payload = encode_test_subplan(&registry.session);

        for (stage_id, task_idx) in [(0_u32, 0_u32), (0, 1), (1, 0), (1, 1)] {
            registry
                .on_subplan(stage_id, task_idx, payload.clone())
                .expect("on_subplan");
        }
        assert_eq!(registry.shipped_subplan_count(), 4);
    }

    #[test]
    fn on_subplan_surfaces_decode_error() {
        let registry = test_registry();
        // Garbage bytes — prost decode should fail loudly with an Internal error pointing
        // at the failed (stage_id, task_idx) so an integration test can pinpoint the breakage.
        let err = registry
            .on_subplan(5, 2, vec![0xFF, 0xFE, 0xFD, 0xFC])
            .expect_err("garbage payload must surface an error");
        let msg = err.to_string();
        assert!(
            msg.contains("decode failed")
                && msg.contains("stage_id=5")
                && msg.contains("task_idx=2"),
            "unexpected error: {msg}"
        );
        // Failed decode does NOT populate the map.
        assert_eq!(registry.shipped_subplan_count(), 0);
    }
}

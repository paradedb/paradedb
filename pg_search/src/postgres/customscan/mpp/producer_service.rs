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

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};

use datafusion::common::DataFusionError;
use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::execution::TaskContext;
use datafusion::physical_plan::ExecutionPlan;
use datafusion::prelude::SessionContext;
use datafusion_distributed::{DistributedExec, DistributedTaskContext, NetworkBoundaryExt};

use crate::postgres::customscan::datafusion::memory::create_memory_pool;
use crate::postgres::customscan::mpp::runtime::MppMesh;
use crate::postgres::customscan::mpp::transport::{
    CooperativeDrainSet, MppFrameHeader, RequestHandler,
};
use crate::postgres::customscan::mpp::worker::run_partition_driver;

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
    active_drivers: Arc<AtomicUsize>,
    /// First driver error observed since startup. The service loop polls this between
    /// `try_drain_pass` iterations and bails out so the worker surfaces a concrete failure
    /// instead of silently hanging on a partition that won't EOF.
    first_error: Arc<Mutex<Option<DataFusionError>>>,
    /// `(stage_id, task_idx) → prepared plan + TaskContext`. Lazy: first Request for a `(stage,
    /// task)` builds it; subsequent partitions of the same `(stage, task)` reuse it.
    prepared: Mutex<HashMap<(u32, u32), PreparedTask>>,
    /// `(stage_id, task_idx, partition)` set of already-dispatched drivers. Repeat Requests are
    /// dropped. Without this, a consumer that re-issued `stream_partition` would cause the
    /// producer to spawn a second driver pushing duplicate frames onto the channel.
    spawned: Mutex<HashSet<(u32, u32, u32)>>,
}

impl ProducerTaskRegistry {
    pub(super) fn new(
        stage_plans: StagePlans,
        session: Arc<SessionContext>,
        mesh: &Arc<MppMesh>,
        work_mem_bytes: usize,
        hash_mem_multiplier: f64,
    ) -> Self {
        Self {
            stage_plans,
            session,
            mesh: Arc::downgrade(mesh),
            work_mem_bytes,
            hash_mem_multiplier,
            active_drivers: Arc::new(AtomicUsize::new(0)),
            first_error: Arc::new(Mutex::new(None)),
            prepared: Mutex::new(HashMap::new()),
            spawned: Mutex::new(HashSet::new()),
        }
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
        let (plan, task_count) = self.stage_plans.lookup(stage_id).ok_or_else(|| {
            DataFusionError::Internal(format!(
                "mpp producer: no plan registered for stage_id={stage_id}"
            ))
        })?;
        // Seed the TaskContext with a `DistributedTaskContext` so nested boundary nodes inside
        // the prepared plan see `(task_index, task_count)` and address the right peer task.
        let cfg = self
            .session
            .state()
            .config()
            .clone()
            .with_extension(Arc::new(DistributedTaskContext {
                task_index: task_idx as usize,
                task_count,
            }));
        let memory_pool = create_memory_pool(&plan, self.work_mem_bytes, self.hash_mem_multiplier);
        let runtime_env = RuntimeEnvBuilder::new()
            .with_memory_pool(memory_pool)
            .build()
            .map_err(|e| {
                DataFusionError::Internal(format!("mpp producer: build RuntimeEnv: {e}"))
            })?;
        let task_ctx = Arc::new(
            TaskContext::default()
                .with_session_config(cfg)
                .with_runtime(Arc::new(runtime_env)),
        );

        // Wrap the stage's local plan in a fresh `DistributedExec` and `prepare_in_process_plan`
        // so nested NetworkShuffleExec / NetworkBroadcastExec / NetworkCoalesceExec dispatch
        // through `ShmMqWorkerTransport` (Stage::Remote) instead of the LocalStage path that
        // errors when task_count > 1.
        let dist = Arc::new(DistributedExec::new(Arc::clone(&plan)));
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

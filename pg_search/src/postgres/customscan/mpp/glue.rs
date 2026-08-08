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

//! High-level glue between PostgreSQL parallel-query callbacks and the
//! leader/worker MPP architecture.
//!
//! The leader-driven launch (`mpp::launch`) calls into this module; everything else (the
//! shared-memory layout, the ring mesh, the `WorkerTransport` plumbing) lives in
//! `datafusion_distributed::shm` and is reached through these thin wrappers:
//!
//! - [`mpp_is_active`] — gate for the customscan path-builder.
//! - [`estimate_dsm_size`] — mesh-region sizing for the plan-first launch.
//! - [`leader_setup`] — leader-side mesh init, called by `launch_mpp` after the workers
//!   spawn. Returns the leader's [`MppLeaderState`] which carries the runtime [`MppMesh`]
//!   handle the customscan installs on its DataFusion `SessionContext`.
//! - [`worker_setup`] — worker-side mesh attach, called from `run_launched_worker`. Returns
//!   the [`WorkerSession`] which carries the worker mesh, plan bytes, and outbound senders.

use std::ffi::c_void;
use std::sync::Arc;

use pgrx::pg_sys;

use datafusion_distributed::TaskKey;
use datafusion_distributed::shm::{self, Interrupt, LeaderSession, MppMesh, Wakeup, WorkerSession};
use datafusion_proto::physical_plan::DefaultPhysicalProtoConverter;

use crate::gucs::mpp_queue_size as gucs_mpp_queue_size;
use crate::postgres::customscan::mpp::pg_seams::{PgInterrupt, PgWakeup, pack_receiver};

/// Minimum total procs for MPP: leader (consumer-only, proc 0) plus at least 2 producers.
/// Below that, `build_mpp_session_context` still clamps `target_partitions` to 2 and the mesh
/// wouldn't have a queue for the second partition. Single source of truth so [`mpp_is_active`]
/// and the launch's clamp floor don't drift on the threshold.
pub const MIN_TOTAL_WORKER_COUNT: u32 = 3;

/// True iff PostgreSQL's parallelism budget allows at least `MIN_TOTAL_WORKER_COUNT - 1`
/// producer workers. There is no MPP-specific worker GUC (#5667):
/// `max_parallel_workers_per_gather` caps the per-query width — the same knob that caps
/// ordinary parallel scans, matching core PG's `compute_parallel_worker` convention — and
/// `max_parallel_workers` is the cluster-wide pool. Setting
/// `max_parallel_workers_per_gather` below 2 (or 0, PG's own parallelism-disable convention)
/// disables MPP. Customscan path-builders gate `parallel_workers` on this.
pub fn mpp_is_active() -> bool {
    producer_worker_cap() >= MIN_TOTAL_WORKER_COUNT - 1
}

// The shared-memory transport pins 8-byte alignment (its ring headers hold `u64` atomics). The
// builder's `shm_toc_allocate` hands out MAXALIGN-aligned blobs for the mesh region, so the two
// must agree or the rings would be misaligned, which is UB-class.
const _: () = assert!(pg_sys::MAXIMUM_ALIGNOF == 8);

/// Per-edge queue size from the GUC.
pub(super) fn mpp_queue_size() -> usize {
    gucs_mpp_queue_size()
}

/// Total DSM bytes the leader will need for the mesh header, the worker plan, and one MPSC
/// inbox per process. `n_procs` is the total proc count (leader + launched producers) the
/// plan-first launch computed from the physical plan — the mesh grows as `n_procs²`, so this
/// is sized from the plan's needs, not the GUC ceiling (#5667). A short launch may initialize a
/// narrower logical mesh in this allocation, but never a wider one.
pub fn estimate_dsm_size(n_procs: u32, plan_bytes_len: usize) -> Result<usize, String> {
    shm::dsm_region_bytes(n_procs, mpp_queue_size(), plan_bytes_len).map_err(|e| e.to_string())
}

/// Maximum number of producer workers a launch may spawn:
/// `min(max_parallel_workers_per_gather, max_parallel_workers)` (#5667). Also the planner's
/// `target_partitions` ceiling. The plan-first launch spawns
/// `clamp(max_stage_task_count, MIN_TOTAL_WORKER_COUNT - 1, this)` producers, so the cap only
/// bounds — the plan's task fragments decide the actual width. When [`mpp_is_active`] holds
/// this is `>= MIN_TOTAL_WORKER_COUNT - 1`.
pub fn producer_worker_cap() -> u32 {
    let max_per_gather = unsafe { pg_sys::max_parallel_workers_per_gather };
    let max_workers = unsafe { pg_sys::max_parallel_workers };
    max_per_gather.min(max_workers).max(0) as u32
}

/// Leader-observable per-phase timing of an MPP launch, in microseconds. Populated so
/// `EXPLAIN ANALYZE` can show where the launch floor goes. Zero means unmeasured. The
/// worker-side cost the leader can't see directly is captured indirectly: `attach_us` is the
/// spawn-plus-fork-plus-ring-attach wait, and `first_frame_us` folds in the workers' plan
/// decode and first scan.
#[derive(Default, Clone, Copy, Debug)]
pub struct MppLaunchTiming {
    /// DSM alloc and scan-state populate, before the dispatch payload is built.
    pub prepare_us: u64,
    /// Leader physical planning.
    pub plan_us: u64,
    /// Dispatch-payload serialization from the leader's own plan.
    pub payload_us: u64,
    /// Wait for every worker to fork and attach to the ring mesh.
    pub attach_us: u64,
    /// Leader ring-mesh init (`leader_setup`).
    pub leader_setup_us: u64,
    /// `plan.execute` stream construction.
    pub exec_us: u64,
    /// First batch out (worker decode, first scan, and network hop to the leader).
    pub first_frame_us: u64,
    /// Producer workers that attached.
    pub workers: u32,
}

impl MppLaunchTiming {
    /// The `MPP Launch` line for `EXPLAIN ANALYZE`, shared by JoinScan and AggregateScan so
    /// the launched width is observable (and regress-assertable) on both paths.
    pub fn explain_text(&self) -> String {
        format!(
            "workers={} prepare={}us plan={}us payload={}us attach={}us \
             leader_setup={}us exec={}us first_frame={}us",
            self.workers,
            self.prepare_us,
            self.plan_us,
            self.payload_us,
            self.attach_us,
            self.leader_setup_us,
            self.exec_us,
            self.first_frame_us,
        )
    }
}

/// Returned to the leader from [`leader_setup`]. The customscan stashes this on its execution
/// state and consults it during `exec_custom_scan`.
pub struct MppLeaderState {
    /// Active leader session handle holding the runtime mesh and control senders.
    pub session: LeaderSession,
    /// The builder handle owning the launched producer workers. The leader controls the launch, so
    /// it owns the teardown too: `end_custom_scan` takes this and calls `wait_for_finish` to join
    /// the workers and destroy the parallel context. `None` until `launch` installs it on the
    /// success path.
    pub finish: Option<crate::parallel_worker::builder::ParallelProcessFinish>,
    /// Per-phase launch timing the leader fills in as it commits, for `EXPLAIN ANALYZE`.
    pub timing: MppLaunchTiming,
}

/// The `(pgprocno, pid)` of this backend, packed into the receiver token the transport stores so a
/// producer's [`PgWakeup`] can `SetLatch` us. Read on the backend thread (both setup paths run
/// synchronously from the launch / worker entry before any tokio runtime spins up).
unsafe fn self_receiver_token() -> u64 {
    // `pg_sys::MyProcNumber` is the PG17+ global; PG15/16 carry the same value on
    // `MyProc->pgprocno` (it moved to a process-global plus a field rename in PG17).
    #[cfg(any(feature = "pg15", feature = "pg16"))]
    let my_pgprocno: i32 = unsafe { (*pg_sys::MyProc).pgprocno };
    #[cfg(not(any(feature = "pg15", feature = "pg16")))]
    let my_pgprocno: i32 = unsafe { pg_sys::MyProcNumber };
    let my_pid: i32 = unsafe { (*pg_sys::MyProc).pid };
    pack_receiver(my_pgprocno, my_pid)
}

/// Initialize the leader's ring mesh in a DSM region and build its [`MppLeaderState`]. Called by
/// the leader-driven [`crate::postgres::customscan::mpp::launch`] on a builder-allocated region.
///
/// # Safety
/// - `coordinate` must be the MPP region pointer (a `ParallelState` byte blob the leader owns).
/// - `n_procs` is the total mesh participant count including the leader: 1 (leader, proc 0)
///   plus the launched producer workers. It must not exceed the count passed to
///   [`estimate_dsm_size`] for this region. A short launch may use fewer processes than the
///   preallocated region was sized for.
/// - `plan_bytes` must have the same length passed to [`estimate_dsm_size`]
///   so the leader doesn't overrun the region.
pub unsafe fn leader_setup(
    coordinate: *mut c_void,
    n_procs: u32,
    plan_bytes: Vec<u8>,
) -> Result<MppLeaderState, String> {
    let wakeup: Arc<dyn Wakeup> = Arc::new(PgWakeup);
    let interrupt: Arc<dyn Interrupt> = Arc::new(PgInterrupt);
    // Register the leader as receiver so producers' wakeups resolve to this backend's procLatch.
    let token = unsafe { self_receiver_token() };
    // `mpp_trace` reads a pgrx GucSetting, which requires the backend thread. Safe here
    // because this runs synchronously on the leader backend from `launch_mpp`, before any
    // tokio runtime spins up.
    let t_setup = crate::gucs::mpp_trace().then(std::time::Instant::now);
    let session = unsafe {
        shm::leader_setup(
            coordinate,
            n_procs,
            mpp_queue_size(),
            &plan_bytes,
            wakeup,
            token,
            interrupt,
            /* attach_senders */ true,
        )
    }
    .map_err(|e| e.to_string())?;
    if let Some(t) = t_setup {
        pgrx::warning!(
            "mpp trace: leader_setup (ring create + self attach) took {:.3} ms",
            t.elapsed().as_secs_f64() * 1000.0
        );
    }
    Ok(MppLeaderState {
        session,
        finish: None,
        timing: MppLaunchTiming::default(),
    })
}

/// `on_dsm_detach` callback that invalidates the leader's mesh handles while the MPP segment is still mapped.
///
/// While the mesh is live, dropping a sender decrements an atomic inside its DSM ring. The
/// callback marks every handle dead while the mapping is live; an escaped clone can subsequently drop
/// without touching DSM. `on_dsm_detach` runs on every teardown path.
#[pgrx::pg_guard]
unsafe extern "C-unwind" fn mark_mesh_detached_on_dsm_detach(
    _seg: *mut pg_sys::dsm_segment,
    arg: pg_sys::Datum,
) {
    let mesh = unsafe { Arc::from_raw(arg.cast_mut_ptr::<MppMesh>()) };
    mesh.mark_detached();
}

impl MppLeaderState {
    /// Register the DSM detach cleanup for the leader mesh liveness flag. PostgreSQL
    /// stores the raw pointer; [`mark_mesh_detached_on_dsm_detach`] turns it back into the same
    /// `Arc` and drops it.
    ///
    /// # Safety
    /// `seg` must be the live DSM segment that owns the ring memory.
    pub unsafe fn register_control_senders_on_detach(&self, seg: *mut pg_sys::dsm_segment) {
        let mesh_ptr = Arc::into_raw(Arc::clone(&self.session.mesh));
        unsafe {
            pg_sys::on_dsm_detach(
                seg,
                Some(mark_mesh_detached_on_dsm_detach),
                pg_sys::Datum::from(mesh_ptr as *mut c_void),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion_distributed::shm::NoInterrupt;
    use std::sync::atomic::Ordering;

    struct NoopWakeup;

    impl Wakeup for NoopWakeup {
        fn wake(&self, _token: u64) {}
    }

    #[test]
    fn mesh_can_use_fewer_processes_than_its_reserved_region() {
        // `launch_mpp` must allocate DSM before PostgreSQL tells it how many workers actually
        // attached. Pin the transport contract that lets a short launch initialize its smaller
        // logical mesh in the region reserved for the requested width.
        let requested_procs = 6;
        let launched_procs = 4;
        let plan = [0_u8; 64];
        let region_bytes = shm::dsm_region_bytes(requested_procs, 64 * 1024, plan.len()).unwrap();
        let mut region = vec![0_u64; region_bytes.div_ceil(std::mem::size_of::<u64>())];

        let attach = unsafe {
            shm::leader_setup(
                region.as_mut_ptr().cast(),
                launched_procs,
                64 * 1024,
                &plan,
                Arc::new(NoopWakeup),
                1,
                Arc::new(NoInterrupt),
                true,
            )
            .unwrap()
        };

        assert_eq!(attach.outbound_senders().len(), launched_procs as usize);
        assert!(attach.outbound_senders()[0].is_none());
        assert_eq!(
            attach.outbound_senders().iter().flatten().count(),
            (launched_procs - 1) as usize
        );
    }

    #[test]
    fn dsm_detach_marks_mesh_before_a_set_plan_clone_can_drop() {
        // Sender Drop may touch ring memory while the mesh is live, so keep the backing region
        // alive through the final drop.
        let region_bytes = shm::dsm_region_bytes(3, 64 * 1024, 0).unwrap();
        let mut region = vec![0_u64; region_bytes.div_ceil(std::mem::size_of::<u64>())];
        let session = unsafe {
            shm::leader_setup(
                region.as_mut_ptr().cast(),
                3,
                64 * 1024,
                &[],
                Arc::new(NoopWakeup),
                1,
                Arc::new(NoInterrupt),
                true,
            )
            .unwrap()
        };
        let mesh = session.mesh;
        mesh.mark_detached();

        assert!(
            !mesh.detached_flag().load(Ordering::Acquire),
            "DSM detach must invalidate mesh handles before delayed SetPlan senders can drop"
        );
    }
}

/// Serializes each stage subplan the coordinator dispatches, with the pg codec
/// (`serialize_physical_plan`): the config-level codec extension point cannot express the
/// UDF-definition handling or the optimization-only wrapper strip, so the coordinator's own
/// encode cannot produce these bytes. The coordinator hands over its ready-to-run per-task
/// plan; scan encodes are context-free recipes, so the plan the leader runs is the plan the
/// workers decode, and each worker specializes its own segment slice. Every task of a stage
/// runs the same subplan here (nothing in a pg plan is task-specialized), so the encode is
/// cached by stage.
#[derive(Default)]
pub struct StagePlanDispatchSource {
    cache: std::sync::Mutex<std::collections::HashMap<(usize, usize), Vec<u8>>>,
}

impl datafusion_distributed::DispatchPlanSource for StagePlanDispatchSource {
    fn dispatch_plan_proto(
        &self,
        task: &TaskKey,
        specialized: &Arc<dyn datafusion::physical_plan::ExecutionPlan>,
    ) -> Option<datafusion::common::Result<Vec<u8>>> {
        // One source per query execution, so the query id never varies here.
        let stage_id = task.stage_id;
        let task_number = task.task_number;
        let key = (stage_id, task_number);
        if let Some(bytes) = self.cache.lock().unwrap().get(&key) {
            return Some(Ok(bytes.clone()));
        }
        // The default converter stamps every expression with its `expression_id`, which is what
        // lets the worker's deduplicating decode re-share dynamic filter instances.
        let bytes = match crate::scan::physical_codec::serialize_physical_plan(
            Arc::clone(specialized),
            &DefaultPhysicalProtoConverter {},
        ) {
            Ok(bytes) => bytes,
            Err(e) => return Some(Err(e)),
        };
        self.cache.lock().unwrap().insert(key, bytes.clone());
        Some(Ok(bytes))
    }
}

/// Body of `initialize_worker_custom_scan`. Reads the header, attaches as
/// sender on this worker's slot row, copies the plan bytes out of DSM.
///
/// # Safety
/// - `coordinate` must be the DSM region pointer PG supplied.
/// - `region_total` must match the DSM's attached size.
pub unsafe fn worker_setup(
    coordinate: *mut c_void,
    region_total: usize,
    worker_number: i32,
) -> Result<WorkerSession, String> {
    if worker_number < 0 {
        return Err("mpp: worker_number < 0".into());
    }
    // Leader is `proc_idx = 0`, workers are `1..n_procs`. Worker N maps from PG's
    // `ParallelWorkerNumber = N` to `proc_idx = N + 1`.
    let proc_idx = (worker_number as u32) + 1;

    let wakeup: Arc<dyn Wakeup> = Arc::new(PgWakeup);
    let interrupt: Arc<dyn Interrupt> = Arc::new(PgInterrupt);
    // Register before the transport starts polling, so a producer racing ahead sees a valid token.
    let token = unsafe { self_receiver_token() };
    // Same backend-thread story as `leader_setup`: this runs on the parallel-worker backend
    // before tokio starts.
    let t_setup = crate::gucs::mpp_trace().then(std::time::Instant::now);
    let session =
        unsafe { shm::worker_setup(coordinate, region_total, proc_idx, wakeup, token, interrupt) }
            .map_err(|e| e.to_string())?;
    if let Some(t) = t_setup {
        pgrx::warning!(
            "mpp trace: worker_setup (attach) took {:.3} ms",
            t.elapsed().as_secs_f64() * 1000.0
        );
    }

    Ok(session)
}

/// Merge the worker fragments' `TaskMetrics` frames into an executed `DistributedExec` plan for
/// EXPLAIN ANALYZE. The workers send their frames as they exit, after the leader's gather
/// already finished, so nothing has drained the leader inbox since; sweep it, file the frames
/// into the plan's metrics store, and rewrite. Returns the rewritten plan, or `None` when there
/// is nothing to merge (serial plan, metrics disabled) or a frame never arrived (the rewrite is
/// bounded rather than trusting `wait_for_metrics`, which would block on a dead worker).
/// Drain the workers' `TaskMetrics` frames off the mesh into the plan's metrics store.
///
/// Must run while the parallel DSM is still mapped: the mesh receivers read ring memory inside
/// it. `shutdown_custom_scan` is the spot; the EXPLAIN hook runs after `ExecShutdownNode` tore
/// the DSM down, so draining there reads unmapped memory.
pub fn drain_worker_metrics(
    plan: &Arc<dyn datafusion::physical_plan::ExecutionPlan>,
    mesh: &Arc<MppMesh>,
) -> Option<()> {
    use datafusion::common::tree_node::{TreeNode, TreeNodeRecursion};
    use datafusion_distributed::shm::CooperativeDrainSet;
    use datafusion_distributed::{DistributedExec, NetworkBoundaryExt};

    let dist = plan.downcast_ref::<DistributedExec>()?;
    let store = dist.metrics_store()?;

    // The wire frames carry (stage, task); the query uuid lives on the plan's own stages. Count
    // the expected reports while walking: one per task of every producer stage.
    let mut query_id = None;
    let mut expected = 0usize;
    let _ = plan.apply(|node| {
        if let Some(nb) = node.as_network_boundary() {
            let stage = nb.input_stage();
            query_id.get_or_insert_with(|| stage.query_id());
            expected += stage.task_count();
        }
        Ok(TreeNodeRecursion::Continue)
    });
    let query_id = query_id?;

    // The workers send their metrics frames right after their last EOF, which may still be in
    // flight when shutdown reaches this node; wait briefly, bounded, and stop as soon as every
    // expected (stage, task) reported. Draining keeps the DSM ring from backing up before detach.
    let mut rx = mesh.take_task_metrics_receiver()?;
    let mut got = crate::api::HashSet::default();
    for _ in 0..100 {
        let _ = mesh.try_drain_pass();
        while let Ok((stage_id, task_number, metrics)) = rx.try_recv() {
            // The frames carry proto metrics; the store holds the decoded in-memory form the rewrite
            // reads. A frame that fails to decode is still counted so the wait doesn't spin.
            if let Ok(metrics) = datafusion_distributed::decode_task_metrics(metrics) {
                store.insert(
                    TaskKey {
                        query_id,
                        stage_id: stage_id as usize,
                        task_number: task_number as usize,
                    },
                    metrics,
                );
            }
            got.insert((stage_id, task_number));
        }
        if got.len() >= expected {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    Some(())
}

/// Rewrite the executed plan with the worker metrics collected by [`drain_worker_metrics`].
/// Mesh-free, so it is safe at EXPLAIN-render time, after the DSM is gone.
///
/// Owns a small timer-enabled runtime: the rewrite waits on the metrics store, and the bound on
/// that wait needs timers, which the scans' cached runtimes don't enable.
pub fn merge_worker_metrics(
    plan: &Arc<dyn datafusion::physical_plan::ExecutionPlan>,
) -> Option<Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
    use datafusion_distributed::DistributedExec;

    plan.downcast_ref::<DistributedExec>()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    runtime
        .block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_millis(250),
                datafusion_distributed::rewrite_distributed_plan_with_metrics(
                    Arc::clone(plan),
                    datafusion_distributed::DistributedMetricsFormat::PerTask,
                ),
            )
            .await
        })
        .ok()?
        .ok()
}

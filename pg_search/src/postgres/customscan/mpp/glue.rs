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
//! Customscan code calls into this module from four hooks; everything else (the shared-memory
//! layout, the ring mesh, the `WorkerTransport` plumbing) lives in
//! `datafusion_distributed::shm` and is reached through these thin wrappers:
//!
//! - [`mpp_is_active`] — gate for the customscan path-builder.
//! - [`estimate_dsm_size`] — `estimate_dsm_custom_scan` body.
//! - [`leader_setup`] — `initialize_dsm_custom_scan` body. Returns the
//!   leader's [`MppLeaderState`] which carries the runtime [`MppMesh`]
//!   handle the customscan installs on its DataFusion `SessionContext`.
//! - [`worker_setup`] — `initialize_worker_custom_scan` body. Returns the
//!   worker's [`MppWorkerState`] which carries the worker's outbound
//!   senders and the deserialized plan bytes the worker runs.

use std::ffi::c_void;
use std::sync::Arc;

use pgrx::pg_sys;

use datafusion_distributed::proto::SetPlanRequest;
use datafusion_distributed::shm::{
    self, proc_for_task, CooperativeDrainSet, Interrupt, MppFrameHeader, MppMesh, MppSender,
    SendBatchStats, SetPlanFrame, Wakeup,
};
use datafusion_distributed::TaskKey;

use crate::postgres::customscan::mpp::dispatch::StagePlan;

use crate::gucs::{
    enable_mpp, mpp_queue_size as gucs_mpp_queue_size, mpp_worker_count as gucs_mpp_worker_count,
};
use crate::postgres::customscan::mpp::pg_seams::{pack_receiver, PgInterrupt, PgWakeup};
use crate::postgres::ParallelScanState;

/// Minimum total procs for MPP: leader (consumer-only) plus at least 2 producers. Single
/// source of truth so [`mpp_is_active`] and [`mpp_worker_count`] don't drift on the
/// threshold. Below 3, [`producer_worker_count`] would be 1 while
/// `build_mpp_session_context` still clamps `target_partitions` to 2; the mesh wouldn't
/// have a queue for the second partition.
const MIN_TOTAL_WORKER_COUNT: i32 = 3;

/// True iff `paradedb.enable_mpp = on` and `paradedb.mpp_worker_count >=
/// MIN_TOTAL_WORKER_COUNT`. Customscan path-builders gate `parallel_workers` on this.
/// Also requires that the system has enough `max_parallel_workers` and
/// `max_parallel_workers_per_gather` to launch the requested number of producers.
pub fn mpp_is_active() -> bool {
    let active = enable_mpp() && gucs_mpp_worker_count() >= MIN_TOTAL_WORKER_COUNT;
    if !active {
        return false;
    }

    let producer_count = gucs_mpp_worker_count() - 1;
    let max_per_gather = unsafe { pg_sys::max_parallel_workers_per_gather };
    let max_workers = unsafe { pg_sys::max_parallel_workers };

    producer_count <= max_per_gather && producer_count <= max_workers
}

/// Total proc count: leader + producers. Equals the GUC value when [`mpp_is_active`] is
/// true. Callers must gate on [`mpp_is_active`] first. Debug builds assert; release builds
/// return the raw GUC, which can leave [`producer_worker_count`] below 2 and break the
/// planner's `target_partitions` / mesh-width invariant.
pub fn mpp_worker_count() -> u32 {
    debug_assert!(
        mpp_is_active(),
        "mpp_worker_count() called when mpp_is_active() is false — callers must gate first"
    );
    gucs_mpp_worker_count() as u32
}

// The shared-memory transport pins 8-byte alignment (its ring headers hold `u64` atomics). The
// builder's `shm_toc_allocate` hands out MAXALIGN-aligned blobs for the mesh region, so the two
// must agree or the rings would be misaligned, which is UB-class.
const _: () = assert!(pg_sys::MAXIMUM_ALIGNOF == 8);

/// Per-edge queue size from the GUC.
pub(super) fn mpp_queue_size() -> usize {
    gucs_mpp_queue_size()
}

/// Body of `estimate_dsm_custom_scan`. Returns the total DSM bytes the leader will need
/// for the header, the worker plan, and one MPSC inbox per process. `n_procs` is the
/// total proc count (leader + `producer_worker_count()` parallel workers).
pub fn estimate_dsm_size(plan_bytes_len: usize) -> Result<usize, String> {
    shm::dsm_region_bytes(mpp_worker_count(), mpp_queue_size(), plan_bytes_len)
        .map_err(|e| e.to_string())
}

/// Number of producer workers PG should launch as `parallel_workers`.
/// `mpp_worker_count - 1` because proc 0 is the leader (consumer-only). Callers must gate
/// on [`mpp_is_active`] first; when active, [`MIN_TOTAL_WORKER_COUNT`] guarantees this is
/// `>= 2` without further clamping.
pub fn producer_worker_count() -> u32 {
    mpp_worker_count() - 1
}

/// Returned to the leader from [`leader_setup`]. The customscan stashes this on its execution
/// state and consults it during `exec_custom_scan`.
///
/// The leader is consumer-only: it gathers fragments from worker procs but doesn't host a
/// producer fragment itself. Its outbound senders are dropped inside `leader_setup`.
pub struct MppLeaderState {
    /// Runtime mesh handle. Install on the leader's `SessionContext` via
    /// `with_extension(Arc::clone(&mesh))` so `ShmChannelResolver` can find
    /// it at execute time.
    pub mesh: Arc<MppMesh>,
    /// The leader's outbound senders, one per peer inbox; the control-plane path for `SetPlan`
    /// frames. Held for the query's lifetime so no ring observes a sender count of zero before
    /// every worker attaches (the rings latch `detached` permanently at zero).
    ///
    /// Dropping one of these decrements a counter inside the DSM ring, so they must never
    /// outlive the mapping: [`MppLeaderState::release_control_senders`] clears them from
    /// `shutdown_custom_scan` on the success path, and a transaction-abort callback (registered
    /// in [`leader_setup`]) clears them on the error path, both before PG detaches the DSM.
    /// The scan state's own drop runs after detach and must find this empty.
    pub control_senders: Arc<std::sync::Mutex<Vec<Option<MppSender>>>>,
    /// Plans for [`deliver_set_plans`], which runs at exec time when the launched workers are
    /// draining their inboxes. Sending from the init callback instead could fill a small ring
    /// with no drainer behind it and wedge the leader. Kept (not drained) so a parallel rescan
    /// can re-deliver to relaunched workers, the frame analog of the plan blob persisting in
    /// DSM. Mutex because the exec hook only sees a shared borrow of the scan state.
    pub stage_plans: std::sync::Mutex<Vec<StagePlan>>,
    /// One delivery per worker generation: set by [`deliver_set_plans`], reset by the rescan
    /// path before workers relaunch.
    pub plans_delivered: std::sync::atomic::AtomicBool,
    /// The builder handle owning the launched producer workers. The leader controls the launch, so
    /// it owns the teardown too: `end_custom_scan` takes this and calls `wait_for_finish` to join
    /// the workers and destroy the parallel context. `None` until `launch` installs it on the
    /// success path.
    pub finish: Option<crate::parallel_worker::builder::ParallelProcessFinish>,
    /// The shared `ParallelScanState` the leader populated in DSM. The leader runs the top fragment
    /// itself, and a non-partitioning source can land there (e.g. the SEMI/ANTI broadcast strategy),
    /// where the scan claims per-source segments against this state just like a worker. The leader
    /// stashes it on its custom state so the codec installs it into those providers. Null until
    /// `launch` sets it.
    pub parallel_state: *mut ParallelScanState,
}

/// The `(pgprocno, pid)` of this backend, packed into the receiver token the transport stores so a
/// producer's [`PgWakeup`] can `SetLatch` us. Read on the backend thread (both setup paths run
/// synchronously from PG's custom-scan init hooks before any tokio runtime spins up).
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
/// - `plan_bytes` must have the same length passed to [`estimate_dsm_size`]
///   so the leader doesn't overrun the region.
pub unsafe fn leader_setup(
    coordinate: *mut c_void,
    plan_bytes: Vec<u8>,
    stage_plans: Vec<StagePlan>,
) -> Result<MppLeaderState, String> {
    let wakeup: Arc<dyn Wakeup> = Arc::new(PgWakeup);
    let interrupt: Arc<dyn Interrupt> = Arc::new(PgInterrupt);
    // Register the leader as receiver so producers' wakeups resolve to this backend's procLatch.
    let token = unsafe { self_receiver_token() };
    // `mpp_trace` reads a pgrx GucSetting, which requires the backend thread. Safe here
    // because this runs synchronously from `initialize_dsm_custom_scan` before any tokio
    // runtime spins up.
    let t_setup = crate::gucs::mpp_trace().then(std::time::Instant::now);
    let attach = unsafe {
        shm::leader_setup(
            coordinate,
            mpp_worker_count(),
            mpp_queue_size(),
            &plan_bytes,
            wakeup,
            token,
            interrupt,
            // The leader ships `SetPlan` frames (and later, work units) through these senders.
            // `MppLeaderState` holds them for the query's lifetime, which keeps the rings'
            // sender count above zero across every worker attach.
            /* attach_senders */
            true,
        )
    }
    .map_err(|e| e.to_string())?;
    let mesh = attach.mesh;
    if let Some(t) = t_setup {
        pgrx::warning!(
            "mpp trace: leader_setup (ring create + self attach) took {:.3} ms",
            t.elapsed().as_secs_f64() * 1000.0
        );
    }
    let control_senders = Arc::new(std::sync::Mutex::new(attach.outbound_senders));
    // Hand the senders to the mesh too, so its early-termination cancel can reach the producers.
    // The mesh shares this `Arc`, so clearing it below releases both views before the DSM unmaps.
    mesh.set_cancel_senders(Arc::clone(&control_senders));
    // On abort, `AbortTransaction` destroys the parallel contexts (unmapping the DSM) before it
    // runs the xact callbacks, so by the time these fire the senders' ring headers are gone.
    // Dropping a sender writes a detach signal into its ring header, which would fault; forget
    // them instead. The signal has no audience anyway: the abort already terminated the workers.
    // The few heap bytes leaked per sender only accrue on aborted MPP queries.
    //
    // The commit hook covers a subtransaction rollback of this scan: no top-level abort fires,
    // and the senders would otherwise sit populated until pgrx drops the unused abort closure at
    // commit, past the DSM's lifetime. A non-empty vec at commit always means the mapping is
    // already gone (the success path cleared it in `shutdown_custom_scan`), so forgetting is
    // the only safe disposal on both events.
    let forget_senders = |senders: &Arc<std::sync::Mutex<Vec<Option<MppSender>>>>| {
        let senders = Arc::clone(senders);
        move || {
            for sender in senders.lock().unwrap().drain(..).flatten() {
                std::mem::forget(sender);
            }
        }
    };
    pgrx::register_xact_callback(
        pgrx::PgXactCallbackEvent::Abort,
        forget_senders(&control_senders),
    );
    pgrx::register_xact_callback(
        pgrx::PgXactCallbackEvent::PreCommit,
        forget_senders(&control_senders),
    );
    Ok(MppLeaderState {
        mesh,
        control_senders,
        stage_plans: std::sync::Mutex::new(stage_plans),
        plans_delivered: std::sync::atomic::AtomicBool::new(false),
        finish: None,
        parallel_state: std::ptr::null_mut(),
    })
}

impl MppLeaderState {
    /// Drop the DSM-backed control senders while the mapping is still attached. Called from
    /// `shutdown_custom_scan`; the abort callback covers the error path. Idempotent.
    pub fn release_control_senders(&self) {
        self.control_senders.lock().unwrap().clear();
    }
}

/// Ship every dispatched plan as `SetPlan` frames: one per `(stage, task)`, to the proc hosting
/// the task, carrying the same `SetPlanRequest` Flight would put on its coordinator stream.
///
/// Runs at exec time, after the launched-worker check: the workers are attaching and draining by
/// then, so a plan bigger than a ring drains through instead of wedging the send spin. One
/// delivery per worker generation; re-execution without a relaunch is a no-op.
pub fn deliver_set_plans(leader: &MppLeaderState) -> Result<(), String> {
    if leader
        .plans_delivered
        .swap(true, std::sync::atomic::Ordering::SeqCst)
    {
        return Ok(());
    }
    let stage_plans = leader.stage_plans.lock().unwrap().clone();
    if stage_plans.is_empty() {
        return Ok(());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("mpp: set-plan runtime build: {e}"))?;
    let n_workers = leader.mesh.n_workers();
    runtime.block_on(async {
        let mut stats = SendBatchStats::default();
        for sp in &stage_plans {
            for task in 0..sp.task_count {
                let dest = proc_for_task(n_workers, task as u32);
                // Clone the sender out under the lock so the guard never spans the await below.
                let sender = {
                    let senders = leader.control_senders.lock().unwrap();
                    let Some(base) = senders.get(dest as usize).and_then(|s| s.as_ref()) else {
                        return Err(format!("mpp: no leader sender for proc {dest}"));
                    };
                    base.clone_with_header(MppFrameHeader::set_plan(sp.stage_num, task as u32, 0))
                        .with_cooperative_drain(
                            Arc::clone(&leader.mesh) as Arc<dyn CooperativeDrainSet>
                        )
                };
                let frame = SetPlanFrame {
                    set_plan: Some(SetPlanRequest {
                        plan_proto: sp.plan_proto.clone(),
                        task_count: sp.task_count as u64,
                        task_key: Some(datafusion_distributed::proto::TaskKey {
                            query_id: sp.query_id.clone(),
                            stage_id: sp.stage_num as u64,
                            task_number: task as u64,
                        }),
                        work_unit_feed_declarations: vec![],
                        target_worker_url: String::new(),
                        query_start_time_ns: 0,
                    }),
                    header_keys: vec![],
                    header_values: vec![],
                };
                sender
                    .send_set_plan_traced(&frame, &mut stats)
                    .await
                    .map_err(|e| format!("mpp: set-plan send failed: {e}"))?;
            }
        }
        Ok(())
    })
}

/// Returned to a worker from [`worker_setup`]. The customscan reads the plan bytes, runs the
/// plan, and pushes resulting batches through `outbound_senders`.
pub struct MppWorkerState {
    /// `outbound_senders[proc_idx]` is the sender that writes to peer `proc_idx`'s inbox.
    /// The entry at `proc_idx == this_proc` is the self-loop in-proc channel installed by
    /// `worker_setup` (since DSM MPSC inboxes have only one receiver per ring, the worker
    /// can't be both producer and consumer on the shm_mq inbox path).
    ///
    /// Each fragment's per-partition output sender is keyed by
    /// `outbound_senders[proc_for_task(n_workers, consumer_task)]`. Each `MppSender` wraps an
    /// `Arc<dyn BatchChannelSender>` so callers can `clone_with_header` to multiplex
    /// `(stage_id, partition)` channels onto one inbox.
    pub outbound_senders: Vec<Option<MppSender>>,
    /// Leader's dispatch payload (framed per-stage physical subplans), copied out of DSM. The
    /// worker decodes it into its fragment assignments via `mpp::dispatch::expand_to_assignments`.
    pub plan_bytes: Vec<u8>,
    /// Worker's MppMesh. The single `inbound_receiver` pulls frames addressed to this
    /// proc from both the DSM MPSC inbox and the in-proc self-loop channel; demux by
    /// `(sender_proc, stage_id, partition)` happens inside the handle's channel-buffer
    /// registry. Read by the multi-fragment dispatcher driven by [`mpp::host::exec_mpp_worker`].
    pub mesh: Arc<MppMesh>,
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
) -> Result<MppWorkerState, String> {
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
    let attach =
        unsafe { shm::worker_setup(coordinate, region_total, proc_idx, wakeup, token, interrupt) }
            .map_err(|e| e.to_string())?;
    if let Some(t) = t_setup {
        pgrx::warning!(
            "mpp trace: worker_setup (attach) took {:.3} ms",
            t.elapsed().as_secs_f64() * 1000.0
        );
    }

    Ok(MppWorkerState {
        outbound_senders: attach.outbound_senders,
        plan_bytes: attach.plan_bytes,
        mesh: attach.mesh,
    })
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

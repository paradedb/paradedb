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
#![allow(rustdoc::private_intra_doc_links)]

//! Tonic/gRPC channel resolver (`DsmGrpcChannelResolver`), leader session-context builders
//! (`build_mpp_plan_context` / `build_mpp_exec_context`), and MPP background worker server loop
//! (`run_mpp_worker`) over DSM `AsyncRead`/`AsyncWrite` SPSC byte rings.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::memory_pool::MemoryPool;
use datafusion::execution::{SessionState, SessionStateBuilder};
use datafusion::prelude::SessionContext;
use datafusion_distributed::grpc::{BoxCloneSyncChannel, create_worker_client};
use datafusion_distributed::{
    ChannelResolver, DistributedConfig, DistributedExt, SessionStateBuilderExt, Worker,
    WorkerChannel, WorkerPlanRewriteEvent, WorkerPlanRewriteEventResponse, WorkerQueryContext,
    WorkerResolver,
};
use hyper_util::rt::TokioIo;
use parking_lot::Mutex;
use pgrx::pg_sys;
use tonic::transport::{Endpoint, Server, Uri};
use tower::service_fn;
use url::Url;

use crate::index::mvcc::SegmentView;
use crate::postgres::ParallelScanState;
use crate::postgres::customscan::datafusion::memory::{WorkMemMemoryPool, build_runtime_env};
use crate::postgres::customscan::datafusion::{all_pg_search_udafs, all_pg_search_udfs};
use crate::postgres::customscan::mpp::dsm_stream::{
    DsmAsyncStream, ServerProc, build_worker_streams, proc_wakeup_slot,
};
use crate::postgres::customscan::mpp::glue::{
    BgWorkerCount, MppBudget, MppLeaderState, WorkerSession, register_worker_streams_on_dsm_detach,
};
use crate::postgres::customscan::mpp::interrupt::{
    HeldInterrupts, PgTokioRuntime, check_for_interrupts,
};
use crate::postgres::customscan::mpp::launch::SCAN_IDX;
use crate::postgres::customscan::mpp::wakeup::WakeupDispatcher;
use crate::postgres::utils::ExprContextGuard;
use crate::scan::execution_plan::{
    pg_search_scan_desired_task_count, pg_search_scan_scale_up_leaf_node,
};
use crate::scan::physical_codec::PgSearchPhysicalExtensionCodec;

/// Worker URL resolver mapping active MPP participants to `http://worker-0` (when
/// `LeaderParticipation::Participating`) followed by `http://worker-1` .. `http://worker-{W}`.
#[derive(Clone, Debug)]
pub struct MppWorkerResolver {
    urls: Vec<Url>,
}

impl MppWorkerResolver {
    pub fn new(budget: MppBudget) -> Self {
        let participants = budget.participants();
        assert!(
            participants.is_mpp_active(),
            "MppWorkerResolver requires at least two MPP participants"
        );
        let mut urls = Vec::with_capacity(participants.as_usize());
        if budget.leader.is_participating() {
            urls.push(Self::worker_url(ServerProc(0)));
        }
        urls.extend((1..=budget.bg_workers.get()).map(|w| Self::worker_url(ServerProc(w))));
        Self { urls }
    }

    pub fn worker_url(worker_proc: ServerProc) -> Url {
        Url::parse(&format!("http://worker-{}", worker_proc.0)).expect("valid worker URL")
    }

    pub fn parse_worker_proc(url: &Url) -> Result<ServerProc> {
        let host = url.host_str().ok_or_else(|| {
            DataFusionError::Internal(format!("mpp: worker URL '{url}' missing host"))
        })?;
        let idx_str = host.strip_prefix("worker-").ok_or_else(|| {
            DataFusionError::Internal(format!("mpp: unexpected worker URL '{url}'"))
        })?;
        let idx: u32 = idx_str.parse().map_err(|e| {
            DataFusionError::Internal(format!("mpp: invalid worker index in '{url}': {e}"))
        })?;
        Ok(ServerProc(idx))
    }
}

impl WorkerResolver for MppWorkerResolver {
    fn get_urls(&self) -> Result<Vec<Url>> {
        Ok(self.urls.clone())
    }
}

struct ResolverState {
    dispatcher: WakeupDispatcher,
    stream_slots: HashMap<ServerProc, Arc<Mutex<Option<DsmAsyncStream>>>>,
    channels: HashMap<ServerProc, BoxCloneSyncChannel>,
}

/// `ChannelResolver` backed by pre-allocated DSM [`DsmAsyncStream`]s.
///
/// Each outbound `(client, server)` stream is consumed once by `Endpoint::connect_with_connector_lazy`
/// and cached as a multiplexed HTTP/2 [`BoxCloneSyncChannel`] for the duration of the query.
pub struct DsmGrpcChannelResolver {
    bg_workers: BgWorkerCount,
    state: Mutex<ResolverState>,
}

impl DsmGrpcChannelResolver {
    pub fn new(
        bg_workers: BgWorkerCount,
        dispatcher: WakeupDispatcher,
        streams: Vec<(ServerProc, DsmAsyncStream)>,
    ) -> Self {
        let stream_slots = streams
            .into_iter()
            .map(|(server, stream)| (server, Arc::new(Mutex::new(Some(stream)))))
            .collect();
        Self {
            bg_workers,
            state: Mutex::new(ResolverState {
                dispatcher,
                stream_slots,
                channels: HashMap::new(),
            }),
        }
    }

    pub fn bg_workers(&self) -> BgWorkerCount {
        self.bg_workers
    }

    /// Ensure the `AsyncFd` wakeup dispatcher is started on `handle` (required before spawning an
    /// inbound Tonic `Worker` server on the leader, since remote peers may write to `ServerProc(0)`
    /// before the leader opens any outbound channel).
    pub fn ensure_dispatcher_started(&self, handle: &tokio::runtime::Handle) -> Result<()> {
        let mut state = self.state.lock();
        if !state.dispatcher.is_started() {
            state.dispatcher.start(handle).map_err(|e| {
                DataFusionError::Internal(format!(
                    "mpp: failed to start AsyncFd wakeup dispatcher: {e}"
                ))
            })?;
        }
        Ok(())
    }

    /// Close all DSM streams and abort the `AsyncFd` wakeup reader task before DSM unmaps.
    pub fn close_all(&self) {
        let mut state = self.state.lock();
        state.channels.clear();
        for (_, slot) in state.stream_slots.drain() {
            if let Some(stream) = slot.lock().take() {
                stream.close_rings();
            }
        }
        state.dispatcher.stop();
    }
}

/// HTTP/2 per-stream flow-control window (`4 MiB`, matching `HTTP2_MAX_FRAME_SIZE`) so shuffle
/// batches do not stall on `65,535`-byte `WINDOW_UPDATE` round-trips over the DSM rings while
/// keeping per-connection transport buffering bounded.
const HTTP2_STREAM_WINDOW_SIZE: u32 = 4 * 1024 * 1024;

/// HTTP/2 connection-level flow-control window (`8 MiB`, also passed to
/// `with_distributed_worker_connection_buffer_budget_bytes`).
const HTTP2_CONNECTION_WINDOW_SIZE: u32 = 8 * 1024 * 1024;

/// HTTP/2 maximum frame payload size (`4 MiB`, up from the `16 KiB` spec default) so `FlightData`
/// messages are sent and received in a single `DATA` frame without `16 KiB` chunking/reassembly.
const HTTP2_MAX_FRAME_SIZE: u32 = 4 * 1024 * 1024;

fn mpp_grpc_server_builder() -> Server {
    Server::builder()
        .initial_stream_window_size(HTTP2_STREAM_WINDOW_SIZE)
        .initial_connection_window_size(HTTP2_CONNECTION_WINDOW_SIZE)
        .max_frame_size(HTTP2_MAX_FRAME_SIZE)
}

#[async_trait]
impl ChannelResolver for DsmGrpcChannelResolver {
    async fn get_worker_client_for_url(&self, url: &Url) -> Result<Box<dyn WorkerChannel>> {
        let server_proc = MppWorkerResolver::parse_worker_proc(url)?;
        let mut state = self.state.lock();
        if let Some(ch) = state.channels.get(&server_proc) {
            return Ok(create_worker_client(ch.clone()));
        }

        if !state.dispatcher.is_started() {
            let handle = tokio::runtime::Handle::try_current().map_err(|e| {
                DataFusionError::Internal(format!(
                    "mpp: DsmGrpcChannelResolver requires an active Tokio runtime: {e}"
                ))
            })?;
            state.dispatcher.start(&handle).map_err(|e| {
                DataFusionError::Internal(format!(
                    "mpp: failed to start AsyncFd wakeup dispatcher: {e}"
                ))
            })?;
        }

        let stream_slot = state
            .stream_slots
            .get(&server_proc)
            .cloned()
            .ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "mpp: no pending DSM stream for worker {} ({url})",
                    server_proc.0
                ))
            })?;

        let endpoint = Endpoint::from_shared(url.as_str().to_owned())
            .map_err(|e| DataFusionError::Internal(format!("mpp: invalid endpoint '{url}': {e}")))?
            .initial_stream_window_size(HTTP2_STREAM_WINDOW_SIZE)
            .initial_connection_window_size(HTTP2_CONNECTION_WINDOW_SIZE)
            .max_frame_size(HTTP2_MAX_FRAME_SIZE);
        let tonic_channel = endpoint.connect_with_connector_lazy(service_fn(move |_: Uri| {
            let slot = Arc::clone(&stream_slot);
            async move {
                let s = slot.lock().take().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::AddrInUse,
                        "mpp: single-use DSM stream already connected; HTTP/2 connection dropped unexpectedly",
                    )
                })?;
                Ok::<_, std::io::Error>(TokioIo::new(s))
            }
        }));
        let boxed = BoxCloneSyncChannel::new(tonic_channel);
        state.channels.insert(server_proc, boxed.clone());
        Ok(create_worker_client(boxed))
    }
}

/// Whether `DistributedExec` and MPP workers (remote background workers and, when participating,
/// leader `worker-0`) should collect and stream `TaskMetrics` over `coordinator_channel` (enabled
/// only under `EXPLAIN ANALYZE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MppMetricsCollection {
    Disabled,
    Enabled,
}

impl MppMetricsCollection {
    pub(crate) fn from_instrument(instrument: *mut pg_sys::Instrumentation) -> Self {
        if instrument.is_null() {
            Self::Disabled
        } else {
            Self::Enabled
        }
    }

    pub(crate) fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

/// Build a distributed `SessionContext` for physical planning or plain `EXPLAIN` (before any DSM
/// worker channels exist).
pub(crate) fn build_mpp_plan_context(
    seed: SessionContext,
    budget: MppBudget,
    metrics: MppMetricsCollection,
) -> SessionContext {
    build_mpp_session_context(seed, None, budget, metrics)
}

/// Build the leader's post-launch execution `SessionContext` from `leader` and, when
/// `leader.leader_participation.is_participating()`, attach the in-process `worker-0`
/// `LocalWorkerContext` and spawn its `ServerProc(0)` Tonic gRPC server task on `runtime`.
///
/// Bundling context construction with `setup_leader_worker` guarantees that `MppWorkerResolver`
/// never advertises `http://worker-0` without a live leader worker serving it.
pub(crate) fn build_mpp_exec_context(
    seed: SessionContext,
    leader: &mut MppLeaderState,
    runtime: &PgTokioRuntime,
    leader_spilled: Arc<std::sync::atomic::AtomicBool>,
    metrics: MppMetricsCollection,
) -> SessionContext {
    let exec_ctx = build_mpp_session_context(
        seed,
        Some(Arc::clone(&leader.session.resolver)),
        MppBudget::new(leader.bg_workers, leader.leader_participation),
        metrics,
    );
    setup_leader_worker(leader, exec_ctx, runtime, leader_spilled)
}

fn build_mpp_session_context(
    seed: SessionContext,
    resolver: Option<Arc<DsmGrpcChannelResolver>>,
    budget: MppBudget,
    metrics: MppMetricsCollection,
) -> SessionContext {
    if let Some(r) = resolver.as_ref() {
        debug_assert_eq!(r.bg_workers(), budget.bg_workers);
    }
    let participants = budget.participants();
    assert!(
        participants.is_mpp_active(),
        "MPP session contexts require at least two participants"
    );
    let n_participants = participants.as_usize();

    let cfg = seed.copied_config().with_target_partitions(n_participants);

    debug_assert!(
        seed.state()
            .config()
            .options()
            .extensions
            .get::<DistributedConfig>()
            .is_none(),
        "build_mpp_session_context: seed already carries a DistributedConfig; the bootstrap \
         below would overwrite it"
    );

    let mut state_builder = SessionStateBuilder::new_from_existing(seed.state())
        .with_config(cfg)
        .with_distributed_option_extension(DistributedConfig::default())
        .with_distributed_worker_resolver(MppWorkerResolver::new(budget));

    if let Some(resolver) = resolver {
        let dyn_resolver: Arc<dyn ChannelResolver + Send + Sync> = resolver;
        state_builder = state_builder.with_distributed_channel_resolver(dyn_resolver);
    }

    let state_builder = state_builder
        .with_distributed_user_codec(PgSearchPhysicalExtensionCodec::default())
        .with_distributed_desired_task_count_handler(pg_search_scan_desired_task_count)
        .with_distributed_scale_up_leaf_node_handler(pg_search_scan_scale_up_leaf_node)
        .with_distributed_desired_task_count_handler(n_participants)
        .with_distributed_compression(None)
        .expect("with_distributed_compression")
        .with_distributed_worker_connection_buffer_budget_bytes(
            HTTP2_CONNECTION_WINDOW_SIZE as usize,
        )
        .expect("with_distributed_worker_connection_buffer_budget_bytes")
        .with_distributed_broadcast_joins(true)
        .expect("with_distributed_broadcast_joins")
        // Disable the use of distributed dynamic filters until we fix
        // https://github.com/paradedb/datafusion-distributed/issues/107
        .with_distributed_remote_dynamic_filters(false)
        .expect("with_distributed_remote_dynamic_filters")
        .with_distributed_dynamic_filter_collection(false)
        .expect("with_distributed_dynamic_filter_collection")
        .with_distributed_metrics_collection(metrics.is_enabled())
        .expect("with_distributed_metrics_collection")
        .with_distributed_planner();
    SessionContext::new_with_state(state_builder.build())
}

#[derive(Clone, Copy)]
struct SpillNotifier(*mut ParallelScanState);

// SAFETY: `ParallelScanState::mark_spilled` only stores to an `AtomicU32` inside the DSM segment.
unsafe impl Send for SpillNotifier {}
unsafe impl Sync for SpillNotifier {}

impl SpillNotifier {
    fn mark_spilled(self) {
        unsafe { (*self.0).mark_spilled() }
    }
}

static WORKER_BASE_SESSION_STATE: std::sync::LazyLock<SessionState> =
    std::sync::LazyLock::new(|| {
        let mut builder = SessionStateBuilder::new()
            .with_default_features()
            .with_analyzer_rules(vec![])
            .with_optimizer_rules(vec![])
            .with_physical_optimizer_rules(vec![]);
        builder
            .scalar_functions()
            .get_or_insert_with(Vec::new)
            .extend(all_pg_search_udfs());
        builder
            .aggregate_functions()
            .get_or_insert_with(Vec::new)
            .extend(all_pg_search_udafs());
        builder.build()
    });

/// Pre-initializes [`WORKER_BASE_SESSION_STATE`] in `postmaster` during `_PG_init` so forked
/// parallel background workers inherit the built function registries via copy-on-write without
/// paying `SessionStateBuilder::with_default_features` startup cost on each query.
pub fn init_worker_base_session_state() {
    std::sync::LazyLock::force(&WORKER_BASE_SESSION_STATE);
}

/// Process-wide base [`SessionState`] for MPP workers: pre-registers DataFusion's default
/// function registries plus `pg_search` UDFs/UDAFs with empty analyzer/optimizer rule lists so
/// neither `Worker` creation nor per-task `coordinator_channel` setup rebuilds them, and applies
/// current GUC settings (`apply_paradedb_session_config` plus `inlist_mode`) once per query.
fn worker_base_session_state(inlist_mode: crate::gucs::InListPushdownMode) -> SessionState {
    let mut cfg = WORKER_BASE_SESSION_STATE.config().clone();
    crate::postgres::customscan::joinscan::scan_state::apply_paradedb_session_config(&mut cfg);
    if inlist_mode == crate::gucs::InListPushdownMode::LazySortProbe {
        cfg.options_mut()
            .optimizer
            .hash_join_inlist_pushdown_max_distinct_values = inlist_mode.max_distinct_values();
    }
    SessionStateBuilder::new_from_existing(WORKER_BASE_SESSION_STATE.clone())
        .with_config(cfg)
        .build()
}

/// Inputs passed to [`run_mpp_worker`] by `launch::run_launched_worker`.
pub(crate) struct MppWorkerInputs {
    pub parallel_state: *mut ParallelScanState,
    pub session: WorkerSession,
}

/// Extract a human-readable message from a panic payload (including `pgrx` `ErrorReportWithLevel`
/// and `CaughtError`).
pub fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    use pgrx::pg_sys::panic::{CaughtError, ErrorReportWithLevel};

    if let Some(report) = payload.downcast_ref::<ErrorReportWithLevel>() {
        return report.message().to_string();
    }
    if let Some(caught) = payload.downcast_ref::<CaughtError>() {
        return match caught {
            CaughtError::RustPanic { ereport, .. } => ereport.message().to_string(),
            CaughtError::PostgresError(report) | CaughtError::ErrorReport(report) => {
                report.message().to_string()
            }
        };
    }
    if let Some(s) = payload.downcast_ref::<&str>() {
        return (*s).to_string();
    }
    if let Some(s) = payload.downcast_ref::<String>() {
        return s.clone();
    }
    "unknown panic payload".to_string()
}

fn build_mpp_worker(
    parallel_state: *mut ParallelScanState,
    expr_context: Option<*mut pg_sys::ExprContext>,
    peer_resolver: Arc<DsmGrpcChannelResolver>,
    on_spill: Arc<dyn Fn() + Send + Sync>,
) -> Worker {
    let plan_sources_count = unsafe { (*parallel_state).source_count() };
    let mut index_segment_views: Vec<SegmentView> =
        vec![SegmentView::default(); plan_sources_count];
    for (i, slot) in index_segment_views.iter_mut().enumerate() {
        *slot = unsafe { (*parallel_state).segment_view_for_source_unlocked(i) };
    }

    let work_mem_bytes = unsafe { pg_sys::work_mem as usize * 1024 };
    let hash_mem_multiplier = unsafe { pg_sys::hash_mem_multiplier };
    let inlist_mode = unsafe { (*parallel_state).inlist_pushdown_mode };
    let codec = PgSearchPhysicalExtensionCodec::new_worker(
        Some(parallel_state),
        index_segment_views,
        expr_context,
    );

    Worker::from_session_builder(move |ctx: WorkerQueryContext| {
        let codec = codec.clone();
        let peer_resolver: Arc<dyn ChannelResolver + Send + Sync> =
            Arc::clone(&peer_resolver) as Arc<dyn ChannelResolver + Send + Sync>;

        // `Worker::coordinator_channel` builds `SessionState` and `TaskContext` before decoding
        // `request.plan`, so start the stage pool with `work_mem` and resize it from the decoded
        // stage plan in `with_distributed_worker_plan_rewrite_handler`.
        let stage_pool = Arc::new(WorkMemMemoryPool::new(work_mem_bytes));
        let runtime_env = build_runtime_env(
            Arc::clone(&stage_pool) as Arc<dyn MemoryPool>,
            Arc::clone(&on_spill),
        );
        async move {
            Ok(ctx
                .builder
                .with_runtime_env(runtime_env)
                .with_distributed_user_codec(codec)
                .with_distributed_channel_resolver(peer_resolver)
                .with_distributed_worker_plan_rewrite_handler(
                    move |ev: WorkerPlanRewriteEvent<'_>| {
                        stage_pool.resize_for_plan(&ev.plan, work_mem_bytes, hash_mem_multiplier);
                        Ok(WorkerPlanRewriteEventResponse::new(ev.plan))
                    },
                )
                .build())
        }
    })
    .with_base_session_state(worker_base_session_state(inlist_mode))
}

/// When `leader.leader_participation.is_participating()`, attach an in-process `Worker` at
/// `http://worker-0` (`LocalWorkerContext`) onto `exec_ctx` and spawn a Tonic gRPC `Worker` server
/// on `runtime` over `ServerProc(0)`'s inbound DSM SPSC streams so remote background workers can
/// also pull shuffle partitions from `worker-0`. No-op when `CoordinatorOnly`.
fn setup_leader_worker(
    leader: &mut MppLeaderState,
    exec_ctx: SessionContext,
    runtime: &PgTokioRuntime,
    leader_spilled: Arc<std::sync::atomic::AtomicBool>,
) -> SessionContext {
    if !leader.leader_participation.is_participating() {
        return exec_ctx;
    }

    let finish = leader
        .finish
        .as_ref()
        .expect("participating leader must hold ParallelProcessFinish");
    let scan_ptr = match finish.state_manager().slice_mut::<u8>(SCAN_IDX) {
        Ok(Some(s)) => s.as_mut_ptr() as *mut ParallelScanState,
        _ => pgrx::error!("mpp leader: parallel scan state missing from parallel state"),
    };
    let server_streams = leader
        .server_streams
        .take()
        .expect("participating leader must have inbound DSM server streams");

    let spill_notifier = SpillNotifier(scan_ptr);
    let on_spill: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
        spill_notifier.mark_spilled();
        leader_spilled.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    let worker = build_mpp_worker(
        scan_ptr,
        None,
        Arc::clone(&leader.session.resolver),
        on_spill,
    );
    let exec_ctx = exec_ctx.with_distributed_local_worker_context(
        worker.to_local_worker_context(MppWorkerResolver::worker_url(ServerProc(0))),
    );

    if let Err(e) = leader
        .session
        .resolver
        .ensure_dispatcher_started(runtime.handle())
    {
        pgrx::error!("mpp leader: {e}");
    }

    let incoming = tokio_stream::StreamExt::chain(
        tokio_stream::iter(
            server_streams
                .incoming_server_streams
                .into_iter()
                .map(Ok::<_, std::io::Error>),
        ),
        tokio_stream::pending(),
    );
    let shutdown_watcher = server_streams.shutdown_watcher;
    let server_task = runtime.spawn(async move {
        mpp_grpc_server_builder()
            .add_service(worker.into_worker_server())
            .serve_with_incoming_shutdown(incoming, shutdown_watcher.wait())
            .await
    });
    leader.session.install_server_task(server_task);
    exec_ctx
}

/// Run a parallel worker's Tonic gRPC `Worker` server over its DSM SPSC streams until the leader
/// signals query teardown or a Postgres cancel/die interrupt arrives.
pub(crate) fn run_mpp_worker(inputs: MppWorkerInputs, runtime: &PgTokioRuntime) {
    let MppWorkerInputs {
        parallel_state,
        session: worker_session,
    } = inputs;

    let worker_proc = worker_session.worker_proc;
    let this_proc = worker_proc.0;

    let expr_context_guard = ExprContextGuard::new();

    let slot = match unsafe { proc_wakeup_slot(worker_session.dsm_base, this_proc) } {
        Ok(s) => s,
        Err(e) => pgrx::error!("mpp worker (proc={this_proc}): {e}"),
    };
    let mut dispatcher = WakeupDispatcher::new(slot);
    if let Err(e) = dispatcher.start(runtime.handle()) {
        pgrx::error!("mpp worker (proc={this_proc}): start wakeup dispatcher failed: {e}");
    }

    let streams =
        match unsafe { build_worker_streams(worker_session.dsm_base, worker_proc, &dispatcher) } {
            Ok(s) => s,
            Err(e) => {
                pgrx::error!("mpp worker (proc={this_proc}): build_worker_streams failed: {e}");
            }
        };
    let detach_handle = streams.detach_handle;
    unsafe {
        register_worker_streams_on_dsm_detach(worker_session.dsm_seg, detach_handle.clone());
    }

    // In the symmetric mesh across `0..n_procs`, every background worker sees `n_procs - 1` peers
    // (`0` plus `n_procs - 2` sibling background workers), so total background workers is
    // `streams.incoming_server_streams.len()`.
    let bg_workers = BgWorkerCount::new(streams.incoming_server_streams.len() as u32);
    let peer_resolver = Arc::new(DsmGrpcChannelResolver::new(
        bg_workers,
        dispatcher,
        streams.client_streams,
    ));

    let spill_state = SpillNotifier(parallel_state);
    let worker = build_mpp_worker(
        parallel_state,
        Some(expr_context_guard.as_ptr()),
        Arc::clone(&peer_resolver),
        Arc::new(move || spill_state.mark_spilled()),
    );
    let incoming = tokio_stream::StreamExt::chain(
        tokio_stream::iter(
            streams
                .incoming_server_streams
                .into_iter()
                .map(Ok::<_, std::io::Error>),
        ),
        tokio_stream::pending(),
    );
    let shutdown_watcher = streams.shutdown_watcher;

    let held = HeldInterrupts::hold();
    let mpp_worker_guard = runtime.enter_mpp_worker();
    let server_task = runtime.spawn(async move {
        mpp_grpc_server_builder()
            .add_service(worker.into_worker_server())
            .serve_with_incoming_shutdown(incoming, shutdown_watcher.wait())
            .await
    });
    let serve_res = runtime.block_on(server_task);
    detach_handle.close_worker_rings();
    peer_resolver.close_all();
    runtime.shutdown_workers();
    drop(mpp_worker_guard);
    drop(held);
    check_for_interrupts();

    match serve_res {
        Ok(Ok(())) => {}
        Ok(Err(e)) => pgrx::error!("mpp worker (proc={this_proc}): gRPC server error: {e}"),
        Err(e) => pgrx::error!("mpp worker (proc={this_proc}): gRPC server task failed: {e}"),
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::postgres::customscan::mpp::dsm_stream::{
        DEFAULT_SPSC_RING_CAPACITY, assign_wakeup_slots, build_leader_streams, dsm_region_bytes,
        init_dsm_mesh,
    };
    use crate::postgres::customscan::mpp::wakeup::reserve_wakeup_slots;
    use arrow_array::{Int32Array, RecordBatch};
    use arrow_schema::{DataType, Field, Schema};
    use datafusion::datasource::MemTable;
    use datafusion::physical_plan::collect;
    use pgrx::pg_test;

    #[repr(C, align(64))]
    #[derive(Clone)]
    struct AlignedBlock([u8; 64]);

    #[pg_test]
    fn worker_executes_distributed_plan_over_dsm_async_stream() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        // Exercise both `(bg_workers = 2, CoordinatorOnly)` and `(bg_workers = 1, Participating)`
        // (including two-stage shuffle between `worker-0` on the leader and `worker-1`).
        for budget in [
            MppBudget::new(
                BgWorkerCount::new(2),
                crate::postgres::customscan::mpp::glue::LeaderParticipation::CoordinatorOnly,
            ),
            MppBudget::new(
                BgWorkerCount::new(1),
                crate::postgres::customscan::mpp::glue::LeaderParticipation::Participating,
            ),
        ] {
            rt.block_on(async move {
                let bg_workers = budget.bg_workers;
                let leader_mode = budget.leader;
                let n_procs = bg_workers.mesh_procs();
                let participants = budget.participants().as_usize();
                let bytes = dsm_region_bytes(n_procs, DEFAULT_SPSC_RING_CAPACITY).unwrap();
                let mut backing = vec![AlignedBlock([0u8; 64]); bytes.div_ceil(64)];
                let base = backing.as_mut_ptr() as *mut u8;

                unsafe {
                    init_dsm_mesh(base, n_procs, DEFAULT_SPSC_RING_CAPACITY).unwrap();
                }

                let reservation = reserve_wakeup_slots(n_procs as usize).unwrap();
                let slots = reservation.slots();
                unsafe {
                    assign_wakeup_slots(base, &slots).unwrap();
                }
                let handle = tokio::runtime::Handle::current();

                let mut worker_tasks = Vec::new();
                for w in 1..=bg_workers.get() {
                    let mut w_disp = WakeupDispatcher::new(slots[w as usize]);
                    w_disp.start(&handle).unwrap();
                    let w_streams =
                        unsafe { build_worker_streams(base, ServerProc(w), &w_disp).unwrap() };
                    let peer_resolver = Arc::new(DsmGrpcChannelResolver::new(
                        bg_workers,
                        w_disp,
                        w_streams.client_streams,
                    ));
                    let peer_resolver_for_shutdown = Arc::clone(&peer_resolver);
                    let worker = Worker::from_session_builder(move |ctx: WorkerQueryContext| {
                        let peer_resolver: Arc<dyn ChannelResolver + Send + Sync> =
                            Arc::clone(&peer_resolver) as Arc<dyn ChannelResolver + Send + Sync>;
                        async move {
                            Ok(ctx
                                .builder
                                .with_distributed_channel_resolver(peer_resolver)
                                .build())
                        }
                    });
                    let incoming = tokio_stream::StreamExt::chain(
                        tokio_stream::iter(
                            w_streams
                                .incoming_server_streams
                                .into_iter()
                                .map(Ok::<_, std::io::Error>),
                        ),
                        tokio_stream::pending(),
                    );
                    let shutdown_watcher = w_streams.shutdown_watcher;
                    worker_tasks.push(tokio::spawn(async move {
                        mpp_grpc_server_builder()
                            .add_service(worker.into_worker_server())
                            .serve_with_incoming_shutdown(incoming, shutdown_watcher.wait())
                            .await
                            .unwrap();
                        peer_resolver_for_shutdown.close_all();
                    }));
                }

                let mut leader_disp = WakeupDispatcher::new(slots[0]);
                leader_disp.start(&handle).unwrap();
                let leader_streams = unsafe { build_leader_streams(base, &leader_disp).unwrap() };
                let leader_resolver = Arc::new(DsmGrpcChannelResolver::new(
                    bg_workers,
                    leader_disp,
                    leader_streams.client_streams,
                ));

                let mut local_worker_opt = None;
                if leader_mode.is_participating() {
                    let leader_peer_resolver = Arc::clone(&leader_resolver);
                    let leader_worker =
                        Worker::from_session_builder(move |ctx: WorkerQueryContext| {
                            let peer_resolver: Arc<dyn ChannelResolver + Send + Sync> =
                                Arc::clone(&leader_peer_resolver)
                                    as Arc<dyn ChannelResolver + Send + Sync>;
                            async move {
                                Ok(ctx
                                    .builder
                                    .with_distributed_channel_resolver(peer_resolver)
                                    .build())
                            }
                        });
                    local_worker_opt = Some(
                        leader_worker
                            .to_local_worker_context(MppWorkerResolver::worker_url(ServerProc(0))),
                    );
                    let incoming = tokio_stream::StreamExt::chain(
                        tokio_stream::iter(
                            leader_streams
                                .incoming_server_streams
                                .into_iter()
                                .map(Ok::<_, std::io::Error>),
                        ),
                        tokio_stream::pending(),
                    );
                    let shutdown_watcher = leader_streams.shutdown_watcher;
                    worker_tasks.push(tokio::spawn(async move {
                        mpp_grpc_server_builder()
                            .add_service(leader_worker.into_worker_server())
                            .serve_with_incoming_shutdown(incoming, shutdown_watcher.wait())
                            .await
                            .unwrap();
                    }));
                } else {
                    drop(leader_streams.incoming_server_streams);
                    drop(leader_streams.shutdown_watcher);
                }

                let dyn_resolver: Arc<dyn ChannelResolver + Send + Sync> =
                    Arc::clone(&leader_resolver) as Arc<dyn ChannelResolver + Send + Sync>;
                let seed = SessionContext::new();
                let cfg = seed.copied_config().with_target_partitions(participants);
                let mut builder = SessionStateBuilder::new_from_existing(seed.state())
                    .with_config(cfg)
                    .with_distributed_option_extension(DistributedConfig::default())
                    .with_distributed_worker_resolver(MppWorkerResolver::new(budget))
                    .with_distributed_channel_resolver(dyn_resolver)
                    .with_distributed_desired_task_count_handler(participants)
                    .with_distributed_compression(None)
                    .expect("with_distributed_compression")
                    .with_distributed_planner();
                if let Some(lw) = local_worker_opt {
                    builder = builder.with_distributed_local_worker_context(lw);
                }
                let ctx = SessionContext::new_with_state(builder.build());
                let schema = Arc::new(Schema::new(vec![
                    Field::new("k", DataType::Int32, false),
                    Field::new("v", DataType::Int32, false),
                ]));
                let b1 = RecordBatch::try_new(
                    Arc::clone(&schema),
                    vec![
                        Arc::new(Int32Array::from(vec![1, 2, 1, 2])),
                        Arc::new(Int32Array::from(vec![10, 20, 30, 40])),
                    ],
                )
                .unwrap();
                let b2 = RecordBatch::try_new(
                    Arc::clone(&schema),
                    vec![
                        Arc::new(Int32Array::from(vec![1, 2, 3])),
                        Arc::new(Int32Array::from(vec![5, 15, 100])),
                    ],
                )
                .unwrap();
                let table = MemTable::try_new(schema, vec![vec![b1], vec![b2]]).unwrap();
                ctx.register_table("t", Arc::new(table)).unwrap();

                use datafusion::functions_aggregate::expr_fn::sum;
                use datafusion::prelude::col;

                let df = ctx
                    .table("t")
                    .await
                    .unwrap()
                    .aggregate(vec![col("k")], vec![sum(col("v")).alias("s")])
                    .unwrap()
                    .sort(vec![col("k").sort(true, true)])
                    .unwrap();
                let plan = df.create_physical_plan().await.unwrap();
                let batches = collect(plan, ctx.task_ctx()).await.unwrap();

                let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
                assert_eq!(total_rows, 3);

                leader_streams.shutdown_handle.signal_shutdown();
                leader_resolver.close_all();
                for t in worker_tasks {
                    t.await.unwrap();
                }
                assert_eq!(leader_streams.shutdown_handle.active_streams(), 0);
                reservation.release();
            });
        }
    }
}

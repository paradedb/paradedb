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

//! Tonic/gRPC channel resolver (`DsmGrpcChannelResolver`) and MPP worker server loop
//! (`run_mpp_worker`) over DSM `AsyncRead`/`AsyncWrite` SPSC byte rings.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::memory_pool::{
    MemoryLimit, MemoryPool, MemoryReservation, UnboundedMemoryPool,
};
use datafusion::execution::{SessionState, SessionStateBuilder};
use datafusion::prelude::SessionContext;
use datafusion_distributed::grpc::{BoxCloneSyncChannel, create_worker_client};
use datafusion_distributed::{
    ChannelResolver, DistributedConfig, DistributedExt, RouteTaskEvent, RouteTaskEventResponse,
    RouteTaskHandler, SessionStateBuilderExt, Worker, WorkerChannel, WorkerPlanRewriteEvent,
    WorkerPlanRewriteEventResponse, WorkerPlanRewriteHandler, WorkerQueryContext, WorkerResolver,
    WorkerSessionBuilder,
};
use hyper_util::rt::TokioIo;
use parking_lot::Mutex;
use pgrx::pg_sys;
use tonic::transport::{Endpoint, Server, Uri};
use tower::service_fn;
use url::Url;

use crate::index::mvcc::SegmentView;
use crate::postgres::ParallelScanState;
use crate::postgres::customscan::datafusion::memory::{build_runtime_env, create_memory_pool};
use crate::postgres::customscan::datafusion::{all_pg_search_udafs, all_pg_search_udfs};
use crate::postgres::customscan::mpp::dsm_stream::{
    DsmAsyncStream, ServerProc, build_worker_streams,
};
use crate::postgres::customscan::mpp::glue::{WorkerSession, producer_worker_cap};
use crate::postgres::customscan::mpp::interrupt::{
    HeldInterrupts, PgTokioRuntime, check_for_interrupts,
};
use crate::postgres::customscan::mpp::wakeup::{WakeupDispatcher, self_wakeup_slot};
use crate::postgres::utils::ExprContextGuard;
use crate::scan::execution_plan::{
    pg_search_scan_desired_task_count, pg_search_scan_scale_up_leaf_node, visit_scan_nodes,
};
use crate::scan::physical_codec::PgSearchPhysicalExtensionCodec;

/// Worker URL resolver mapping `n_workers` parallel workers to `http://worker-1` .. `http://worker-{n}`.
#[derive(Clone, Debug)]
pub struct MppWorkerResolver {
    urls: Vec<Url>,
}

impl MppWorkerResolver {
    pub fn new(n_workers: usize) -> Self {
        let urls = (1..=n_workers)
            .map(|w| Self::worker_url(ServerProc(w as u32)))
            .collect();
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
        if idx == 0 {
            return Err(DataFusionError::Internal(format!(
                "mpp: worker index must be >= 1 in '{url}'"
            )));
        }
        Ok(ServerProc(idx))
    }
}

impl WorkerResolver for MppWorkerResolver {
    fn get_urls(&self) -> Result<Vec<Url>> {
        Ok(self.urls.clone())
    }
}

/// Deterministic `task_number % W` task router.
///
/// Ensures task `i` of every stage lands on `http://worker-{(i % W) + 1}` so that co-located
/// tasks across stages short-circuit in memory via `LocalWorkerContext` (`InProcessWorkerChannel`)
/// without touching DSM.
struct RoundRobinRouteTaskHandler;

#[async_trait]
impl RouteTaskHandler for RoundRobinRouteTaskHandler {
    async fn handle(&self, ev: RouteTaskEvent<'_>) -> Option<Result<RouteTaskEventResponse>> {
        let urls = datafusion_distributed::ok_or_some_err!(ev.worker_resolver.get_urls());
        if urls.is_empty() {
            return None;
        }
        let url = urls[ev.task_key.task_number % urls.len()].clone();
        Some(ev.dialer.dial(url).await)
    }
}

/// `ChannelResolver` backed by pre-allocated DSM [`DsmAsyncStream`]s.
///
/// Each outbound `(client, server)` stream is consumed once by `Endpoint::connect_with_connector_lazy`
/// and cached as a multiplexed HTTP/2 [`BoxCloneSyncChannel`] for the duration of the query.
pub struct DsmGrpcChannelResolver {
    n_workers: usize,
    dispatcher: Mutex<WakeupDispatcher>,
    pending_streams: Mutex<HashMap<ServerProc, DsmAsyncStream>>,
    channels: tokio::sync::Mutex<HashMap<ServerProc, BoxCloneSyncChannel>>,
}

impl DsmGrpcChannelResolver {
    pub fn new(
        n_workers: usize,
        dispatcher: WakeupDispatcher,
        streams: Vec<(ServerProc, DsmAsyncStream)>,
    ) -> Self {
        let mut map = HashMap::with_capacity(streams.len());
        for (server, stream) in streams {
            map.insert(server, stream);
        }
        Self {
            n_workers,
            dispatcher: Mutex::new(dispatcher),
            pending_streams: Mutex::new(map),
            channels: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    pub fn n_workers(&self) -> usize {
        self.n_workers
    }

    /// Close all DSM streams and abort the `AsyncFd` wakeup reader task before DSM unmaps.
    pub fn close_all(&self) {
        if let Ok(mut channels) = self.channels.try_lock() {
            channels.clear();
        }
        for (_, stream) in self.pending_streams.lock().drain() {
            stream.close_rings();
        }
        self.dispatcher.lock().stop();
    }

    fn ensure_dispatcher_started(&self) -> Result<()> {
        let mut disp = self.dispatcher.lock();
        if !disp.is_started() {
            let handle = tokio::runtime::Handle::try_current().map_err(|e| {
                DataFusionError::Internal(format!(
                    "mpp: DsmGrpcChannelResolver requires an active Tokio runtime: {e}"
                ))
            })?;
            disp.start(&handle).map_err(|e| {
                DataFusionError::Internal(format!(
                    "mpp: failed to start AsyncFd wakeup dispatcher: {e}"
                ))
            })?;
        }
        Ok(())
    }
}

/// HTTP/2 per-stream flow-control window (`16 MiB`) so large shuffle batches never stall on
/// `65,535`-byte `WINDOW_UPDATE` round-trips over the DSM rings.
const HTTP2_STREAM_WINDOW_SIZE: u32 = 16 * 1024 * 1024;

/// HTTP/2 connection-level flow-control window (`32 MiB`).
const HTTP2_CONNECTION_WINDOW_SIZE: u32 = 32 * 1024 * 1024;

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
        let mut channels = self.channels.lock().await;
        if let Some(ch) = channels.get(&server_proc) {
            return Ok(create_worker_client(ch.clone()));
        }

        self.ensure_dispatcher_started()?;

        let stream = self
            .pending_streams
            .lock()
            .remove(&server_proc)
            .ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "mpp: no pending DSM stream for worker {} ({url})",
                    server_proc.0
                ))
            })?;

        let stream_slot = Arc::new(Mutex::new(Some(stream)));
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
        channels.insert(server_proc, boxed.clone());
        Ok(create_worker_client(boxed))
    }
}

/// Build the distributed session context.
///
/// The leader calls this with `resolver = None` at plan/EXPLAIN time, and with `resolver = Some(...)`
/// at execution time after launching parallel workers and initializing the DSM rings.
pub(crate) fn build_mpp_session_context(
    seed: SessionContext,
    resolver: Option<Arc<DsmGrpcChannelResolver>>,
) -> SessionContext {
    let n_workers = match resolver.as_ref() {
        Some(r) => r.n_workers(),
        None => producer_worker_cap() as usize,
    };
    assert!(
        n_workers >= 2,
        "MPP session contexts require at least two producer workers"
    );

    let cfg = seed.copied_config().with_target_partitions(n_workers);

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
        .with_distributed_worker_resolver(MppWorkerResolver::new(n_workers))
        .with_distributed_route_task_handler(RoundRobinRouteTaskHandler);

    if let Some(resolver) = resolver {
        let dyn_resolver: Arc<dyn ChannelResolver + Send + Sync> = resolver;
        state_builder = state_builder.with_distributed_channel_resolver(dyn_resolver);
    }

    let state_builder = state_builder
        .with_distributed_user_codec(PgSearchPhysicalExtensionCodec::default())
        .with_distributed_desired_task_count_handler(pg_search_scan_desired_task_count)
        .with_distributed_scale_up_leaf_node_handler(pg_search_scan_scale_up_leaf_node)
        .with_distributed_desired_task_count_handler(n_workers)
        .with_distributed_compression(None)
        .expect("with_distributed_compression")
        .with_distributed_broadcast_joins(true)
        .expect("with_distributed_broadcast_joins")
        // Disable the use of distributed dynamic filters until we fix
        // https://github.com/paradedb/datafusion-distributed/issues/107
        .with_distributed_remote_dynamic_filters(false)
        .expect("with_distributed_remote_dynamic_filters")
        .with_distributed_planner();
    SessionContext::new_with_state(state_builder.build())
}

/// Memory pool wrapper that starts unbounded during `Worker::coordinator_channel` plan decode
/// and swaps in the plan-sized `WorkMemMemoryPool` inside [`MppWorkerPlanRewriteHandler`] before
/// execution begins.
#[derive(Debug)]
struct DelegatingMemoryPool {
    inner: Mutex<Arc<dyn MemoryPool>>,
}

impl DelegatingMemoryPool {
    fn new(initial: Arc<dyn MemoryPool>) -> Self {
        Self {
            inner: Mutex::new(initial),
        }
    }

    fn set_inner(&self, pool: Arc<dyn MemoryPool>) {
        *self.inner.lock() = pool;
    }
}

impl std::fmt::Display for DelegatingMemoryPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DelegatingMemoryPool")
    }
}

impl MemoryPool for DelegatingMemoryPool {
    fn name(&self) -> &str {
        "DelegatingMemoryPool"
    }

    fn grow(&self, reservation: &MemoryReservation, additional: usize) {
        let pool = Arc::clone(&self.inner.lock());
        pool.grow(reservation, additional);
    }

    fn shrink(&self, reservation: &MemoryReservation, returned: usize) {
        let pool = Arc::clone(&self.inner.lock());
        pool.shrink(reservation, returned);
    }

    fn try_grow(&self, reservation: &MemoryReservation, additional: usize) -> Result<()> {
        let pool = Arc::clone(&self.inner.lock());
        pool.try_grow(reservation, additional)
    }

    fn reserved(&self) -> usize {
        let pool = Arc::clone(&self.inner.lock());
        pool.reserved()
    }

    fn memory_limit(&self) -> MemoryLimit {
        let pool = Arc::clone(&self.inner.lock());
        pool.memory_limit()
    }
}

#[derive(Clone, Copy)]
struct SpillNotifier(*mut ParallelScanState);

unsafe impl Send for SpillNotifier {}
unsafe impl Sync for SpillNotifier {}

impl SpillNotifier {
    fn mark_spilled(self) {
        unsafe { (*self.0).mark_spilled() }
    }
}

struct SendPtr<T>(*mut T);

impl<T> Clone for SendPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SendPtr<T> {}

unsafe impl<T> Send for SendPtr<T> {}
unsafe impl<T> Sync for SendPtr<T> {}

/// Validates decoded `PgSearchScanPlan` source indices and swaps the stage's
/// [`DelegatingMemoryPool`] from `UnboundedMemoryPool` to a plan-sized `WorkMemMemoryPool`
/// without altering the `ExecutionPlan` topology (preserving pre-order `TaskMetrics` indices).
struct MppWorkerPlanRewriteHandler {
    plan_sources_count: usize,
    this_proc: u32,
    work_mem_bytes: usize,
    hash_mem_multiplier: f64,
}

#[async_trait]
impl WorkerPlanRewriteHandler for MppWorkerPlanRewriteHandler {
    async fn handle(
        &self,
        ev: WorkerPlanRewriteEvent<'_>,
    ) -> Result<WorkerPlanRewriteEventResponse> {
        let mut bad_idx = None;
        visit_scan_nodes(&ev.plan, &mut |scan| {
            if let Some(idx) = scan.source_idx()
                && idx >= self.plan_sources_count
            {
                bad_idx = Some(idx);
            }
        });
        if let Some(idx) = bad_idx {
            return Err(DataFusionError::Internal(format!(
                "mpp worker: scan claims from segment pool {idx} but the DSM holds \
                 {} pools (proc={})",
                self.plan_sources_count, self.this_proc
            )));
        }

        if let Some(stage_pool) = ev.session_config.get_extension::<DelegatingMemoryPool>() {
            let work_mem_pool =
                create_memory_pool(&ev.plan, self.work_mem_bytes, self.hash_mem_multiplier);
            stage_pool.set_inner(work_mem_pool);
        }

        Ok(WorkerPlanRewriteEventResponse { plan: ev.plan })
    }
}

#[derive(Clone)]
struct MppWorkerSessionBuilder {
    parallel_state: SendPtr<ParallelScanState>,
    index_segment_views: Vec<SegmentView>,
    expr_context: SendPtr<pg_sys::ExprContext>,
    peer_resolver: Arc<DsmGrpcChannelResolver>,
    plan_sources_count: usize,
    this_proc: u32,
    work_mem_bytes: usize,
    hash_mem_multiplier: f64,
}

#[async_trait]
impl WorkerSessionBuilder for MppWorkerSessionBuilder {
    async fn build_session_state(&self, ctx: WorkerQueryContext) -> Result<SessionState> {
        let codec = PgSearchPhysicalExtensionCodec::new_worker(
            Some(self.parallel_state.0),
            self.index_segment_views.clone(),
            Some(self.expr_context.0),
        );
        let rewrite_handler = MppWorkerPlanRewriteHandler {
            plan_sources_count: self.plan_sources_count,
            this_proc: self.this_proc,
            work_mem_bytes: self.work_mem_bytes,
            hash_mem_multiplier: self.hash_mem_multiplier,
        };

        // Each stage task gets its own `DelegatingMemoryPool` + `RuntimeEnv`: unbounded during
        // `request.plan.decode(&task_ctx)`, then swapped to `WorkMemMemoryPool` in
        // `MppWorkerPlanRewriteHandler::handle`.
        let stage_pool = Arc::new(DelegatingMemoryPool::new(Arc::new(
            UnboundedMemoryPool::default(),
        )));
        let spill_state = SpillNotifier(self.parallel_state.0);
        let runtime_env = build_runtime_env(
            Arc::clone(&stage_pool) as Arc<dyn MemoryPool>,
            Arc::new(move || spill_state.mark_spilled()),
        );

        let peer_resolver: Arc<dyn ChannelResolver + Send + Sync> =
            Arc::clone(&self.peer_resolver) as Arc<dyn ChannelResolver + Send + Sync>;
        let mut builder = ctx
            .builder
            .with_runtime_env(runtime_env)
            .with_distributed_user_codec(codec)
            .with_distributed_channel_resolver(peer_resolver)
            .with_distributed_worker_plan_rewrite_handler(rewrite_handler);

        builder
            .scalar_functions()
            .get_or_insert_with(Vec::new)
            .extend(all_pg_search_udfs());
        builder
            .aggregate_functions()
            .get_or_insert_with(Vec::new)
            .extend(all_pg_search_udafs());

        if let Some(cfg) = builder.config().as_mut() {
            crate::postgres::customscan::joinscan::scan_state::apply_paradedb_session_config(cfg);
            cfg.set_extension(stage_pool);
        }

        Ok(builder.build())
    }
}

/// Inputs passed to [`run_mpp_worker`] by `launch::run_launched_worker`.
pub(crate) struct MppWorkerInputs {
    pub parallel_state: *mut ParallelScanState,
    pub plan_sources_count: usize,
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

/// Run a parallel worker's Tonic gRPC `Worker` server over its DSM SPSC streams until the leader
/// signals query teardown or a Postgres cancel/die interrupt arrives.
pub(crate) fn run_mpp_worker(inputs: MppWorkerInputs, runtime: &PgTokioRuntime) {
    let MppWorkerInputs {
        parallel_state,
        plan_sources_count,
        session: worker_session,
    } = inputs;

    let worker_proc = worker_session.worker_proc;
    let this_proc = worker_proc.0;

    let mut index_segment_views: Vec<SegmentView> =
        vec![SegmentView::default(); plan_sources_count];
    for (i, slot) in index_segment_views.iter_mut().enumerate() {
        *slot = unsafe { (*parallel_state).segment_view_for_source_unlocked(i) };
    }

    let expr_context_guard = ExprContextGuard::new();
    let work_mem_bytes = unsafe { pg_sys::work_mem as usize * 1024 };
    let hash_mem_multiplier = unsafe { pg_sys::hash_mem_multiplier };

    let mut dispatcher = WakeupDispatcher::new(self_wakeup_slot());
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

    let n_workers = streams.incoming_server_streams.len();
    let peer_resolver = Arc::new(DsmGrpcChannelResolver::new(
        n_workers,
        dispatcher,
        streams.client_streams,
    ));

    let session_builder = MppWorkerSessionBuilder {
        parallel_state: SendPtr(parallel_state),
        index_segment_views,
        expr_context: SendPtr(expr_context_guard.as_ptr()),
        peer_resolver: Arc::clone(&peer_resolver),
        plan_sources_count,
        this_proc,
        work_mem_bytes,
        hash_mem_multiplier,
    };

    let worker = Worker::from_session_builder(session_builder);
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
    let serve_res = runtime.block_on(async move {
        mpp_grpc_server_builder()
            .add_service(worker.into_worker_server())
            .serve_with_incoming_shutdown(incoming, shutdown_watcher.wait())
            .await
    });
    peer_resolver.close_all();
    drop(held);
    check_for_interrupts();

    if let Err(e) = serve_res {
        pgrx::error!("mpp worker (proc={this_proc}): gRPC server error: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::customscan::mpp::dsm_stream::{
        DEFAULT_SPSC_RING_CAPACITY, build_leader_streams, dsm_region_bytes, init_dsm_mesh,
    };
    use crate::postgres::customscan::mpp::wakeup::WakeupSlot;
    use arrow_array::{Int32Array, RecordBatch};
    use arrow_schema::{DataType, Field, Schema};
    use datafusion::datasource::MemTable;
    use datafusion::physical_plan::collect;

    #[repr(C, align(64))]
    #[derive(Clone)]
    struct AlignedBlock([u8; 64]);

    #[tokio::test]
    async fn worker_executes_distributed_plan_over_dsm_async_stream() {
        let n_procs = 3; // 1 leader + 2 workers
        let bytes = dsm_region_bytes(n_procs, DEFAULT_SPSC_RING_CAPACITY).unwrap();
        let mut backing = vec![AlignedBlock([0u8; 64]); bytes.div_ceil(64)];
        let base = backing.as_mut_ptr() as *mut u8;

        unsafe {
            init_dsm_mesh(base, n_procs, DEFAULT_SPSC_RING_CAPACITY).unwrap();
        }

        let handle = tokio::runtime::Handle::current();

        // Spawn 2 Tonic Worker servers over the DSM streams.
        let mut worker_tasks = Vec::new();
        for w in 1..=2 {
            let mut w_disp = WakeupDispatcher::new(WakeupSlot(w));
            w_disp.start(&handle).unwrap();
            let w_streams = unsafe { build_worker_streams(base, ServerProc(w), &w_disp).unwrap() };
            let peer_resolver = Arc::new(DsmGrpcChannelResolver::new(
                2,
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

        // Leader setup.
        let mut leader_disp = WakeupDispatcher::new(WakeupSlot(0));
        leader_disp.start(&handle).unwrap();
        let (leader_streams, shutdown_handle) =
            unsafe { build_leader_streams(base, &leader_disp).unwrap() };
        let leader_resolver = Arc::new(DsmGrpcChannelResolver::new(2, leader_disp, leader_streams));

        let dyn_resolver: Arc<dyn ChannelResolver + Send + Sync> =
            Arc::clone(&leader_resolver) as Arc<dyn ChannelResolver + Send + Sync>;
        let seed = SessionContext::new();
        let cfg = seed.copied_config().with_target_partitions(2);
        let state = SessionStateBuilder::new_from_existing(seed.state())
            .with_config(cfg)
            .with_distributed_option_extension(DistributedConfig::default())
            .with_distributed_worker_resolver(MppWorkerResolver::new(2))
            .with_distributed_route_task_handler(RoundRobinRouteTaskHandler)
            .with_distributed_channel_resolver(dyn_resolver)
            .with_distributed_desired_task_count_handler(2usize)
            .with_distributed_compression(None)
            .expect("with_distributed_compression")
            .with_distributed_planner()
            .build();
        let ctx = SessionContext::new_with_state(state);
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

        let df = ctx
            .sql("SELECT k, SUM(v) AS s FROM t GROUP BY k ORDER BY k")
            .await
            .unwrap();
        let plan = df.create_physical_plan().await.unwrap();
        let batches = collect(plan, ctx.task_ctx()).await.unwrap();

        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 3);

        // Shut down workers and verify clean exit.
        shutdown_handle.signal_shutdown();
        leader_resolver.close_all();
        for t in worker_tasks {
            t.await.unwrap();
        }
    }
}

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

//! # Coordinated Parallel Execution (Local Distributed Engine)
//!
//! This module implements the infrastructure for process-parallel execution of JoinScan.
//! It follows a "Local Distributed Engine" model where the PostgreSQL leader backend
//! acts as a scheduler, and multiple background workers act as executors.
//!
//! ## Important Note on Parallelism
//!
//! This implementation **explicitly does not use** the standard PostgreSQL `CustomScan`
//! parallelism strategy (which relies on `EstimateDSMCustomScan`, `InitializeDSMCustomScan`,
//! etc.). Instead, it uses the `parallel_worker` framework to launch a deterministic number
//! of background workers and manually coordinates them via a unified DataFusion plan.

use crate::launch_parallel_process;
use crate::parallel_worker::builder::ParallelProcessMessageQueue;
use crate::parallel_worker::mqueue::MessageQueueSender;
use crate::parallel_worker::{
    ParallelProcess, ParallelState, ParallelStateManager, ParallelStateType, ParallelWorker,
    ParallelWorkerNumber, WorkerStyle,
};
use crate::postgres::customscan::joinscan::transport::TransportMesh;
use crate::postgres::customscan::joinscan::transport::{
    MultiplexedDsmReader, MultiplexedDsmWriter, RingBufferHeader, SignalBridge,
};
use crate::postgres::locks::Spinlock;
use crate::scan::PgSearchExtensionCodec;
use parking_lot::Mutex;
use std::sync::Arc;

use super::scan_state::create_session_context;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct JoinSharedState {
    pub mutex: Spinlock,
    pub nlaunched: usize,
    pub sockets_ready: usize,
}

impl ParallelStateType for JoinSharedState {}

impl JoinSharedState {
    pub fn set_launched_workers(&mut self, nlaunched: usize) {
        let _lock = self.mutex.acquire();
        self.nlaunched = nlaunched;
    }

    pub fn launched_workers(&mut self) -> usize {
        let _lock = self.mutex.acquire();
        self.nlaunched
    }

    pub fn inc_sockets_ready(&mut self) {
        let _lock = self.mutex.acquire();
        self.sockets_ready += 1;
    }

    pub fn sockets_ready(&mut self) -> usize {
        let _lock = self.mutex.acquire();
        self.sockets_ready
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct JoinConfig {
    pub max_memory: usize,
    pub nworkers: usize,
    pub total_participants: usize,
    pub leader_participation: bool,
    pub session_id: uuid::Bytes,
    pub region_size: usize,
}

impl ParallelStateType for JoinConfig {}

pub struct ParallelJoin {
    pub state: JoinSharedState,
    pub config: JoinConfig,
    /// One serialized logical plan slice per worker.
    pub plan_slices: Vec<Vec<u8>>,
    /// A single flat buffer containing all ring buffer regions.
    /// Layout: [Region 0][Region 1]...[Region P*P-1]
    pub ring_buffer: Vec<u8>,
}

impl ParallelJoin {
    pub fn new(config: JoinConfig, plan_slices: Vec<Vec<u8>>, ring_buffer: Vec<u8>) -> Self {
        Self {
            state: JoinSharedState {
                mutex: Spinlock::default(),
                nlaunched: 0,
                sockets_ready: 0,
            },
            config,
            plan_slices,
            ring_buffer,
        }
    }
}

impl ParallelProcess for ParallelJoin {
    fn state_values(&self) -> Vec<&dyn ParallelState> {
        let mut values: Vec<&dyn ParallelState> = vec![&self.state, &self.config];
        for slice in &self.plan_slices {
            values.push(slice);
        }
        // Push the single flat buffer
        values.push(&self.ring_buffer);
        values
    }
}

pub struct JoinWorker<'a> {
    pub state: &'a mut JoinSharedState,
    pub config: JoinConfig,
    pub plan_slice: Vec<u8>,
    pub ring_buffer_ptr: *mut u8,
}

impl ParallelWorker for JoinWorker<'_> {
    fn new_parallel_worker(
        state_manager: ParallelStateManager,
        worker_number: ParallelWorkerNumber,
    ) -> Self {
        let state = state_manager
            .object::<JoinSharedState>(0)
            .expect("wrong type for state")
            .expect("missing state value");
        let config = state_manager
            .object::<JoinConfig>(1)
            .expect("wrong type for config")
            .expect("missing config value");

        // Plan slices start at index 2.
        let plan_slice = state_manager
            .slice::<u8>(2 + worker_number.0 as usize)
            .expect("wrong type for plan_slice")
            .expect("missing plan_slice value");

        // The ring buffer is at index 2 + nworkers
        let ring_buffer_slice = state_manager
            .slice::<u8>(2 + config.nworkers)
            .expect("missing ring_buffer value")
            .expect("missing ring_buffer value");

        Self {
            state,
            config: *config,
            plan_slice: plan_slice.to_vec(),
            ring_buffer_ptr: ring_buffer_slice.as_ptr() as *mut u8,
        }
    }

    fn run(
        self,
        _mq_sender: &MessageQueueSender,
        worker_number: ParallelWorkerNumber,
    ) -> anyhow::Result<()> {
        // Wait for all workers to launch to ensure deterministic behavior.
        while self.state.launched_workers() == 0 {
            pgrx::check_for_interrupts!();
            std::thread::yield_now();
        }
        let total_participants = self.state.launched_workers();

        let participant_index =
            worker_number.to_participant_index(self.config.leader_participation);

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let session_id = uuid::Uuid::from_bytes(self.config.session_id);
        let bridge = runtime
            .block_on(SignalBridge::new(participant_index, session_id))
            .expect("Failed to initialize SignalBridge");
        let bridge = Arc::new(bridge);

        // Signal readiness
        self.state.inc_sockets_ready();

        // Wait for all participants to create their sockets
        while self.state.sockets_ready() < total_participants {
            pgrx::check_for_interrupts!();
            std::thread::yield_now();
        }

        let transport = unsafe {
            TransportMesh::init(
                self.ring_buffer_ptr,
                self.config.region_size,
                participant_index,
                total_participants,
                bridge.clone(),
            )
        };

        // Register the DSM mesh for this worker process.
        let mesh = crate::postgres::customscan::joinscan::exchange::DsmMesh {
            transport,
            registry: Mutex::new(
                crate::postgres::customscan::joinscan::exchange::StreamRegistry::default(),
            ),
        };
        crate::postgres::customscan::joinscan::exchange::register_dsm_mesh(mesh);

        let ctx = create_session_context(
            participant_index,
            total_participants,
            self.config.max_memory,
        );

        let codec = PgSearchExtensionCodec::default();
        let _ = physical_plan_from_bytes_with_extension_codec(
            &self.plan_slice,
            &ctx.task_ctx(),
            &codec,
        )
        .expect("Failed to parse physical plan");

        let task_ctx = ctx.task_ctx();

        // The Worker Loop:
        // 1. Deserializing the plan populated the local StreamRegistry via the physical codec.
        // 2. Start the "Listener" (Control Service) to accept incoming RPC calls (StartStream).
        // 3. Park the main thread and wait for session termination.
        runtime.block_on(async {
            let local = tokio::task::LocalSet::new();

            // Start the control service to listen for stream requests
            crate::postgres::customscan::joinscan::exchange::spawn_control_service(
                &local,
                task_ctx.clone(),
            );

            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("Failed to create SIGTERM listener");

            local
                .run_until(async move {
                    tokio::select! {
                        _ = futures::future::pending::<()>() => {
                            // Should not be reachable
                        }
                        _ = sigterm.recv() => {
                            // Normal exit on SIGTERM
                            pgrx::warning!("JoinWorker: SIGTERM received, shutting down");
                        }
                    }
                })
                .await
        });

        Ok(())
    }
}

impl JoinWorker<'_> {}

use datafusion::physical_plan::ExecutionPlan;
use datafusion_proto::bytes::{
    physical_plan_from_bytes_with_extension_codec, physical_plan_to_bytes_with_extension_codec,
};

/// The result of launching parallel join workers.
pub type LaunchedJoinWorkers = (
    ParallelProcessMessageQueue,
    Option<Arc<dyn ExecutionPlan>>,
    Vec<Arc<Mutex<MultiplexedDsmWriter>>>,
    Vec<Arc<Mutex<MultiplexedDsmReader>>>,
    uuid::Uuid,
    Arc<SignalBridge>,
);

/// Launches parallel workers for a JoinScan.
pub fn launch_join_workers(
    runtime: &tokio::runtime::Runtime,
    leader_plan: Arc<dyn ExecutionPlan>,
    nworkers: usize,
    max_memory: usize,
    leader_participation: bool,
) -> Option<LaunchedJoinWorkers> {
    if nworkers == 0 {
        return None;
    }

    let total_participants = if leader_participation {
        nworkers + 1
    } else {
        nworkers
    };

    let codec = PgSearchExtensionCodec::default();
    let leader_plan_bytes =
        physical_plan_to_bytes_with_extension_codec(leader_plan.clone(), &codec)
            .expect("Failed to serialize physical plan to bytes");

    let mut plan_slices = Vec::with_capacity(nworkers);
    for _ in 0..nworkers {
        plan_slices.push(leader_plan_bytes.to_vec());
    }

    let session_id = uuid::Uuid::new_v4();

    // Allocate 128MB ring buffers per worker.
    // TODO: This is temporary! Should implement support for reconstructing a larger buffer without
    // needing this much dedicated space.
    let ring_buffer_size = 128 * 1024 * 1024;
    // We increase the control buffer size to 64KB (from 4KB) to prevent dropping
    // CancelStream messages during high-concurrency teardown, which could lead to deadlocks.
    let control_size = 65536;
    // Data Header + Data + Control Header + Control Data + padding
    let region_size = size_of::<RingBufferHeader>()
        + ring_buffer_size
        + size_of::<RingBufferHeader>()
        + control_size
        + 64;

    let config = JoinConfig {
        max_memory,
        nworkers,
        total_participants,
        leader_participation,
        session_id: *session_id.as_bytes(),
        region_size,
    };

    let total_size = region_size * total_participants * total_participants;
    let mut ring_buffer = vec![0u8; total_size];

    let base_ptr = ring_buffer.as_mut_ptr();

    // Initialize all headers in the single flat buffer
    for i in 0..(total_participants * total_participants) {
        let offset = i * region_size;

        let (header, _, _) =
            unsafe { RingBufferHeader::from_raw_parts(base_ptr, offset, region_size) };

        unsafe {
            RingBufferHeader::init(header, size_of::<RingBufferHeader>() + ring_buffer_size);

            // Initialize Control Header
            let header_ptr = header as *mut u8;
            let control_ptr = header_ptr.add((*header).control_offset);
            let control_header = control_ptr as *mut RingBufferHeader;
            RingBufferHeader::init(control_header, 0);
        }
    }

    // Initialize leader's bridge
    let bridge = runtime
        .block_on(SignalBridge::new(
            0, // Leader is index 0
            session_id,
        ))
        .expect("Failed to initialize SignalBridge");
    let bridge = Arc::new(bridge);

    let process = ParallelJoin::new(config, plan_slices, ring_buffer);

    if let Some(mut launched) = launch_parallel_process!(
        ParallelJoin<JoinWorker>,
        process,
        WorkerStyle::Query,
        nworkers,
        16384
    ) {
        let nlaunched = launched.launched_workers() + if leader_participation { 1 } else { 0 };

        // Signal readiness and wait for workers
        {
            let state_manager = launched.state_manager_mut();
            let shared_state = state_manager.object::<JoinSharedState>(0).unwrap().unwrap();

            shared_state.set_launched_workers(nlaunched);
            shared_state.inc_sockets_ready();

            while shared_state.sockets_ready() < total_participants {
                pgrx::check_for_interrupts!();
                std::thread::yield_now();
            }
        }

        let leader_participant_index = 0;

        // Retrieve the ring buffer slice from DSM
        let ring_buffer_slice = launched
            .state_manager()
            .slice::<u8>(2 + nworkers)
            .expect("wrong type for ring_buffer_slice")
            .expect("missing ring_buffer_slice value");

        let base_ptr = ring_buffer_slice.as_ptr() as *mut u8;

        let transport = unsafe {
            TransportMesh::init(
                base_ptr,
                region_size,
                leader_participant_index,
                total_participants,
                bridge.clone(),
            )
        };

        let mux_writers = transport.mux_writers.clone();
        let mux_readers = transport.mux_readers.clone();

        Some((
            launched.into_iter(),
            Some(leader_plan),
            mux_writers,
            mux_readers,
            session_id,
            bridge,
        ))
    } else {
        None
    }
}
#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use crate::launch_parallel_process;
    use crate::parallel_worker::mqueue::MessageQueueSender;
    use crate::parallel_worker::{
        ParallelProcess, ParallelState, ParallelStateManager, ParallelStateType, ParallelWorker,
        ParallelWorkerNumber, WorkerStyle,
    };
    use crate::postgres::customscan::joinscan::exchange::{
        register_dsm_mesh, DsmExchangeConfig, DsmMesh, DsmReaderExec, DsmWriterExec, ExchangeMode,
    };
    use crate::postgres::customscan::joinscan::transport::{
        LogicalStreamId, MultiplexedDsmReader, MultiplexedDsmWriter, RingBufferHeader,
        SignalBridge, TransportMesh,
    };
    use crate::postgres::locks::Spinlock;
    use crate::scan::table_provider::MppParticipantConfig;
    use arrow_array::{Int32Array, RecordBatch};
    use arrow_schema::{DataType, Field, Schema, SchemaRef};
    use datafusion::common::Result;
    use datafusion::execution::context::{SessionConfig, SessionContext, TaskContext};
    use datafusion::execution::{RecordBatchStream, SendableRecordBatchStream};
    use datafusion::physical_expr::{EquivalenceProperties, Partitioning};
    use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
    use datafusion::physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties};
    use futures::{Stream, StreamExt};
    use parking_lot::Mutex;
    use std::any::Any;
    use std::fmt::Formatter;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll};

    #[derive(Debug)]
    struct MockExec {
        batch: RecordBatch,
        schema: SchemaRef,
        properties: PlanProperties,
    }

    impl MockExec {
        fn new(batch: RecordBatch, schema: SchemaRef) -> Self {
            let properties = PlanProperties::new(
                EquivalenceProperties::new(schema.clone()),
                Partitioning::UnknownPartitioning(1),
                EmissionType::Incremental,
                Boundedness::Bounded,
            );
            Self {
                batch,
                schema,
                properties,
            }
        }
    }

    impl DisplayAs for MockExec {
        fn fmt_as(&self, _t: DisplayFormatType, f: &mut Formatter) -> std::fmt::Result {
            write!(f, "MockExec")
        }
    }

    impl ExecutionPlan for MockExec {
        fn name(&self) -> &str {
            "MockExec"
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn properties(&self) -> &PlanProperties {
            &self.properties
        }
        fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
            vec![]
        }
        fn with_new_children(
            self: Arc<Self>,
            _c: Vec<Arc<dyn ExecutionPlan>>,
        ) -> Result<Arc<dyn ExecutionPlan>> {
            Ok(self)
        }
        fn execute(&self, _p: usize, _c: Arc<TaskContext>) -> Result<SendableRecordBatchStream> {
            Ok(Box::pin(MockStream {
                schema: self.schema.clone(),
                batch: Some(self.batch.clone()),
            }))
        }
    }

    struct MockStream {
        schema: SchemaRef,
        batch: Option<RecordBatch>,
    }

    impl RecordBatchStream for MockStream {
        fn schema(&self) -> SchemaRef {
            self.schema.clone()
        }
    }

    impl Stream for MockStream {
        type Item = Result<RecordBatch>;
        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            if let Some(b) = self.batch.take() {
                Poll::Ready(Some(Ok(b)))
            } else {
                Poll::Ready(None)
            }
        }
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct DsmTestSharedState {
        pub mutex: Spinlock,
        pub nlaunched: usize,
    }

    impl ParallelStateType for DsmTestSharedState {}

    impl DsmTestSharedState {
        pub fn set_launched_workers(&mut self, nlaunched: usize) {
            let _lock = self.mutex.acquire();
            self.nlaunched = nlaunched;
        }

        pub fn launched_workers(&mut self) -> usize {
            let _lock = self.mutex.acquire();
            self.nlaunched
        }
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct DsmTestConfig {
        pub total_participants: usize,
        pub session_id: uuid::Bytes,
    }

    impl ParallelStateType for DsmTestConfig {}

    pub struct DsmTestProcess {
        pub state: DsmTestSharedState,
        pub config: DsmTestConfig,
        pub ring_buffer_regions: Vec<Vec<u8>>,
    }

    impl DsmTestProcess {
        pub fn new(total_participants: usize) -> Self {
            let session_id = uuid::Uuid::new_v4();
            // Allocate ring buffers
            let ring_buffer_size = 1024 * 1024;
            // Data Header + Data + Control Header + Control Data + padding
            let control_size = 65536;
            let total_size = size_of::<RingBufferHeader>()
                + ring_buffer_size
                + size_of::<RingBufferHeader>()
                + control_size
                + 64;

            let mut ring_buffer_regions =
                Vec::with_capacity(total_participants * total_participants);
            for _ in 0..(total_participants * total_participants) {
                let mut region = vec![0u8; total_size];

                // Initialize Data Header

                let base_ptr = region.as_mut_ptr();

                let (header, _, _) =
                    unsafe { RingBufferHeader::from_raw_parts(base_ptr, 0, total_size) };
                unsafe {
                    RingBufferHeader::init(
                        header,
                        size_of::<RingBufferHeader>() + ring_buffer_size,
                    );

                    // Initialize Control Header
                    let header_ptr = header as *mut u8;
                    let control_ptr = header_ptr.add((*header).control_offset);
                    let control_header = control_ptr as *mut RingBufferHeader;
                    RingBufferHeader::init(control_header, 0);
                }

                ring_buffer_regions.push(region);
            }

            Self {
                state: DsmTestSharedState {
                    mutex: Spinlock::default(),
                    nlaunched: 0,
                },
                config: DsmTestConfig {
                    total_participants,
                    session_id: *session_id.as_bytes(),
                },
                ring_buffer_regions,
            }
        }
    }

    impl ParallelProcess for DsmTestProcess {
        fn state_values(&self) -> Vec<&dyn ParallelState> {
            let mut values: Vec<&dyn ParallelState> = vec![&self.state, &self.config];
            for region in &self.ring_buffer_regions {
                values.push(region);
            }
            values
        }
    }

    #[derive(Clone, Copy)]
    pub struct RegionInfo {
        header: *mut RingBufferHeader,
        data: *mut u8,
        data_len: usize,
    }
    unsafe impl Send for RegionInfo {}

    pub struct DsmTestWorker<'a> {
        pub state: &'a mut DsmTestSharedState,
        pub config: DsmTestConfig,
        pub writer_regions: Vec<RegionInfo>,
        pub reader_regions: Vec<RegionInfo>,
    }

    impl ParallelWorker for DsmTestWorker<'_> {
        fn new_parallel_worker(
            state_manager: ParallelStateManager,
            worker_number: ParallelWorkerNumber,
        ) -> Self {
            let state = state_manager
                .object::<DsmTestSharedState>(0)
                .unwrap()
                .unwrap();
            let config = state_manager.object::<DsmTestConfig>(1).unwrap().unwrap();

            let participant_index = worker_number.to_participant_index(true); // Leader is 0
            let total_participants = config.total_participants;

            let mut writer_regions = Vec::new();
            let mut reader_regions = Vec::new();
            let p = total_participants;

            for j in 0..p {
                // Region for writer from us (participant_index) to participant j.
                let writer_region_idx = 2 + (participant_index * p + j);
                let ring_buffer_slice = state_manager
                    .slice::<u8>(writer_region_idx)
                    .unwrap()
                    .unwrap();
                let region_size = ring_buffer_slice.len();
                let base_ptr = ring_buffer_slice.as_ptr() as *mut u8;
                let (header, data, data_len) =
                    unsafe { RingBufferHeader::from_raw_parts(base_ptr, 0, region_size) };

                writer_regions.push(RegionInfo {
                    header,
                    data,
                    data_len,
                });

                // Region for reader from participant j to us (participant_index).
                let reader_region_idx = 2 + (j * p + participant_index);
                let ring_buffer_slice = state_manager
                    .slice::<u8>(reader_region_idx)
                    .unwrap()
                    .unwrap();
                let region_size = ring_buffer_slice.len();
                let base_ptr = ring_buffer_slice.as_ptr() as *mut u8;
                let (header, data, data_len) =
                    unsafe { RingBufferHeader::from_raw_parts(base_ptr, 0, region_size) };

                reader_regions.push(RegionInfo {
                    header,
                    data,
                    data_len,
                });
            }

            Self {
                state,
                config: *config,
                writer_regions,
                reader_regions,
            }
        }

        fn run(
            self,
            _mq_sender: &MessageQueueSender,
            worker_number: ParallelWorkerNumber,
        ) -> anyhow::Result<()> {
            let participant_index = worker_number.to_participant_index(true);
            let total_participants = self.config.total_participants;

            // Signal readiness
            let current = self.state.launched_workers();
            self.state.set_launched_workers(current + 1);
            while self.state.launched_workers() < total_participants {
                pgrx::check_for_interrupts!();
                std::thread::yield_now();
            }

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let session_id = uuid::Uuid::from_bytes(self.config.session_id);
            let bridge = runtime
                .block_on(SignalBridge::new(participant_index, session_id))
                .unwrap();
            let bridge = Arc::new(bridge);

            let mut mux_writers = Vec::with_capacity(total_participants);
            for (j, region) in self.writer_regions.iter().enumerate() {
                mux_writers.push(Arc::new(Mutex::new(MultiplexedDsmWriter::new(
                    region.header,
                    region.data,
                    region.data_len,
                    bridge.clone(),
                    j,
                ))));
            }

            let mut mux_readers = Vec::with_capacity(total_participants);
            for (j, region) in self.reader_regions.iter().enumerate() {
                mux_readers.push(Arc::new(Mutex::new(MultiplexedDsmReader::new(
                    region.header,
                    region.data,
                    region.data_len,
                    bridge.clone(),
                    j,
                ))));
            }

            let transport = TransportMesh {
                mux_writers,
                mux_readers,
                bridge,
            };
            let mesh = DsmMesh {
                transport,
                registry: Mutex::new(
                    crate::postgres::customscan::joinscan::exchange::StreamRegistry::default(),
                ),
            };
            register_dsm_mesh(mesh);

            // --- Build Plan ---
            // Input: MockExec with some data.
            let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)]));
            let batch = RecordBatch::try_new(
                schema.clone(),
                vec![Arc::new(Int32Array::from(vec![
                    participant_index as i32 * 10,
                ]))],
            )
            .unwrap();

            let input = Arc::new(MockExec::new(batch, schema.clone()));

            // Exchange: Gather to node 0.
            let partitioning = Partitioning::UnknownPartitioning(total_participants);
            let config = DsmExchangeConfig {
                stream_id: LogicalStreamId(0),
                total_participants,
                mode: ExchangeMode::Gather,
            };
            let writer =
                DsmWriterExec::try_new(input, partitioning.clone(), config.clone()).unwrap();
            let reader =
                DsmReaderExec::try_new(Arc::new(writer), config.clone(), partitioning).unwrap();
            // Wrap in CoalescePartitionsExec so that execute(0) pulls from ALL workers
            let plan: Arc<dyn ExecutionPlan> = Arc::new(
                datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec::new(
                    Arc::new(reader),
                ),
            );

            let mut session_config = SessionConfig::new().with_target_partitions(1);
            session_config
                .options_mut()
                .extensions
                .insert(MppParticipantConfig {
                    index: participant_index,
                    total_participants,
                });
            let session_state = SessionContext::new_with_config(session_config).state();
            let task_ctx = Arc::new(TaskContext::from(&session_state));

            runtime.block_on(async {
                let local = tokio::task::LocalSet::new();

                // Register the writer
                let mut sources = Vec::new();
                crate::postgres::customscan::joinscan::exchange::collect_dsm_writers(
                    plan.clone(),
                    &mut sources,
                );
                for source in sources {
                    crate::postgres::customscan::joinscan::exchange::register_stream_source(
                        source,
                        participant_index,
                    );
                }

                // Start Control Service
                crate::postgres::customscan::joinscan::exchange::spawn_control_service(
                    &local,
                    task_ctx.clone(),
                );

                local
                    .run_until(async {
                        let mut stream = plan.execute(0, task_ctx).unwrap();
                        if let Some(batch) = stream.next().await {
                            let batch = batch.unwrap();
                            // Worker (participant 1) is not node 0, so it should not receive anything in Gather mode
                            panic!("Worker received data in Gather mode! {:?}", batch);
                        }
                    })
                    .await;
            });

            Ok(())
        }
    }

    #[pgrx::pg_test]
    fn test_dsm_gather_execution() {
        let total_participants = 2; // Leader + 1 Worker
        let process = DsmTestProcess::new(total_participants);
        let session_id_bytes = process.config.session_id;

        let mut launched = launch_parallel_process!(
            DsmTestProcess<DsmTestWorker>,
            process,
            WorkerStyle::Query,
            1, // 1 worker
            16384
        )
        .expect("Failed to launch parallel process");

        let state = launched
            .state_manager_mut()
            .object::<DsmTestSharedState>(0)
            .unwrap()
            .unwrap();
        state.set_launched_workers(1); // Leader counts as 1

        // Wait for worker to launch
        while state.launched_workers() < total_participants {
            pgrx::check_for_interrupts!();
            std::thread::yield_now();
        }

        // Leader Setup
        let session_id = uuid::Uuid::from_bytes(session_id_bytes);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let bridge = runtime
            .block_on(SignalBridge::new(
                0, // Leader index
                session_id,
            ))
            .unwrap();
        let bridge = Arc::new(bridge);

        let mut mux_writers = Vec::new();
        let mut mux_readers = Vec::new();
        let p = total_participants;
        let participant_index = 0;

        for j in 0..p {
            let writer_region_idx = 2 + (participant_index * p + j);
            let ring_buffer_slice = launched
                .state_manager()
                .slice::<u8>(writer_region_idx)
                .unwrap()
                .unwrap();
            let region_size = ring_buffer_slice.len();
            let base_ptr = ring_buffer_slice.as_ptr() as *mut u8;
            let (header, data, data_len) =
                unsafe { RingBufferHeader::from_raw_parts(base_ptr, 0, region_size) };

            mux_writers.push(Arc::new(Mutex::new(MultiplexedDsmWriter::new(
                header,
                data,
                data_len,
                bridge.clone(),
                j,
            ))));

            let reader_region_idx = 2 + (j * p + participant_index);
            let ring_buffer_slice = launched
                .state_manager()
                .slice::<u8>(reader_region_idx)
                .unwrap()
                .unwrap();
            let region_size = ring_buffer_slice.len();
            let base_ptr = ring_buffer_slice.as_ptr() as *mut u8;
            let (header, data, data_len) =
                unsafe { RingBufferHeader::from_raw_parts(base_ptr, 0, region_size) };

            mux_readers.push(Arc::new(Mutex::new(MultiplexedDsmReader::new(
                header,
                data,
                data_len,
                bridge.clone(),
                j,
            ))));
        }

        let transport = TransportMesh {
            mux_writers,
            mux_readers,
            bridge,
        };
        let mesh = DsmMesh {
            transport,
            registry: Mutex::new(
                crate::postgres::customscan::joinscan::exchange::StreamRegistry::default(),
            ),
        };
        register_dsm_mesh(mesh);

        // --- Execute Leader Plan ---
        let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)]));
        let batch = RecordBatch::try_new(schema.clone(), vec![Arc::new(Int32Array::from(vec![0]))])
            .unwrap();

        let input = Arc::new(MockExec::new(batch, schema.clone()));

        let partitioning = Partitioning::UnknownPartitioning(total_participants);
        let config = DsmExchangeConfig {
            stream_id: LogicalStreamId(0),
            total_participants,
            mode: ExchangeMode::Gather,
        };
        let writer = DsmWriterExec::try_new(input, partitioning.clone(), config.clone()).unwrap();
        let reader = DsmReaderExec::try_new(Arc::new(writer), config, partitioning).unwrap();
        // Wrap in CoalescePartitionsExec so that execute(0) pulls from ALL workers
        let plan: Arc<dyn ExecutionPlan> = Arc::new(
            datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec::new(Arc::new(
                reader,
            )),
        );

        let mut session_config = SessionConfig::new().with_target_partitions(1);
        session_config
            .options_mut()
            .extensions
            .insert(MppParticipantConfig {
                index: participant_index,
                total_participants,
            });
        let session_state = SessionContext::new_with_config(session_config).state();
        let task_ctx = Arc::new(TaskContext::from(&session_state));

        runtime.block_on(async {
            let local = tokio::task::LocalSet::new();

            // Register Leader sources
            let mut sources = Vec::new();
            crate::postgres::customscan::joinscan::exchange::collect_dsm_writers(
                plan.clone(),
                &mut sources,
            );
            for source in sources {
                crate::postgres::customscan::joinscan::exchange::register_stream_source(
                    source,
                    participant_index,
                );
            }

            // Start Leader Control Service
            crate::postgres::customscan::joinscan::exchange::spawn_control_service(
                &local,
                task_ctx.clone(),
            );

            local
                .run_until(async {
                    let mut stream = plan.execute(0, task_ctx).unwrap();
                    let mut results = Vec::new();
                    while let Some(batch) = stream.next().await {
                        results.push(batch.unwrap());
                    }

                    // We expect 2 batches: one from leader (0), one from worker (10).
                    // They might come in any order.
                    assert_eq!(results.len(), 2);
                    let values: Vec<i32> = results
                        .iter()
                        .map(|b| {
                            b.column(0)
                                .as_any()
                                .downcast_ref::<Int32Array>()
                                .unwrap()
                                .value(0)
                        })
                        .collect();
                    assert!(values.contains(&0));
                    assert!(values.contains(&10));
                })
                .await;
        });
    }
}

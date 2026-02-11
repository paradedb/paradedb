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
    WorkerStyle,
};
use crate::postgres::customscan::joinscan::dsm_transfer::{
    dsm_shared_memory_reader, MultiplexedDsmReader, MultiplexedDsmWriter, RingBufferHeader,
};
use crate::postgres::locks::Spinlock;
use crate::scan::PgSearchExtensionCodec;
use arrow_schema::SchemaRef;
use datafusion::execution::SendableRecordBatchStream;
use datafusion_proto::bytes::logical_plan_from_bytes_with_extension_codec;
use futures::StreamExt;
use parking_lot::Mutex;
use pgrx::pg_sys;
use std::sync::Arc;

use super::scan_state::{build_joinscan_physical_plan, create_session_context};

use datafusion::logical_expr::LogicalPlan;
use datafusion_proto::bytes::logical_plan_to_bytes_with_extension_codec;

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
}

impl ParallelStateType for JoinConfig {}

pub struct ParallelJoin {
    pub state: JoinSharedState,
    pub config: JoinConfig,
    /// One serialized logical plan slice per worker.
    pub plan_slices: Vec<Vec<u8>>,
    /// $P \times P$ ring buffer regions, where $P$ is total participants.
    /// Ordered as [producer_0_to_consumer_0, producer_0_to_consumer_1, ..., producer_P_to_consumer_P]
    pub ring_buffer_regions: Vec<Vec<u8>>,
}

impl ParallelJoin {
    pub fn new(
        config: JoinConfig,
        plan_slices: Vec<Vec<u8>>,
        ring_buffer_regions: Vec<Vec<u8>>,
    ) -> Self {
        Self {
            state: JoinSharedState {
                mutex: Spinlock::default(),
                nlaunched: 0,
                sockets_ready: 0,
            },
            config,
            plan_slices,
            ring_buffer_regions,
        }
    }
}

impl ParallelProcess for ParallelJoin {
    fn state_values(&self) -> Vec<&dyn ParallelState> {
        let mut values: Vec<&dyn ParallelState> = vec![&self.state, &self.config];
        for slice in &self.plan_slices {
            values.push(slice);
        }
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

pub struct JoinWorker<'a> {
    pub state: &'a mut JoinSharedState,
    pub config: JoinConfig,
    pub plan_slice: Vec<u8>,
    pub writer_regions: Vec<RegionInfo>,
    pub reader_regions: Vec<RegionInfo>,
}

impl ParallelWorker for JoinWorker<'_> {
    fn new_parallel_worker(state_manager: ParallelStateManager) -> Self {
        let state = state_manager
            .object::<JoinSharedState>(0)
            .expect("wrong type for state")
            .expect("missing state value");
        let config = state_manager
            .object::<JoinConfig>(1)
            .expect("wrong type for config")
            .expect("missing config value");

        let worker_number = unsafe { pg_sys::ParallelWorkerNumber } as usize;
        let participant_index = if config.leader_participation {
            worker_number + 1
        } else {
            worker_number
        };
        let total_participants = config.total_participants;

        // Plan slices start at index 2.
        let plan_slice = state_manager
            .slice::<u8>(2 + worker_number)
            .expect("wrong type for plan_slice")
            .expect("missing plan_slice value");

        let mut writer_regions = Vec::new();
        let mut reader_regions = Vec::new();

        let nworkers = config.nworkers;
        let p = total_participants;

        for j in 0..p {
            // Region for writer from us (participant_index) to participant j.
            let writer_region_idx = 2 + nworkers + (participant_index * p + j);
            let ring_buffer_slice = state_manager
                .slice::<u8>(writer_region_idx)
                .expect("missing ring_buffer_slice for writer")
                .expect("missing ring_buffer_slice value");

            let header = ring_buffer_slice.as_ptr() as *mut RingBufferHeader;
            let data = unsafe {
                ring_buffer_slice
                    .as_ptr()
                    .add(size_of::<RingBufferHeader>())
            } as *mut u8;
            let data_len = ring_buffer_slice.len() - size_of::<RingBufferHeader>();
            writer_regions.push(RegionInfo {
                header,
                data,
                data_len,
            });

            // Region for reader from participant j to us (participant_index).
            let reader_region_idx = 2 + nworkers + (j * p + participant_index);
            let ring_buffer_slice = state_manager
                .slice::<u8>(reader_region_idx)
                .expect("missing ring_buffer_slice for reader")
                .expect("missing ring_buffer_slice value");

            let header = ring_buffer_slice.as_ptr() as *mut RingBufferHeader;
            let data = unsafe {
                ring_buffer_slice
                    .as_ptr()
                    .add(size_of::<RingBufferHeader>())
            } as *mut u8;
            let data_len = ring_buffer_slice.len() - size_of::<RingBufferHeader>();
            reader_regions.push(RegionInfo {
                header,
                data,
                data_len,
            });
        }

        Self {
            state,
            config: *config,
            plan_slice: plan_slice.to_vec(),
            writer_regions,
            reader_regions,
        }
    }

    fn run(self, _mq_sender: &MessageQueueSender, worker_number: i32) -> anyhow::Result<()> {
        // Wait for all workers to launch to ensure deterministic behavior.
        while self.state.launched_workers() == 0 {
            pgrx::check_for_interrupts!();
            std::thread::yield_now();
        }
        let total_participants = self.state.launched_workers();

        let participant_index = if self.config.leader_participation {
            (worker_number + 1) as usize
        } else {
            worker_number as usize
        };

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let session_id = uuid::Uuid::from_bytes(self.config.session_id);
        let bridge = runtime
            .block_on(
                crate::postgres::customscan::joinscan::dsm_transfer::SignalBridge::new(
                    participant_index,
                    session_id,
                ),
            )
            .expect("Failed to initialize SignalBridge");
        let bridge = Arc::new(bridge);

        // Signal readiness
        self.state.inc_sockets_ready();

        // Wait for all participants to create their sockets
        while self.state.sockets_ready() < total_participants {
            pgrx::check_for_interrupts!();
            std::thread::yield_now();
        }

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

        // Register the DSM mesh for this worker process.
        let mesh = crate::postgres::customscan::joinscan::exchange::DsmMesh {
            total_participants,
            mux_writers: mux_writers.clone(),
            mux_readers: mux_readers.clone(),
            bridge,
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
        let logical_plan =
            logical_plan_from_bytes_with_extension_codec(&self.plan_slice, &ctx.task_ctx(), &codec)
                .expect("Failed to deserialize logical plan");

        let physical_plan = runtime
            .block_on(build_joinscan_physical_plan(&ctx, logical_plan))
            .expect("Failed to create execution plan");

        let task_ctx = ctx.task_ctx();

        // The Worker Loop:
        // 1. Register potential work ("Procedures") in the Registry.
        // 2. Start the "Listener" (Control Service) to accept RPC calls.
        // 3. Execute the main plan (which may just wait for completion if it's a Gather worker).
        // 4. Listen for SIGTERM to exit gracefully.
        runtime.block_on(async {
            let local = tokio::task::LocalSet::new();

            // Register all writers in the plan
            let mut sources = Vec::new();
            crate::postgres::customscan::joinscan::exchange::collect_dsm_writers(
                physical_plan.clone(),
                &mut sources,
            );
            for source in sources {
                crate::postgres::customscan::joinscan::exchange::register_stream_source(source);
            }

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
                        _ = async {
                            let mut stream = physical_plan
                                .execute(0, task_ctx)
                                .expect("Failed to execute DataFusion plan");

                            while let Some(batch) = stream.next().await {
                                batch.expect("DataFusion execution failed in parallel");
                                // The DsmShuffleWriterExec handled writing.
                            }

                            for mux_writer in mux_writers {
                                mux_writer
                                    .lock()
                                    .finish()
                                    .expect("Failed to finish multiplexed DSM transfer");
                            }
                        } => {
                            // Normal completion
                        }
                        _ = sigterm.recv() => {
                            // Normal exit on SIGTERM
                        }
                    }
                })
                .await
        });

        Ok(())
    }
}

/// The result of launching parallel join workers.
pub type LaunchedJoinWorkers = (
    ParallelProcessMessageQueue,
    Vec<SendableRecordBatchStream>,
    Option<LogicalPlan>,
    Vec<Arc<Mutex<MultiplexedDsmWriter>>>,
    Vec<Arc<Mutex<MultiplexedDsmReader>>>,
    uuid::Uuid,
    Arc<crate::postgres::customscan::joinscan::dsm_transfer::SignalBridge>,
);

/// Launches parallel workers for a JoinScan.
pub fn launch_join_workers(
    runtime: &tokio::runtime::Runtime,
    leader_plan: LogicalPlan,
    _ordering_rti: pg_sys::Index,
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
    let leader_plan_bytes = logical_plan_to_bytes_with_extension_codec(&leader_plan, &codec)
        .expect("Failed to serialize leader plan");

    let mut plan_slices = Vec::with_capacity(nworkers);
    for _ in 0..nworkers {
        plan_slices.push(leader_plan_bytes.to_vec());
    }

    let session_id = uuid::Uuid::new_v4();

    let config = JoinConfig {
        max_memory,
        nworkers,
        total_participants,
        leader_participation,
        session_id: *session_id.as_bytes(),
    };

    // Allocate 128MB ring buffers per worker.
    // TODO: This is temporary! Should implement support for reconstructing a larger buffer without
    // needing this much dedicated space.
    let ring_buffer_size = 128 * 1024 * 1024;
    // We increase the control buffer size to 64KB (from 4KB) to prevent dropping
    // CancelStream messages during high-concurrency teardown, which could lead to deadlocks.
    let control_size = 65536;
    // Data Header + Data + Control Header + Control Data + padding
    let total_size = size_of::<RingBufferHeader>()
        + ring_buffer_size
        + size_of::<RingBufferHeader>()
        + control_size
        + 64;

    let mut ring_buffer_regions = Vec::with_capacity(total_participants * total_participants);
    for _ in 0..(total_participants * total_participants) {
        let mut region = vec![0u8; total_size];

        // Initialize Data Header to point to Control Header
        let header = region.as_mut_ptr() as *mut RingBufferHeader;
        unsafe {
            RingBufferHeader::init(header, size_of::<RingBufferHeader>() + ring_buffer_size);

            // Initialize Control Header
            let control_ptr = region.as_mut_ptr().add((*header).control_offset);
            let control_header = control_ptr as *mut RingBufferHeader;
            RingBufferHeader::init(control_header, 0);
        }

        ring_buffer_regions.push(region);
    }

    // Initialize leader's bridge
    let bridge = runtime
        .block_on(
            crate::postgres::customscan::joinscan::dsm_transfer::SignalBridge::new(
                0, // Leader is index 0
                session_id,
            ),
        )
        .expect("Failed to initialize SignalBridge");
    let bridge = Arc::new(bridge);

    let process = ParallelJoin::new(config, plan_slices, ring_buffer_regions);

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

        let schema_ref: SchemaRef = Arc::new(leader_plan.schema().as_arrow().clone());

        let mut dsm_readers = Vec::with_capacity(total_participants);
        let mut leader_mux_writers = Vec::with_capacity(total_participants);
        let mut leader_mux_readers = Vec::with_capacity(total_participants);

        let leader_participant_index = 0;

        let p = total_participants;
        for j in 0..p {
            // Leader's writers to all participants j.
            let writer_region_idx = 2 + nworkers + (leader_participant_index * p + j);
            let ring_buffer_slice = launched
                .state_manager()
                .slice::<u8>(writer_region_idx)
                .expect("wrong type for ring_buffer_slice")
                .expect("missing ring_buffer_slice value");

            let header = ring_buffer_slice.as_ptr() as *mut RingBufferHeader;
            let data = unsafe {
                ring_buffer_slice
                    .as_ptr()
                    .add(size_of::<RingBufferHeader>())
            } as *mut u8;
            let data_len = ring_buffer_slice.len() - size_of::<RingBufferHeader>();

            let mux_writer = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
                header,
                data,
                data_len,
                bridge.clone(),
                j,
            )));
            leader_mux_writers.push(mux_writer);

            // Leader's readers from all participants j.
            let reader_region_idx = 2 + nworkers + (j * p + leader_participant_index);
            let ring_buffer_slice = launched
                .state_manager()
                .slice::<u8>(reader_region_idx)
                .expect("wrong type for ring_buffer_slice")
                .expect("missing ring_buffer_slice value");

            let header = ring_buffer_slice.as_ptr() as *mut RingBufferHeader;
            let data = unsafe {
                ring_buffer_slice
                    .as_ptr()
                    .add(size_of::<RingBufferHeader>())
            } as *mut u8;
            let data_len = ring_buffer_slice.len() - size_of::<RingBufferHeader>();

            let mux_reader = Arc::new(Mutex::new(MultiplexedDsmReader::new(
                header,
                data,
                data_len,
                bridge.clone(),
                j,
            )));
            leader_mux_readers.push(mux_reader.clone());

            if leader_participation && j == leader_participant_index {
                continue;
            }

            // For now, only stream 0.
            let reader = dsm_shared_memory_reader(mux_reader, 0, j, schema_ref.clone());
            dsm_readers.push(reader);
        }

        Some((
            launched.into_iter(),
            dsm_readers,
            Some(leader_plan),
            leader_mux_writers,
            leader_mux_readers,
            session_id,
            bridge,
        ))
    } else {
        None
    }
}

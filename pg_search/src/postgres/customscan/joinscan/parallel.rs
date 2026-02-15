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
use crate::postgres::customscan::joinscan::dsm_transfer::{
    MultiplexedDsmReader, MultiplexedDsmWriter, RingBufferHeader,
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

        let participant_index = worker_number.to_participant_index(config.leader_participation);
        let total_participants = config.total_participants;

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

        let mut writer_regions = Vec::new();
        let mut reader_regions = Vec::new();

        let p = total_participants;
        let region_size = config.region_size;

        // Base pointer of the shared memory region
        let base_ptr = ring_buffer_slice.as_ptr() as *mut u8;

        for j in 0..p {
            // Region for writer from us (participant_index) to participant j.
            let writer_idx = participant_index * p + j;
            let offset = writer_idx * region_size;

            let (header, data, data_len) =
                unsafe { RingBufferHeader::from_raw_parts(base_ptr, offset, region_size) };

            writer_regions.push(RegionInfo {
                header,
                data,
                data_len,
            });

            // Region for reader from participant j to us (participant_index).
            let reader_idx = j * p + participant_index;
            let offset = reader_idx * region_size;

            let (header, data, data_len) =
                unsafe { RingBufferHeader::from_raw_parts(base_ptr, offset, region_size) };

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
    Arc<crate::postgres::customscan::joinscan::dsm_transfer::SignalBridge>,
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
        .block_on(
            crate::postgres::customscan::joinscan::dsm_transfer::SignalBridge::new(
                0, // Leader is index 0
                session_id,
            ),
        )
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

        let mut leader_mux_writers = Vec::with_capacity(total_participants);
        let mut leader_mux_readers = Vec::with_capacity(total_participants);

        let leader_participant_index = 0;

        // Retrieve the ring buffer slice from DSM
        let ring_buffer_slice = launched
            .state_manager()
            .slice::<u8>(2 + nworkers)
            .expect("wrong type for ring_buffer_slice")
            .expect("missing ring_buffer_slice value");

        let base_ptr = ring_buffer_slice.as_ptr() as *mut u8;

        let p = total_participants;
        for j in 0..p {
            // Leader's writers to all participants j.
            let writer_idx = leader_participant_index * p + j;
            let offset = writer_idx * region_size;

            let (header, data, data_len) =
                unsafe { RingBufferHeader::from_raw_parts(base_ptr, offset, region_size) };

            let mux_writer = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
                header,
                data,
                data_len,
                bridge.clone(),
                j,
            )));
            leader_mux_writers.push(mux_writer);

            // Leader's readers from all participants j.
            let reader_idx = j * p + leader_participant_index;
            let offset = reader_idx * region_size;

            let (header, data, data_len) =
                unsafe { RingBufferHeader::from_raw_parts(base_ptr, offset, region_size) };

            let mux_reader = Arc::new(Mutex::new(MultiplexedDsmReader::new(
                header,
                data,
                data_len,
                bridge.clone(),
                j,
            )));
            leader_mux_readers.push(mux_reader.clone());
        }

        Some((
            launched.into_iter(),
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

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

//! Parallel Exchange Operator for MPP Execution.
//!
//! This module implements the `DsmExchangeExec` operator, which serves as the boundary
//! between parallel execution stages. It handles data shuffling across processes
//! using shared memory ring buffers.
//!
//! # The "RPC-Server" Architecture
//!
//! This implementation follows a "Lazy Request" / "RPC-Server" model rather than the
//! traditional "Eager Push" model used by Spark or Ballista.
//!
//! ## Core Concepts
//!
//! ### 1. Unique Physical Stream ID
//!
//! We distinguish between the **Logical Stream** (the shuffle operation in the plan) and
//! the **Physical Stream** (the point-to-point connection).
//!
//! - **Logical Stream ID**: Assigned by `EnforceDsmShuffle` optimizer rule (sequential IDs).
//! - **Physical Stream ID**: `(Logical ID << 16) | Sender Index`.
//!   - Example: Logical Stream 1 from Worker 3 has Physical ID `0x00010003`.
//!
//! ### 2. Control Channel Protocol
//!
//! The "Control Channel" (reverse direction from Reader -> Writer) implements an RPC-like protocol:
//! - StartStream - Begin executing a pre-arranged stream.
//! - CancelStream - Cancel a stream after starting it.
//!
//! ### 3. Stream Registry (The "Listener")
//!
//! Each process (Leader and Workers) maintains a `StreamRegistry`.
//!
//! - **Registration Phase**: During startup (plan deserialization), we traverse the physical plan.
//!   Every `DsmExchangeExec` encountered is "registered" but not executed. We store its input plan.
//! - **Listening Phase**: The process runs a `Control Service` loop that polls the Control Channels
//!   of all its `MultiplexedDsmWriter`s.
//! - **Execution Phase**: When a `StartStream(id)` message arrives:
//!   1.  The Control Service calls `trigger_stream(id)`.
//!   2.  It looks up the registered plan for `id`.
//!   3.  It spawns a background Tokio task to execute that sub-plan.
//!   4.  The task writes data to the DSM buffer.
//!
//! ## Sanitization (Deadlock Prevention)
//!
//! To prevent deadlocks caused by DataFusion operators pinning shared memory buffers (e.g. `SortExec`),
//! `DsmExchangeExec` supports a `sanitized` mode. When enabled (`config.sanitized = true`):
//! - The reader side performs a deep copy of the data *immediately* upon reading from DSM.
//! - This ensures that the downstream operators (like Sort/Join) hold references to heap-allocated
//!   memory (Copy) rather than the shared memory ring buffer (Zero-Copy).
//! - This is controlled by the `EnforceSanitization` optimizer rule, which detects unsafe patterns
//!   and enables sanitization on the exchange.
//!
//! ## Analogy: RPC Tree
//!
//! The system can be visualized as a tree of RPC calls.
//!
//! 1.  **Leader** executes the Root Plan.
//! 2.  When it hits a `DsmExchangeExec` (acting as Consumer), it makes an "RPC Call" (`StartStream`)
//!     to a specific Worker node (or itself).
//! 3.  The **Worker** receives the call. It looks up the "Procedure" (the sub-plan corresponding
//!     to that stream) and executes it.
//! 4.  If that sub-plan contains more `DsmExchangeExec` nodes (acting as Consumers), the Worker
//!     recursively makes more RPC calls to other nodes.
//! 5.  Data flows back up the tree as the response stream.

use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::io::ErrorKind;
use std::sync::Arc;
use std::task::Poll;

use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use async_stream::try_stream;
use datafusion::common::Result;
use datafusion::config::ConfigOptions;
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::Partitioning;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::execution_plan::EmissionType;
use datafusion::physical_plan::repartition::{BatchPartitioner, RepartitionExec};
use datafusion::physical_plan::sorts::sort_preserving_merge::SortPreservingMergeExec;
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, ExecutionPlanProperties, PlanProperties,
};
use futures::{future::poll_fn, Stream, StreamExt};
use parking_lot::Mutex;
use tokio::sync::watch;

use crate::postgres::customscan::joinscan::transport::TransportMesh;
use crate::postgres::customscan::joinscan::transport::{
    dsm_reader, ControlMessage, DsmWriter, LogicalStreamId, ParticipantId, PhysicalStreamId,
    SignalBridge,
};
use crate::scan::table_provider::MppParticipantConfig;

use crate::api::HashMap;
use tokio::task::JoinHandle;

pub struct StreamSource {
    pub input: Arc<dyn ExecutionPlan>,
    pub partitioning: Partitioning,
    pub config: DsmExchangeConfig,
}

/// Registry for managing the lifecycle of lazily executed streams.
///
/// This registry is the core of the "Lazy Request" / "RPC-style" execution model.
/// Instead of workers eagerly executing their entire plan, they register their available
/// "Procedures" (stream sources) here.
///
/// When a consumer (Reader) needs data, it sends a `StartStream` request (RPC).
/// The `Control Service` receives this request, looks up the plan in this registry,
/// and spawns the task to produce the data.
///
/// This prevents stream reuse bugs and race conditions by ensuring every stream execution
/// is strictly causal: Request -> Execution -> Response.
#[derive(Default)]
pub struct StreamRegistry {
    /// Maps Physical Stream ID -> The execution parameters.
    sources: HashMap<PhysicalStreamId, StreamSource>,
    /// Maps Physical Stream ID -> The handle of the running task (to prevent duplicate spawning).
    running_tasks: HashMap<PhysicalStreamId, JoinHandle<()>>,
    /// Maps Physical Stream ID -> Abort handle for the task.
    abort_handles: HashMap<PhysicalStreamId, tokio::task::AbortHandle>,
    /// Maps Physical Stream ID -> A completion signal (for local waiting).
    completions: HashMap<PhysicalStreamId, watch::Sender<bool>>,
}

/// A registry for process-local DSM communication channels.
pub struct DsmMesh {
    pub transport: TransportMesh,
    pub registry: Mutex<StreamRegistry>,
}

lazy_static::lazy_static! {
    pub static ref DSM_MESH: Mutex<Option<DsmMesh>> = Mutex::new(None);
}

pub fn register_dsm_mesh(mesh: DsmMesh) {
    let mut guard = DSM_MESH.lock();
    *guard = Some(mesh);
}

pub fn clear_dsm_mesh() {
    let mut guard = DSM_MESH.lock();
    *guard = None;
}

impl Drop for DsmMesh {
    fn drop(&mut self) {
        self.transport.detach();
    }
}

pub fn register_stream_source(source: StreamSource, participant_id: ParticipantId) {
    let mut guard = DSM_MESH.lock();
    if let Some(mesh) = guard.as_mut() {
        // Calculate physical ID: (Logical << 16) | Sender (us)
        // participant_id is "us" (the sender).
        let physical_id = PhysicalStreamId::new(source.config.stream_id, participant_id);

        let mut registry = mesh.registry.lock();
        registry.sources.insert(physical_id, source);
    }
}

struct SignalOnDrop(Option<watch::Sender<bool>>);
impl Drop for SignalOnDrop {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(true);
        }
    }
}

pub fn trigger_stream(physical_stream_id: PhysicalStreamId, context: Arc<TaskContext>) {
    let mut guard = DSM_MESH.lock();
    let mesh = guard.as_mut().expect("DSM mesh not registered");
    let mut registry = mesh.registry.lock();

    if registry.running_tasks.contains_key(&physical_stream_id) {
        return;
    }

    if let Some(source) = registry.sources.get(&physical_stream_id) {
        let input = source.input.clone();
        let partitioning = source.partitioning.clone();
        let config = source.config.clone();

        // Prepare completion notifier. Create it if it doesn't exist (DsmExchangeExec hasn't run yet).
        let tx = registry
            .completions
            .entry(physical_stream_id)
            .or_insert_with(|| {
                let (tx, _rx) = watch::channel(false);
                tx
            })
            .clone();

        let task = tokio::task::spawn_local(async move {
            let _guard = SignalOnDrop(Some(tx));
            DsmExchangeExec::producer_task(input, partitioning, config, context).await;
        });

        registry
            .abort_handles
            .insert(physical_stream_id, task.abort_handle());
        registry.running_tasks.insert(physical_stream_id, task);
    } else {
        // No-op
    }
}

pub fn cancel_triggered_stream(physical_stream_id: PhysicalStreamId) {
    let mut guard = DSM_MESH.lock();
    if let Some(mesh) = guard.as_mut() {
        let mut registry = mesh.registry.lock();
        if let Some(handle) = registry.abort_handles.remove(&physical_stream_id) {
            handle.abort();
        }
        registry.running_tasks.remove(&physical_stream_id);
    }
}

use tokio::task::LocalSet;

/// Spawns the background Control Service.
///
/// This service acts as the "RPC Listener" for the process. It continually polls the
/// control channels (reverse channels) of all DSM Writers assigned to this process.
///
/// When a `StartStream` message is received (analogous to an incoming RPC call),
/// it triggers the execution of the corresponding sub-plan registered in `StreamRegistry`.
pub fn spawn_control_service(local_set: &LocalSet, task_ctx: Arc<TaskContext>) {
    local_set.spawn_local(async move {
        loop {
            // Get writers and bridge from global mesh
            let (mux_writers, bridge) = {
                let guard = DSM_MESH.lock();
                if let Some(mesh) = guard.as_ref() {
                    (
                        mesh.transport.mux_writers.clone(),
                        mesh.transport.bridge.clone(),
                    )
                } else {
                    return; // Mesh destroyed?
                }
            };

            futures::future::poll_fn(|cx| {
                bridge.register_waker(cx.waker().clone(), None);
                let mut work_done = false;

                for mux in &mux_writers {
                    let mut guard = mux.lock();
                    let frames = guard.read_control_frames();
                    if !frames.is_empty() {
                        work_done = true;
                        for (msg_type, payload) in frames {
                            if let Some(msg) = ControlMessage::try_from_frame(msg_type, &payload) {
                                match msg {
                                    ControlMessage::StartStream(id) => {
                                        trigger_stream(id, task_ctx.clone());
                                    }
                                    ControlMessage::CancelStream(id) => {
                                        // Mark stream as cancelled in the transport layer
                                        guard.mark_stream_cancelled(id);
                                        // Cancel the execution task
                                        cancel_triggered_stream(id);
                                    }
                                }
                            }
                        }
                    }
                }

                if work_done {
                    // We processed messages. Yield to let triggered tasks run,
                    // but return Ready(()) to allow the loop to continue.
                    // Actually, if we return Ready, we exit poll_fn. The loop continues and calls poll_fn again immediately.
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;

            // Yield to allow spawned tasks to progress
            tokio::task::yield_now().await;
        }
    });
}

pub fn get_dsm_writer(
    participant: usize,
    stream_id: LogicalStreamId,
    sender_id: ParticipantId,
    schema: SchemaRef,
) -> Option<DsmWriter> {
    let guard = DSM_MESH.lock();
    if let Some(mesh) = guard.as_ref() {
        if participant < mesh.transport.mux_writers.len() {
            return Some(DsmWriter::new(
                mesh.transport.mux_writers[participant].clone(),
                stream_id,
                sender_id,
                schema,
            ));
        }
    }
    None
}

pub fn get_dsm_reader(
    participant: usize,
    stream_id: LogicalStreamId,
    sender_id: ParticipantId,
    schema: SchemaRef,
    sanitized: bool,
) -> Option<SendableRecordBatchStream> {
    let guard = DSM_MESH.lock();
    if let Some(mesh) = guard.as_ref() {
        if participant < mesh.transport.mux_readers.len() {
            let reader = dsm_reader(
                mesh.transport.mux_readers[participant].clone(),
                stream_id,
                sender_id,
                schema,
                sanitized,
            );
            return Some(reader);
        }
    }
    None
}

/// A physical optimizer rule that replaces standard `RepartitionExec` with `DsmExchangeExec`.
#[derive(Debug)]
pub struct EnforceDsmShuffle {
    pub total_participants: usize,
}

impl EnforceDsmShuffle {
    fn wrap_in_dsm_exchange(
        &self,
        input: Arc<dyn ExecutionPlan>,
        producer_partitioning: Partitioning,
        output_partitioning: Partitioning,
        stream_id: u16,
        mode: ExchangeMode,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let config = DsmExchangeConfig {
            stream_id: LogicalStreamId(stream_id),
            total_participants: self.total_participants,
            mode,
            sanitized: false,
        };

        Ok(Arc::new(DsmExchangeExec::try_new(
            input,
            producer_partitioning,
            output_partitioning,
            config,
        )?))
    }
}

impl PhysicalOptimizerRule for EnforceDsmShuffle {
    fn name(&self) -> &str {
        "EnforceDsmShuffle"
    }

    fn schema_check(&self) -> bool {
        true
    }

    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let mut stream_id_counter: u16 = 0;

        // Use a recursive function to wrap nodes top-down.
        fn wrap_node(
            node: Arc<dyn ExecutionPlan>,
            rule: &EnforceDsmShuffle,
            counter: &mut u16,
        ) -> Result<Arc<dyn ExecutionPlan>> {
            // First, recursively optimize children.
            let children: Vec<_> = node
                .children()
                .into_iter()
                .map(|c| wrap_node(c.clone(), rule, counter))
                .collect::<Result<Vec<_>>>()?;

            let node = if children.is_empty() {
                node
            } else {
                node.with_new_children(children)?
            };

            // Now, check if this node needs wrapping.
            if let Some(repartition) = node.as_any().downcast_ref::<RepartitionExec>() {
                let producer_partitioning = repartition.partitioning().clone();
                let (mode, output_partitioning) =
                    if matches!(producer_partitioning, Partitioning::UnknownPartitioning(1)) {
                        (
                            ExchangeMode::Gather,
                            Partitioning::UnknownPartitioning(rule.total_participants),
                        )
                    } else {
                        (
                            ExchangeMode::Redistribute,
                            Partitioning::UnknownPartitioning(
                                producer_partitioning.partition_count(),
                            ),
                        )
                    };

                let stream_id = *counter;
                *counter = counter.checked_add(1).ok_or_else(|| {
                    datafusion::common::DataFusionError::Internal(
                        "Too many shuffle stages (max 65535)".to_string(),
                    )
                })?;

                let input = repartition.children()[0].clone();
                rule.wrap_in_dsm_exchange(
                    input,
                    producer_partitioning,
                    output_partitioning,
                    stream_id,
                    mode,
                )
            } else if let Some(merge) = node.as_any().downcast_ref::<SortPreservingMergeExec>() {
                let input = merge.children()[0].clone();
                if input.output_partitioning().partition_count() > 1
                    && !input.as_any().is::<DsmExchangeExec>()
                {
                    let partitioning = Partitioning::UnknownPartitioning(rule.total_participants);
                    let stream_id = *counter;
                    *counter = counter.checked_add(1).ok_or_else(|| {
                        datafusion::common::DataFusionError::Internal(
                            "Too many shuffle stages (max 65535)".to_string(),
                        )
                    })?;

                    let reader = rule.wrap_in_dsm_exchange(
                        input,
                        partitioning.clone(),
                        partitioning,
                        stream_id,
                        ExchangeMode::Gather,
                    )?;

                    Ok(Arc::new(merge.clone()).with_new_children(vec![reader])?)
                } else {
                    Ok(node)
                }
            } else if let Some(coalesce) = node.as_any().downcast_ref::<CoalescePartitionsExec>() {
                let input = coalesce.children()[0].clone();
                if input.output_partitioning().partition_count() > 1
                    && !input.as_any().is::<DsmExchangeExec>()
                {
                    let partitioning = Partitioning::UnknownPartitioning(rule.total_participants);
                    let stream_id = *counter;
                    *counter = counter.checked_add(1).ok_or_else(|| {
                        datafusion::common::DataFusionError::Internal(
                            "Too many shuffle stages (max 65535)".to_string(),
                        )
                    })?;

                    let reader = rule.wrap_in_dsm_exchange(
                        input,
                        partitioning.clone(),
                        partitioning,
                        stream_id,
                        ExchangeMode::Gather,
                    )?;

                    Ok(Arc::new(CoalescePartitionsExec::new(reader)) as Arc<dyn ExecutionPlan>)
                } else {
                    Ok(node)
                }
            } else {
                Ok(node)
            }
        }

        wrap_node(plan, self, &mut stream_id_counter)
    }
}

#[cfg(any(test, feature = "pg_test"))]
pub fn collect_dsm_exchanges(plan: Arc<dyn ExecutionPlan>, sources: &mut Vec<StreamSource>) {
    if let Some(exchange) = plan.as_any().downcast_ref::<DsmExchangeExec>() {
        sources.push(StreamSource {
            input: exchange.input.clone(),
            partitioning: exchange.producer_partitioning.clone(),
            config: exchange.config.clone(),
        });
    }

    for child in plan.children() {
        collect_dsm_exchanges(child.clone(), sources);
    }
}

pub fn get_dsm_bridge() -> Arc<SignalBridge> {
    let guard = DSM_MESH.lock();
    let mesh = guard.as_ref().expect("DSM mesh not registered");
    mesh.transport.bridge.clone()
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ExchangeMode {
    /// Every node sends to every node based on partitioning hash.
    Redistribute,
    /// Every node sends all its data to the leader (node 0).
    Gather,
}

/// Shared configuration for DSM exchange nodes.
///
/// This struct holds all metadata required to coordinate the shuffle boundary,
/// ensuring that both the producer (Writer) and consumer (Reader) sides
/// are configured identically for a given logical stream.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DsmExchangeConfig {
    pub stream_id: LogicalStreamId,
    pub total_participants: usize,
    pub mode: ExchangeMode,
    #[serde(default)]
    pub sanitized: bool,
}

pub(crate) fn get_mpp_config(ctx: &TaskContext) -> (usize, usize) {
    ctx.session_config()
        .options()
        .extensions
        .get::<MppParticipantConfig>()
        .map(|c| (c.index, c.total_participants))
        .unwrap_or((0, 1))
}

/// A physical operator that handles both the production (passive) and consumption (active) of shuffled data.
///
/// This single node replaces the `DsmReaderExec` / `DsmWriterExec` pair.
///
/// **As a Consumer (Reader)**:
/// When `execute()` is called, it initiates the stream by sending `StartStream` to the producer(s).
/// If `config.sanitized` is true, it performs a deep copy of incoming batches to prevent deadlocks
/// with downstream blocking operators.
///
/// **As a Producer (Writer)**:
/// It holds the `input` plan and `producer_partitioning`. It registers these in the `StreamRegistry`.
/// When triggered via `StartStream`, `producer_task` is executed to run the input plan and write to DSM.
#[derive(Debug)]
pub struct DsmExchangeExec {
    pub input: Arc<dyn ExecutionPlan>,
    pub producer_partitioning: Partitioning,
    pub config: DsmExchangeConfig,
    pub properties: PlanProperties,
}

impl DsmExchangeExec {
    pub fn try_new(
        input: Arc<dyn ExecutionPlan>,
        producer_partitioning: Partitioning,
        output_partitioning: Partitioning,
        config: DsmExchangeConfig,
    ) -> Result<Self> {
        let properties = PlanProperties::new(
            input.equivalence_properties().clone(),
            output_partitioning,
            EmissionType::Incremental,
            input.boundedness(),
        );

        Ok(Self {
            input,
            producer_partitioning,
            config,
            properties,
        })
    }

    pub(crate) async fn producer_task(
        input: Arc<dyn ExecutionPlan>,
        partitioning: Partitioning,
        config: DsmExchangeConfig,
        context: Arc<TaskContext>,
    ) {
        let (participant_index, total_participants) = get_mpp_config(&context);
        let participant_id = ParticipantId(participant_index as u16);
        let num_partitions = input.output_partitioning().partition_count();
        let mut streams = Vec::with_capacity(num_partitions);
        for i in 0..num_partitions {
            streams.push(
                input
                    .execute(i, context.clone())
                    .expect("Failed to execute input"),
            );
        }
        let input_stream = futures::stream::select_all(streams).fuse();
        let mut input_stream = Box::pin(input_stream);
        let schema = input.schema();

        let mut writers = Vec::new();
        for i in 0..total_participants {
            writers.push(
                get_dsm_writer(i, config.stream_id, participant_id, schema.clone())
                    .expect("Failed to get DSM writer"),
            );
        }

        // Initialize partitioner ONCE if needed
        let mut partitioner = if let ExchangeMode::Redistribute = config.mode {
            Some(
                BatchPartitioner::try_new(
                    partitioning,
                    datafusion::physical_plan::metrics::Time::default(),
                    0,
                    1,
                )
                .expect("Failed to create partitioner"),
            )
        } else {
            None
        };

        // Each writer has its own queue of pending batches
        let mut out_queues: Vec<std::collections::VecDeque<RecordBatch>> =
            vec![std::collections::VecDeque::new(); total_participants];

        let mut input_done = false;
        let bridge = get_dsm_bridge();

        poll_fn(|cx| {
            loop {
                let mut progress = false;

                // 1. Try to drain all queues
                let mut all_queues_empty = true;
                let mut blocked_on_write = false;

                for i in 0..total_participants {
                    while let Some(batch) = out_queues[i].front() {
                        match writers[i].write_batch(batch) {
                            Ok(_) => {
                                out_queues[i].pop_front();
                                progress = true;
                            }
                            Err(datafusion::error::DataFusionError::IoError(ref msg))
                                if msg.kind() == ErrorKind::WouldBlock =>
                            {
                                // Check-Register-Check pattern
                                bridge.register_waker(cx.waker().clone(), None);

                                // Retry immediately to avoid race condition where space became available
                                // after the first check but before we registered the waker.
                                match writers[i].write_batch(batch) {
                                    Ok(_) => {
                                        out_queues[i].pop_front();
                                        progress = true;
                                    }
                                    Err(datafusion::error::DataFusionError::IoError(ref msg))
                                        if msg.kind() == ErrorKind::WouldBlock =>
                                    {
                                        blocked_on_write = true;
                                        all_queues_empty = false;
                                        break;
                                    }
                                    Err(e) => panic!("Producer failed on retry: {}", e),
                                }
                                break;
                            }
                            Err(datafusion::error::DataFusionError::IoError(ref msg))
                                if msg.kind() == ErrorKind::BrokenPipe =>
                            {
                                // Receiver closed
                                out_queues[i].clear();
                                break;
                            }
                            Err(e) => panic!("Producer failed: {}", e),
                        }
                    }
                    if !out_queues[i].is_empty() {
                        all_queues_empty = false;
                    }
                }

                // 2. Poll input stream if not done
                if !input_done {
                    match input_stream.as_mut().poll_next(cx) {
                        Poll::Ready(Some(Ok(batch))) => {
                            match config.mode {
                                ExchangeMode::Redistribute => {
                                    partitioner
                                        .as_mut()
                                        .unwrap()
                                        .partition(batch, |dest_idx, partitioned_batch| {
                                            if dest_idx < out_queues.len() {
                                                out_queues[dest_idx].push_back(partitioned_batch);
                                            }
                                            Ok(())
                                        })
                                        .expect("Partitioning failed");
                                }
                                ExchangeMode::Gather => {
                                    out_queues[0].push_back(batch);
                                }
                            }
                            progress = true;
                        }
                        Poll::Ready(Some(Err(e))) => panic!("Input stream failed: {}", e),
                        Poll::Ready(None) => {
                            input_done = true;
                            progress = true; // State change counts as progress
                        }
                        Poll::Pending => {
                            // Input not ready
                        }
                    }
                }

                if input_done && all_queues_empty {
                    return Poll::Ready(());
                }

                // If we made progress, try again immediately to drain more
                if progress {
                    continue;
                }

                // No progress made.
                // If blocked on write, register waker with bridge.
                if blocked_on_write {
                    bridge.register_waker(cx.waker().clone(), None);
                }

                // If input is pending, it already registered waker.

                return Poll::Pending;
            }
        })
        .await;

        for writer in writers {
            let _ = writer.finish();
        }
    }

    fn create_consumer_stream(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let (participant_index, total_participants) = get_mpp_config(&context);
        let schema = self.input.schema();

        // Return the consumer side stream for THIS partition.
        match self.config.mode {
            ExchangeMode::Redistribute => {
                // In Redistribute mode, every node k consumes Partition k.
                if partition != participant_index {
                    // pgrx::warning!(
                    //     "[Worker {}] DsmReaderExec: Redistribute execute({}) requested, but I am participant {}. Returning empty stream.",
                    //     unsafe { pgrx::pg_sys::ParallelWorkerNumber },
                    //     partition,
                    //     participant_index
                    // );
                    return Ok(Box::pin(RecordBatchStreamAdapter::new(
                        schema,
                        futures::stream::empty(),
                    )));
                }

                let mut readers = Vec::new();
                for i in 0..total_participants {
                    // Read from Participant i, Logical Stream S, Sender Index i.
                    if let Some(reader) = get_dsm_reader(
                        i,
                        self.config.stream_id,
                        ParticipantId(i as u16),
                        schema.clone(),
                        self.config.sanitized,
                    ) {
                        readers.push(shared_memory_stream(reader));
                    }
                }

                Ok(Box::pin(RecordBatchStreamAdapter::new(
                    schema.clone(),
                    futures::stream::select_all(readers),
                )))
            }
            ExchangeMode::Gather => {
                // In Gather mode, only the Leader (node 0) consumes.
                if participant_index != 0 {
                    // Worker side: we are not the consumer.
                    // But DataFusion might call execute(p) for ALL p if it's merging.
                    // Workers should only 'wait' once per logical stream.
                    // We choose to wait when partition == participant_index.
                    if partition == participant_index {
                        let physical_id = PhysicalStreamId::new(
                            self.config.stream_id,
                            ParticipantId(participant_index as u16),
                        );

                        let rx = {
                            let mut guard = DSM_MESH.lock();
                            let mesh = guard.as_mut().expect("DSM mesh not registered");
                            let mut registry = mesh.registry.lock();
                            if let Some(tx) = registry.completions.get(&physical_id) {
                                tx.subscribe()
                            } else {
                                let (tx, rx) = watch::channel(false);
                                registry.completions.insert(physical_id, tx);
                                rx
                            }
                        };
                        return Ok(wait_for_producer_stream(schema, rx));
                    } else {
                        return Ok(Box::pin(RecordBatchStreamAdapter::new(
                            schema,
                            futures::stream::empty(),
                        )));
                    }
                }

                // Leader: Pull Partition `partition` from Worker `partition`.

                if let Some(reader) = get_dsm_reader(
                    partition,
                    self.config.stream_id,
                    ParticipantId(partition as u16),
                    schema.clone(),
                    self.config.sanitized,
                ) {
                    Ok(shared_memory_stream(reader))
                } else {
                    Ok(Box::pin(RecordBatchStreamAdapter::new(
                        schema,
                        futures::stream::empty(),
                    )))
                }
            }
        }
    }
}

impl DisplayAs for DsmExchangeExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut Formatter) -> std::fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                write!(
                    f,
                    "DsmExchangeExec(stream_id={}, producer_partitioning={:?}, sanitized={})",
                    self.config.stream_id, self.producer_partitioning, self.config.sanitized
                )
            }
            _ => Ok(()),
        }
    }
}

impl ExecutionPlan for DsmExchangeExec {
    fn name(&self) -> &str {
        "DsmExchangeExec"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn properties(&self) -> &PlanProperties {
        &self.properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.input]
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(Arc::new(Self::try_new(
            children[0].clone(),
            self.producer_partitioning.clone(),
            self.properties.output_partitioning().clone(),
            self.config.clone(),
        )?))
    }

    fn execute(
        &self,

        partition: usize,

        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        // In the RPC-Server model, the execution of the input plan (production)
        // is decoupled from the execution of this node (consumption).
        //
        // 1. The input plan is executed by the `producer_task`, which is triggered
        //    asynchronously by a `StartStream` control message.
        // 2. This `execute` method acts as the "Client": it initiates the stream
        //    by sending `StartStream` and returning a consumer stream that reads
        //    from the shared memory ring buffer.
        //
        // Therefore, we do NOT execute `self.input` here.

        self.create_consumer_stream(partition, context)
    }
}

/// A stream that waits for the producer task to complete before finishing.
/// This is used on worker nodes for "Gather" operations to ensure the producer
/// task runs to completion even though no data is consumed locally.
fn wait_for_producer_stream(
    schema: SchemaRef,
    mut rx: watch::Receiver<bool>,
) -> SendableRecordBatchStream {
    let schema_clone = schema.clone();
    let stream = try_stream! {
        while !*rx.borrow() {
            if rx.changed().await.is_err() {
                return;
            }
        }
        // Phantom yield to satisfy type inference (T=RecordBatch)
        if false {
            yield RecordBatch::new_empty(schema_clone);
        }
    };
    Box::pin(RecordBatchStreamAdapter::new(schema, Box::pin(stream)))
}

/// Adapts a `SharedMemoryReader` (which requires manual waker registration via `SignalBridge`)
/// into a standard `RecordBatchStream`.
fn shared_memory_stream(mut reader: SendableRecordBatchStream) -> SendableRecordBatchStream {
    let schema = reader.schema();
    let stream = try_stream! {
        while let Some(batch) = reader.next().await {
            yield batch?;
        }
    };
    Box::pin(RecordBatchStreamAdapter::new(schema, Box::pin(stream)))
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use arrow_schema::{DataType, Field, Schema};
    use datafusion::physical_expr::EquivalenceProperties;
    use datafusion::physical_plan::execution_plan::Boundedness;
    use datafusion::physical_plan::repartition::RepartitionExec;
    use std::sync::Arc;

    #[derive(Debug)]
    struct MockLeaf {
        properties: PlanProperties,
    }

    impl MockLeaf {
        fn new(schema: SchemaRef) -> Self {
            let properties = PlanProperties::new(
                EquivalenceProperties::new(schema),
                Partitioning::UnknownPartitioning(1),
                EmissionType::Incremental,
                Boundedness::Bounded,
            );
            Self { properties }
        }
    }

    impl DisplayAs for MockLeaf {
        fn fmt_as(&self, _t: DisplayFormatType, f: &mut Formatter) -> std::fmt::Result {
            write!(f, "MockLeaf")
        }
    }

    impl ExecutionPlan for MockLeaf {
        fn name(&self) -> &str {
            "MockLeaf"
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
            unreachable!()
        }
    }

    #[pgrx::pg_test]
    fn test_enforce_dsm_shuffle_rule() {
        let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)]));
        let leaf = Arc::new(MockLeaf::new(schema));

        let repartition =
            Arc::new(RepartitionExec::try_new(leaf, Partitioning::RoundRobinBatch(2)).unwrap());

        let rule = EnforceDsmShuffle {
            total_participants: 2,
        };
        let optimized = rule
            .optimize(repartition, &ConfigOptions::default())
            .unwrap();

        assert!(optimized.as_any().is::<DsmExchangeExec>());
        assert_eq!(optimized.children().len(), 1);
        // The child is now the input of DsmExchangeExec (MockLeaf), not DsmWriterExec
        assert!(optimized.children()[0].as_any().is::<MockLeaf>());
    }
}

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

use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::io::ErrorKind;
use std::sync::Arc;
use std::task::Poll;

use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use async_stream::try_stream;
use datafusion::common::tree_node::{Transformed, TreeNode};
use datafusion::common::Result;
use datafusion::config::ConfigOptions;
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::{EquivalenceProperties, Partitioning};
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::repartition::{BatchPartitioner, RepartitionExec};
use datafusion::physical_plan::sorts::sort_preserving_merge::SortPreservingMergeExec;
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, ExecutionPlanProperties, PlanProperties,
};
use futures::StreamExt;
use parking_lot::Mutex;
use tokio::sync::watch;

use crate::postgres::customscan::joinscan::dsm_transfer::{
    dsm_shared_memory_reader, DsmSharedMemoryWriter, MultiplexedDsmReader, MultiplexedDsmWriter,
    SignalBridge,
};

use std::collections::HashMap;
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
    sources: HashMap<u32, StreamSource>,
    /// Maps Physical Stream ID -> The handle of the running task (to prevent duplicate spawning).
    running_tasks: HashMap<u32, JoinHandle<()>>,
    /// Maps Physical Stream ID -> Abort handle for the task.
    abort_handles: HashMap<u32, tokio::task::AbortHandle>,
    /// Maps Physical Stream ID -> A completion signal (for local waiting).
    completions: HashMap<u32, watch::Sender<bool>>,
}

/// A registry for process-local DSM communication channels.
pub struct DsmMesh {
    pub total_participants: usize,
    pub mux_writers: Vec<Arc<Mutex<MultiplexedDsmWriter>>>,
    pub mux_readers: Vec<Arc<Mutex<MultiplexedDsmReader>>>,
    pub bridge: Arc<SignalBridge>,
    pub registry: Mutex<StreamRegistry>,
}

lazy_static::lazy_static! {
    pub static ref DSM_MESH: Mutex<Option<DsmMesh>> = Mutex::new(None);
}

pub fn register_dsm_mesh(mesh: DsmMesh) {
    let mut guard = DSM_MESH.lock();
    *guard = Some(mesh);
}

pub fn register_stream_source(source: StreamSource) {
    let mut guard = DSM_MESH.lock();
    if let Some(mesh) = guard.as_mut() {
        // Calculate physical ID: (Logical << 16) | Sender (us)
        // source.config.participant_index is "us" (the sender).
        let physical_id =
            (source.config.stream_id << 16) | ((source.config.participant_index as u32) & 0xFFFF);

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

pub fn trigger_stream(physical_stream_id: u32, context: Arc<TaskContext>) {
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

        // Prepare completion notifier if it exists
        let tx = registry.completions.get(&physical_stream_id).cloned();

        let task = tokio::task::spawn_local(async move {
            let _guard = SignalOnDrop(tx);
            DsmWriterExec::producer_task(input, partitioning, config, context).await;
        });

        registry
            .abort_handles
            .insert(physical_stream_id, task.abort_handle());
        registry.running_tasks.insert(physical_stream_id, task);
    } else {
        pgrx::warning!(
            "[PID {}] trigger_stream: Request for unknown stream physical_id={}",
            std::process::id(),
            physical_stream_id
        );
    }
}

pub fn cancel_triggered_stream(physical_stream_id: u32) {
    let mut guard = DSM_MESH.lock();
    if let Some(mesh) = guard.as_mut() {
        let mut registry = mesh.registry.lock();
        if let Some(handle) = registry.abort_handles.remove(&physical_stream_id) {
            handle.abort();
        }
        registry.running_tasks.remove(&physical_stream_id);
    }
}

pub fn get_completion_receiver(physical_stream_id: u32) -> Option<watch::Receiver<bool>> {
    let mut guard = DSM_MESH.lock();
    if let Some(mesh) = guard.as_mut() {
        let mut registry = mesh.registry.lock();
        let (tx, rx) = watch::channel(false);
        registry.completions.entry(physical_stream_id).or_insert(tx);
        return Some(rx);
    }
    None
}

use crate::postgres::customscan::joinscan::dsm_stream::ControlMessage;
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
                    (mesh.mux_writers.clone(), mesh.bridge.clone())
                } else {
                    return; // Mesh destroyed?
                }
            };

            futures::future::poll_fn(|cx| {
                bridge.register_waker(cx.waker().clone());
                let mut work_done = false;

                for mux in &mux_writers {
                    let mut guard = mux.lock();
                    let msgs = guard.read_control_messages();
                    if !msgs.is_empty() {
                        work_done = true;
                        for msg in msgs {
                            match msg {
                                ControlMessage::StartStream(id) => {
                                    trigger_stream(id, task_ctx.clone());
                                }
                                ControlMessage::CancelStream(id) => {
                                    cancel_triggered_stream(id);
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
    stream_id: u32,
    sender_index: usize,
    schema: SchemaRef,
) -> Option<DsmSharedMemoryWriter> {
    let guard = DSM_MESH.lock();
    if let Some(mesh) = guard.as_ref() {
        if participant < mesh.mux_writers.len() {
            return Some(DsmSharedMemoryWriter::new(
                mesh.mux_writers[participant].clone(),
                stream_id,
                sender_index,
                participant,
                schema,
            ));
        }
    }
    None
}

pub fn get_dsm_reader(
    participant: usize,
    stream_id: u32,
    sender_index: usize,
    schema: SchemaRef,
) -> Option<SendableRecordBatchStream> {
    let guard = DSM_MESH.lock();
    if let Some(mesh) = guard.as_ref() {
        if participant < mesh.mux_readers.len() {
            let reader = dsm_shared_memory_reader(
                mesh.mux_readers[participant].clone(),
                stream_id,
                sender_index,
                schema,
            );
            return Some(reader);
        }
    }
    None
}

/// A physical optimizer rule that replaces standard `RepartitionExec` with `DsmWriterExec` and `DsmReaderExec`.
#[derive(Debug)]
pub struct EnforceDsmShuffle {
    pub participant_index: usize,
    pub total_participants: usize,
}

impl EnforceDsmShuffle {
    fn wrap_in_dsm_exchange(
        &self,
        input: Arc<dyn ExecutionPlan>,
        partitioning: Partitioning,
        stream_id: u32,
        mode: ExchangeMode,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let config = DsmExchangeConfig {
            stream_id,
            participant_index: self.participant_index,
            total_participants: self.total_participants,
            mode,
        };

        let writer = DsmWriterExec::try_new(input, partitioning.clone(), config.clone())?;
        let reader = DsmReaderExec::try_new(Arc::new(writer), config, partitioning)?;

        Ok(Arc::new(reader))
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
        let mut stream_id_counter = 0;

        plan.transform_up(|node| {
            if let Some(repartition) = node.as_any().downcast_ref::<RepartitionExec>() {
                let partitioning = repartition.partitioning().clone();
                let mode = if matches!(partitioning, Partitioning::UnknownPartitioning(1)) {
                    ExchangeMode::Gather
                } else {
                    ExchangeMode::Redistribute
                };

                let stream_id = stream_id_counter;
                stream_id_counter += 1;

                let input = repartition.children()[0].clone();
                let reader = self.wrap_in_dsm_exchange(input, partitioning, stream_id, mode)?;

                Ok(Transformed::yes(reader))
            } else if let Some(merge) = node.as_any().downcast_ref::<SortPreservingMergeExec>() {
                let input = merge.children()[0].clone();
                if input.output_partitioning().partition_count() > 1
                    && !input.as_any().is::<DsmReaderExec>()
                {
                    let partitioning = input.output_partitioning().clone();
                    let stream_id = stream_id_counter;
                    stream_id_counter += 1;

                    let reader = self.wrap_in_dsm_exchange(
                        input,
                        partitioning,
                        stream_id,
                        ExchangeMode::Gather,
                    )?;

                    Ok(Transformed::yes(
                        Arc::new(merge.clone()).with_new_children(vec![reader])?,
                    ))
                } else {
                    Ok(Transformed::no(node))
                }
            } else if let Some(coalesce) = node.as_any().downcast_ref::<CoalescePartitionsExec>() {
                let input = coalesce.children()[0].clone();
                if input.output_partitioning().partition_count() > 1
                    && !input.as_any().is::<DsmReaderExec>()
                {
                    let partitioning = input.output_partitioning().clone();
                    let stream_id = stream_id_counter;
                    stream_id_counter += 1;

                    let reader = self.wrap_in_dsm_exchange(
                        input,
                        partitioning,
                        stream_id,
                        ExchangeMode::Gather,
                    )?;

                    Ok(Transformed::yes(
                        Arc::new(CoalescePartitionsExec::new(reader)) as Arc<dyn ExecutionPlan>,
                    ))
                } else {
                    Ok(Transformed::no(node))
                }
            } else {
                Ok(Transformed::no(node))
            }
        })
        .map(|t| t.data)
    }
}

pub fn collect_dsm_writers(plan: Arc<dyn ExecutionPlan>, sources: &mut Vec<StreamSource>) {
    if let Some(writer) = plan.as_any().downcast_ref::<DsmWriterExec>() {
        sources.push(StreamSource {
            input: writer.input.clone(),
            partitioning: writer.partitioning.clone(),
            config: writer.config.clone(),
        });
    }

    for child in plan.children() {
        collect_dsm_writers(child.clone(), sources);
    }
}

pub fn get_dsm_bridge() -> Arc<SignalBridge> {
    let guard = DSM_MESH.lock();
    let mesh = guard.as_ref().expect("DSM mesh not registered");
    mesh.bridge.clone()
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
#[derive(Debug, Clone)]
pub struct DsmExchangeConfig {
    pub stream_id: u32,
    pub participant_index: usize,
    pub total_participants: usize,
    pub mode: ExchangeMode,
}

/// A physical operator that handles producing shuffled data.
///
/// In the Lazy/RPC model, `DsmWriterExec` is a **passive** operator.
/// It does not execute logic when `execute()` is called by DataFusion (it returns an empty stream).
///
/// Instead, it serves as a marker in the plan. Its configuration and input plan are extracted
/// and stored in the `StreamRegistry`. The actual execution logic (`producer_task`) is
/// spawned only when a `StartStream` request is received.
#[derive(Debug)]
pub struct DsmWriterExec {
    input: Arc<dyn ExecutionPlan>,
    partitioning: Partitioning,
    config: DsmExchangeConfig,
    properties: PlanProperties,
}

impl DsmWriterExec {
    pub fn try_new(
        input: Arc<dyn ExecutionPlan>,
        partitioning: Partitioning,
        config: DsmExchangeConfig,
    ) -> Result<Self> {
        let properties = PlanProperties::new(
            EquivalenceProperties::new(input.schema()),
            Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );

        Ok(Self {
            input,
            partitioning,
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
        // Execute all input partitions and merge them.
        let num_partitions = input.output_partitioning().partition_count();
        let mut streams = Vec::with_capacity(num_partitions);
        for i in 0..num_partitions {
            streams.push(
                input
                    .execute(i, context.clone())
                    .expect("Failed to execute input"),
            );
        }
        let mut stream = futures::stream::select_all(streams);
        let schema = input.schema();

        let (total_participants, _bridge) = {
            let guard = DSM_MESH.lock();
            let mesh = guard.as_ref().expect("DSM mesh not registered");
            (mesh.total_participants, mesh.bridge.clone())
        };

        let mut writers = Vec::new();
        for i in 0..total_participants {
            writers.push(
                get_dsm_writer(
                    i,
                    config.stream_id,
                    config.participant_index,
                    schema.clone(),
                )
                .expect("Failed to get DSM writer"),
            );
        }

        match config.mode {
            ExchangeMode::Redistribute => {
                let mut partitioner = BatchPartitioner::try_new(
                    partitioning,
                    datafusion::physical_plan::metrics::Time::default(),
                    0,
                    1,
                )
                .expect("Failed to create partitioner");

                while let Some(batch) = stream.next().await {
                    let batch = batch.expect("Input stream failed");
                    let mut blocked_batch = Some(batch);
                    let mut finished_partitions = std::collections::HashSet::new();

                    while let Some(batch) = blocked_batch.take() {
                        let mut blocked = false;
                        let mut signaled_participants = std::collections::HashSet::new();

                        partitioner
                            .partition(batch.clone(), |dest_idx, partitioned_batch| {
                                if blocked || finished_partitions.contains(&dest_idx) {
                                    return Ok(());
                                }

                                if dest_idx < writers.len() {
                                    match writers[dest_idx].write_batch(&partitioned_batch) {
                                        Ok(_) => {
                                            signaled_participants.insert(dest_idx);
                                            finished_partitions.insert(dest_idx);
                                        }
                                        Err(datafusion::error::DataFusionError::IoError(
                                            ref msg,
                                        )) if msg.kind() == std::io::ErrorKind::WouldBlock => {
                                            blocked = true;
                                        }
                                        Err(datafusion::error::DataFusionError::IoError(
                                            ref msg,
                                        )) if msg.kind() == std::io::ErrorKind::BrokenPipe => {
                                            return Ok(());
                                        }
                                        Err(e) => {
                                            pgrx::warning!(
                                                "[PID {}] producer_task ERROR writing to {}: {}",
                                                std::process::id(),
                                                dest_idx,
                                                e
                                            );
                                            return Err(e);
                                        }
                                    }
                                }
                                Ok(())
                            })
                            .expect("Partitioning failed");

                        if blocked {
                            blocked_batch = Some(batch);
                            tokio::task::yield_now().await;
                        }
                    }
                }
            }
            ExchangeMode::Gather => {
                // In Gather mode, everyone sends everything to Node 0.
                while let Some(batch) = stream.next().await {
                    let batch = batch.expect("Input stream failed");
                    let mut blocked_batch = Some(batch);

                    while let Some(batch) = blocked_batch.take() {
                        match writers[0].write_batch(&batch) {
                            Ok(_) => {}
                            Err(datafusion::error::DataFusionError::IoError(ref msg))
                                if msg.kind() == ErrorKind::WouldBlock =>
                            {
                                blocked_batch = Some(batch);
                                tokio::task::yield_now().await;
                            }
                            Err(datafusion::error::DataFusionError::IoError(ref msg))
                                if msg.kind() == ErrorKind::BrokenPipe =>
                            {
                                return; // Graceful exit
                            }
                            Err(e) => {
                                pgrx::warning!(
                                    "[PID {}] producer_task (Gather) ERROR: {}",
                                    std::process::id(),
                                    e
                                );
                                panic!("Gather failed: {}", e);
                            }
                        }
                    }
                }
            }
        }

        for writer in writers {
            writer.finish().expect("Failed to finish writer");
        }
    }
}

impl DisplayAs for DsmWriterExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut Formatter) -> std::fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                write!(
                    f,
                    "DsmWriterExec(stream_id={}, partitioning={:?})",
                    self.config.stream_id, self.partitioning
                )
            }
            _ => Ok(()),
        }
    }
}

impl ExecutionPlan for DsmWriterExec {
    fn name(&self) -> &str {
        "DsmWriterExec"
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
            self.partitioning.clone(),
            self.config.clone(),
        )?))
    }

    fn execute(
        &self,
        partition: usize,
        _context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        assert_eq!(partition, 0);
        // In the lazy execution model, execution is triggered via the registry/control channel,
        // not by DataFusion calling execute() on this node (except when we manually trigger it).
        // However, if we are manually triggering it via producer_task, we don't call this method.
        Ok(Box::pin(RecordBatchStreamAdapter::new(
            self.input.schema(),
            futures::stream::empty(),
        )))
    }
}

/// A physical operator that handles consuming shuffled data.
///
/// `DsmReaderExec` acts as the **RPC Client**. When it executes, it initiates the
/// data stream by sending a `StartStream` control message to the corresponding Writer
/// (which may be on a remote node or the same node).
///
/// This "Pull" triggers the execution of the upstream plan fragment.
#[derive(Debug)]
pub struct DsmReaderExec {
    input: Arc<dyn ExecutionPlan>,
    config: DsmExchangeConfig,
    properties: PlanProperties,
}

impl DsmReaderExec {
    pub fn try_new(
        input: Arc<dyn ExecutionPlan>,
        config: DsmExchangeConfig,
        partitioning: Partitioning,
    ) -> Result<Self> {
        let properties = PlanProperties::new(
            EquivalenceProperties::new(input.schema()),
            partitioning,
            EmissionType::Incremental,
            Boundedness::Bounded,
        );

        Ok(Self {
            input,
            config,
            properties,
        })
    }

    fn create_consumer_stream(
        &self,
        partition: usize,
        bridge: Arc<SignalBridge>,
    ) -> Result<SendableRecordBatchStream> {
        // Return the consumer side stream for THIS partition.
        match self.config.mode {
            ExchangeMode::Redistribute => {
                // In Redistribute mode, each node k is responsible for partition k.
                if partition != self.config.participant_index {
                    // We are a worker/leader waiting for a background task (that we just spawned via registry)
                    // to finish producing data for a different partition that we are NOT consuming.
                    // But wait, DsmReaderExec is executed for *output* partitions.
                    // If we are executing partition `p`, we are the consumer for `p`.
                    // If `p != participant_index`, then we are being asked to produce a partition
                    // that belongs to someone else? No, DataFusion only asks us to produce our own partition usually.
                    // BUT, if the plan is distributed, we might have a different arrangement.
                    // In the current JoinScan logic, "Partition K is on Node K".
                    // So if we are Node K, we only ever execute Partition K.
                    // If DataFusion asks us to execute Partition J (!= K), it's a bug or a misconfiguration.

                    // However, the original code had:
                    // if partition != self.config.participant_index { return Ok(wait_for_producer_stream(...)); }
                    // This implies we DO get asked to execute other partitions, or it's a safeguard.
                    // AND: In Gather mode, Workers execute Partition 0 (which belongs to Leader) to run the side-effects (producer).

                    // In the NEW design:
                    // The "Listener" Loop spawns the tasks.
                    // The `DsmReaderExec::execute` is ONLY called if we are the consumer.
                    // If we are a Worker in Gather mode, we are NOT the consumer.
                    // BUT, `ParallelJoin` (the DataFusion wrapper) might iterate all partitions?
                    // No, `ParallelJoin::run` executes `physical_plan.execute(0, ...)`.
                    // For a Worker, `execute(0)` calls `DsmReaderExec::execute(0)`.
                    // `DsmReaderExec` for Gather has 1 partition.
                    // So the Worker calls `execute(0)`.
                    // In Gather mode, Worker is Node J. Consumer is Node 0.
                    // So `participant_index != 0`.
                    // So we enter the `wait_for_producer` block.

                    // We need to fetch the completion receiver for the LOCAL stream that corresponds to this.
                    // The Worker is producing data for Stream `config.stream_id`.
                    // The Physical ID is `(stream_id << 16) | participant_index`.
                    // This matches the task we expect to be running (triggered by the Leader's StartStream request).
                    let physical_id = (self.config.stream_id << 16)
                        | ((self.config.participant_index as u32) & 0xFFFF);
                    if let Some(rx) = get_completion_receiver(physical_id) {
                        return Ok(wait_for_producer_stream(self.input.schema(), rx));
                    } else {
                        // Should not happen if registry is set up correct?
                        // Or maybe the request hasn't arrived yet?
                        // If the request hasn't arrived, `get_completion_receiver` creates the channel.
                        // So we wait.
                        // BUT: `trigger_stream` might not have been called yet.
                        // The task isn't running. We wait on the channel.
                        // When `trigger_stream` is eventually called (by Leader sending request),
                        // the task will start, then finish, then send `true` to channel.
                        // So this logic holds.
                        let mut guard = DSM_MESH.lock();
                        let mesh = guard.as_mut().expect("DSM mesh not registered");
                        let mut registry = mesh.registry.lock();
                        let (tx, rx) = watch::channel(false);
                        registry.completions.insert(physical_id, tx);
                        return Ok(wait_for_producer_stream(self.input.schema(), rx));
                    }
                }

                let schema = self.input.schema();
                let mut readers = Vec::new();
                for i in 0..self.config.total_participants {
                    if let Some(reader) =
                        get_dsm_reader(i, self.config.stream_id, i, schema.clone())
                    {
                        readers.push(reader);
                    }
                }

                let streams: Vec<SendableRecordBatchStream> = readers
                    .into_iter()
                    .map(|r| shared_memory_stream(r, bridge.clone()))
                    .collect();

                Ok(Box::pin(RecordBatchStreamAdapter::new(
                    schema.clone(),
                    futures::stream::select_all(streams),
                )))
            }
            ExchangeMode::Gather => {
                // In Gather mode, only the Leader (node 0) consumes.
                // It consumes partition i from Node i.
                if self.config.participant_index != 0 {
                    // Worker waiting for its local producer (triggered by leader) to finish.
                    let physical_id = (self.config.stream_id << 16)
                        | ((self.config.participant_index as u32) & 0xFFFF);

                    // Helper to get-or-create rx
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
                    return Ok(wait_for_producer_stream(self.input.schema(), rx));
                }

                if partition != 0 {
                    // Gather only has 1 partition (partition 0)
                    return Ok(Box::pin(RecordBatchStreamAdapter::new(
                        self.input.schema(),
                        futures::stream::empty(),
                    )));
                }

                let schema = self.input.schema();
                let mut readers = Vec::new();
                for i in 0..self.config.total_participants {
                    if let Some(reader) =
                        get_dsm_reader(i, self.config.stream_id, i, schema.clone())
                    {
                        readers.push(reader);
                    }
                }

                let streams: Vec<SendableRecordBatchStream> = readers
                    .into_iter()
                    .map(|r| shared_memory_stream(r, bridge.clone()))
                    .collect();

                Ok(Box::pin(RecordBatchStreamAdapter::new(
                    schema.clone(),
                    futures::stream::select_all(streams),
                )))
            }
        }
    }
}

impl DisplayAs for DsmReaderExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut Formatter) -> std::fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                write!(f, "DsmReaderExec(stream_id={})", self.config.stream_id)
            }
            _ => Ok(()),
        }
    }
}

impl ExecutionPlan for DsmReaderExec {
    fn name(&self) -> &str {
        "DsmReaderExec"
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
            self.config.clone(),
            self.properties.output_partitioning().clone(),
        )?))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let _ = self.input.execute(0, context.clone())?;
        let bridge = get_dsm_bridge();

        self.create_consumer_stream(partition, bridge)
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
fn shared_memory_stream(
    mut reader: SendableRecordBatchStream,
    bridge: Arc<SignalBridge>,
) -> SendableRecordBatchStream {
    let schema = reader.schema();
    let stream = try_stream! {
        loop {
            let item = futures::future::poll_fn(|cx| {
                match reader.as_mut().poll_next(cx) {
                    Poll::Pending => {
                        bridge.register_waker(cx.waker().clone());
                        Poll::Pending
                    }
                    r => r,
                }
            })
            .await;

            match item {
                Some(Ok(batch)) => yield batch,
                Some(Err(e)) => Err(e)?,
                None => break,
            }
        }
    };
    Box::pin(RecordBatchStreamAdapter::new(schema, Box::pin(stream)))
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use arrow_schema::{DataType, Field, Schema};
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
            participant_index: 0,
            total_participants: 2,
        };
        let optimized = rule
            .optimize(repartition, &ConfigOptions::default())
            .unwrap();

        assert!(optimized.as_any().is::<DsmReaderExec>());
        assert_eq!(optimized.children().len(), 1);
        assert!(optimized.children()[0].as_any().is::<DsmWriterExec>());
    }
}

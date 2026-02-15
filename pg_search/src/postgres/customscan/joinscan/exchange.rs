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

use crate::postgres::customscan::joinscan::dsm_transfer::{
    dsm_shared_memory_reader, DsmSharedMemoryWriter, MultiplexedDsmReader, MultiplexedDsmWriter,
    SignalBridge,
};
use crate::scan::table_provider::MppParticipantConfig;

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

pub fn register_stream_source(source: StreamSource, participant_index: usize) {
    let mut guard = DSM_MESH.lock();
    if let Some(mesh) = guard.as_mut() {
        // Calculate physical ID: (Logical << 16) | Sender (us)
        // participant_index is "us" (the sender).
        let physical_id = PhysicalStreamId::new(source.config.stream_id, participant_index);

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

        // Prepare completion notifier. Create it if it doesn't exist (DsmReaderExec hasn't run yet).
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
            DsmWriterExec::producer_task(input, partitioning, config, context).await;
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

use crate::postgres::customscan::joinscan::dsm_stream::ControlMessage;
use crate::postgres::customscan::joinscan::dsm_stream::{LogicalStreamId, PhysicalStreamId};
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
    stream_id: LogicalStreamId,
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
                schema,
            ));
        }
    }
    None
}

pub fn get_dsm_reader(
    participant: usize,
    stream_id: LogicalStreamId,
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
    pub total_participants: usize,
}

impl EnforceDsmShuffle {
    fn wrap_in_dsm_exchange(
        &self,
        input: Arc<dyn ExecutionPlan>,
        partitioning: Partitioning,
        stream_id: u16,
        mode: ExchangeMode,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let config = DsmExchangeConfig {
            stream_id: LogicalStreamId(stream_id),
            total_participants: self.total_participants,
            mode,
        };

        let writer = Arc::new(DsmWriterExec::try_new(
            input,
            partitioning.clone(),
            config.clone(),
        )?);
        let reader = DsmReaderExec::try_new(writer, config, partitioning)?;

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
                let partitioning = repartition.partitioning().clone();
                let (mode, reader_partitioning) =
                    if matches!(partitioning, Partitioning::UnknownPartitioning(1)) {
                        (
                            ExchangeMode::Gather,
                            Partitioning::UnknownPartitioning(rule.total_participants),
                        )
                    } else {
                        (ExchangeMode::Redistribute, partitioning.clone())
                    };

                let stream_id = *counter;
                *counter = counter.checked_add(1).ok_or_else(|| {
                    datafusion::common::DataFusionError::Internal(
                        "Too many shuffle stages (max 65535)".to_string(),
                    )
                })?;

                let input = repartition.children()[0].clone();
                rule.wrap_in_dsm_exchange(input, reader_partitioning, stream_id, mode)
            } else if let Some(merge) = node.as_any().downcast_ref::<SortPreservingMergeExec>() {
                let input = merge.children()[0].clone();
                if input.output_partitioning().partition_count() > 1
                    && !input.as_any().is::<DsmReaderExec>()
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
                    && !input.as_any().is::<DsmReaderExec>()
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
}

pub(crate) fn get_mpp_config(ctx: &TaskContext) -> (usize, usize) {
    ctx.session_config()
        .options()
        .extensions
        .get::<MppParticipantConfig>()
        .map(|c| (c.index, c.total_participants))
        .unwrap_or((0, 1))
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
    pub input: Arc<dyn ExecutionPlan>,
    pub partitioning: Partitioning,
    pub config: DsmExchangeConfig,
    pub properties: PlanProperties,
}

impl DsmWriterExec {
    pub fn try_new(
        input: Arc<dyn ExecutionPlan>,
        partitioning: Partitioning,
        config: DsmExchangeConfig,
    ) -> Result<Self> {
        let properties = PlanProperties::new(
            input.equivalence_properties().clone(),
            partitioning.clone(),
            EmissionType::Incremental,
            input.boundedness(),
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
        let (participant_index, total_participants) = get_mpp_config(&context);
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
                get_dsm_writer(i, config.stream_id, participant_index, schema.clone())
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
                                bridge.register_waker(cx.waker().clone());

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
                    bridge.register_waker(cx.waker().clone());
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
        // In the RPC-server model, workers do not eagerly execute the plan tree.
        // DsmWriterExec is a passive marker in the plan.
        // Execution is triggered by StartStream requests and handled in producer_task.
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
    pub input: Arc<dyn ExecutionPlan>,
    pub config: DsmExchangeConfig,
    pub properties: PlanProperties,
}

impl DsmReaderExec {
    pub fn try_new(
        input: Arc<dyn ExecutionPlan>,
        config: DsmExchangeConfig,
        partitioning: Partitioning,
    ) -> Result<Self> {
        let properties = PlanProperties::new(
            input.equivalence_properties().clone(),
            Partitioning::UnknownPartitioning(partitioning.partition_count()),
            EmissionType::Incremental,
            input.boundedness(),
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
        context: Arc<TaskContext>,
        bridge: Arc<SignalBridge>,
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
                    if let Some(reader) =
                        get_dsm_reader(i, self.config.stream_id, i, schema.clone())
                    {
                        readers.push(shared_memory_stream(reader, bridge.clone()));
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
                        let physical_id =
                            PhysicalStreamId::new(self.config.stream_id, participant_index);

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

                if let Some(reader) =
                    get_dsm_reader(partition, self.config.stream_id, partition, schema.clone())
                {
                    Ok(shared_memory_stream(reader, bridge))
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

        self.create_consumer_stream(partition, context, bridge)
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

        assert!(optimized.as_any().is::<DsmReaderExec>());
        assert_eq!(optimized.children().len(), 1);
        assert!(optimized.children()[0].as_any().is::<DsmWriterExec>());
    }
}

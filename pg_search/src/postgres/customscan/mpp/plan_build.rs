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

//! MPP physical-plan primitives.
//!
//! [`wrap_with_mpp_shuffle`] wraps a child plan in an [`MppShuffleExec`].
//! For a representative scalar-agg-over-join query, the per-worker plan is:
//!
//! ```text
//!     AggregateExec(Partial, COUNT(*))            ← emits one partial row per worker
//!       HashJoinExec(PartitionMode::Partitioned)
//!         wrap_with_mpp_shuffle([Scan f → Filter], hash=[f.id])
//!         wrap_with_mpp_shuffle([Scan p         ], hash=[p.fileId])
//! ```
//!
//! Each wrap consumes one directed mesh edge — a binary hash-join needs two
//! independent meshes (one per side). PG's `Gather` + `Finalize Aggregate`
//! handle the cross-worker reduction.

use std::sync::Arc;

use datafusion::arrow::datatypes::SchemaRef;
use datafusion::common::{DataFusionError, Result};
use datafusion::physical_expr::PhysicalExpr;
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::{ExecutionPlan, ExecutionPlanProperties};

use crate::postgres::customscan::mpp::shuffle::{MppShuffleExec, ShuffleWiring};
use crate::postgres::customscan::mpp::transport::DrainHandle;
use datafusion_distributed::{NetworkBoundary, Stage};

/// Inputs to [`wrap_with_mpp_shuffle`].
pub struct MppShuffleInputs {
    /// Plan whose output rows the shuffle consumes.
    pub child: Arc<dyn ExecutionPlan>,
    /// Outbound half of this participant's mesh edge.
    pub wiring: ShuffleWiring,
    /// Inbound half (peer-shipped rows).
    pub drain_handle: Arc<DrainHandle>,
    /// Schema both `child.schema()` and peer Arrow IPC batches resolve to.
    pub wrapped_schema: SchemaRef,
    /// Diagnostic label used in `mpp_log!` row-count reports.
    pub tag: &'static str,
    /// Stage descriptor stamped onto the emitted [`MppShuffleExec`].
    pub stage: Option<Stage>,
    /// `Some(keys)` → `Partitioning::Hash(keys, N)`; `None` →
    /// `UnknownPartitioning(1)` (scalar-final gather).
    pub hash_keys: Option<Vec<Arc<dyn PhysicalExpr>>>,
    /// Output partition that does the producer + drain work on this
    /// participant. Hash shuffles: `participant_index`. Fixed-target:
    /// the target on every participant.
    pub drive_partition: u32,
}

/// Wrap `child` in a single [`MppShuffleExec`] that consumes both mesh
/// halves and merges the producer + drain streams on `drive_partition`.
/// Other partitions return empty. See [`MppShuffleInputs`] for fields.
pub fn wrap_with_mpp_shuffle(
    inputs: MppShuffleInputs,
) -> Result<Arc<dyn ExecutionPlan>, DataFusionError> {
    let MppShuffleInputs {
        child,
        wiring,
        drain_handle,
        wrapped_schema,
        tag,
        stage,
        hash_keys,
        drive_partition,
    } = inputs;
    let _ = wrapped_schema; // Schema is inferred from `child.schema()`.

    // Coalesce multi-partition children: the shuffle producer only consumes
    // partition 0, so without this every partition beyond the first is
    // silently dropped — produces correct group counts but wrong per-group
    // aggregates.
    let child: Arc<dyn ExecutionPlan> = if child.output_partitioning().partition_count() > 1 {
        Arc::new(CoalescePartitionsExec::new(child))
    } else {
        child
    };

    let shuffle_node =
        MppShuffleExec::new(child, wiring, drain_handle, hash_keys, drive_partition, tag);
    match stage {
        Some(s) => NetworkBoundary::with_input_stage(&shuffle_node, s),
        None => Ok(Arc::new(shuffle_node)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::customscan::mpp::shuffle::tests::ModuloPartitioner;
    use crate::postgres::customscan::mpp::transport::{
        in_proc_channel, DrainBuffer, DrainConfig, DrainItem, MppReceiver, MppSender,
    };
    use datafusion::arrow::array::{Int32Array, RecordBatch, StringArray};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::datasource::memory::MemorySourceConfig;
    use datafusion::physical_plan::ExecutionPlanProperties;
    use datafusion::prelude::SessionContext;
    use futures::StreamExt;
    use std::thread;
    use std::time::Duration;

    fn sample_batch(rows: i32) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]));
        let ids = Int32Array::from_iter_values(0..rows);
        let names = StringArray::from_iter_values((0..rows).map(|i| format!("n{i}")));
        RecordBatch::try_new(schema, vec![Arc::new(ids), Arc::new(names)]).unwrap()
    }

    /// Drive a two-participant mesh in-process: our side shuffles a 10-row
    /// input at participant 0 with `ModuloPartitioner(2)`, a simulated peer ships
    /// synthetic batches into our inbound drain. The wrapped stream should
    /// emit:
    ///
    /// - self-partition rows (even IDs: 0,2,4,6,8)
    /// - peer-partition rows (synthetic IDs 100,200 from the peer)
    ///
    /// and the outbound channel should carry the odd IDs that ShuffleExec
    /// shipped away (1,3,5,7,9).
    #[test]
    fn wrap_with_mpp_shuffle_splices_self_and_peer() {
        let batch = sample_batch(10);
        let schema = batch.schema();
        let input = MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch]).unwrap();

        let (out_tx, out_rx) = in_proc_channel(16);
        let (in_tx, in_rx) = in_proc_channel(16);

        let wiring = ShuffleWiring {
            partitioner: Arc::new(ModuloPartitioner::new(2)),
            outbound_senders: vec![None, Some(MppSender::new(Box::new(out_tx)))],
            participant_index: 0,
            cooperative_drain: None,
        };

        // Simulated peer: ship two synthetic batches with a small delay so
        // the drain thread actually waits on the waker path at least once.
        let peer_schema = schema.clone();
        let peer_thread = thread::spawn(move || {
            let sender = MppSender::new(Box::new(in_tx));
            for (id, name) in [(100i32, "peer100"), (200i32, "peer200")] {
                thread::sleep(Duration::from_millis(10));
                let b = RecordBatch::try_new(
                    peer_schema.clone(),
                    vec![
                        Arc::new(Int32Array::from_iter_values([id])),
                        Arc::new(StringArray::from_iter_values([name.to_string()])),
                    ],
                )
                .unwrap();
                sender.send_batch(&b).unwrap();
            }
        });

        let inbound_recv = MppReceiver::new(Box::new(in_rx));
        let drain_buffer = DrainBuffer::new(1);
        let drain_handle = DrainHandle::spawn(DrainConfig::new(
            vec![inbound_recv],
            Arc::clone(&drain_buffer),
        ));

        let wrapped = wrap_with_mpp_shuffle(MppShuffleInputs {
            child: input,
            wiring,
            drain_handle: Arc::new(drain_handle),
            wrapped_schema: schema.clone(),
            tag: "test",
            stage: None,
            hash_keys: None,
            drive_partition: 0,
        })
        .unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ctx = SessionContext::new();

        let mut emitted_ids = Vec::new();
        rt.block_on(async {
            let n = wrapped.output_partitioning().partition_count();
            for p in 0..n {
                let mut s = wrapped.execute(p, ctx.task_ctx()).unwrap();
                while let Some(b) = s.next().await {
                    let b = b.unwrap();
                    let ids = b.column(0).as_any().downcast_ref::<Int32Array>().unwrap();
                    emitted_ids.extend(ids.values().iter().copied());
                }
            }
        });

        // Drain the outbound channel to verify the peer-destined rows
        // actually got shipped.
        let out_receiver = MppReceiver::new(Box::new(out_rx));
        let out_buffer = DrainBuffer::new(1);
        let out_handle = DrainHandle::spawn(DrainConfig::new(
            vec![out_receiver],
            Arc::clone(&out_buffer),
        ));
        let mut outbound_ids = Vec::new();
        while let DrainItem::Batch(b) = out_buffer.pop_front() {
            outbound_ids.extend(
                b.column(0)
                    .as_any()
                    .downcast_ref::<Int32Array>()
                    .unwrap()
                    .values()
                    .iter()
                    .copied(),
            );
        }
        out_handle.shutdown().unwrap();
        peer_thread.join().unwrap();

        // Union output: self-partition (even IDs) + peer-simulated (100, 200).
        emitted_ids.sort();
        assert_eq!(emitted_ids, vec![0, 2, 4, 6, 8, 100, 200]);
        // Outbound: ShuffleExec shipped odd IDs (rows that hashed to participant 1).
        outbound_ids.sort();
        assert_eq!(outbound_ids, vec![1, 3, 5, 7, 9]);
    }
}

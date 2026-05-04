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

#![allow(dead_code)]
//! Worker-side leaf operator for the coordinator/worker MPP architecture.
//!
//! [`ShmMqProducerExec`] drives its single child to completion, hash-partitions
//! every row to one of the leader's `n_partitions` consumer queues, and pushes
//! the resulting sub-batches through the senders found on the [`MppRpcMesh`]
//! attached to the [`TaskContext`]. It returns no rows of its own — workers
//! don't emit data back to PostgreSQL; all output flows through the `shm_mq`
//! mesh to the leader's `NetworkShuffleExec`.
//!
//! The producer half of what `MppShuffleExec` used to do in the peer-to-peer
//! mesh; PR3 retires the legacy operator once the walker emits this directly.

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use datafusion::arrow::datatypes::{Schema, SchemaRef};
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties,
};
use futures::StreamExt;

use crate::postgres::customscan::mpp::rpc_mesh::MppRpcMesh;
use crate::postgres::customscan::mpp::shuffle::{split_batch_by_partition, RowPartitioner};
use crate::postgres::customscan::mpp::transport::SendBatchStats;

/// Pulls all rows from `child`, hash-routes them into `n_partitions` consumer
/// queues on the leader, and emits zero rows back to its caller.
///
/// Construction is planning-time (carries `child`, `partitioner`, and
/// `n_partitions`); the [`MppRpcMesh`] holding the live senders is fetched
/// from [`TaskContext::session_config`] at execute time. Doing the lookup
/// late keeps the operator [`Clone`]-friendly and lets DataFusion's plan
/// rewriter pass it through `with_new_children` without the senders along
/// for the ride.
pub struct ShmMqProducerExec {
    child: Arc<dyn ExecutionPlan>,
    partitioner: Arc<dyn RowPartitioner>,
    n_partitions: usize,
    properties: Arc<PlanProperties>,
}

impl fmt::Debug for ShmMqProducerExec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShmMqProducerExec")
            .field("n_partitions", &self.n_partitions)
            .finish()
    }
}

impl ShmMqProducerExec {
    pub fn try_new(
        child: Arc<dyn ExecutionPlan>,
        partitioner: Arc<dyn RowPartitioner>,
        n_partitions: usize,
    ) -> Result<Self> {
        if n_partitions == 0 {
            return Err(DataFusionError::Internal(
                "ShmMqProducerExec: n_partitions must be > 0".into(),
            ));
        }
        if partitioner.total_participants() as usize != n_partitions {
            return Err(DataFusionError::Internal(format!(
                "ShmMqProducerExec: partitioner.total_participants={} != n_partitions={}",
                partitioner.total_participants(),
                n_partitions
            )));
        }
        // Output schema is empty — workers don't emit rows back to the planner.
        let schema: SchemaRef = Arc::new(Schema::empty());
        let properties = PlanProperties::new(
            EquivalenceProperties::new(schema),
            Partitioning::UnknownPartitioning(1),
            EmissionType::Final,
            Boundedness::Bounded,
        );
        Ok(Self {
            child,
            partitioner,
            n_partitions,
            properties: Arc::new(properties),
        })
    }
}

impl DisplayAs for ShmMqProducerExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ShmMqProducerExec: n_partitions={}", self.n_partitions)
    }
}

impl ExecutionPlan for ShmMqProducerExec {
    fn name(&self) -> &str {
        "ShmMqProducerExec"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn properties(&self) -> &Arc<PlanProperties> {
        &self.properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.child]
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        if children.len() != 1 {
            return Err(DataFusionError::Internal(format!(
                "ShmMqProducerExec: expected 1 child, got {}",
                children.len()
            )));
        }
        let mut child_iter = children.into_iter();
        let child = child_iter.next().expect("checked above");
        Ok(Arc::new(Self {
            child,
            partitioner: Arc::clone(&self.partitioner),
            n_partitions: self.n_partitions,
            properties: Arc::clone(&self.properties),
        }))
    }

    fn execute(
        &self,
        partition: usize,
        ctx: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        if partition != 0 {
            return Err(DataFusionError::Internal(format!(
                "ShmMqProducerExec emits a single empty partition; got partition={partition}"
            )));
        }
        let mesh = ctx
            .session_config()
            .get_extension::<MppRpcMesh>()
            .ok_or_else(|| {
                DataFusionError::Internal(
                    "ShmMqProducerExec: MppRpcMesh missing from session extensions; \
                     the customscan must install it before plan execution"
                        .into(),
                )
            })?;
        if mesh.outbound_senders.len() != self.n_partitions {
            return Err(DataFusionError::Internal(format!(
                "ShmMqProducerExec: mesh.outbound_senders.len()={} != n_partitions={}",
                mesh.outbound_senders.len(),
                self.n_partitions
            )));
        }

        let mut child_stream = self.child.execute(0, Arc::clone(&ctx))?;
        let partitioner = Arc::clone(&self.partitioner);
        let n_partitions = self.n_partitions;
        let mesh = Arc::clone(&mesh);

        let stream = async_stream::stream! {
            let mut send_stats = SendBatchStats::default();
            while let Some(batch_result) = child_stream.next().await {
                let batch = batch_result?;
                if batch.num_rows() == 0 {
                    continue;
                }
                let dests = partitioner.partition_for_each_row(&batch)?;
                let subs = split_batch_by_partition(&batch, &dests, n_partitions as u32)?;
                for (partition_idx, sub) in subs.into_iter().enumerate() {
                    let Some(sub) = sub else { continue };
                    let sender = mesh.outbound_sender(partition_idx).ok_or_else(|| {
                        DataFusionError::Internal(format!(
                            "ShmMqProducerExec: no sender for partition {partition_idx}"
                        ))
                    })?;
                    sender.send_batch_traced(&sub, &mut send_stats).await?;
                }
            }
            // Workers emit zero rows; drop child stream so peers observe EOF.
            drop(child_stream);
            // The unreachable yield ties the generator's item type to
            // Result<RecordBatch>; control flow exits through the `while`
            // loop's normal end above.
            if false {
                yield Err::<datafusion::arrow::array::RecordBatch, DataFusionError>(
                    DataFusionError::Internal("unreachable".into()),
                );
            }
        };

        Ok(Box::pin(RecordBatchStreamAdapter::new(
            self.schema(),
            stream,
        )))
    }
}

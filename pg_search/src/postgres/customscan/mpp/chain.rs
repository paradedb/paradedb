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

//! Single-partition sequential chain of two `ExecutionPlan` children.
//!
//! The MPP shuffle topology needs to merge a `ShuffleExec` and a
//! `DrainGatherExec` into a single output stream per participant:
//!
//! - `ShuffleExec` drives the local scan, hash-partitions, and ships peer-
//!   bound rows via `MppSender`. Its output stream is the self-partition.
//! - `DrainGatherExec` reads peer-shipped rows out of the shared
//!   `DrainBuffer` (populated asynchronously by the drain thread).
//!
//! `UnionExec` exposes these as two partitions; downstream operators that
//! only drive `execute(0)` would leave peer rows unread. `CoalescePartitionsExec`
//! merges them into one partition but spawns a `tokio::spawn` task per input,
//! which deadlocks when both tasks block in blocking `shm_mq_send` calls
//! (not enough async yields to unblock).
//!
//! [`ChainExec`] avoids both problems by producing a single partition that
//! polls the first child to exhaustion, then the second. Correctness relies
//! on two structural guarantees:
//!
//! 1. The drain thread reads peer shipments into `DrainBuffer` on its own
//!    `std::thread`, independent of operator polling order. Peer rows pile
//!    up while we're still polling the shuffle side.
//! 2. Peer `ShuffleExec`s drop their outbound senders as soon as their
//!    local scan exhausts, regardless of whether we're actively reading.
//!    EOF on the drain therefore fires based on peer progress, not our own.
//!
//! Both guarantees hold because the drain thread + shm_mq lifecycle is
//! driven by the participants' scan completions, not by operator-level poll
//! cadence. See `plan_build::wrap_with_mpp_shuffle` for the full reasoning.

#![allow(dead_code)] // wired in via wrap_with_mpp_shuffle.

use std::any::Any;
use std::sync::Arc;
use std::task::{Context, Poll};

use datafusion::arrow::datatypes::SchemaRef;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties,
};
use futures::StreamExt;

/// Two-child sequential merge: poll `first` to exhaustion, then `second`.
/// Output has exactly one partition; both children must have exactly one
/// partition and share an identical schema.
#[derive(Debug)]
pub struct ChainExec {
    first: Arc<dyn ExecutionPlan>,
    second: Arc<dyn ExecutionPlan>,
    schema: SchemaRef,
    plan_properties: Arc<PlanProperties>,
}

impl ChainExec {
    /// Build a `ChainExec`. Returns `Err` if the children don't share a
    /// schema or don't each have exactly one output partition.
    pub fn try_new(
        first: Arc<dyn ExecutionPlan>,
        second: Arc<dyn ExecutionPlan>,
        schema: SchemaRef,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        if first.schema() != schema {
            return Err(DataFusionError::Plan(
                "ChainExec: first child's schema does not match declared schema".into(),
            ));
        }
        if second.schema() != schema {
            return Err(DataFusionError::Plan(
                "ChainExec: second child's schema does not match declared schema".into(),
            ));
        }
        let eq = EquivalenceProperties::new(schema.clone());
        let plan_properties = Arc::new(PlanProperties::new(
            eq,
            Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        ));
        Ok(Arc::new(Self {
            first,
            second,
            schema,
            plan_properties,
        }))
    }
}

impl DisplayAs for ChainExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChainExec")
    }
}

impl ExecutionPlan for ChainExec {
    fn name(&self) -> &str {
        "ChainExec"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn properties(&self) -> &Arc<PlanProperties> {
        &self.plan_properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.first, &self.second]
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        if children.len() != 2 {
            return Err(DataFusionError::Plan(format!(
                "ChainExec: expected 2 children, got {}",
                children.len()
            )));
        }
        let mut iter = children.into_iter();
        let first = iter.next().unwrap();
        let second = iter.next().unwrap();
        ChainExec::try_new(first, second, self.schema.clone())
    }

    fn execute(
        &self,
        partition: usize,
        ctx: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        if partition != 0 {
            return Err(DataFusionError::Plan(format!(
                "ChainExec: requested partition {partition} but only partition 0 exists"
            )));
        }
        let first = self.first.execute(0, Arc::clone(&ctx))?;
        let second = self.second.execute(0, ctx)?;
        let merged = ChainStream {
            first: Some(first),
            second: Some(second),
        };
        Ok(Box::pin(RecordBatchStreamAdapter::new(
            self.schema.clone(),
            merged,
        )))
    }
}

struct ChainStream {
    first: Option<SendableRecordBatchStream>,
    second: Option<SendableRecordBatchStream>,
}

impl futures::Stream for ChainStream {
    type Item = Result<datafusion::arrow::array::RecordBatch>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        loop {
            if let Some(s) = self.first.as_mut() {
                match s.poll_next_unpin(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Some(item)) => return Poll::Ready(Some(item)),
                    Poll::Ready(None) => {
                        // First child exhausted — drop it and start polling
                        // the second on the next loop iteration.
                        self.first = None;
                        continue;
                    }
                }
            }
            if let Some(s) = self.second.as_mut() {
                match s.poll_next_unpin(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Some(item)) => return Poll::Ready(Some(item)),
                    Poll::Ready(None) => {
                        self.second = None;
                        return Poll::Ready(None);
                    }
                }
            }
            return Poll::Ready(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::array::{Int32Array, RecordBatch};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::datasource::memory::MemorySourceConfig;
    use datafusion::prelude::SessionContext;

    fn batch(vals: &[i32]) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        RecordBatch::try_new(schema, vec![Arc::new(Int32Array::from(vals.to_vec()))]).unwrap()
    }

    #[test]
    fn chain_emits_first_then_second() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let first =
            MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch(&[1, 2])]).unwrap();
        let second =
            MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch(&[10, 20])])
                .unwrap();
        let chain = ChainExec::try_new(first, second, schema.clone()).unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ctx = SessionContext::new();
        let mut emitted = Vec::new();
        rt.block_on(async {
            let mut s = chain.execute(0, ctx.task_ctx()).unwrap();
            while let Some(b) = s.next().await {
                let b = b.unwrap();
                let ids = b.column(0).as_any().downcast_ref::<Int32Array>().unwrap();
                emitted.extend(ids.values().iter().copied());
            }
        });
        assert_eq!(emitted, vec![1, 2, 10, 20]);
    }

    #[test]
    fn chain_rejects_schema_mismatch() {
        let schema_a = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let schema_b = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));
        let first =
            MemorySourceConfig::try_new_from_batches(schema_a.clone(), vec![batch(&[1])]).unwrap();
        let second_b = RecordBatch::try_new(
            schema_b.clone(),
            vec![Arc::new(datafusion::arrow::array::Int64Array::from(vec![
                2i64,
            ]))],
        )
        .unwrap();
        let second = MemorySourceConfig::try_new_from_batches(schema_b, vec![second_b]).unwrap();
        let err = ChainExec::try_new(first, second, schema_a).unwrap_err();
        assert!(format!("{err}").contains("schema"), "unexpected: {err}");
    }

    #[test]
    fn chain_reports_single_partition() {
        let schema = Arc::new(Schema::new(vec![Field::new("id", DataType::Int32, false)]));
        let first =
            MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch(&[1])]).unwrap();
        let second =
            MemorySourceConfig::try_new_from_batches(schema.clone(), vec![batch(&[2])]).unwrap();
        let chain = ChainExec::try_new(first, second, schema).unwrap();
        use datafusion::physical_plan::ExecutionPlanProperties;
        assert_eq!(chain.output_partitioning().partition_count(), 1);
    }
}

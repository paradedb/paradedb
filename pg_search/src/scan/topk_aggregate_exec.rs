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

//! Fused TopK selection execution plan for aggregate-on-join queries.
//!
//! Replaces `SortExec → GlobalLimitExec` (or `SortExec(fetch=K)`) with a
//! single node that consumes all aggregate output and emits only the top-K
//! rows using partial sort, avoiding a full sort of all rows.
//!
//! # Plan Transformation
//!
//! ```text
//! BEFORE:
//!   [GlobalLimitExec(fetch=K)]
//!     SortExec(sort=[agg_col DESC])
//!       AggregateExec(...)
//!
//! AFTER:
//!   TopKAggregateExec(k=K, sort=[agg_col DESC])
//!     AggregateExec(...)
//! ```

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use arrow_array::{RecordBatch, UInt32Array};
use arrow_schema::SchemaRef;
use datafusion::arrow::row::{RowConverter, SortField};
use datafusion::common::Result;
use datafusion::execution::TaskContext;
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_expr::{LexOrdering, PhysicalSortExpr};
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::metrics::{ExecutionPlanMetricsSet, MetricBuilder};
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties,
    SendableRecordBatchStream,
};

use crate::scan::execution_plan::UnsafeSendStream;

/// Fused TopK selection for aggregate output.
///
/// Consumes all input batches, selects the top-K rows using partial sort
/// (via `select_nth_unstable_by` + sort of the K winners), and emits them
/// in sorted order as a single RecordBatch.
///
/// # TODO: streaming BinaryHeap approach
///
/// The current implementation materializes all input into a single RecordBatch
/// before selecting the top-K rows, holding O(N) memory. For high-cardinality
/// GROUP BY queries with small K, a streaming `BinaryHeap<K>` that processes
/// rows batch-by-batch would reduce memory to O(K) and time to O(N log K).
/// This is acceptable while the DataFusion TopK path is disabled (see #4493),
/// but should be revisited before enabling it for production workloads.
#[derive(Debug)]
pub struct TopKAggregateExec {
    input: Arc<dyn ExecutionPlan>,
    sort_exprs: LexOrdering,
    k: usize,
    schema: SchemaRef,
    properties: Arc<PlanProperties>,
    metrics: ExecutionPlanMetricsSet,
}

impl TopKAggregateExec {
    pub fn new(input: Arc<dyn ExecutionPlan>, sort_exprs: LexOrdering, k: usize) -> Self {
        let schema = input.schema();

        // Build properties that reflect the actual output: sorted by sort_exprs,
        // single partition, and final emission (all rows emitted in one batch).
        let mut eq_props = EquivalenceProperties::new(schema.clone());
        eq_props.add_ordering(sort_exprs.clone());
        let properties = Arc::new(PlanProperties::new(
            eq_props,
            Partitioning::UnknownPartitioning(1),
            EmissionType::Final,
            Boundedness::Bounded,
        ));

        Self {
            input,
            sort_exprs,
            k,
            schema,
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
        }
    }

    #[allow(dead_code)]
    pub fn k(&self) -> usize {
        self.k
    }

    #[allow(dead_code)]
    pub fn sort_exprs(&self) -> &[PhysicalSortExpr] {
        &self.sort_exprs
    }
}

impl DisplayAs for TopKAggregateExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut fmt::Formatter) -> fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                let sort_strs: Vec<String> =
                    self.sort_exprs.iter().map(|e| e.to_string()).collect();
                write!(
                    f,
                    "TopKAggregateExec: k={}, sort=[{}]",
                    self.k,
                    sort_strs.join(", ")
                )
            }
            _ => {
                write!(f, "TopKAggregateExec: k={}", self.k)
            }
        }
    }
}

impl ExecutionPlan for TopKAggregateExec {
    fn name(&self) -> &str {
        "TopKAggregateExec"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.input]
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        if children.len() != 1 {
            return Err(datafusion::common::DataFusionError::Internal(format!(
                "TopKAggregateExec expects 1 child, got {}",
                children.len()
            )));
        }
        Ok(Arc::new(TopKAggregateExec::new(
            children[0].clone(),
            self.sort_exprs.clone(),
            self.k,
        )))
    }

    fn properties(&self) -> &Arc<PlanProperties> {
        &self.properties
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let mut input_stream = self.input.execute(partition, context)?;
        let schema = self.schema();
        let sort_exprs = self.sort_exprs.clone();
        let k = self.k;
        let elapsed_compute = MetricBuilder::new(&self.metrics).elapsed_compute(partition);

        let stream = async_stream::try_stream! {
            use futures::StreamExt;

            let timer = elapsed_compute.timer();

            // Collect all input batches
            let mut batches: Vec<RecordBatch> = Vec::new();
            while let Some(batch) = input_stream.next().await {
                let batch = batch?;
                if batch.num_rows() > 0 {
                    batches.push(batch);
                }
            }

            if batches.is_empty() {
                timer.done();
                return;
            }

            // Concatenate into a single batch and drop originals to free memory
            let combined = arrow_select::concat::concat_batches(&schema, &batches)
                .map_err(|e| datafusion::common::DataFusionError::ArrowError(Box::new(e), None))?;
            drop(batches);

            let total_rows = combined.num_rows();
            if total_rows == 0 {
                timer.done();
                return;
            }

            let effective_k = k.min(total_rows);

            // Build RowConverter for sort comparison.
            // SortField options encode ASC/DESC and NULLS ordering so that
            // the natural byte ordering of converted rows matches the
            // desired sort order.
            let sort_fields: Vec<SortField> = sort_exprs
                .iter()
                .map(|e| {
                    let dt = e.expr.data_type(&schema)?;
                    Ok(SortField::new_with_options(dt, e.options))
                })
                .collect::<Result<_>>()?;

            let converter = RowConverter::new(sort_fields)
                .map_err(|e| datafusion::common::DataFusionError::ArrowError(Box::new(e), None))?;

            // Evaluate sort expressions on the combined batch
            let sort_arrays: Vec<arrow_array::ArrayRef> = sort_exprs
                .iter()
                .map(|e| {
                    e.expr
                        .evaluate(&combined)?
                        .into_array(total_rows)
                })
                .collect::<Result<_>>()?;

            let rows = converter
                .convert_columns(&sort_arrays)
                .map_err(|e| datafusion::common::DataFusionError::ArrowError(Box::new(e), None))?;

            // Select top-K indices using partial sort: O(N + K log K) instead of O(N log N)
            let mut indices: Vec<usize> = (0..total_rows).collect();
            if effective_k < total_rows {
                indices.select_nth_unstable_by(effective_k, |&a, &b| rows.row(a).cmp(&rows.row(b)));
                indices.truncate(effective_k);
            }
            indices.sort_by(|&a, &b| rows.row(a).cmp(&rows.row(b)));

            // Build output batch using arrow_select::take
            let index_array = UInt32Array::from(
                indices.iter().map(|&i| i as u32).collect::<Vec<_>>(),
            );

            let result = arrow_select::take::take_record_batch(&combined, &index_array)
                .map_err(|e| datafusion::common::DataFusionError::ArrowError(Box::new(e), None))?;

            timer.done();
            yield result;
        };

        let stream = unsafe { UnsafeSendStream::new(stream, self.schema()) };
        Ok(Box::pin(stream))
    }
}

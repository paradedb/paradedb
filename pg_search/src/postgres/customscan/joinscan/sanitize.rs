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

//! # Zero-Copy Sanitization
//!
//! This module implements a strategy to prevent deadlocks when feeding zero-copy
//! Shared Memory (DSM) buffers into DataFusion operators.
//!
//! ## The Problem
//!
//! DataFusion operators like `SortExec`, `JoinExec`, and `AggregateExec` can buffer
//! `RecordBatch`es or hold long-lived references (e.g., `StringViewArray`) to input data.
//! If that input data is a zero-copy view into a fixed-size Ring Buffer (DSM),
//! these operators can "pin" the Ring Buffer memory, preventing the producer from
//! reclaiming space and causing a deadlock.
//!
//! ## The Solution
//!
//! We implement a `DsmSanitizeExec` operator that performs a **Deep Copy** of passing batches,
//! effectively moving them from the Ring Buffer to the Heap.
//!
//! We then use `EnforceSanitization`, a physical optimizer rule, to inject this operator
//! immediately before any "Unsafe" (Blocking/Buffering) node, ensuring that long-lived
//! operators only ever hold Heap memory.

use std::any::Any;
use std::fmt::Formatter;
use std::sync::Arc;

use arrow_array::{Array, RecordBatch, UInt32Array};
use arrow_buffer::Buffer;
use datafusion::arrow::array::ArrayData;
use datafusion::arrow::compute::take;
use datafusion::common::Result;
use datafusion::config::ConfigOptions;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::coalesce_batches::CoalesceBatchesExec;
use datafusion::physical_plan::coalesce_partitions::CoalescePartitionsExec;
use datafusion::physical_plan::empty::EmptyExec;
use datafusion::physical_plan::explain::ExplainExec;
use datafusion::physical_plan::filter::FilterExec;
use datafusion::physical_plan::joins::{CrossJoinExec, HashJoinExec, NestedLoopJoinExec};
use datafusion::physical_plan::limit::{GlobalLimitExec, LocalLimitExec};
use datafusion::physical_plan::projection::ProjectionExec;
use datafusion::physical_plan::repartition::RepartitionExec;
use datafusion::physical_plan::union::UnionExec;
use datafusion::physical_plan::unnest::UnnestExec;
use datafusion::physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties};
use futures::StreamExt;

use crate::postgres::customscan::joinscan::exchange::{DsmExchangeExec, DSM_MESH};

/// Represents a virtual memory region to watch.
#[derive(Clone, Copy)]
struct MemoryRegion {
    start: usize,
    end: usize,
}

/// Helper to detect if Arrow buffers are backed by specific memory regions.
pub struct SharedMemoryDetector {
    regions: Vec<MemoryRegion>,
}

impl SharedMemoryDetector {
    /// Create a new detector from the active DSM mesh.
    pub fn new() -> Self {
        let regions = {
            let guard = DSM_MESH.lock();
            if let Some(mesh) = guard.as_ref() {
                mesh.transport
                    .memory_regions()
                    .into_iter()
                    .map(|(start, size)| MemoryRegion {
                        start,
                        end: start + size,
                    })
                    .collect()
            } else {
                Vec::new()
            }
        };
        Self { regions }
    }

    /// Checks if a specific buffer resides in any of the watched shared memory regions.
    #[inline]
    pub fn is_shared(&self, buffer: &Buffer) -> bool {
        // We look at the pointer of the underlying allocation.
        // buffer.as_ptr() returns the pointer to the start of the slice.
        // This is sufficient for detection if the slice is within the region.
        let ptr = buffer.as_ptr() as usize;

        for region in &self.regions {
            if ptr >= region.start && ptr < region.end {
                return true;
            }
        }
        false
    }

    /// Recursively checks if an ArrayData (and its children) holds any shared memory.
    pub fn has_shared_memory(&self, data: &ArrayData) -> bool {
        // 1. Check the validity (null) buffer
        if let Some(nulls) = data.nulls() {
            if self.is_shared(nulls.buffer()) {
                return true;
            }
        }

        // 2. Check all data buffers
        for buffer in data.buffers() {
            if self.is_shared(buffer) {
                return true;
            }
        }

        // 3. Recursively check child data
        for child in data.child_data() {
            if self.has_shared_memory(child) {
                return true;
            }
        }

        false
    }

    /// Entry point: Does this batch need sanitization?
    pub fn batch_needs_sanitization(&self, batch: &RecordBatch) -> bool {
        if self.regions.is_empty() {
            return false;
        }
        for column in batch.columns() {
            if self.has_shared_memory(&column.to_data()) {
                return true;
            }
        }
        false
    }
}

/// Force-copies a RecordBatch into new, standard heap-allocated buffers.
pub fn sanitize_batch(batch: &RecordBatch) -> Result<RecordBatch> {
    // 1. Create an index array [0, 1, 2, ... len-1]
    let indices = UInt32Array::from_iter_values(0..batch.num_rows() as u32);

    // 2. "Take" every row from every column.
    // The `take` kernel allocates new buffers for the result.
    let new_columns = batch
        .columns()
        .iter()
        .map(|col| {
            take(col.as_ref(), &indices, None)
                .map_err(|e| datafusion::error::DataFusionError::ArrowError(Box::new(e), None))
        })
        .collect::<Result<Vec<_>>>()?;

    RecordBatch::try_new(batch.schema(), new_columns)
        .map_err(|e| datafusion::error::DataFusionError::ArrowError(Box::new(e), None))
}

/// A physical operator that performs a deep copy of the input batches ONLY if they reside in shared memory.
#[derive(Debug)]
pub struct DsmSanitizeExec {
    input: Arc<dyn ExecutionPlan>,
    properties: PlanProperties,
}

impl DsmSanitizeExec {
    pub fn new(input: Arc<dyn ExecutionPlan>) -> Self {
        let properties = input.properties().clone();
        Self { input, properties }
    }
}

impl DisplayAs for DsmSanitizeExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut Formatter) -> std::fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                write!(f, "DsmSanitizeExec")
            }
            _ => Ok(()),
        }
    }
}

impl ExecutionPlan for DsmSanitizeExec {
    fn name(&self) -> &str {
        "DsmSanitizeExec"
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
        Ok(Arc::new(DsmSanitizeExec::new(children[0].clone())))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<datafusion::execution::TaskContext>,
    ) -> Result<datafusion::execution::SendableRecordBatchStream> {
        let stream = self.input.execute(partition, context)?;
        let schema = stream.schema();

        // Capture detector state at execution time
        let detector = Arc::new(SharedMemoryDetector::new());

        let sanitized_stream = stream.map(move |batch| {
            let batch = batch?;
            if detector.batch_needs_sanitization(&batch) {
                sanitize_batch(&batch)
            } else {
                Ok(batch)
            }
        });

        Ok(Box::pin(
            datafusion::physical_plan::stream::RecordBatchStreamAdapter::new(
                schema,
                Box::pin(sanitized_stream),
            ),
        ))
    }
}

/// Optimizer rule to inject `DsmSanitizeExec` before unsafe operators.
#[derive(Debug)]
pub struct EnforceSanitization;

impl EnforceSanitization {
    pub fn new() -> Self {
        Self
    }
}

impl PhysicalOptimizerRule for EnforceSanitization {
    fn name(&self) -> &str {
        "EnforceSanitization"
    }

    fn schema_check(&self) -> bool {
        true
    }

    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        enforce_sanitization_recursive(plan)
    }
}

fn enforce_sanitization_recursive(plan: Arc<dyn ExecutionPlan>) -> Result<Arc<dyn ExecutionPlan>> {
    // 1. Recursively optimize children first
    let new_children = plan
        .children()
        .iter()
        .map(|child| enforce_sanitization_recursive(Arc::clone(child)))
        .collect::<Result<Vec<_>>>()?;

    let plan = if new_children.is_empty() {
        plan
    } else {
        plan.with_new_children(new_children)?
    };

    // 2. Special handling for Joins that only buffer Left side
    //
    // TODO: SortMergeJoinExec could potentially be optimized to only sanitize one side
    // depending on the JoinType, but for safety (and due to complexity of key duplication buffering),
    // we currently treat it as unsafe and sanitize both inputs.
    if plan.as_any().is::<HashJoinExec>()
        || plan.as_any().is::<CrossJoinExec>()
        || plan.as_any().is::<NestedLoopJoinExec>()
    {
        let children = plan.children();
        // Left (0) is Build/Buffered -> Sanitize
        let left = sanitize_input(children[0].clone())?;
        // Right (1) is Probe/Streaming -> Safe (don't sanitize)
        let right = children[1].clone();
        return plan.with_new_children(vec![left, right]);
    }

    // 3. Default Unsafe Check (needs sanitized inputs)
    if is_unsafe_node(plan.as_ref()) {
        let mut new_inputs = Vec::new();
        for child in plan.children() {
            new_inputs.push(sanitize_input(child.clone())?);
        }
        return plan.with_new_children(new_inputs);
    }

    Ok(plan)
}

fn sanitize_input(child: Arc<dyn ExecutionPlan>) -> Result<Arc<dyn ExecutionPlan>> {
    // Check if child is DsmExchangeExec. If so, modify it in place.
    if let Some(exchange) = child.as_any().downcast_ref::<DsmExchangeExec>() {
        // Optimization: Instead of inserting DsmSanitizeExec, tell DsmExchangeExec to sanitize.
        let mut new_config = exchange.config.clone();
        new_config.sanitized = true;

        let new_exchange = DsmExchangeExec::try_new(
            exchange.input.clone(),
            exchange.producer_partitioning.clone(),
            exchange.properties().output_partitioning().clone(),
            new_config,
        )?;
        Ok(Arc::new(new_exchange) as Arc<dyn ExecutionPlan>)
    }
    // Otherwise, inject DsmSanitizeExec if it comes from DSM and isn't already sanitized.
    else if subtree_contains_dsm(child.as_ref()) && !child.as_any().is::<DsmSanitizeExec>() {
        Ok(Arc::new(DsmSanitizeExec::new(child.clone())) as Arc<dyn ExecutionPlan>)
    } else {
        Ok(child)
    }
}

/// Returns true if the node is "Safe" (Streaming), false otherwise.
///
/// Safe nodes process data incrementally and do not hold long-lived references
/// to previous batches (or they drop them quickly).
fn is_safe_node(plan: &dyn ExecutionPlan) -> bool {
    // Allowlist of streaming operators
    if plan.as_any().is::<FilterExec>() {
        return true;
    }
    if plan.as_any().is::<ProjectionExec>() {
        return true;
    }
    if plan.as_any().is::<RepartitionExec>() {
        return true;
    }
    if plan.as_any().is::<CoalesceBatchesExec>() {
        return true;
    }
    if plan.as_any().is::<CoalescePartitionsExec>() {
        return true;
    }
    if plan.as_any().is::<UnionExec>() {
        return true;
    }
    if plan.as_any().is::<GlobalLimitExec>() {
        return true;
    }
    if plan.as_any().is::<LocalLimitExec>() {
        return true;
    }
    if plan.as_any().is::<UnnestExec>() {
        return true;
    }
    if plan.as_any().is::<EmptyExec>() {
        return true;
    }
    if plan.as_any().is::<ExplainExec>() {
        return true;
    }
    if plan.as_any().is::<DsmSanitizeExec>() {
        return true;
    }
    if plan.as_any().is::<DsmExchangeExec>() {
        return true; // Source is safe (it produces the problem, but doesn't block)
    }

    // Also check for MockLeaf from tests if needed
    if plan.name() == "MockLeaf" {
        return true;
    }

    false
}

fn is_unsafe_node(plan: &dyn ExecutionPlan) -> bool {
    !is_safe_node(plan)
}

/// Recursively checks if the plan has a DsmExchangeExec in its subtree.
fn subtree_contains_dsm(plan: &dyn ExecutionPlan) -> bool {
    if plan.as_any().is::<DsmExchangeExec>() {
        // If the exchange is ALREADY sanitizing, then the subtree effectively DOES NOT contain DSM pointers.
        // We can check config.sanitized.
        if let Some(exchange) = plan.as_any().downcast_ref::<DsmExchangeExec>() {
            if exchange.config.sanitized {
                return false;
            }
        }
        return true;
    }

    // If we hit a SanitizeExec, the stream is clean.
    if plan.as_any().is::<DsmSanitizeExec>() {
        return false;
    }

    for child in plan.children() {
        if subtree_contains_dsm(child.as_ref()) {
            return true;
        }
    }
    false
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::postgres::customscan::joinscan::exchange::{
        DsmExchangeConfig, DsmExchangeExec, ExchangeMode,
    };
    use crate::postgres::customscan::joinscan::transport::LogicalStreamId;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::physical_expr::Partitioning;
    use datafusion::physical_plan::sorts::sort::SortExec;

    use datafusion::arrow::compute::SortOptions;
    use datafusion::physical_expr::expressions::Column;
    use datafusion::physical_expr::LexOrdering;
    use datafusion::physical_expr::PhysicalSortExpr;
    use datafusion::scalar::ScalarValue;

    #[pgrx::pg_test]
    fn test_enforce_sanitization_modifies_exchange_in_place() {
        let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)]));

        // 1. Create a DSM source (sanitized=false)
        let empty_input = Arc::new(EmptyExec::new(schema.clone()));
        let config = DsmExchangeConfig {
            stream_id: LogicalStreamId(1),
            total_participants: 1,
            mode: ExchangeMode::Gather,
            sanitized: false,
        };
        let dsm_source = Arc::new(
            DsmExchangeExec::try_new(
                empty_input,
                Partitioning::UnknownPartitioning(1),
                Partitioning::UnknownPartitioning(1),
                config,
            )
            .expect("Failed to create DsmExchangeExec"),
        );

        // 2. Create an Unsafe node (Sort)
        let sort_expr = PhysicalSortExpr {
            expr: Arc::new(Column::new("a", 0)),
            options: SortOptions::default(),
        };
        let sort_exprs = LexOrdering::new(vec![sort_expr]).expect("Failed to create LexOrdering");
        let sort = Arc::new(SortExec::new(sort_exprs, dsm_source));

        // 3. Optimize
        let rule = EnforceSanitization::new();
        let optimized = rule
            .optimize(sort, &ConfigOptions::default())
            .expect("Optimization failed");

        pgrx::warning!("Optimized plan: {:?}", optimized);
        let child = optimized.children()[0].clone();
        pgrx::warning!("Child plan: {:?}", child);
        pgrx::warning!("Child type: {:?}", child.as_any().type_id());

        // 4. Verify Sort child is DsmExchangeExec with sanitized=true
        assert!(optimized.as_any().is::<SortExec>());

        if let Some(exchange) = child.as_any().downcast_ref::<DsmExchangeExec>() {
            assert!(
                exchange.config.sanitized,
                "DsmExchangeExec should be sanitized"
            );
        } else {
            panic!("Child is not DsmExchangeExec. Actual plan: {:?}", child);
        }
    }

    #[pgrx::pg_test]
    fn test_enforce_sanitization_inserts_exec_when_nested() {
        let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)]));

        // 1. Create a DSM source
        let empty_input = Arc::new(EmptyExec::new(schema.clone()));
        let config = DsmExchangeConfig {
            stream_id: LogicalStreamId(1),
            total_participants: 1,
            mode: ExchangeMode::Gather,
            sanitized: false,
        };
        let dsm_source = Arc::new(
            DsmExchangeExec::try_new(
                empty_input,
                Partitioning::UnknownPartitioning(1),
                Partitioning::UnknownPartitioning(1),
                config,
            )
            .expect("Failed to create DsmExchangeExec"),
        );

        // 2. Nested safe node (Filter)
        let filter = Arc::new(
            FilterExec::try_new(
                Arc::new(datafusion::physical_expr::expressions::Literal::new(
                    ScalarValue::Boolean(Some(true)),
                )),
                dsm_source,
            )
            .expect("Failed to create FilterExec"),
        );

        // 3. Unsafe node (Sort)
        let sort_expr = PhysicalSortExpr {
            expr: Arc::new(Column::new("a", 0)),
            options: SortOptions::default(),
        };
        let sort_exprs = LexOrdering::new(vec![sort_expr]).expect("Failed to create LexOrdering");
        let sort = Arc::new(SortExec::new(sort_exprs, filter));

        // 4. Optimize
        let rule = EnforceSanitization::new();
        let optimized = rule
            .optimize(sort, &ConfigOptions::default())
            .expect("Optimization failed");

        pgrx::warning!("Optimized plan: {:?}", optimized);
        let sanitize = optimized.children()[0].clone();
        pgrx::warning!("Sanitize node: {:?}", sanitize);

        // 5. Verify structure: Sort -> DsmSanitizeExec -> Filter -> DsmExchangeExec
        assert!(optimized.as_any().is::<SortExec>());
        assert!(sanitize.as_any().is::<DsmSanitizeExec>());
        let filter = sanitize.children()[0].clone();
        assert!(filter.as_any().is::<FilterExec>());
        let exchange = filter.children()[0].clone();
        assert!(exchange.as_any().is::<DsmExchangeExec>());

        if let Some(exchange_node) = exchange.as_any().downcast_ref::<DsmExchangeExec>() {
            // The original exchange was NOT modified because it wasn't immediate child
            assert!(!exchange_node.config.sanitized);
        } else {
            panic!(
                "Grandchild is not DsmExchangeExec. Actual plan: {:?}",
                exchange
            );
        }
    }
}

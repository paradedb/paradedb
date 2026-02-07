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
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use arrow_array::RecordBatch;
use arrow_schema::{SchemaRef, SortOptions};
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::{RecordBatchStream, SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::expressions::Column;
use datafusion::physical_expr::{EquivalenceProperties, LexOrdering, PhysicalSortExpr};
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::sorts::sort_preserving_merge::SortPreservingMergeExec;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties,
};
use futures::Stream;

use crate::index::fast_fields_helper::FFHelper;
use crate::postgres::customscan::explain::ExplainFormat;
use crate::postgres::options::{SortByDirection, SortByField};
use crate::query::SearchQueryInput;
use crate::scan::{Scanner, VisibilityChecker};

/// A wrapper that implements Send + Sync unconditionally.
/// UNSAFE: Only use this when you guarantee single-threaded access or manual synchronization.
/// This is safe in pg_search because Postgres extensions run single-threaded.
pub(crate) struct UnsafeSendSync<T>(pub T);

unsafe impl<T> Send for UnsafeSendSync<T> {}
unsafe impl<T> Sync for UnsafeSendSync<T> {}

/// State for a scan partition.
/// Uses Arc<FFHelper> so the same FFHelper can be shared across multiple partitions.
pub type ScanState = (Scanner, Arc<FFHelper>, Box<dyn VisibilityChecker>);

/// Factory function that creates a ScanState for a given partition on demand.
/// This enables lazy segment checkout - segments are only checked out when execute() is called.
///
/// Wrapped in UnsafeSendSync because the factory may capture Postgres state that is not
/// Send/Sync (like VisibilityChecker), but pg_search operates in a single-threaded context.
pub(crate) type CheckoutFactory = UnsafeSendSync<Arc<dyn Fn(usize) -> ScanState>>;

/// Creates a CheckoutFactory from a closure.
///
/// # Safety
/// The factory closure may capture non-Send/Sync types. This is safe because pg_search
/// operates in a single-threaded Tokio executor within Postgres, and these objects
/// will never cross thread boundaries.
pub fn make_checkout_factory<F>(factory: F) -> CheckoutFactory
where
    F: Fn(usize) -> ScanState + 'static,
{
    UnsafeSendSync(Arc::new(factory))
}

/// A DataFusion `ExecutionPlan` for scanning a single segment of a `pg_search` index.
pub struct SegmentPlan {
    // We use a Mutex to allow taking the fields during execute()
    // We wrap the state in UnsafeSendSync to satisfy ExecutionPlan's Send+Sync requirements
    // This is safe because we are running in a single-threaded environment (Postgres)
    state: Mutex<Option<UnsafeSendSync<ScanState>>>,
    properties: PlanProperties,
    /// Query to display in EXPLAIN output. None displays as "all".
    query_for_display: Option<SearchQueryInput>,
}

impl std::fmt::Debug for SegmentPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SegmentPlan")
            .field("properties", &self.properties)
            .finish()
    }
}

impl SegmentPlan {
    pub fn new(
        scanner: Scanner,
        ffhelper: FFHelper,
        visibility: Box<dyn VisibilityChecker>,
        query_for_display: Option<SearchQueryInput>,
    ) -> Self {
        let schema = scanner.schema();
        let properties = PlanProperties::new(
            EquivalenceProperties::new(schema),
            Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );
        Self {
            state: Mutex::new(Some(UnsafeSendSync((
                scanner,
                Arc::new(ffhelper),
                visibility,
            )))),
            properties,
            query_for_display,
        }
    }

    /// Creates a new SegmentPlan with a shared FFHelper.
    ///
    /// This variant accepts an `Arc<FFHelper>` allowing the FFHelper to be shared
    /// across multiple plans or with other components.
    pub fn new_with_shared_ffhelper(
        scanner: Scanner,
        ffhelper: Arc<FFHelper>,
        visibility: Box<dyn VisibilityChecker>,
        query_for_display: Option<SearchQueryInput>,
    ) -> Self {
        let schema = scanner.schema();
        let properties = PlanProperties::new(
            EquivalenceProperties::new(schema),
            Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );
        Self {
            state: Mutex::new(Some(UnsafeSendSync((scanner, ffhelper, visibility)))),
            properties,
            query_for_display,
        }
    }
}

/// Build `EquivalenceProperties` with the specified sort ordering.
///
/// If `sort_order` is `Some`, the returned properties will declare that the
/// data is sorted by the specified field in the specified direction.
/// If `sort_order` is `None`, returns empty equivalence properties.
fn build_equivalence_properties(
    schema: SchemaRef,
    sort_order: Option<&SortByField>,
) -> EquivalenceProperties {
    let mut eq_properties = EquivalenceProperties::new(schema.clone());

    if let Some(sort_field) = sort_order {
        // Find the column index for the sort field
        let field_name = sort_field.field_name.as_ref();
        if let Some((col_idx, _)) = schema.column_with_name(field_name) {
            let sort_options = SortOptions {
                descending: matches!(sort_field.direction, SortByDirection::Desc),
                // Tantivy's sort behavior:
                // - ASC: nulls sort first
                // - DESC: nulls sort last
                nulls_first: matches!(sort_field.direction, SortByDirection::Asc),
            };

            let sort_expr = PhysicalSortExpr {
                expr: Arc::new(Column::new(field_name, col_idx)),
                options: sort_options,
            };

            // Add the ordering to the equivalence properties
            eq_properties.add_ordering(std::iter::once(sort_expr));
        }
    }

    eq_properties
}

impl DisplayAs for SegmentPlan {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.query_for_display {
            Some(query) => write!(f, "PgSearchScan: {}", query.explain_format()),
            None => write!(f, "PgSearchScan: all"),
        }
    }
}

impl ExecutionPlan for SegmentPlan {
    fn name(&self) -> &str {
        "PgSearchScan"
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
        _children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    fn execute(
        &self,
        _partition: usize,
        _context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let mut state = self.state.lock().map_err(|e| {
            DataFusionError::Internal(format!("Failed to lock SegmentPlan state: {e}"))
        })?;
        let UnsafeSendSync((scanner, ffhelper, visibility)) = state.take().ok_or_else(|| {
            DataFusionError::Internal("SegmentPlan can only be executed once".to_string())
        })?;

        // SAFETY: pg_search operates in a single-threaded Tokio executor within Postgres,
        // so it is safe to wrap !Send types for use within DataFusion.
        let stream = unsafe {
            UnsafeSendStream::new(ScanStream {
                scanner,
                ffhelper,
                visibility,
                schema: self.properties.eq_properties.schema().clone(),
            })
        };
        Ok(Box::pin(stream))
    }
}

struct ScanStream {
    scanner: Scanner,
    ffhelper: Arc<FFHelper>,
    visibility: Box<dyn VisibilityChecker>,
    schema: SchemaRef,
}

impl Stream for ScanStream {
    type Item = Result<RecordBatch>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.scanner.next(&this.ffhelper, &mut *this.visibility) {
            Some(batch) => Poll::Ready(Some(Ok(batch.to_record_batch(&this.schema)))),
            None => Poll::Ready(None),
        }
    }
}

impl RecordBatchStream for ScanStream {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

/// A wrapper that unsafely implements Send for a Stream.
///
/// This is used to wrap `ScanStream` which is !Send because it contains Tantivy and Postgres
/// state that is not Send. This is safe because pg_search operates in a single-threaded
/// Tokio executor within Postgres, and these objects will never cross thread boundaries.
struct UnsafeSendStream<T>(T);

impl<T> UnsafeSendStream<T> {
    unsafe fn new(t: T) -> Self {
        Self(t)
    }
}

unsafe impl<T> Send for UnsafeSendStream<T> {}

impl<T: Stream> Stream for UnsafeSendStream<T> {
    type Item = T::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().0).poll_next(cx) }
    }
}

impl<T: RecordBatchStream> RecordBatchStream for UnsafeSendStream<T> {
    fn schema(&self) -> SchemaRef {
        self.0.schema()
    }
}

// ============================================================================
// Multi-partition MultiSegmentPlan for sorted segment scanning
// ============================================================================

/// A DataFusion `ExecutionPlan` that scans multiple segments in partitions.
///
/// Each partition corresponds to one Tantivy segment. When the index is sorted (with `sort_by`),
/// each partition produces sorted output, which can then be merged using
/// `SortPreservingMergeExec` to produce a globally sorted result.
///
/// Uses lazy segment checkout - segments are checked out on-demand when `execute()`
/// is called, rather than upfront at plan creation time. This defers memory allocation
/// until the partition is actually executed.
pub struct MultiSegmentPlan {
    /// Number of segments/partitions.
    segment_count: usize,
    /// Factory function that creates a ScanState for a given partition on demand.
    checkout_factory: CheckoutFactory,
    /// Tracks which partitions have been executed (to prevent double execution).
    checked_out: Mutex<Vec<bool>>,
    properties: PlanProperties,
}

impl std::fmt::Debug for MultiSegmentPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiSegmentPlan")
            .field("properties", &self.properties)
            .finish()
    }
}

impl MultiSegmentPlan {
    /// Creates a new MultiSegmentPlan with lazy segment checkout.
    ///
    /// Instead of building all segment states upfront, this constructor accepts
    /// a factory function that creates states on-demand when `execute()` is called.
    /// This defers memory allocation until the partition is actually executed.
    ///
    /// # Arguments
    ///
    /// * `segment_count` - The number of segments/partitions
    /// * `checkout_factory` - Factory function (wrapped in UnsafeSendSync) that creates a `ScanState`
    /// * `schema` - Arrow schema for the output
    /// * `sort_order` - Optional sort order declaration for equivalence properties
    pub fn new(
        segment_count: usize,
        checkout_factory: CheckoutFactory,
        schema: SchemaRef,
        sort_order: Option<&SortByField>,
    ) -> Self {
        let eq_properties = build_equivalence_properties(schema, sort_order);

        let properties = PlanProperties::new(
            eq_properties,
            Partitioning::UnknownPartitioning(segment_count),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );

        Self {
            segment_count,
            checkout_factory,
            checked_out: Mutex::new(vec![false; segment_count]),
            properties,
        }
    }
}

impl DisplayAs for MultiSegmentPlan {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PgSearchSegmentScan(segments={})", self.segment_count)
    }
}

impl ExecutionPlan for MultiSegmentPlan {
    fn name(&self) -> &str {
        "PgSearchSegmentScan"
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
        _children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    fn execute(
        &self,
        partition: usize,
        _context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        if partition >= self.segment_count {
            return Err(DataFusionError::Internal(format!(
                "Partition {} out of range (have {} segments)",
                partition, self.segment_count
            )));
        }

        let mut checked_out = self.checked_out.lock().map_err(|e| {
            DataFusionError::Internal(format!("Failed to lock MultiSegmentPlan state: {e}"))
        })?;

        if checked_out[partition] {
            return Err(DataFusionError::Internal(format!(
                "Segment {} has already been executed",
                partition
            )));
        }
        checked_out[partition] = true;

        // Call the factory to create the state NOW (deferred/lazy checkout)
        let UnsafeSendSync(factory) = &self.checkout_factory;
        let (scanner, ffhelper, visibility) = factory(partition);

        // SAFETY: pg_search operates in a single-threaded Tokio executor within Postgres
        let stream = unsafe {
            UnsafeSendStream::new(ScanStream {
                scanner,
                ffhelper,
                visibility,
                schema: self.properties.eq_properties.schema().clone(),
            })
        };
        Ok(Box::pin(stream))
    }
}

// ============================================================================
// Builder for creating sorted scans with SortPreservingMergeExec
// ============================================================================

/// Creates a sorted scan plan with `SortPreservingMergeExec` to merge sorted segments.
///
/// Uses lazy segment checkout - segments are checked out on-demand when `execute()` is
/// called, rather than upfront at plan creation time.
///
/// When there is only one segment, returns the `MultiSegmentPlan` directly without
/// the merge layer (no merging needed for a single partition).
///
/// Returns `None` if the sort field is not present in the schema (e.g., the sort column
/// was not projected in the scan). In this case, the caller should fall back to an
/// unsorted scan to avoid producing incorrectly ordered results.
///
/// # Arguments
///
/// * `segment_count` - The number of segments to scan
/// * `checkout_factory` - Factory function that creates a `ScanState` for a given partition
/// * `schema` - Arrow schema for the output
/// * `sort_order` - Sort order for the merge operation
pub fn create_sorted_scan(
    segment_count: usize,
    checkout_factory: CheckoutFactory,
    schema: SchemaRef,
    sort_order: &SortByField,
) -> Result<Arc<dyn ExecutionPlan>> {
    // Validate that the sort field exists in the schema
    let field_name = sort_order.field_name.as_ref();
    let col_idx = match schema.column_with_name(field_name) {
        Some((idx, _)) => idx,
        None => {
            // Sort field is not in the schema - cannot create sorted merge.
            return Err(DataFusionError::Internal(format!(
                "Sort field '{}' not found in scan schema",
                field_name
            )));
        }
    };

    let segment_scan = Arc::new(MultiSegmentPlan::new(
        segment_count,
        checkout_factory,
        schema.clone(),
        Some(sort_order),
    ));

    // For a single segment, no merging is needed
    if segment_count == 1 {
        return Ok(segment_scan);
    }

    let sort_options = SortOptions {
        descending: matches!(sort_order.direction, SortByDirection::Desc),
        nulls_first: matches!(sort_order.direction, SortByDirection::Asc),
    };

    let sort_expr = PhysicalSortExpr {
        expr: Arc::new(Column::new(field_name, col_idx)),
        options: sort_options,
    };

    let ordering =
        LexOrdering::new(vec![sort_expr]).expect("sort expression should create valid ordering");

    // Wrap with SortPreservingMergeExec to merge sorted partitions
    Ok(Arc::new(SortPreservingMergeExec::new(
        ordering,
        segment_scan,
    )))
}

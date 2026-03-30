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
use std::fmt::Debug;
use std::sync::Arc;

use arrow_schema::SchemaRef;
use async_trait::async_trait;
use datafusion::catalog::{Session, TableProvider};
use datafusion::common::{DataFusionError, Result, Statistics};
use datafusion::logical_expr::{Expr, TableProviderFilterPushDown, TableType};
use datafusion::physical_plan::ExecutionPlan;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};
use tantivy::index::SegmentId;

use crate::index::fast_fields_helper::{CanonicalColumn, FFHelper, WhichFastField};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::parallel::{checkout_segment, list_segment_ids};
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::ParallelScanState;
use crate::query::SearchQueryInput;
use crate::scan::execution_plan::{PgSearchScanPlan, ScanState};
use crate::scan::filter_pushdown::{combine_with_and, FilterAnalyzer};
use crate::scan::info::{RowEstimate, ScanInfo};
use crate::scan::late_materialization::DeferredField;
use crate::scan::Scanner;

use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VisibilitySourceMetadata {
    pub plan_position: usize,
    pub heap_oid: pg_sys::Oid,
    pub table_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) enum VisibilityMode {
    #[default]
    Eager,
    Deferred {
        plan_position: usize,
    },
}

impl VisibilityMode {
    pub(crate) fn deferred_plan_position(self) -> Option<usize> {
        match self {
            Self::Eager => None,
            Self::Deferred { plan_position } => Some(plan_position),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PgSearchTableProvider {
    scan_info: ScanInfo,
    fields: Vec<WhichFastField>,
    #[serde(skip)]
    schema: std::sync::OnceLock<SchemaRef>,
    #[serde(skip)]
    late_materialization_schema: std::sync::OnceLock<SchemaRef>,
    is_parallel: bool,
    /// Parallel state is skipped during serialization because it's a raw pointer
    /// to shared memory that is only valid in the current process. It is
    /// re-injected by the execution codec during deserialization if
    /// `is_parallel` is true.
    #[serde(skip)]
    parallel_state: Option<*mut ParallelScanState>,
    /// Postgres expression context, skipped during serialization and
    /// re-injected by the execution codec during deserialization.
    #[serde(skip)]
    expr_context: Option<*mut pg_sys::ExprContext>,
    /// Executor planstate, skipped during serialization and re-injected only for execution paths
    /// that need to solve runtime Postgres expressions before opening a real reader.
    #[serde(skip)]
    planstate: Option<*mut pg_sys::PlanState>,
    /// Position of this source in the non-partitioning source list (0-based), or `None`
    /// if this is the partitioning source or a serial scan.
    ///
    /// This is serialized so the execution codec can inject the correct canonical
    /// segment IDs in both the leader and parallel workers.
    non_partitioning_index: Option<usize>,
    /// Canonical segment IDs for replicated-parallel execution.
    ///
    /// When `Some`, `scan()` uses `MvccSatisfies::ParallelWorker(ids)` instead of
    /// `MvccSatisfies::Snapshot`. Injected by the execution codec based on
    /// `non_partitioning_index`; `None` for the partitioning source and serial scans.
    #[serde(skip)]
    canonical_segment_ids: Option<crate::api::HashSet<SegmentId>>,
    /// The visibility strategy for this source.
    ///
    /// `Deferred { plan_position }` means the scan emits packed DocAddresses in its
    /// `ctid_<plan_position>` column and `VisibilityFilterExec` resolves visibility later.
    /// `Eager` means the scan performs visibility checks itself and emits normal ctids.
    visibility_mode: VisibilityMode,

    /// A lifecycle toggle that dictates what schema is exposed to DataFusion.
    ///
    /// - **Phase 1 (false) - SQL Planning:** During initial plan construction (`joinscan`),
    ///   this provider must expose a standard relational schema (i.e. `Utf8View` for strings)
    ///   so that DataFusion's SQL expression builder and `TypeCoercion` pass don't panic
    ///   when trying to apply normal string functions/sorts to a `Union` type.
    ///
    /// - **Phase 2 (true) - Logical Optimization:** Once the plan is structurally validated,
    ///   our `LateMaterializationRule` flips this to `true` (via interior mutability).
    ///   The provider immediately begins returning the physical `Union` schema. The rule
    ///   then updates the `TableScan.projected_schema` to match, allowing the `Union`
    ///   types to legally bubble up to our `LateMaterializeNode` anchor.
    ///
    /// SAFETY: Relaxed ordering is sufficient because the store (in LateMaterializationRule)
    /// and load (in get_schema) execute sequentially within the same single-threaded optimization pass.
    #[serde(with = "atomic_bool_serde")]
    late_materialization_active: AtomicBool,
}

mod atomic_bool_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::sync::atomic::{AtomicBool, Ordering};

    pub fn serialize<S>(val: &AtomicBool, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(val.load(Ordering::Relaxed))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<AtomicBool, D::Error>
    where
        D: Deserializer<'de>,
    {
        let b = bool::deserialize(deserializer)?;
        Ok(AtomicBool::new(b))
    }
}

unsafe impl Send for PgSearchTableProvider {}
unsafe impl Sync for PgSearchTableProvider {}

impl PgSearchTableProvider {
    pub fn new(scan_info: ScanInfo, fields: Vec<WhichFastField>, is_parallel: bool) -> Self {
        Self {
            scan_info,
            fields,
            schema: std::sync::OnceLock::new(),
            late_materialization_schema: std::sync::OnceLock::new(),
            is_parallel,
            parallel_state: None,
            expr_context: None,
            planstate: None,
            non_partitioning_index: None,
            canonical_segment_ids: None,
            visibility_mode: VisibilityMode::Eager,
            late_materialization_active: AtomicBool::new(false),
        }
    }

    /// Transitions the provider from Phase 1 (`Utf8View`) into Phase 2 (`Union`)
    pub fn enable_late_materialization_schema(&self) {
        self.late_materialization_active
            .store(true, Ordering::Relaxed);
    }

    pub(crate) fn is_parallel(&self) -> bool {
        self.is_parallel
    }

    pub(crate) fn set_parallel_state(&mut self, parallel_state: Option<*mut ParallelScanState>) {
        assert!(self.is_parallel);
        self.parallel_state = parallel_state;
    }

    /// Mark this provider as a non-partitioning source at position `idx` in the
    /// non-partitioning source list. The execution codec uses this index to
    /// inject the correct canonical segment IDs during deserialization.
    pub(crate) fn set_non_partitioning_index(&mut self, idx: usize) {
        self.non_partitioning_index = Some(idx);
    }

    /// Return the position of this provider in the non-partitioning source list, or `None`.
    pub(crate) fn non_partitioning_index(&self) -> Option<usize> {
        self.non_partitioning_index
    }

    /// Inject the canonical segment IDs for this replicated-parallel provider.
    pub(crate) fn set_canonical_segment_ids(&mut self, ids: crate::api::HashSet<SegmentId>) {
        self.canonical_segment_ids = Some(ids);
    }

    pub(crate) fn set_expr_context(&mut self, expr_context: Option<*mut pg_sys::ExprContext>) {
        self.expr_context = expr_context;
    }

    pub(crate) fn set_planstate(&mut self, planstate: Option<*mut pg_sys::PlanState>) {
        self.planstate = planstate;
    }
    fn enable_deferred_columns(&mut self, required_early_columns: &crate::api::HashSet<String>) {
        for wff in self.fields.iter_mut() {
            if let WhichFastField::Named(name, field_type) = wff {
                let is_string_or_bytes = matches!(
                    field_type.arrow_data_type(),
                    arrow_schema::DataType::Utf8View
                        | arrow_schema::DataType::BinaryView
                        | arrow_schema::DataType::LargeUtf8
                        | arrow_schema::DataType::LargeBinary
                );
                if is_string_or_bytes && !required_early_columns.contains(name.as_str()) {
                    let cloned_name = name.clone();
                    let cloned_type = *field_type;
                    *wff = WhichFastField::Deferred(cloned_name, cloned_type);
                }
            }
        }
    }

    fn enable_deferred_visibility(&mut self, plan_position: usize) {
        // Defer ctid for deferred visibility (joinscan path only).
        // Emits packed DocAddresses instead of real ctids so that visibility
        // checking can be done in batch by VisibilityFilterExec after the join.
        let mut deferred_ctid = false;
        for wff in self.fields.iter_mut() {
            if matches!(wff, WhichFastField::Ctid) {
                *wff = WhichFastField::DeferredCtid(
                    crate::postgres::customscan::joinscan::CtidColumn::new(plan_position)
                        .to_string(),
                );
                deferred_ctid = true;
            }
        }
        if deferred_ctid {
            self.visibility_mode = VisibilityMode::Deferred { plan_position };
        }
    }

    /// Configures deferred output modes for this scan source.
    ///
    /// This may defer:
    /// - text/bytes fast fields that are not needed before lookup
    /// - ctid resolution for JoinScan deferred visibility
    pub fn configure_deferred_outputs(
        &mut self,
        required_early_columns: &crate::api::HashSet<String>,
        visibility_mode: VisibilityMode,
    ) {
        self.enable_deferred_columns(required_early_columns);
        if let VisibilityMode::Deferred { plan_position } = visibility_mode {
            self.enable_deferred_visibility(plan_position);
        }
    }

    pub(crate) fn visibility_mode(&self) -> VisibilityMode {
        self.visibility_mode
    }

    /// Returns the JoinScan source identity when visibility has been deferred.
    pub(crate) fn deferred_ctid_plan_position(&self) -> Option<usize> {
        self.visibility_mode().deferred_plan_position()
    }

    /// Returns the per-source metadata needed by `VisibilityFilterOptimizerRule`.
    ///
    /// This is only available once the provider has deferred its ctid column.
    pub(crate) fn visibility_source_metadata(&self) -> Option<VisibilitySourceMetadata> {
        let plan_position = self.deferred_ctid_plan_position()?;
        let table_name = self
            .scan_info
            .alias
            .clone()
            .unwrap_or_else(|| format!("source_{plan_position}"));

        Some(VisibilitySourceMetadata {
            plan_position,
            heap_oid: self.scan_info.heaprelid,
            table_name,
        })
    }

    pub fn deferred_fields(&self) -> Vec<DeferredField> {
        let mut deferred = Vec::new();
        for (ff_index, wff) in self.fields.iter().enumerate() {
            if let WhichFastField::Deferred(name, field_type) = wff {
                let is_bytes = matches!(
                    field_type.arrow_data_type(),
                    arrow_schema::DataType::BinaryView | arrow_schema::DataType::LargeBinary
                );
                deferred.push(DeferredField {
                    name: name.clone(),
                    is_bytes,
                    canonical: CanonicalColumn {
                        indexrelid: self.scan_info.indexrelid.to_u32(),
                        ff_index,
                    },
                });
            }
        }
        deferred
    }

    fn get_schema(&self) -> SchemaRef {
        if self.late_materialization_active.load(Ordering::Relaxed) {
            self.late_materialization_schema
                .get_or_init(|| crate::index::fast_fields_helper::build_arrow_schema(&self.fields))
                .clone()
        } else {
            self.schema
                .get_or_init(|| {
                    let logical_fields: Vec<_> = self
                        .fields
                        .iter()
                        .map(|wff| {
                            if let WhichFastField::Deferred(name, ty) = wff {
                                WhichFastField::Named(name.clone(), *ty)
                            } else {
                                wff.clone()
                            }
                        })
                        .collect();
                    crate::index::fast_fields_helper::build_arrow_schema(&logical_fields)
                })
                .clone()
        }
    }

    fn projected_fields_and_schema(
        &self,
        projection: Option<&Vec<usize>>,
    ) -> Result<(Vec<WhichFastField>, SchemaRef)> {
        let active_fields: Vec<_> = if self.late_materialization_active.load(Ordering::Relaxed) {
            self.fields.clone()
        } else {
            self.fields
                .iter()
                .map(|wff| {
                    if let WhichFastField::Deferred(name, ty) = wff {
                        WhichFastField::Named(name.clone(), *ty)
                    } else {
                        wff.clone()
                    }
                })
                .collect()
        };

        let schema = self.get_schema();
        match projection {
            None => Ok((active_fields, schema)),
            Some(indices) => {
                let mut fields = Vec::with_capacity(indices.len());
                for &idx in indices {
                    let field = active_fields.get(idx).ok_or_else(|| {
                        DataFusionError::Execution(format!(
                            "Projection index {idx} out of bounds for {} fields",
                            active_fields.len()
                        ))
                    })?;
                    fields.push(field.clone());
                }
                let schema = Arc::new(schema.project(indices)?);
                Ok((fields, schema))
            }
        }
    }

    fn analyzer(&self) -> FilterAnalyzer<'_> {
        // Note: baserestrictinfo predicates (in scan_info.query) are single-table predicates
        // applied at the base relation level. The filters we analyze here are join-level
        // predicates that couldn't be applied earlier - they are different predicates,
        // not duplicates.
        FilterAnalyzer::new(&self.fields, self.scan_info.indexrelid)
    }

    /// Combine the base query with any pushed-down filters.
    ///
    /// The base query comes from scan_info.query (single-table predicates from baserestrictinfo).
    /// The filters come from DataFusion's supports_filters_pushdown mechanism - these are
    /// join-level predicates that couldn't be applied at the base relation level, including:
    /// - SearchPredicateUDF: @@@ predicates from cross-table conditions
    /// - Regular SQL predicates: equality, range, IN list on indexed columns
    fn combine_query_with_filters(
        &self,
        base_query: SearchQueryInput,
        filters: &[Expr],
    ) -> SearchQueryInput {
        if filters.is_empty() {
            return base_query;
        }

        let analyzer = self.analyzer();
        let filter_queries: Vec<SearchQueryInput> =
            filters.iter().map(|f| analyzer.analyze(f)).collect();

        if filter_queries.is_empty() {
            return base_query;
        }

        let mut all_queries = vec![base_query];
        all_queries.extend(filter_queries);
        combine_with_and(all_queries).unwrap_or(SearchQueryInput::All)
    }

    /// Creates a single scan partition representing exactly one segment.
    ///
    /// This function directly opens the index segment, calculates the estimated number of
    /// documents contained within it, and initializes a `Scanner` bounded to this specific segment.
    #[allow(clippy::too_many_arguments)]
    fn create_scan_partition(
        &self,
        reader: &SearchIndexReader,
        segment_id: SegmentId,
        fields: &[WhichFastField],
        ffhelper: &Arc<FFHelper>,
        visibility: &VisibilityChecker,
        heap_relid: pg_sys::Oid,
    ) -> ScanState {
        let search_results = reader.search_segments(std::iter::once(segment_id));

        let scanner = Scanner::new(search_results, None, fields.to_vec(), heap_relid.into());
        (
            scanner,
            ffhelper.clone(),
            Box::new(visibility.clone()) as Box<VisibilityChecker>,
        )
    }
    /// Creates a PgSearchScanPlan from a list of segments.
    fn create_scan(
        &self,
        segments: Vec<ScanState>,
        schema: SchemaRef,
        query_for_display: SearchQueryInput,
        sort_order: Option<&crate::postgres::options::SortByField>,
        ffhelper: Arc<FFHelper>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // Only declare sort order if the field exists in the schema
        let actual_sort_order = sort_order.and_then(|so| {
            let field_name = so.field_name.as_ref();
            if schema.column_with_name(field_name).is_some() {
                Some(so)
            } else {
                None
            }
        });

        let deferred = self.deferred_fields();
        let deferred_ctid_plan_position = self.deferred_ctid_plan_position();

        // Expose one shared FFHelper when this scan participates in either
        // late materialization or deferred visibility. Downstream callers
        // decide whether they should use it by checking the deferred metadata.
        let ffhelper = if deferred.is_empty() && deferred_ctid_plan_position.is_none() {
            None
        } else {
            Some(ffhelper.clone())
        };

        Ok(Arc::new(PgSearchScanPlan::new(
            segments,
            schema,
            query_for_display,
            actual_sort_order,
            deferred,
            ffhelper,
            self.scan_info.indexrelid.to_u32(),
            deferred_ctid_plan_position,
        )))
    }

    /// Creates a multi-partition `PgSearchScanPlan` for throttled parallel scans.
    ///
    /// This method is specifically designed for parallel execution paths that require
    /// sorted outputs. Because the results need to be globally sorted, DataFusion needs
    /// to see all segments concurrently as distinct partitions so it can apply a
    /// `SortPreservingMergeExec` across them.
    ///
    /// To ensure fair work distribution among multiple Postgres parallel workers, this
    /// method uses a throttled loop to claim segments from the shared `parallel_state`.
    /// When a worker claims a segment, it explicitly prefetches a single batch from that
    /// segment before attempting to claim the next one. This small amount of work prevents
    /// a single fast-starting worker from immediately checking out all available segments
    /// and starving the other workers.
    #[allow(clippy::too_many_arguments)]
    fn create_throttled_scan(
        &self,
        parallel_state: *mut ParallelScanState,
        reader: &SearchIndexReader,
        fields: &[WhichFastField],
        ffhelper: FFHelper,
        visibility: VisibilityChecker,
        heap_relid: pg_sys::Oid,
        schema: SchemaRef,
        query_for_display: SearchQueryInput,
        sort_order: Option<&crate::postgres::options::SortByField>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let mut segments = Vec::new();
        let ffhelper = Arc::new(ffhelper);

        loop {
            pgrx::check_for_interrupts!();

            let segment_id = unsafe { checkout_segment(parallel_state) };
            let Some(segment_id) = segment_id else {
                break;
            };

            let mut partition = self.create_scan_partition(
                reader,
                segment_id,
                fields,
                &ffhelper,
                &visibility,
                heap_relid,
            );
            // Do real work between checkouts to avoid one worker claiming all segments.
            partition.0.prefetch_next(&ffhelper, &mut partition.2, None);

            segments.push(partition);
        }

        self.create_scan(segments, schema, query_for_display, sort_order, ffhelper)
    }

    /// Creates a multi-partition `PgSearchScanPlan` for eager scans.
    ///
    /// This method is used when we need to expose every segment as its own distinct
    /// DataFusion partition, but we are *not* dynamically claiming segments from a shared
    /// parallel pool (e.g., serial execution with an `ORDER BY`, or a fully replicated
    /// parallel source where every worker must scan the entire table).
    ///
    /// It opens all segments sequentially upfront and assigns each to a distinct `Scanner`
    /// partition. If a sort order is specified, this allows DataFusion to apply a
    /// `SortPreservingMergeExec` across the partitions. Since there is no competition
    /// with other workers for these segments, no prefetching or throttling is necessary.
    #[allow(clippy::too_many_arguments)]
    fn create_eager_scan(
        &self,
        reader: &SearchIndexReader,
        fields: &[WhichFastField],
        ffhelper: FFHelper,
        visibility: VisibilityChecker,
        heap_relid: pg_sys::Oid,
        schema: SchemaRef,
        query_for_display: SearchQueryInput,
        sort_order: Option<&crate::postgres::options::SortByField>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let ffhelper = Arc::new(ffhelper);
        let segments: Vec<ScanState> = reader
            .segment_readers()
            .iter()
            .map(|r| {
                self.create_scan_partition(
                    reader,
                    r.segment_id(),
                    fields,
                    &ffhelper,
                    &visibility,
                    heap_relid,
                )
            })
            .collect();

        self.create_scan(segments, schema, query_for_display, sort_order, ffhelper)
    }

    /// Creates a single-partition `PgSearchScanPlan` for lazy scans.
    ///
    /// This method is used when the output does *not* need to be globally sorted, meaning
    /// DataFusion doesn't need to perform a `SortPreservingMergeExec` across multiple streams.
    /// Thus, we can optimize execution by exposing exactly 1 partition natively and chaining
    /// segments end-to-end.
    ///
    /// In the parallel case, segments are dynamically checked out from `parallel_state` only
    /// when the previous segment is exhausted. This inherently yields execution time to other
    /// parallel workers at segment boundaries without requiring explicit prefetching. In the
    /// serial case, it simply chains all segments.
    ///
    /// We require `planner_estimated_rows` to be passed in because a lazy scan cannot know
    /// exactly which segments it will end up scanning at planning time to sum their individual
    /// sizes. Instead, it relies on the estimated partition size computed during query planning.
    #[allow(clippy::too_many_arguments)]
    fn create_lazy_scan(
        &self,
        parallel_state: Option<*mut ParallelScanState>,
        reader: &SearchIndexReader,
        fields: &[WhichFastField],
        ffhelper: FFHelper,
        visibility: VisibilityChecker,
        heap_relid: pg_sys::Oid,
        schema: SchemaRef,
        query_for_display: SearchQueryInput,
        planner_estimated_rows: u64,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let search_results = if let Some(parallel_state) = parallel_state {
            reader.search_lazy(parallel_state, planner_estimated_rows)
        } else {
            // Unsorted Serial
            reader.search()
        };

        let scanner = Scanner::new(
            search_results,
            None, // batch size hint
            fields.to_vec(),
            heap_relid.into(),
        );

        let ffhelper_arc = Arc::new(ffhelper);
        let state = (
            scanner,
            ffhelper_arc.clone(),
            Box::new(visibility) as Box<VisibilityChecker>,
        );

        self.create_scan(
            vec![state],
            schema,
            query_for_display,
            None, // no sort order
            ffhelper_arc,
        )
    }
}

#[async_trait]
impl TableProvider for PgSearchTableProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.get_schema()
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    fn statistics(&self) -> Option<Statistics> {
        use datafusion::common::stats::{ColumnStatistics, Precision};

        let num_rows = match self.scan_info.estimate {
            RowEstimate::Known(n) => Precision::Inexact(n as usize),
            RowEstimate::Unknown => Precision::Absent,
        };

        let column_statistics = self
            .get_schema()
            .fields
            .iter()
            .map(|_| ColumnStatistics::default())
            .collect();

        Some(Statistics {
            num_rows,
            total_byte_size: Precision::Absent,
            column_statistics,
        })
    }

    fn supports_filters_pushdown(
        &self,
        filters: &[&Expr],
    ) -> Result<Vec<TableProviderFilterPushDown>> {
        let analyzer = self.analyzer();
        let results = filters
            .iter()
            .map(|filter| {
                if analyzer.supports(filter) {
                    TableProviderFilterPushDown::Exact
                } else {
                    TableProviderFilterPushDown::Unsupported
                }
            })
            .collect();
        Ok(results)
    }

    async fn scan(
        &self,
        _state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        _limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // TODO: We should support limit pushdown here to allow providing a batch size hint
        // to the Scanner.

        let (projected_fields, projected_schema) = self.projected_fields_and_schema(projection)?;
        let heap_relid = self.scan_info.heaprelid;
        let index_relid = self.scan_info.indexrelid;
        let expr_context = self.expr_context;
        let planstate = self.planstate;
        let parallel_state = self.parallel_state;
        let canonical_segment_ids = self.canonical_segment_ids.clone();

        let heap_rel = PgSearchRelation::open(heap_relid);
        let index_rel = PgSearchRelation::open(index_relid);

        // Solve runtime Postgres expressions (prepared-statement params, etc.) here in scan()
        // rather than earlier because each process (leader and workers) independently
        // deserializes the logical plan in its own executor context. The
        // planstate and expr_context are injected by the execution codec.
        let mut query = self.combine_query_with_filters(self.scan_info.query.clone(), filters);
        if query.has_postgres_expressions() {
            let Some(planstate) = planstate else {
                return Err(DataFusionError::Internal(
                    "postgres expressions have not been solved: missing planstate".to_string(),
                ));
            };
            let Some(expr_context) = expr_context else {
                return Err(DataFusionError::Internal(
                    "postgres expressions have not been solved: missing expr_context".to_string(),
                ));
            };
            query.init_postgres_expressions(planstate);
            query.solve_postgres_expressions(expr_context);
        }

        // Determine MVCC strategy based on whether we are running in parallel mode.
        //
        // For the partitioning source (`parallel_state` is Some):
        //   - Leader: Snapshot (claims segments dynamically; its own snapshot is the source
        //             of truth for which segments exist).
        //   - Workers: ParallelWorker(partitioning_segment_ids) – frozen set from leader.
        //
        // For non-partitioning / replicated sources (`canonical_segment_ids` is Some):
        //   - Both leader and workers use ParallelWorker(canonical_ids) so that every
        //     participant opens exactly the same frozen segment set, preventing DocAddress
        //     mismatches when late materialization stores (segment_ord, doc_id) pairs.
        //
        // Serial scans (both fields None): Snapshot.
        let mvcc_style = if let Some(ids) = canonical_segment_ids {
            // Non-partitioning source in a parallel join scan: use the frozen segment list
            // that the leader snapshotted and wrote to shared memory.
            MvccSatisfies::ParallelWorker(ids)
        } else if let Some(parallel_state) = parallel_state {
            unsafe {
                if pg_sys::ParallelWorkerNumber == -1 {
                    // Leader only sees snapshot-visible segments
                    MvccSatisfies::Snapshot
                } else {
                    // Workers see all segments listed in shared state
                    MvccSatisfies::ParallelWorker(list_segment_ids(parallel_state))
                }
            }
        } else {
            MvccSatisfies::Snapshot
        };

        let reader = SearchIndexReader::open_with_context(
            &index_rel,
            query.clone(),
            self.scan_info.score_needed,
            mvcc_style,
            expr_context.and_then(std::ptr::NonNull::new),
            None,
        )
        .map_err(|e| DataFusionError::Internal(format!("Failed to open reader: {e}")))?;

        let ffhelper = FFHelper::with_fields(&reader, &projected_fields);
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let visibility = VisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);
        let sort_order = self.scan_info.sort_order.as_ref();

        if let Some(sort_order) = sort_order {
            if let Some(parallel_state) = parallel_state {
                // In joinscan, the "partitioning source" (the first source) uses the throttled checkout
                // strategy to dynamically claim segments.
                self.create_throttled_scan(
                    parallel_state,
                    &reader,
                    &projected_fields,
                    ffhelper,
                    visibility,
                    heap_relid,
                    projected_schema,
                    query,
                    Some(sort_order),
                )
            } else {
                // Serial sorted scan (or replicated source with sorting)
                self.create_eager_scan(
                    &reader,
                    &projected_fields,
                    ffhelper,
                    visibility,
                    heap_relid,
                    projected_schema,
                    query,
                    Some(sort_order),
                )
            }
        } else {
            // When parallel execution is planned, we expect `estimated_rows_per_worker` to be explicitly computed.
            // For serial execution, it will also be computed (divided by 1).
            let total_estimated_rows = self.scan_info.estimated_rows_per_worker.unwrap_or_else(|| {
                panic!("PgSearchTableProvider requires `estimated_rows_per_worker` to be explicitly set during planning");
            });

            self.create_lazy_scan(
                parallel_state,
                &reader,
                &projected_fields,
                ffhelper,
                visibility,
                heap_relid,
                projected_schema,
                query,
                total_estimated_rows,
            )
        }
    }
}

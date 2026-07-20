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

use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use arrow_schema::SchemaRef;
use async_trait::async_trait;
use datafusion::catalog::{Session, TableProvider};
use datafusion::common::stats::{ColumnStatistics, Precision};
use datafusion::common::{DataFusionError, Result, Statistics};
use datafusion::logical_expr::{Expr, TableProviderFilterPushDown, TableType};
use datafusion::physical_plan::ExecutionPlan;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};

use crate::api::HashSet;
use crate::index::fast_fields_helper::{CanonicalColumn, FFHelper, WhichFastField};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::ParallelScanState;
use crate::query::SearchQueryInput;
use crate::scan::execution_plan::{PgSearchScanPlan, ScanState};
use crate::scan::filter_pushdown::{combine_with_and, FilterAnalyzer};
use crate::scan::info::{RowEstimate, ScanInfo};
use crate::scan::late_materialization::DeferredField;

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
    schema: OnceLock<SchemaRef>,
    #[serde(skip)]
    late_materialization_schema: OnceLock<SchemaRef>,
    /// Parallel state is skipped during serialization because it's a raw pointer
    /// to shared memory that is only valid in the current process. It is
    /// re-injected by the execution codec during deserialization if
    /// `source_idx` is some.
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

    /// Source position in the unified-sources array. When set, the codec's
    /// `parallel_state` routes per-source claims via
    /// `checkout_segment_for_source(source_idx)`. `None` for serial scans.
    source_idx: Option<usize>,
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
    pub fn new(
        scan_info: ScanInfo,
        fields: Vec<WhichFastField>,
        source_idx: Option<usize>,
    ) -> Self {
        Self {
            scan_info,
            fields,
            schema: OnceLock::new(),
            late_materialization_schema: OnceLock::new(),
            parallel_state: None,
            expr_context: None,
            planstate: None,
            visibility_mode: VisibilityMode::Eager,
            late_materialization_active: AtomicBool::new(false),
            source_idx,
        }
    }

    /// Transitions the provider from Phase 1 (`Utf8View`) into Phase 2 (`Union`)
    pub fn enable_late_materialization_schema(&self) {
        self.late_materialization_active
            .store(true, Ordering::Relaxed);
    }

    pub(crate) fn set_parallel_state(&mut self, parallel_state: Option<*mut ParallelScanState>) {
        // Parallel scans need `parallel_state` to claim segments.
        assert!(self.source_idx.is_some());
        self.parallel_state = parallel_state;
    }

    pub(crate) fn set_expr_context(&mut self, expr_context: Option<*mut pg_sys::ExprContext>) {
        self.expr_context = expr_context;
    }

    pub(crate) fn set_planstate(&mut self, planstate: Option<*mut pg_sys::PlanState>) {
        self.planstate = planstate;
    }

    pub(crate) fn source_idx(&self) -> Option<usize> {
        self.source_idx
    }

    fn enable_deferred_columns(&mut self, required_early_columns: &HashSet<String>) {
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
        required_early_columns: &HashSet<String>,
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
                    // Resolvable from any fragment: reads the segment list from
                    // the worker's `ParallelScanState` (claiming only divides the scan, not a
                    // reader opened over the whole list).
                    rebuild: Some(crate::scan::late_materialization::DeferredLookupRebuild {
                        field_name: name.clone(),
                        field_type: *field_type,
                        source_idx: self.source_idx,
                    }),
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

    /// Creates a PgSearchScanPlan from a list of segments.
    #[allow(clippy::too_many_arguments)]
    fn create_scan(
        &self,
        segments: Vec<ScanState>,
        schema: SchemaRef,
        resolved_query: SearchQueryInput,
        ffhelper: Arc<FFHelper>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let deferred = self.deferred_fields();
        let deferred_ctid_plan_position = self.deferred_ctid_plan_position();

        // Expose one shared FFHelper when this scan participates in either
        // late materialization or deferred visibility. Downstream callers
        // decide whether they should use it by checking the deferred metadata.
        let ffhelper_arg = if deferred.is_empty() && deferred_ctid_plan_position.is_none() {
            None
        } else {
            Some(ffhelper)
        };

        Ok(Arc::new(PgSearchScanPlan::new(
            segments,
            schema,
            resolved_query,
            None,
            deferred,
            ffhelper_arg,
            self.scan_info.indexrelid.to_u32(),
            deferred_ctid_plan_position,
        )))
    }

    /// Creates a single-partition `PgSearchScanPlan` for lazy scans.
    ///
    /// Exposes exactly 1 partition natively and chains segments end-to-end.
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
        which_fast_fields: Vec<WhichFastField>,
        ffhelper: FFHelper,
        visibility: VisibilityChecker,
        heap_relid: pg_sys::Oid,
        schema: SchemaRef,
        resolved_query: SearchQueryInput,
        planner_estimated_rows: u64,
        source_idx: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let ffhelper = Arc::new(ffhelper);
        let scanner_config = crate::scan::execution_plan::ScannerConfig {
            which_fast_fields: which_fast_fields.clone(),
            heap_relid: heap_relid.into(),
            batch_size_hint: None,
            score_needed: self.scan_info.score_needed,
        };
        let recipe = crate::scan::execution_plan::ScanRecipe::Lazy {
            parallel_state,
            source_idx,
            planner_estimated_rows,
            scanner_config,
        };
        let state = ScanState {
            recipe,
            ffhelper: ffhelper.clone(),
            visibility: Box::new(visibility) as Box<VisibilityChecker>,
            reader: reader.clone(),
        };

        self.create_scan(vec![state], schema, resolved_query, ffhelper)
    }
}

#[async_trait]
impl TableProvider for PgSearchTableProvider {
    fn schema(&self) -> SchemaRef {
        self.get_schema()
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    fn statistics(&self) -> Option<Statistics> {
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
        state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        self.scan_inner(state, projection, filters, limit).await
    }
}

impl PgSearchTableProvider {
    async fn scan_inner(
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

        let heap_rel = PgSearchRelation::open(heap_relid);
        let index_rel = PgSearchRelation::open(index_relid);

        // Solve runtime Postgres expressions (prepared-statement params, etc.) here in scan()
        // rather than earlier because each process (leader and workers) independently
        // deserializes the logical plan in its own executor context. The
        // planstate and expr_context are injected by the execution codec.
        let mut query = self.combine_query_with_filters(self.scan_info.query.clone(), filters);
        if query.has_postgres_expressions() || query.has_parameters() {
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
        // MVCC dispatch by `source_idx`:
        //
        // Parallel worker or leader (`source_idx` Some):
        // Retrieve the specific frozen segment IDs for this source from `ParallelScanState`.
        //
        // Serial (otherwise): Snapshot.

        let mvcc_style = if let Some(parallel_state) = parallel_state {
            unsafe {
                if pg_sys::ParallelWorkerNumber == -1 {
                    // Leader only sees snapshot-visible segments
                    MvccSatisfies::Snapshot
                } else {
                    let ids = (*parallel_state).segment_ids_for_source(
                        self.source_idx.expect("parallel_state implies source_idx"),
                    );
                    MvccSatisfies::ParallelWorker(ids)
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
            query.needs_tokenizer(),
        )
        .map_err(|e| DataFusionError::Internal(format!("Failed to open reader: {e}")))?;

        let ffhelper = FFHelper::with_fields(&reader, &projected_fields);
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let visibility = VisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);

        let total_estimated_rows = self.scan_info.estimate.as_planner_estimate();

        self.create_lazy_scan(
            parallel_state,
            &reader,
            projected_fields,
            ffhelper,
            visibility,
            heap_relid,
            projected_schema,
            query,
            total_estimated_rows,
            self.source_idx,
        )
    }
}

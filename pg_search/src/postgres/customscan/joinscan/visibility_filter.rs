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

//! Deferred Visibility Filter for JoinScan.
//!
//! When deferred visibility is enabled, the PgSearchScanPlan emits packed DocAddresses
//! instead of real ctids, and skips per-row visibility checking. After the join (or
//! at a barrier), `TantivyLookupExec` resolves the packed DocAddresses to real ctids,
//! and `VisibilityFilterExec` performs batch visibility checking, filters invisible
//! rows, and replaces ctids with HOT-resolved values.
//!
//! # Architecture
//!
//! 1. `VisibilityFilterOptimizerRule` (logical optimizer) — walks the logical plan
//!    bottom-up and inserts `VisibilityFilterNode` at barrier points (or the plan root).
//! 2. `VisibilityExtensionPlanner` (extension physical planner) — converts
//!    `VisibilityFilterNode` → `VisibilityFilterExec`.
//! 3. `LateMaterializationRule` sees `VisibilityFilterExec` as an anchor, so
//!    `TantivyLookupExec` is inserted just below it to resolve packed DocAddresses.
//! 4. `VisibilityFilterExec` (physical execution) — opens heap relations, creates
//!    `VisibilityChecker` per relation, and filters batches on the resolved ctids.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use arrow_array::{Array, ArrayRef, RecordBatch, UInt64Array};
use arrow_schema::SchemaRef;
use async_trait::async_trait;
use datafusion::common::tree_node::Transformed;
use datafusion::common::{DFSchemaRef, DataFusionError, Result};
use datafusion::execution::{
    RecordBatchStream, SendableRecordBatchStream, SessionState, TaskContext,
};
use datafusion::logical_expr::{Extension, LogicalPlan, UserDefinedLogicalNode};
use datafusion::optimizer::optimizer::ApplyOrder;
use datafusion::optimizer::{OptimizerConfig, OptimizerRule};
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::metrics::{BaselineMetrics, ExecutionPlanMetricsSet, MetricsSet};
use datafusion::physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties};
use datafusion::physical_planner::{DefaultPhysicalPlanner, ExtensionPlanner, PhysicalPlanner};
use futures::Stream;
use pgrx::pg_sys;

use crate::postgres::customscan::joinscan::build::JoinCSClause;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::scan::execution_plan::UnsafeSendStream;

// ---------------------------------------------------------------------------
// Logical Node
// ---------------------------------------------------------------------------

/// A logical node indicating that visibility checking should be applied to the
/// specified range table indexes' ctid columns.
#[derive(Debug, Clone)]
pub struct VisibilityFilterNode {
    pub input: LogicalPlan,
    /// (rti, heap_oid) pairs whose `ctid_{rti}` columns need visibility checking.
    pub rti_oids: Vec<(pg_sys::Index, pg_sys::Oid)>,
    schema: DFSchemaRef,
}

impl VisibilityFilterNode {
    pub fn new(input: LogicalPlan, rti_oids: Vec<(pg_sys::Index, pg_sys::Oid)>) -> Self {
        let schema = input.schema().clone();
        Self {
            input,
            rti_oids,
            schema,
        }
    }
}

// Manual PartialEq / Eq / Hash / PartialOrd for UserDefinedLogicalNode requirements.
impl PartialEq for VisibilityFilterNode {
    fn eq(&self, other: &Self) -> bool {
        self.rti_oids == other.rti_oids && self.input == other.input
    }
}
impl Eq for VisibilityFilterNode {}

impl Hash for VisibilityFilterNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.rti_oids.hash(state);
        self.input.hash(state);
    }
}

impl PartialOrd for VisibilityFilterNode {
    fn partial_cmp(&self, _other: &Self) -> Option<std::cmp::Ordering> {
        // Not meaningfully orderable; satisfy trait requirement.
        None
    }
}

impl datafusion::logical_expr::UserDefinedLogicalNodeCore for VisibilityFilterNode {
    fn name(&self) -> &str {
        "VisibilityFilter"
    }

    fn inputs(&self) -> Vec<&LogicalPlan> {
        vec![&self.input]
    }

    fn schema(&self) -> &DFSchemaRef {
        &self.schema
    }

    fn expressions(&self) -> Vec<datafusion::logical_expr::Expr> {
        vec![]
    }

    fn prevent_predicate_push_down_columns(&self) -> std::collections::HashSet<String> {
        // Prevent predicates on ctid columns from being pushed below this node.
        // Before visibility resolution, ctid columns hold packed DocAddresses
        // (not real ctids), so any predicate referencing them would be incorrect.
        self.rti_oids
            .iter()
            .map(|(rti, _)| format!("ctid_{}", rti))
            .collect()
    }

    fn fmt_for_explain(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "VisibilityFilter: rtis=[{}]",
            self.rti_oids
                .iter()
                .map(|(r, _)| r.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn with_exprs_and_inputs(
        &self,
        _exprs: Vec<datafusion::logical_expr::Expr>,
        mut inputs: Vec<LogicalPlan>,
    ) -> Result<Self> {
        let input = inputs.pop().ok_or_else(|| {
            DataFusionError::Internal("VisibilityFilterNode requires exactly one input".into())
        })?;
        Ok(Self::new(input, self.rti_oids.clone()))
    }

    fn supports_limit_pushdown(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Optimizer Rule (Logical)
// ---------------------------------------------------------------------------

/// Logical optimizer rule that inserts `VisibilityFilterNode` below barrier
/// nodes and at lineage-drop points using per-relation verification state.
#[derive(Debug)]
pub struct VisibilityFilterOptimizerRule {
    join_clause: JoinCSClause,
}

impl VisibilityFilterOptimizerRule {
    pub fn new(join_clause: JoinCSClause) -> Self {
        Self { join_clause }
    }
}

impl OptimizerRule for VisibilityFilterOptimizerRule {
    fn name(&self) -> &str {
        "VisibilityFilterInjection"
    }

    fn apply_order(&self) -> Option<ApplyOrder> {
        // We handle the entire tree in one pass via `rewrite`.
        None
    }

    fn rewrite(
        &self,
        plan: LogicalPlan,
        _config: &dyn OptimizerConfig,
    ) -> Result<Transformed<LogicalPlan>> {
        let mut rti_to_heap_oid = BTreeMap::<pg_sys::Index, pg_sys::Oid>::new();
        let mut base_relations = Vec::new();
        self.join_clause.collect_base_relations(&mut base_relations);
        for base in &base_relations {
            rti_to_heap_oid.insert(base.heap_rti, base.heaprelid);
        }

        if rti_to_heap_oid.is_empty() {
            return Ok(Transformed::no(plan));
        }

        let (result, final_state) = analyze_and_inject(plan, &rti_to_heap_oid)?;

        // Root boundary fallback: any RTI still unverified must be checked here.
        let unverified: BTreeSet<pg_sys::Index> = final_state
            .iter()
            .filter(|(_, s)| **s == VisibilityStatus::Unverified)
            .map(|(rti, _)| *rti)
            .collect();

        if unverified.is_empty() {
            return Ok(result);
        }

        let wrapped = wrap_with_visibility_if_needed(result.data, &unverified, &rti_to_heap_oid)?;
        Ok(Transformed::new_transformed(
            wrapped.data,
            wrapped.transformed || result.transformed,
        ))
    }
}

// ---------------------------------------------------------------------------
// Barrier Detection (for future outer join support)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibilityStatus {
    Unverified,
    Verified,
}

type RelationStates = BTreeMap<pg_sys::Index, VisibilityStatus>;

fn extract_ctid_lineage(schema: &DFSchemaRef) -> BTreeSet<pg_sys::Index> {
    schema
        .fields()
        .iter()
        .filter_map(|field| {
            field
                .name()
                .strip_prefix("ctid_")?
                .parse::<pg_sys::Index>()
                .ok()
        })
        .collect()
}

fn existing_visibility_rtis(plan: &LogicalPlan) -> Option<BTreeSet<pg_sys::Index>> {
    let LogicalPlan::Extension(ext) = plan else {
        return None;
    };
    let vf = ext.node.as_any().downcast_ref::<VisibilityFilterNode>()?;
    Some(vf.rti_oids.iter().map(|(rti, _)| *rti).collect())
}

fn wrap_with_visibility(
    input: LogicalPlan,
    rtis: &BTreeSet<pg_sys::Index>,
    rti_to_heap_oid: &BTreeMap<pg_sys::Index, pg_sys::Oid>,
) -> Result<LogicalPlan> {
    let mut rti_oids = Vec::with_capacity(rtis.len());
    for &rti in rtis {
        let heap_oid = rti_to_heap_oid.get(&rti).ok_or_else(|| {
            DataFusionError::Internal(format!(
                "VisibilityFilterInjection: missing heap oid for rti {}",
                rti
            ))
        })?;
        rti_oids.push((rti, *heap_oid));
    }

    Ok(LogicalPlan::Extension(Extension {
        node: Arc::new(VisibilityFilterNode::new(input, rti_oids)),
    }))
}

fn wrap_with_visibility_if_needed(
    input: LogicalPlan,
    rtis: &BTreeSet<pg_sys::Index>,
    rti_to_heap_oid: &BTreeMap<pg_sys::Index, pg_sys::Oid>,
) -> Result<Transformed<LogicalPlan>> {
    if rtis.is_empty() {
        return Ok(Transformed::no(input));
    }

    if let Some(existing) = existing_visibility_rtis(&input) {
        let missing: BTreeSet<pg_sys::Index> = rtis.difference(&existing).copied().collect();
        if missing.is_empty() {
            return Ok(Transformed::no(input));
        }
        let wrapped = wrap_with_visibility(input, &missing, rti_to_heap_oid)?;
        return Ok(Transformed::yes(wrapped));
    }

    let wrapped = wrap_with_visibility(input, rtis, rti_to_heap_oid)?;
    Ok(Transformed::yes(wrapped))
}

fn analyze_and_inject(
    plan: LogicalPlan,
    rti_to_heap_oid: &BTreeMap<pg_sys::Index, pg_sys::Oid>,
) -> Result<(Transformed<LogicalPlan>, RelationStates)> {
    // 1) Recurse into children and collect per-child relation states.
    let children: Vec<LogicalPlan> = plan.inputs().into_iter().cloned().collect();
    let mut new_children = Vec::with_capacity(children.len());
    let mut child_states = Vec::with_capacity(children.len());
    let mut any_modified = false;

    for child in children {
        let (result, state) = analyze_and_inject(child, rti_to_heap_oid)?;
        any_modified |= result.transformed;
        new_children.push(result.data);
        child_states.push(state);
    }

    // 2) Leaf: infer RTIs from ctid lineage and mark unverified.
    if new_children.is_empty() {
        let mut leaf_state = RelationStates::new();
        for rti in extract_ctid_lineage(plan.schema()) {
            if rti_to_heap_oid.contains_key(&rti) {
                leaf_state.insert(rti, VisibilityStatus::Unverified);
            }
        }
        return Ok((Transformed::new_transformed(plan, any_modified), leaf_state));
    }

    // 3) Merge child states (Unverified wins).
    let mut merged = RelationStates::new();
    for child_state in &child_states {
        for (&rti, &status) in child_state {
            let entry = merged.entry(rti).or_insert(status);
            if status == VisibilityStatus::Unverified {
                *entry = VisibilityStatus::Unverified;
            }
        }
    }

    // 3b) Recognize existing VisibilityFilter nodes so repeated optimizer
    // passes do not keep re-wrapping.
    if let LogicalPlan::Extension(ext) = &plan {
        if let Some(vf) = ext.node.as_any().downcast_ref::<VisibilityFilterNode>() {
            for &(rti, _) in &vf.rti_oids {
                merged.insert(rti, VisibilityStatus::Verified);
            }
        }
    }

    // 4) Determine ctid lineage at this node output.
    let parent_lineage: BTreeSet<pg_sys::Index> = extract_ctid_lineage(plan.schema())
        .into_iter()
        .filter(|rti| rti_to_heap_oid.contains_key(rti))
        .collect();

    // If lineage appears first at this node (e.g. projection alias), mark it unverified.
    for &rti in &parent_lineage {
        merged.entry(rti).or_insert(VisibilityStatus::Unverified);
    }

    // 5) Decide which RTIs must be checked at this boundary.
    let needs_barrier = is_barrier(&plan);
    let lineage_dropped: BTreeSet<pg_sys::Index> = merged
        .iter()
        .filter(|(rti, status)| {
            **status == VisibilityStatus::Unverified && !parent_lineage.contains(rti)
        })
        .map(|(rti, _)| *rti)
        .collect();
    let force_rtis: BTreeSet<pg_sys::Index> = if needs_barrier {
        merged
            .iter()
            .filter(|(_, status)| **status == VisibilityStatus::Unverified)
            .map(|(rti, _)| *rti)
            .collect()
    } else {
        lineage_dropped
    };

    if !force_rtis.is_empty() {
        // Insert per-child visibility wrappers only for the RTIs that flow through that child.
        let wrapped_children: Vec<Transformed<LogicalPlan>> = new_children
            .into_iter()
            .enumerate()
            .map(|(i, child)| {
                let child_lineage: BTreeSet<pg_sys::Index> = child_states
                    .get(i)
                    .map(|cs| cs.keys().copied().collect())
                    .unwrap_or_default();
                let to_check: BTreeSet<pg_sys::Index> = force_rtis
                    .iter()
                    .filter(|rti| child_lineage.contains(rti))
                    .copied()
                    .collect();
                if to_check.is_empty() {
                    Ok(Transformed::no(child))
                } else {
                    wrap_with_visibility_if_needed(child, &to_check, rti_to_heap_oid)
                }
            })
            .collect::<Result<Vec<_>>>()?;
        let wrapped_any = wrapped_children.iter().any(|child| child.transformed);
        let wrapped_children: Vec<LogicalPlan> = wrapped_children
            .into_iter()
            .map(|child| child.data)
            .collect();

        for rti in &force_rtis {
            merged.insert(*rti, VisibilityStatus::Verified);
        }

        if wrapped_any || any_modified {
            let new_plan = plan.with_new_exprs(plan.expressions(), wrapped_children)?;
            return Ok((Transformed::yes(new_plan), merged));
        }
        return Ok((Transformed::no(plan), merged));
    }

    // 6) No insertion here; just propagate child rewrites.
    if any_modified {
        let new_plan = plan.with_new_exprs(plan.expressions(), new_children)?;
        Ok((Transformed::yes(new_plan), merged))
    } else {
        Ok((Transformed::no(plan), merged))
    }
}

/// Returns true if the given plan node is a "barrier" — a point where visibility
/// must be checked before proceeding. Used for outer joins where visibility must
/// be applied per-side before the join.
fn is_barrier(plan: &LogicalPlan) -> bool {
    use datafusion::logical_expr::logical_plan::*;
    matches!(
        plan,
        LogicalPlan::Limit(_)
            | LogicalPlan::Aggregate(_)
            | LogicalPlan::Distinct(_)
            | LogicalPlan::Window(_)
    ) || match plan {
        LogicalPlan::Sort(sort) => sort.fetch.is_some(),
        LogicalPlan::Join(join) => !matches!(join.join_type, datafusion::common::JoinType::Inner),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Extension Planner (Logical → Physical)
// ---------------------------------------------------------------------------

/// Converts `VisibilityFilterNode` into `VisibilityFilterExec`.
pub struct VisibilityExtensionPlanner {
    snapshot: pg_sys::Snapshot,
}

// SAFETY: VisibilityExtensionPlanner holds a raw pg_sys::Snapshot pointer.
// This is safe because the snapshot is valid for the entire transaction lifetime
// and pg_search runs DataFusion on a single-threaded Tokio runtime within the
// Postgres backend process — the pointer never crosses thread boundaries.
unsafe impl Send for VisibilityExtensionPlanner {}
unsafe impl Sync for VisibilityExtensionPlanner {}

impl VisibilityExtensionPlanner {
    pub fn new(snapshot: pg_sys::Snapshot) -> Self {
        Self { snapshot }
    }
}

impl fmt::Debug for VisibilityExtensionPlanner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VisibilityExtensionPlanner").finish()
    }
}

#[async_trait]
impl ExtensionPlanner for VisibilityExtensionPlanner {
    async fn plan_extension(
        &self,
        _planner: &dyn PhysicalPlanner,
        node: &dyn UserDefinedLogicalNode,
        _logical_inputs: &[&LogicalPlan],
        physical_inputs: &[Arc<dyn ExecutionPlan>],
        _session_state: &SessionState,
    ) -> Result<Option<Arc<dyn ExecutionPlan>>> {
        let Some(vis_node) = node.as_any().downcast_ref::<VisibilityFilterNode>() else {
            return Ok(None);
        };

        let input = physical_inputs.first().ok_or_else(|| {
            DataFusionError::Internal("VisibilityFilterExec requires exactly one input".into())
        })?;
        let exec =
            VisibilityFilterExec::new(input.clone(), vis_node.rti_oids.clone(), self.snapshot)?;
        Ok(Some(Arc::new(exec)))
    }
}

// ---------------------------------------------------------------------------
// Query Planner wrapper
// ---------------------------------------------------------------------------

/// Wraps `DefaultPhysicalPlanner` (which has our ExtensionPlanner) and implements
/// `QueryPlanner` so it can be set on the SessionState.
pub struct VisibilityQueryPlanner {
    pub phys_planner: DefaultPhysicalPlanner,
}

impl fmt::Debug for VisibilityQueryPlanner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VisibilityQueryPlanner").finish()
    }
}

impl datafusion::execution::context::QueryPlanner for VisibilityQueryPlanner {
    fn create_physical_plan<'life0, 'life1, 'life2, 'async_trait>(
        &'life0 self,
        logical_plan: &'life1 LogicalPlan,
        session_state: &'life2 SessionState,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Arc<dyn ExecutionPlan>>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            self.phys_planner
                .create_physical_plan(logical_plan, session_state)
                .await
        })
    }
}

// ---------------------------------------------------------------------------
// Physical Execution Plan
// ---------------------------------------------------------------------------

/// Physical plan node that performs batch visibility checking on ctid columns.
///
/// For each `rti` in `rtis_to_check`, it:
/// 1. Reads the `ctid_{rti}` column from the input batch
/// 2. Runs `VisibilityChecker::check_batch()` to determine visible rows
/// 3. Filters the batch to only visible rows
/// 4. Replaces ctid values with HOT-resolved ctids
pub struct VisibilityFilterExec {
    input: Arc<dyn ExecutionPlan>,
    /// (rti, heap_oid) pairs for visibility checking.
    rti_oids: Vec<(pg_sys::Index, pg_sys::Oid)>,
    snapshot: pg_sys::Snapshot,
    properties: PlanProperties,
    metrics: ExecutionPlanMetricsSet,
}

// SAFETY: VisibilityFilterExec holds a raw pg_sys::Snapshot pointer and
// (rti, Oid) pairs. This is safe because the snapshot is valid for the entire
// transaction lifetime and pg_search runs DataFusion on a single-threaded
// Tokio runtime within the Postgres backend process — the pointer never
// crosses real thread boundaries.
unsafe impl Send for VisibilityFilterExec {}
unsafe impl Sync for VisibilityFilterExec {}

impl fmt::Debug for VisibilityFilterExec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VisibilityFilterExec")
            .field(
                "rtis",
                &self.rti_oids.iter().map(|(r, _)| *r).collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl VisibilityFilterExec {
    pub fn new(
        input: Arc<dyn ExecutionPlan>,
        rti_oids: Vec<(pg_sys::Index, pg_sys::Oid)>,
        snapshot: pg_sys::Snapshot,
    ) -> Result<Self> {
        // Visibility filtering only removes rows — it never reorders them.
        // Forward the input's equivalence properties so DataFusion knows
        // sort order is preserved (avoids unnecessary re-sorts).
        let properties = PlanProperties::new(
            input.properties().equivalence_properties().clone(),
            input.properties().output_partitioning().clone(),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );
        Ok(Self {
            input,
            rti_oids,
            snapshot,
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
        })
    }
}

impl DisplayAs for VisibilityFilterExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "VisibilityFilterExec: rtis=[{}]",
            self.rti_oids
                .iter()
                .map(|(r, _)| r.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl ExecutionPlan for VisibilityFilterExec {
    fn name(&self) -> &str {
        "VisibilityFilterExec"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn metrics(&self) -> Option<MetricsSet> {
        Some(self.metrics.clone_inner())
    }

    fn properties(&self) -> &PlanProperties {
        &self.properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![&self.input]
    }

    fn with_new_children(
        self: Arc<Self>,
        mut children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let child = children.pop().ok_or_else(|| {
            DataFusionError::Internal("VisibilityFilterExec requires exactly one child".into())
        })?;
        Ok(Arc::new(VisibilityFilterExec::new(
            child,
            self.rti_oids.clone(),
            self.snapshot,
        )?))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let input_stream = self.input.execute(partition, context)?;
        let schema = self.schema();

        // Open heap relations and create visibility checkers.
        let mut checkers: Vec<(pg_sys::Index, usize, VisibilityChecker)> =
            Vec::with_capacity(self.rti_oids.len());
        for &(rti, heap_oid) in &self.rti_oids {
            let col_name = format!("ctid_{}", rti);
            let (col_idx, _) = schema.column_with_name(&col_name).ok_or_else(|| {
                DataFusionError::Execution(format!(
                    "VisibilityFilterExec: missing ctid column '{}'",
                    col_name
                ))
            })?;
            let heaprel = PgSearchRelation::open(heap_oid);
            let visibility = VisibilityChecker::with_rel_and_snap(&heaprel, self.snapshot);
            checkers.push((rti, col_idx, visibility));
        }

        let baseline_metrics = BaselineMetrics::new(&self.metrics, partition);
        // SAFETY: VisibilityFilterStream contains VisibilityChecker (holding raw
        // Postgres relation and snapshot pointers). These are safe because we run
        // on a single-threaded Tokio runtime within the Postgres backend process.
        let stream = unsafe {
            UnsafeSendStream::new(VisibilityFilterStream {
                input: input_stream,
                schema,
                checkers,
                baseline_metrics,
            })
        };
        Ok(Box::pin(stream))
    }
}

// ---------------------------------------------------------------------------
// Stream implementation
// ---------------------------------------------------------------------------

struct VisibilityFilterStream {
    input: SendableRecordBatchStream,
    schema: SchemaRef,
    checkers: Vec<(pg_sys::Index, usize, VisibilityChecker)>,
    baseline_metrics: BaselineMetrics,
}

impl VisibilityFilterStream {
    /// Runs visibility check for a single relation's ctid column.
    /// Returns HOT-resolved ctids (None for invisible/null rows).
    fn check_column_visibility(
        checker: &mut VisibilityChecker,
        ctid_array: &UInt64Array,
        num_rows: usize,
    ) -> Vec<Option<u64>> {
        let ctids: Vec<Option<u64>> = (0..num_rows)
            .map(|i| {
                if ctid_array.is_null(i) {
                    None
                } else {
                    Some(ctid_array.value(i))
                }
            })
            .collect();

        let mut results = vec![None; num_rows];

        if ctid_array.null_count() == 0 {
            checker.check_batch(&ctids, &mut results);
            return results;
        }

        // check_batch expects all Some, so filter out nulls, call on
        // the non-null subset, then merge back.
        let non_null_indices: Vec<usize> = ctids
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.as_ref().map(|_| i))
            .collect();

        if non_null_indices.is_empty() {
            return results;
        }

        let subset_ctids: Vec<Option<u64>> = non_null_indices.iter().map(|&i| ctids[i]).collect();
        let mut subset_results = vec![None; subset_ctids.len()];
        checker.check_batch(&subset_ctids, &mut subset_results);
        for (j, &orig_idx) in non_null_indices.iter().enumerate() {
            results[orig_idx] = subset_results[j];
        }
        results
    }

    fn filter_batch(&mut self, batch: RecordBatch) -> Result<RecordBatch> {
        if batch.num_rows() == 0 {
            return Ok(batch);
        }

        let num_rows = batch.num_rows();

        // Start with all rows visible.
        let mut visible_mask = vec![true; num_rows];

        // For each rti, check visibility and collect HOT-resolved ctids.
        let mut resolved_ctids: Vec<(usize, Vec<Option<u64>>)> =
            Vec::with_capacity(self.checkers.len());

        for (_, col_idx, checker) in &mut self.checkers {
            let ctid_array = batch
                .column(*col_idx)
                .as_any()
                .downcast_ref::<UInt64Array>()
                .ok_or_else(|| {
                    DataFusionError::Execution(
                        "VisibilityFilterExec: ctid column is not UInt64".into(),
                    )
                })?;

            let results = Self::check_column_visibility(checker, ctid_array, num_rows);

            // Update visible_mask: rows with None result are invisible.
            for (i, result) in results.iter().enumerate() {
                if result.is_none() {
                    visible_mask[i] = false;
                }
            }

            resolved_ctids.push((*col_idx, results));
        }

        // Count visible rows.
        let visible_count = visible_mask.iter().filter(|&&v| v).count();
        if visible_count == 0 {
            // All rows filtered — return empty batch.
            let empty_cols: Vec<ArrayRef> = self
                .schema
                .fields()
                .iter()
                .map(|f| arrow_array::new_null_array(f.data_type(), 0))
                .collect();
            return RecordBatch::try_new(self.schema.clone(), empty_cols)
                .map_err(|e| DataFusionError::ArrowError(Box::new(e), None));
        }

        if visible_count == num_rows {
            // All rows visible — just replace ctids with HOT-resolved values.
            return self.replace_ctids(batch, &resolved_ctids);
        }

        // Build indices of visible rows.
        let visible_indices: Vec<usize> = visible_mask
            .iter()
            .enumerate()
            .filter(|(_, &v)| v)
            .map(|(i, _)| i)
            .collect();

        // Filter each column.
        let mut new_columns: Vec<ArrayRef> = Vec::with_capacity(batch.num_columns());
        let indices_array = arrow_array::UInt32Array::from(
            visible_indices
                .iter()
                .map(|&i| i as u32)
                .collect::<Vec<_>>(),
        );

        for col_idx in 0..batch.num_columns() {
            // Check if this is a ctid column that needs HOT resolution.
            if let Some((_, resolved)) = resolved_ctids.iter().find(|(ci, _)| *ci == col_idx) {
                // Build resolved ctid array for visible rows only.
                let mut builder = arrow_array::builder::UInt64Builder::with_capacity(visible_count);
                for &row_idx in &visible_indices {
                    match resolved[row_idx] {
                        Some(ctid) => builder.append_value(ctid),
                        None => builder.append_null(),
                    }
                }
                new_columns.push(Arc::new(builder.finish()) as ArrayRef);
            } else {
                let col = batch.column(col_idx);
                let filtered = arrow_select::take::take(col.as_ref(), &indices_array, None)
                    .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
                new_columns.push(filtered);
            }
        }

        RecordBatch::try_new(self.schema.clone(), new_columns)
            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
    }

    /// Replace ctid columns with HOT-resolved values (all rows visible).
    fn replace_ctids(
        &self,
        batch: RecordBatch,
        resolved_ctids: &[(usize, Vec<Option<u64>>)],
    ) -> Result<RecordBatch> {
        let mut columns: Vec<ArrayRef> = batch.columns().to_vec();
        for (col_idx, resolved) in resolved_ctids {
            let mut builder = arrow_array::builder::UInt64Builder::with_capacity(batch.num_rows());
            for val in resolved {
                match val {
                    Some(ctid) => builder.append_value(*ctid),
                    None => builder.append_null(),
                }
            }
            columns[*col_idx] = Arc::new(builder.finish()) as ArrayRef;
        }
        RecordBatch::try_new(self.schema.clone(), columns)
            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
    }
}

impl Stream for VisibilityFilterStream {
    type Item = Result<RecordBatch>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let poll = Pin::new(&mut self.input).poll_next(cx);
        let final_poll = match poll {
            Poll::Ready(Some(Ok(batch))) => {
                let result = self.filter_batch(batch);
                Poll::Ready(Some(result))
            }
            other => other,
        };
        self.baseline_metrics.record_poll(final_poll)
    }
}

impl RecordBatchStream for VisibilityFilterStream {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::logical_expr::{col, lit, LogicalPlanBuilder};
    use datafusion::optimizer::optimizer::OptimizerContext;

    use crate::postgres::customscan::joinscan::build::JoinSource;
    use crate::scan::ScanInfo;

    const TEST_RTI: pg_sys::Index = 1;

    fn make_rule() -> VisibilityFilterOptimizerRule {
        let test_heap_oid = pg_sys::Oid::from(42);
        let source = JoinSource {
            scan_info: ScanInfo {
                heap_rti: TEST_RTI,
                heaprelid: test_heap_oid,
                ..Default::default()
            },
        };
        VisibilityFilterOptimizerRule::new(JoinCSClause::new().add_source(source))
    }

    fn make_ctid_plan() -> Result<LogicalPlan> {
        LogicalPlanBuilder::values(vec![vec![lit(1_u64)]])?
            .project(vec![col("column1").alias(format!("ctid_{}", TEST_RTI))])?
            .build()
    }

    fn count_visibility_nodes(plan: &LogicalPlan) -> usize {
        let current = match plan {
            LogicalPlan::Extension(ext)
                if ext
                    .node
                    .as_any()
                    .downcast_ref::<VisibilityFilterNode>()
                    .is_some() =>
            {
                1
            }
            _ => 0,
        };
        current
            + plan
                .inputs()
                .iter()
                .map(|child| count_visibility_nodes(child))
                .sum::<usize>()
    }

    fn limit_child_is_visibility(plan: &LogicalPlan) -> bool {
        let LogicalPlan::Limit(limit) = plan else {
            return false;
        };
        let LogicalPlan::Extension(ext) = limit.input.as_ref() else {
            return false;
        };
        ext.node
            .as_any()
            .downcast_ref::<VisibilityFilterNode>()
            .is_some()
    }

    #[test]
    fn root_injection_is_idempotent() -> Result<()> {
        let config = OptimizerContext::new();
        let rule = make_rule();
        let plan = make_ctid_plan()?;

        let first = rule.rewrite(plan, &config)?;
        assert!(first.transformed);
        assert_eq!(count_visibility_nodes(&first.data), 1);

        let second = rule.rewrite(first.data.clone(), &config)?;
        assert!(!second.transformed);
        assert_eq!(count_visibility_nodes(&second.data), 1);
        assert_eq!(first.data, second.data);
        Ok(())
    }

    #[test]
    fn inserts_visibility_below_limit_barrier() -> Result<()> {
        let config = OptimizerContext::new();
        let rule = make_rule();
        let plan = LogicalPlanBuilder::from(make_ctid_plan()?)
            .limit(0, Some(5))?
            .build()?;

        let first = rule.rewrite(plan, &config)?;
        assert!(first.transformed);
        assert!(limit_child_is_visibility(&first.data));
        assert_eq!(count_visibility_nodes(&first.data), 1);

        let second = rule.rewrite(first.data.clone(), &config)?;
        assert!(!second.transformed);
        assert!(limit_child_is_visibility(&second.data));
        assert_eq!(count_visibility_nodes(&second.data), 1);
        assert_eq!(first.data, second.data);
        Ok(())
    }

    /// Returns true if the first child of a barrier node is a VisibilityFilterNode.
    fn first_child_is_visibility(plan: &LogicalPlan) -> bool {
        let children = plan.inputs();
        let Some(child) = children.first() else {
            return false;
        };
        let LogicalPlan::Extension(ext) = child else {
            return false;
        };
        ext.node
            .as_any()
            .downcast_ref::<VisibilityFilterNode>()
            .is_some()
    }

    /// Helper to assert barrier injection + idempotency.
    fn assert_barrier_injection(plan: LogicalPlan) -> Result<()> {
        let config = OptimizerContext::new();
        let rule = make_rule();

        let first = rule.rewrite(plan, &config)?;
        assert!(first.transformed, "first pass should transform");
        assert_eq!(count_visibility_nodes(&first.data), 1);
        assert!(
            first_child_is_visibility(&first.data),
            "visibility should be inserted below barrier"
        );

        let second = rule.rewrite(first.data.clone(), &config)?;
        assert!(!second.transformed, "second pass should be idempotent");
        assert_eq!(count_visibility_nodes(&second.data), 1);
        assert_eq!(first.data, second.data);
        Ok(())
    }

    #[test]
    fn inserts_visibility_below_aggregate_barrier() -> Result<()> {
        use datafusion::functions_aggregate::count::count;
        let plan = LogicalPlanBuilder::from(make_ctid_plan()?)
            .aggregate(
                Vec::<datafusion::logical_expr::Expr>::new(),
                vec![count(col(format!("ctid_{}", TEST_RTI)))],
            )?
            .build()?;
        assert_barrier_injection(plan)
    }

    #[test]
    fn inserts_visibility_below_distinct_barrier() -> Result<()> {
        let plan = LogicalPlanBuilder::from(make_ctid_plan()?)
            .distinct()?
            .build()?;
        assert_barrier_injection(plan)
    }

    #[test]
    fn inserts_visibility_below_sort_with_fetch_barrier() -> Result<()> {
        let plan = LogicalPlanBuilder::from(make_ctid_plan()?)
            .sort_with_limit(
                vec![col(format!("ctid_{}", TEST_RTI)).sort(true, false)],
                Some(10),
            )?
            .build()?;
        assert_barrier_injection(plan)
    }

    #[test]
    fn sort_without_fetch_is_not_barrier() -> Result<()> {
        let config = OptimizerContext::new();
        let rule = make_rule();
        let plan = LogicalPlanBuilder::from(make_ctid_plan()?)
            .sort(vec![col(format!("ctid_{}", TEST_RTI)).sort(true, false)])?
            .build()?;

        let result = rule.rewrite(plan, &config)?;
        assert!(result.transformed);
        // Visibility should be at root, not below the sort.
        assert_eq!(count_visibility_nodes(&result.data), 1);
        assert!(!first_child_is_visibility(&result.data));
        Ok(())
    }

    #[test]
    fn multi_relation_join() -> Result<()> {
        let config = OptimizerContext::new();

        const RTI_A: pg_sys::Index = 1;
        const RTI_B: pg_sys::Index = 2;
        let oid_a = pg_sys::Oid::from(42);
        let oid_b = pg_sys::Oid::from(43);

        let rule = VisibilityFilterOptimizerRule::new(
            JoinCSClause::new()
                .add_source(JoinSource {
                    scan_info: ScanInfo {
                        heap_rti: RTI_A,
                        heaprelid: oid_a,
                        ..Default::default()
                    },
                })
                .add_source(JoinSource {
                    scan_info: ScanInfo {
                        heap_rti: RTI_B,
                        heaprelid: oid_b,
                        ..Default::default()
                    },
                }),
        );

        // Build two leaf plans and join them (inner join = not a barrier).
        let left = LogicalPlanBuilder::values(vec![vec![lit(1_u64)]])?
            .project(vec![col("column1").alias(format!("ctid_{}", RTI_A))])?
            .build()?;
        let right = LogicalPlanBuilder::values(vec![vec![lit(2_u64)]])?
            .project(vec![col("column1").alias(format!("ctid_{}", RTI_B))])?
            .build()?;

        let plan = LogicalPlanBuilder::from(left).cross_join(right)?.build()?;

        let first = rule.rewrite(plan, &config)?;
        assert!(first.transformed);
        // Both RTIs should get visibility — single node at root covers both.
        assert_eq!(count_visibility_nodes(&first.data), 1);

        // Extract the VisibilityFilterNode and check it covers both RTIs.
        if let LogicalPlan::Extension(ext) = &first.data {
            let vf = ext
                .node
                .as_any()
                .downcast_ref::<VisibilityFilterNode>()
                .expect("root should be VisibilityFilterNode");
            let rtis: BTreeSet<pg_sys::Index> = vf.rti_oids.iter().map(|(r, _)| *r).collect();
            assert!(rtis.contains(&RTI_A));
            assert!(rtis.contains(&RTI_B));
        } else {
            panic!("expected root to be VisibilityFilterNode");
        }

        // Idempotent.
        let second = rule.rewrite(first.data.clone(), &config)?;
        assert!(!second.transformed);
        assert_eq!(count_visibility_nodes(&second.data), 1);
        Ok(())
    }

    #[test]
    fn left_join_injects_per_side_visibility() -> Result<()> {
        let config = OptimizerContext::new();

        const RTI_A: pg_sys::Index = 1;
        const RTI_B: pg_sys::Index = 2;
        let oid_a = pg_sys::Oid::from(42);
        let oid_b = pg_sys::Oid::from(43);

        let rule = VisibilityFilterOptimizerRule::new(
            JoinCSClause::new()
                .add_source(JoinSource {
                    scan_info: ScanInfo {
                        heap_rti: RTI_A,
                        heaprelid: oid_a,
                        ..Default::default()
                    },
                })
                .add_source(JoinSource {
                    scan_info: ScanInfo {
                        heap_rti: RTI_B,
                        heaprelid: oid_b,
                        ..Default::default()
                    },
                }),
        );

        let left = LogicalPlanBuilder::values(vec![vec![lit(1_u64)]])?
            .project(vec![col("column1").alias(format!("ctid_{}", RTI_A))])?
            .build()?;
        let right = LogicalPlanBuilder::values(vec![vec![lit(2_u64)]])?
            .project(vec![col("column1").alias(format!("ctid_{}", RTI_B))])?
            .build()?;

        // LEFT join is a barrier — visibility must be injected per-side.
        let plan = LogicalPlanBuilder::from(left)
            .join_on(
                right,
                datafusion::common::JoinType::Left,
                vec![col(format!("ctid_{}", RTI_A)).eq(col(format!("ctid_{}", RTI_B)))],
            )?
            .build()?;

        let first = rule.rewrite(plan, &config)?;
        assert!(first.transformed);
        // LEFT join is a barrier, so visibility filters are injected into
        // each child that has unverified RTIs — 2 total.
        assert_eq!(count_visibility_nodes(&first.data), 2);

        // Idempotent.
        let second = rule.rewrite(first.data.clone(), &config)?;
        assert!(!second.transformed);
        assert_eq!(count_visibility_nodes(&second.data), 2);
        Ok(())
    }
}

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
//! at a barrier), `VisibilityFilterExec` resolves the packed DocAddresses to real ctids
//! and performs batch visibility checking, filtering invisible rows and replacing ctids
//! with HOT-resolved values.
//!
//! # Architecture
//!
//! 1. `VisibilityFilterOptimizerRule` (logical optimizer) — walks the logical plan
//!    bottom-up and inserts `VisibilityFilterNode` at barrier points (or the plan root).
//! 2. `VisibilityExtensionPlanner` (extension physical planner) — converts
//!    `VisibilityFilterNode` → `VisibilityFilterExec`.
//! 3. `VisibilityCtidResolverRule` (physical optimizer) — wires FFHelper from
//!    `PgSearchScanPlan` into `VisibilityFilterExec` so it can resolve packed
//!    DocAddresses to real ctids. `TantivyLookupExec` only handles text/bytes.
//! 4. `VisibilityFilterExec` (physical execution) — resolves packed DocAddresses
//!    to real ctids via FFHelper, opens heap relations, creates `VisibilityChecker`
//!    per relation, and filters batches on the resolved ctids.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
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

use crate::index::fast_fields_helper::FFHelper;
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
            .map(|(rti, _)| crate::scan::ctid_column_name(*rti))
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
// Barrier Detection & Visibility State Tracking
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
            // Only match UInt64 fields to avoid misclassifying user columns
            // that happen to be named `ctid_<n>`. Internal ctid columns are
            // always UInt64 (real ctids or packed DocAddresses); no user-facing
            // Postgres type maps to Arrow UInt64.
            if field.data_type() == &arrow_schema::DataType::UInt64 {
                crate::scan::parse_ctid_rti(field.name())
            } else {
                None
            }
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

    // 3) Merge child states — Unverified wins over Verified.
    // The same RTI can appear in multiple children (e.g., a self-join where both
    // sides scan the same table). If one child already verified visibility but
    // the other didn't, we must treat the RTI as unverified: the unverified
    // child's rows still have unchecked packed DocAddresses.
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
    //
    // Two triggers force visibility injection:
    //   a) Barrier nodes (Limit, Aggregate, non-inner Join, etc.) — all unverified
    //      RTIs must be checked before the barrier consumes rows.
    //   b) Lineage drop — if an unverified RTI's ctid column disappears from the
    //      output schema at this node (e.g., a projection drops it), this is our
    //      last chance to check it. Without injection here, no ancestor could
    //      resolve the packed DocAddress because the column no longer exists.
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
/// must be checked before proceeding. Barriers include non-inner joins (semi, outer),
/// aggregates, distinct, window functions, and sort-with-limit.
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

/// Physical plan node that resolves packed DocAddresses and performs batch
/// visibility checking on ctid columns.
///
/// For each `(rti, heap_oid)` in `rti_oids`, it:
/// 1. Resolves packed DocAddresses to real ctids via FFHelper
/// 2. Reads the resolved `ctid_{rti}` column from the batch
/// 3. Runs `VisibilityChecker::check_batch()` to determine visible rows
/// 4. Filters the batch to only visible rows
/// 5. Replaces ctid values with HOT-resolved ctids
pub struct VisibilityFilterExec {
    input: Arc<dyn ExecutionPlan>,
    /// (rti, heap_oid) pairs for visibility checking.
    rti_oids: Vec<(pg_sys::Index, pg_sys::Oid)>,
    snapshot: pg_sys::Snapshot,
    properties: PlanProperties,
    metrics: ExecutionPlanMetricsSet,
    /// Per-RTI FFHelper for resolving packed DocAddresses to real ctids.
    /// Wired by `VisibilityCtidResolverRule` after plan construction.
    ctid_resolvers: Mutex<BTreeMap<pg_sys::Index, Arc<FFHelper>>>,
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
            ctid_resolvers: Mutex::new(BTreeMap::new()),
        })
    }

    /// Wire an FFHelper for resolving packed DocAddresses to real ctids for the given RTI.
    pub fn set_ctid_resolver(&self, rti: pg_sys::Index, ffhelper: Arc<FFHelper>) {
        self.ctid_resolvers
            .lock()
            .expect("ctid_resolvers lock poisoned")
            .insert(rti, ffhelper);
    }

    pub fn rti_oids(&self) -> &[(pg_sys::Index, pg_sys::Oid)] {
        &self.rti_oids
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
        if children.len() != 1 {
            return Err(DataFusionError::Internal(format!(
                "VisibilityFilterExec requires exactly 1 child, got {}",
                children.len()
            )));
        }
        let new_exec =
            VisibilityFilterExec::new(children.remove(0), self.rti_oids.clone(), self.snapshot)?;
        // Transfer ctid resolvers to the new instance.
        {
            let resolvers = self
                .ctid_resolvers
                .lock()
                .expect("ctid_resolvers lock poisoned");
            let mut new_resolvers = new_exec
                .ctid_resolvers
                .lock()
                .expect("ctid_resolvers lock poisoned");
            for (rti, ffhelper) in resolvers.iter() {
                new_resolvers.insert(*rti, Arc::clone(ffhelper));
            }
        }
        Ok(Arc::new(new_exec))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let input_stream = self.input.execute(partition, context)?;
        let schema = self.schema();

        // Snapshot ctid resolvers before building the stream.
        let resolvers = self
            .ctid_resolvers
            .lock()
            .expect("ctid_resolvers lock poisoned")
            .clone();

        // Open heap relations and create visibility checkers.
        let mut checkers: Vec<CtidCheckerEntry> = Vec::with_capacity(self.rti_oids.len());
        for &(rti, heap_oid) in &self.rti_oids {
            let col_name = crate::scan::ctid_column_name(rti);
            let (col_idx, _) = schema.column_with_name(&col_name).ok_or_else(|| {
                DataFusionError::Execution(format!(
                    "VisibilityFilterExec: missing ctid column '{}'",
                    col_name
                ))
            })?;
            let heaprel = PgSearchRelation::open(heap_oid);
            let visibility = VisibilityChecker::with_rel_and_snap(&heaprel, self.snapshot);
            let resolver = resolvers.get(&rti).cloned().ok_or_else(|| {
                DataFusionError::Execution(format!(
                    "VisibilityFilterExec: no ctid resolver wired for rti {rti}. \
                     VisibilityCtidResolverRule must run before execute."
                ))
            })?;
            checkers.push(CtidCheckerEntry {
                col_idx,
                checker: visibility,
                resolver,
            });
        }

        let baseline_metrics = BaselineMetrics::new(&self.metrics, partition);
        // SAFETY: VisibilityFilterStream contains VisibilityChecker (holding raw
        // Postgres relation and snapshot pointers). These are safe because we run
        // on a single-threaded Tokio runtime within the Postgres backend process.
        let stream = unsafe {
            UnsafeSendStream::new(
                VisibilityFilterStream {
                    input: input_stream,
                    schema: schema.clone(),
                    checkers,
                    baseline_metrics,
                },
                schema,
            )
        };
        Ok(Box::pin(stream))
    }
}

// ---------------------------------------------------------------------------
// Stream implementation
// ---------------------------------------------------------------------------

/// Per-RTI state for ctid resolution and visibility checking.
struct CtidCheckerEntry {
    /// Index of the `ctid_{rti}` column in the batch.
    col_idx: usize,
    /// Checks heap visibility for this relation.
    checker: VisibilityChecker,
    /// Resolves packed DocAddresses to real ctids before visibility checking.
    /// Always present: `VisibilityCtidResolverRule` guarantees wiring, and
    /// `execute()` validates at runtime.
    resolver: Arc<FFHelper>,
}

struct VisibilityFilterStream {
    input: SendableRecordBatchStream,
    schema: SchemaRef,
    checkers: Vec<CtidCheckerEntry>,
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
        let mut results = vec![None; num_rows];

        if ctid_array.null_count() == 0 {
            // Fast path: no nulls — pass values directly via shared backing buffer.
            let ctids: Vec<Option<u64>> = ctid_array.values().iter().copied().map(Some).collect();
            checker.check_batch(&ctids, &mut results);
            return results;
        }

        // Slow path: collect non-null indices, check visibility on the subset,
        // then merge back.
        let mut non_null_indices = Vec::new();
        let mut subset_ctids = Vec::new();
        for i in 0..num_rows {
            if !ctid_array.is_null(i) {
                non_null_indices.push(i);
                subset_ctids.push(Some(ctid_array.value(i)));
            }
        }

        if non_null_indices.is_empty() {
            return results;
        }

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

        // Phase 1: Resolve packed DocAddresses to real ctids.
        // Replace ctid columns in-place in the columns vec — no intermediate
        // RecordBatch needed between phases.
        let mut columns: Vec<ArrayRef> = batch.columns().to_vec();
        for entry in &self.checkers {
            let col = &columns[entry.col_idx];
            let doc_addr_array = col.as_any().downcast_ref::<UInt64Array>().ok_or_else(|| {
                DataFusionError::Execution(format!(
                    "VisibilityFilterExec: ctid column (idx {}) is not UInt64 \
                     during DocAddress resolution",
                    entry.col_idx
                ))
            })?;
            let resolved = crate::scan::tantivy_lookup_exec::materialize_deferred_ctid(
                &entry.resolver,
                doc_addr_array,
                num_rows,
            )?;
            columns[entry.col_idx] = resolved;
        }

        // Phase 2: Visibility checking on the (now real) ctids.
        let mut visible_mask = vec![true; num_rows];

        // For each rti, check visibility and collect HOT-resolved ctids.
        let mut resolved_ctids: Vec<(usize, Vec<Option<u64>>)> =
            Vec::with_capacity(self.checkers.len());

        for entry in &mut self.checkers {
            let ctid_array = columns[entry.col_idx]
                .as_any()
                .downcast_ref::<UInt64Array>()
                .ok_or_else(|| {
                    DataFusionError::Execution(format!(
                        "VisibilityFilterExec: ctid column (idx {}) is not UInt64 \
                         during visibility checking",
                        entry.col_idx
                    ))
                })?;

            let results = Self::check_column_visibility(&mut entry.checker, ctid_array, num_rows);

            // Update visible_mask: rows with None result are invisible.
            for (i, result) in results.iter().enumerate() {
                if result.is_none() {
                    visible_mask[i] = false;
                }
            }

            resolved_ctids.push((entry.col_idx, results));
        }

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
            // All rows visible — replace ctid columns with HOT-resolved values.
            for (col_idx, resolved) in &resolved_ctids {
                columns[*col_idx] =
                    crate::scan::tantivy_lookup_exec::uint64_array_from_options(resolved);
            }
            return RecordBatch::try_new(self.schema.clone(), columns)
                .map_err(|e| DataFusionError::ArrowError(Box::new(e), None));
        }

        let visible_indices: Vec<usize> = visible_mask
            .iter()
            .enumerate()
            .filter(|(_, &v)| v)
            .map(|(i, _)| i)
            .collect();

        let mut new_columns: Vec<ArrayRef> = Vec::with_capacity(columns.len());
        let indices_array = arrow_array::UInt32Array::from(
            visible_indices
                .iter()
                .map(|&i| i as u32)
                .collect::<Vec<_>>(),
        );

        for (col_idx, col) in columns.iter().enumerate() {
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
                let filtered = arrow_select::take::take(col.as_ref(), &indices_array, None)
                    .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
                new_columns.push(filtered);
            }
        }

        RecordBatch::try_new(self.schema.clone(), new_columns)
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

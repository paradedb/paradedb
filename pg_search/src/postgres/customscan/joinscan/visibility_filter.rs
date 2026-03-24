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
//! instead of real ctids, and skips per-row visibility checking. This avoids paying
//! heap-access and MVCC visibility costs for rows that will be discarded by the join,
//! LIMIT, DISTINCT, or other downstream operators anyway. After the join (or at a
//! barrier), `VisibilityFilterExec` resolves the packed DocAddresses to real ctids
//! and performs batch visibility checking, filtering invisible rows and replacing ctids
//! with HOT-resolved values.
//!
//! Packed DocAddresses are used because they preserve Tantivy row identity without
//! opening the heap. Looking up a real ctid is exactly the expensive heap work we are
//! trying to defer until we know which joined rows survive.
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
use datafusion::physical_planner::{ExtensionPlanner, PhysicalPlanner};
use futures::Stream;
use pgrx::pg_sys;

use crate::index::fast_fields_helper::FFHelper;
use crate::postgres::customscan::joinscan::build::JoinCSClause;
use crate::postgres::customscan::joinscan::CtidColumn;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::scan::deferred_encode::unpack_doc_address;
use crate::scan::execution_plan::UnsafeSendStream;

// ---------------------------------------------------------------------------
// Logical Node
// ---------------------------------------------------------------------------

/// A logical node indicating that visibility checking should be applied to the
/// specified plan positions' ctid columns.
#[derive(Debug, Clone)]
pub struct VisibilityFilterNode {
    pub input: LogicalPlan,
    /// (plan_position, heap_oid) pairs whose `ctid_{plan_position}` columns need visibility checking.
    pub plan_pos_oids: Vec<(usize, pg_sys::Oid)>,
    schema: DFSchemaRef,
}

impl VisibilityFilterNode {
    pub fn new(input: LogicalPlan, plan_pos_oids: Vec<(usize, pg_sys::Oid)>) -> Self {
        let schema = input.schema().clone();
        Self {
            input,
            plan_pos_oids,
            schema,
        }
    }
}

impl PartialEq for VisibilityFilterNode {
    fn eq(&self, other: &Self) -> bool {
        self.plan_pos_oids == other.plan_pos_oids && self.input == other.input
    }
}
impl Eq for VisibilityFilterNode {}

impl Hash for VisibilityFilterNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.plan_pos_oids.hash(state);
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
        self.plan_pos_oids
            .iter()
            .map(|(plan_pos, _)| CtidColumn::new(*plan_pos).to_string())
            .collect()
    }

    fn fmt_for_explain(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "VisibilityFilter: plan_positions=[{}]",
            self.plan_pos_oids
                .iter()
                .map(|(p, _)| p.to_string())
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
        Ok(Self::new(input, self.plan_pos_oids.clone()))
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
        let mut plan_pos_to_heap_oid = BTreeMap::<usize, pg_sys::Oid>::new();
        for source in self.join_clause.plan.sources() {
            plan_pos_to_heap_oid.insert(source.plan_position, source.scan_info.heaprelid);
        }

        if plan_pos_to_heap_oid.is_empty() {
            return Ok(Transformed::no(plan));
        }

        let (result, final_state) = analyze_and_inject(plan, &plan_pos_to_heap_oid)?;

        // Root boundary fallback: any plan_position still unverified must be checked here.
        let unverified: BTreeSet<usize> = final_state
            .iter()
            .filter(|(_, s)| **s == VisibilityStatus::Unverified)
            .map(|(plan_pos, _)| *plan_pos)
            .collect();

        if unverified.is_empty() {
            return Ok(result);
        }

        let wrapped =
            wrap_with_visibility_if_needed(result.data, &unverified, &plan_pos_to_heap_oid)?;
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

// Ordered containers are used throughout this file so plan_position iteration stays
// deterministic across optimizer rewrites, EXPLAIN output, and test assertions.
type RelationStates = BTreeMap<usize, VisibilityStatus>;

fn extract_ctid_lineage(schema: &DFSchemaRef) -> BTreeSet<usize> {
    schema
        .fields()
        .iter()
        .filter_map(|field| {
            // Only match UInt64 fields to avoid misclassifying user columns
            // that happen to be named `ctid_<n>`. Internal ctid columns are
            // always UInt64 (real ctids or packed DocAddresses); no user-facing
            // Postgres type maps to Arrow UInt64.
            if field.data_type() == &arrow_schema::DataType::UInt64 {
                CtidColumn::try_from(field.name().as_str())
                    .ok()
                    .map(|c| c.plan_position())
            } else {
                None
            }
        })
        .collect()
}

fn existing_visibility_plan_positions(plan: &LogicalPlan) -> Option<BTreeSet<usize>> {
    let LogicalPlan::Extension(ext) = plan else {
        return None;
    };
    let vf = ext.node.as_any().downcast_ref::<VisibilityFilterNode>()?;
    Some(vf.plan_pos_oids.iter().map(|(pp, _)| *pp).collect())
}

fn wrap_with_visibility(
    input: LogicalPlan,
    plan_positions: &BTreeSet<usize>,
    plan_pos_to_heap_oid: &BTreeMap<usize, pg_sys::Oid>,
) -> Result<LogicalPlan> {
    let mut plan_pos_oids = Vec::with_capacity(plan_positions.len());
    for &plan_pos in plan_positions {
        let heap_oid = plan_pos_to_heap_oid.get(&plan_pos).ok_or_else(|| {
            DataFusionError::Internal(format!(
                "VisibilityFilterInjection: missing heap oid for plan_position {}",
                plan_pos
            ))
        })?;
        plan_pos_oids.push((plan_pos, *heap_oid));
    }

    Ok(LogicalPlan::Extension(Extension {
        node: Arc::new(VisibilityFilterNode::new(input, plan_pos_oids)),
    }))
}

fn wrap_with_visibility_if_needed(
    input: LogicalPlan,
    plan_positions: &BTreeSet<usize>,
    plan_pos_to_heap_oid: &BTreeMap<usize, pg_sys::Oid>,
) -> Result<Transformed<LogicalPlan>> {
    if plan_positions.is_empty() {
        return Ok(Transformed::no(input));
    }

    if let Some(existing) = existing_visibility_plan_positions(&input) {
        let missing: BTreeSet<usize> = plan_positions.difference(&existing).copied().collect();
        if missing.is_empty() {
            return Ok(Transformed::no(input));
        }
        let wrapped = wrap_with_visibility(input, &missing, plan_pos_to_heap_oid)?;
        return Ok(Transformed::yes(wrapped));
    }

    let wrapped = wrap_with_visibility(input, plan_positions, plan_pos_to_heap_oid)?;
    Ok(Transformed::yes(wrapped))
}

fn analyze_and_inject(
    plan: LogicalPlan,
    plan_pos_to_heap_oid: &BTreeMap<usize, pg_sys::Oid>,
) -> Result<(Transformed<LogicalPlan>, RelationStates)> {
    // 1) Recurse into children and collect per-child relation states.
    let children: Vec<LogicalPlan> = plan.inputs().into_iter().cloned().collect();
    let mut new_children = Vec::with_capacity(children.len());
    let mut child_states = Vec::with_capacity(children.len());
    let mut any_modified = false;

    for child in children {
        let (result, state) = analyze_and_inject(child, plan_pos_to_heap_oid)?;
        any_modified |= result.transformed;
        new_children.push(result.data);
        child_states.push(state);
    }

    // 2) Leaf: infer plan_positions from ctid lineage and mark unverified.
    if new_children.is_empty() {
        let mut leaf_state = RelationStates::new();
        for plan_pos in extract_ctid_lineage(plan.schema()) {
            if plan_pos_to_heap_oid.contains_key(&plan_pos) {
                leaf_state.insert(plan_pos, VisibilityStatus::Unverified);
            }
        }
        return Ok((Transformed::new_transformed(plan, any_modified), leaf_state));
    }

    // 3) Merge child states. Plan_positions are unique per source, so children
    // never overlap — this is a simple union of disjoint maps.
    let mut merged = RelationStates::new();
    for child_state in &child_states {
        for (&plan_pos, &status) in child_state {
            let entry = merged.entry(plan_pos).or_insert(status);
            if status == VisibilityStatus::Unverified {
                *entry = VisibilityStatus::Unverified;
            }
        }
    }

    // 3b) Recognize existing VisibilityFilter nodes so repeated optimizer
    // passes do not keep re-wrapping.
    if let LogicalPlan::Extension(ext) = &plan {
        if let Some(vf) = ext.node.as_any().downcast_ref::<VisibilityFilterNode>() {
            for &(plan_pos, _) in &vf.plan_pos_oids {
                merged.insert(plan_pos, VisibilityStatus::Verified);
            }
        }
    }

    // 4) Determine ctid lineage at this node output.
    let parent_lineage: BTreeSet<usize> = extract_ctid_lineage(plan.schema())
        .into_iter()
        .filter(|plan_pos| plan_pos_to_heap_oid.contains_key(plan_pos))
        .collect();

    // If lineage appears first at this node (e.g. projection alias), mark it unverified.
    for &plan_pos in &parent_lineage {
        merged
            .entry(plan_pos)
            .or_insert(VisibilityStatus::Unverified);
    }

    // 5) Decide which plan_positions must be checked at this boundary.
    //
    // Two triggers force visibility injection:
    //   a) Barrier nodes (Limit, Aggregate, non-inner Join, etc.) — all unverified
    //      plan_positions must be checked before the barrier consumes rows.
    //   b) Lineage drop — if an unverified plan_position's ctid column disappears from the
    //      output schema at this node (e.g., a projection drops it), this is our
    //      last chance to check it. Without injection here, no ancestor could
    //      resolve the packed DocAddress because the column no longer exists.
    let needs_barrier = is_barrier(&plan);
    let lineage_dropped: BTreeSet<usize> = merged
        .iter()
        .filter(|(plan_pos, status)| {
            **status == VisibilityStatus::Unverified && !parent_lineage.contains(plan_pos)
        })
        .map(|(plan_pos, _)| *plan_pos)
        .collect();
    let force_positions: BTreeSet<usize> = if needs_barrier {
        merged
            .iter()
            .filter(|(_, status)| **status == VisibilityStatus::Unverified)
            .map(|(plan_pos, _)| *plan_pos)
            .collect()
    } else {
        lineage_dropped
    };

    if !force_positions.is_empty() {
        // Insert per-child visibility wrappers only for the plan_positions that flow through that child.
        let wrapped_children: Vec<Transformed<LogicalPlan>> = new_children
            .into_iter()
            .enumerate()
            .map(|(i, child)| {
                let child_lineage: BTreeSet<usize> = child_states
                    .get(i)
                    .map(|cs| cs.keys().copied().collect())
                    .unwrap_or_default();
                let to_check: BTreeSet<usize> = force_positions
                    .iter()
                    .filter(|plan_pos| child_lineage.contains(plan_pos))
                    .copied()
                    .collect();
                if to_check.is_empty() {
                    Ok(Transformed::no(child))
                } else {
                    wrap_with_visibility_if_needed(child, &to_check, plan_pos_to_heap_oid)
                }
            })
            .collect::<Result<Vec<_>>>()?;
        let wrapped_any = wrapped_children.iter().any(|child| child.transformed);
        let wrapped_children: Vec<LogicalPlan> = wrapped_children
            .into_iter()
            .map(|child| child.data)
            .collect();

        for plan_pos in &force_positions {
            merged.insert(*plan_pos, VisibilityStatus::Verified);
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
///
/// A plain `Sort` is not a barrier because it only reorders rows; deferred ctids can
/// safely flow through it unchanged. `Sort` with `fetch` is a barrier because Top-N
/// semantics can discard rows permanently, so visibility must be resolved first.
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
        let exec = VisibilityFilterExec::new(
            input.clone(),
            vis_node.plan_pos_oids.clone(),
            self.snapshot,
        )?;
        Ok(Some(Arc::new(exec)))
    }
}

// ---------------------------------------------------------------------------
// Physical Execution Plan
// ---------------------------------------------------------------------------

/// Physical plan node that resolves packed DocAddresses and performs batch
/// visibility checking on ctid columns.
///
/// For each `(plan_position, heap_oid)` in `plan_pos_oids`, it:
/// 1. Resolves packed DocAddresses to real ctids via FFHelper
/// 2. Reads the resolved `ctid_{plan_position}` column from the batch
/// 3. Runs `VisibilityChecker::check_batch()` to determine visible rows
/// 4. Filters the batch to only visible rows
/// 5. Replaces ctid values with HOT-resolved ctids
pub struct VisibilityFilterExec {
    input: Arc<dyn ExecutionPlan>,
    /// (plan_position, heap_oid) pairs for visibility checking.
    plan_pos_oids: Vec<(usize, pg_sys::Oid)>,
    snapshot: pg_sys::Snapshot,
    properties: Arc<PlanProperties>,
    metrics: ExecutionPlanMetricsSet,
    /// Per-plan_position FFHelper for resolving packed DocAddresses to real ctids.
    /// Wired by `VisibilityCtidResolverRule` after plan construction.
    ctid_resolvers: Mutex<BTreeMap<usize, Arc<FFHelper>>>,
}

// SAFETY: VisibilityFilterExec holds a raw pg_sys::Snapshot pointer and
// (plan_position, Oid) pairs. This is safe because the snapshot is valid for the entire
// transaction lifetime and pg_search runs DataFusion on a single-threaded
// Tokio runtime within the Postgres backend process — the pointer never
// crosses real thread boundaries.
unsafe impl Send for VisibilityFilterExec {}
unsafe impl Sync for VisibilityFilterExec {}

impl fmt::Debug for VisibilityFilterExec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VisibilityFilterExec")
            .field(
                "plan_positions",
                &self
                    .plan_pos_oids
                    .iter()
                    .map(|(p, _)| *p)
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl VisibilityFilterExec {
    pub fn new(
        input: Arc<dyn ExecutionPlan>,
        plan_pos_oids: Vec<(usize, pg_sys::Oid)>,
        snapshot: pg_sys::Snapshot,
    ) -> Result<Self> {
        // Visibility filtering only removes rows — it never reorders them.
        // Forward the input's equivalence properties so DataFusion knows
        // sort order is preserved (avoids unnecessary re-sorts).
        let properties = Arc::new(PlanProperties::new(
            input.properties().equivalence_properties().clone(),
            input.properties().output_partitioning().clone(),
            EmissionType::Incremental,
            Boundedness::Bounded,
        ));
        Ok(Self {
            input,
            plan_pos_oids,
            snapshot,
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
            ctid_resolvers: Mutex::new(BTreeMap::new()),
        })
    }

    /// Wire an FFHelper for resolving packed DocAddresses to real ctids for the given plan_position.
    pub fn set_ctid_resolver(&self, plan_pos: usize, ffhelper: Arc<FFHelper>) {
        self.ctid_resolvers
            .lock()
            .expect("ctid_resolvers lock poisoned")
            .insert(plan_pos, ffhelper);
    }

    pub fn plan_pos_oids(&self) -> &[(usize, pg_sys::Oid)] {
        &self.plan_pos_oids
    }
}

impl DisplayAs for VisibilityFilterExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "VisibilityFilterExec: plan_positions=[{}]",
            self.plan_pos_oids
                .iter()
                .map(|(p, _)| p.to_string())
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

    fn properties(&self) -> &Arc<PlanProperties> {
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
        let new_exec = VisibilityFilterExec::new(
            children.remove(0),
            self.plan_pos_oids.clone(),
            self.snapshot,
        )?;
        // with_new_children constructs a fresh exec node, so preserve any
        // resolver wiring already attached to this instance.
        {
            let resolvers = self
                .ctid_resolvers
                .lock()
                .expect("ctid_resolvers lock poisoned");
            let mut new_resolvers = new_exec
                .ctid_resolvers
                .lock()
                .expect("ctid_resolvers lock poisoned");
            for (plan_pos, ffhelper) in resolvers.iter() {
                new_resolvers.insert(*plan_pos, Arc::clone(ffhelper));
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

        let resolvers = self
            .ctid_resolvers
            .lock()
            .expect("ctid_resolvers lock poisoned")
            .clone();

        let mut checkers: Vec<CtidCheckerEntry> = Vec::with_capacity(self.plan_pos_oids.len());
        for &(plan_pos, heap_oid) in &self.plan_pos_oids {
            let col_name = CtidColumn::new(plan_pos).to_string();
            let (col_idx, _) = schema.column_with_name(&col_name).ok_or_else(|| {
                DataFusionError::Execution(format!(
                    "VisibilityFilterExec: missing ctid column '{}'",
                    col_name
                ))
            })?;
            let heaprel = PgSearchRelation::open(heap_oid);
            let visibility = VisibilityChecker::with_rel_and_snap(&heaprel, self.snapshot);
            let resolver = resolvers.get(&plan_pos).cloned().ok_or_else(|| {
                DataFusionError::Execution(format!(
                    "VisibilityFilterExec: no ctid resolver wired for plan_position {plan_pos}. \
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
// Deferred ctid materialization
// ---------------------------------------------------------------------------

/// Resolves packed DocAddresses (UInt64) to real ctids via FFHelper.
///
/// Each packed value encodes (segment_ord, doc_id). The FFHelper's ctid()
/// column is used to look up the real ctid for each document.
pub fn materialize_deferred_ctid(
    ffhelper: &FFHelper,
    doc_addr_array: &UInt64Array,
    num_rows: usize,
) -> Result<ArrayRef> {
    // Group rows by segment_ord for sequential fast-field access within each
    // segment, which is significantly faster than random lookups across segments.
    let mut seg_groups: BTreeMap<u32, Vec<(usize, u32)>> = BTreeMap::new();
    for i in 0..num_rows {
        if !doc_addr_array.is_null(i) {
            let (seg_ord, doc_id) = unpack_doc_address(doc_addr_array.value(i));
            seg_groups.entry(seg_ord).or_default().push((i, doc_id));
        }
    }

    let mut result: Vec<Option<u64>> = vec![None; num_rows];
    for (seg_ord, rows) in &seg_groups {
        let ctid_col = ffhelper.ctid(*seg_ord);
        for &(row_idx, doc_id) in rows {
            result[row_idx] = ctid_col.as_u64(doc_id);
        }
    }

    Ok(uint64_array_from_options(&result))
}

/// Builds a UInt64Array from a slice of Option<u64> values.
fn uint64_array_from_options(values: &[Option<u64>]) -> ArrayRef {
    Arc::new(UInt64Array::from_iter(values.iter().copied())) as ArrayRef
}

// ---------------------------------------------------------------------------
// Stream implementation
// ---------------------------------------------------------------------------

/// Per-plan_position state for ctid resolution and visibility checking.
struct CtidCheckerEntry {
    /// Index of the `ctid_{plan_position}` column in the batch.
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
            // Fast path: no nulls — wrap primitive values for check_batch's Option<u64> API.
            let mut ctids = Vec::with_capacity(num_rows);
            ctids.extend(ctid_array.values().iter().copied().map(Some));
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
            let resolved = materialize_deferred_ctid(&entry.resolver, doc_addr_array, num_rows)?;
            columns[entry.col_idx] = resolved;
        }

        // Phase 2: Visibility checking on the (now real) ctids.
        let mut visible_mask = vec![true; num_rows];

        // For each plan_position, check visibility and collect HOT-resolved ctids.
        let mut resolved_ctids: BTreeMap<usize, Vec<Option<u64>>> = BTreeMap::new();

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

            for (i, result) in results.iter().enumerate() {
                if result.is_none() {
                    visible_mask[i] = false;
                }
            }

            resolved_ctids.insert(entry.col_idx, results);
        }

        let visible_count = visible_mask.iter().filter(|&&v| v).count();
        if visible_count == 0 {
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
            for (&col_idx, resolved) in &resolved_ctids {
                columns[col_idx] = uint64_array_from_options(resolved);
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
            if let Some(resolved) = resolved_ctids.get(&col_idx) {
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

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use std::collections::BTreeSet;

    use datafusion::common::Result;
    use datafusion::logical_expr::{col, lit, LogicalPlan, LogicalPlanBuilder};
    use datafusion::optimizer::optimizer::OptimizerContext;
    use datafusion::optimizer::OptimizerRule;
    use pgrx::pg_sys;
    use pgrx::prelude::*;

    use crate::postgres::customscan::joinscan::build::{JoinCSClause, JoinSource, RelNode};
    use crate::postgres::customscan::joinscan::visibility_filter::{
        VisibilityFilterNode, VisibilityFilterOptimizerRule,
    };
    use crate::postgres::customscan::joinscan::CtidColumn;
    use crate::scan::ScanInfo;

    const TEST_PLAN_POS: usize = 0;

    fn make_rule() -> VisibilityFilterOptimizerRule {
        let test_heap_oid = pg_sys::Oid::from(42);
        let source = JoinSource {
            plan_position: TEST_PLAN_POS,
            root_id: None,
            scan_info: ScanInfo {
                heap_rti: 1,
                heaprelid: test_heap_oid,
                ..Default::default()
            },
        };
        VisibilityFilterOptimizerRule::new(JoinCSClause::new(RelNode::Scan(Box::new(source))))
    }

    fn make_ctid_plan() -> Result<LogicalPlan> {
        LogicalPlanBuilder::values(vec![vec![lit(1_u64)]])?
            .project(vec![
                col("column1").alias(CtidColumn::new(TEST_PLAN_POS).to_string())
            ])?
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

    #[pg_test]
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

    /// Builds a barrier plan, asserts injection + idempotency.
    fn assert_barrier(build: impl FnOnce(LogicalPlanBuilder) -> Result<LogicalPlan>) -> Result<()> {
        let plan = build(LogicalPlanBuilder::from(make_ctid_plan()?))?;
        assert_barrier_injection(plan)
    }

    #[pg_test]
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
        assert_eq!(first.data, second.data);
        Ok(())
    }

    #[pg_test]
    fn inserts_visibility_below_aggregate_barrier() -> Result<()> {
        use datafusion::functions_aggregate::count::count;
        assert_barrier(|b| {
            b.aggregate(
                Vec::<datafusion::logical_expr::Expr>::new(),
                vec![count(col(CtidColumn::new(TEST_PLAN_POS).to_string()))],
            )?
            .build()
        })
    }

    #[pg_test]
    fn inserts_visibility_below_distinct_barrier() -> Result<()> {
        assert_barrier(|b| b.distinct()?.build())
    }

    #[pg_test]
    fn inserts_visibility_below_sort_with_fetch_barrier() -> Result<()> {
        assert_barrier(|b| {
            b.sort_with_limit(
                vec![col(CtidColumn::new(TEST_PLAN_POS).to_string()).sort(true, false)],
                Some(10),
            )?
            .build()
        })
    }

    #[pg_test]
    fn sort_without_fetch_is_not_barrier() -> Result<()> {
        let config = OptimizerContext::new();
        let rule = make_rule();
        let plan = LogicalPlanBuilder::from(make_ctid_plan()?)
            .sort(vec![
                col(CtidColumn::new(TEST_PLAN_POS).to_string()).sort(true, false)
            ])?
            .build()?;

        let result = rule.rewrite(plan, &config)?;
        assert!(result.transformed);
        assert_eq!(count_visibility_nodes(&result.data), 1);
        assert!(!first_child_is_visibility(&result.data));
        Ok(())
    }

    #[pg_test]
    fn multi_relation_join() -> Result<()> {
        let config = OptimizerContext::new();

        const POS_A: usize = 0;
        const POS_B: usize = 1;
        let oid_a = pg_sys::Oid::from(42);
        let oid_b = pg_sys::Oid::from(43);

        let source_a = JoinSource {
            plan_position: POS_A,
            root_id: None,
            scan_info: ScanInfo {
                heap_rti: 1,
                heaprelid: oid_a,
                ..Default::default()
            },
        };
        let source_b = JoinSource {
            plan_position: POS_B,
            root_id: None,
            scan_info: ScanInfo {
                heap_rti: 2,
                heaprelid: oid_b,
                ..Default::default()
            },
        };

        let plan_node = RelNode::Join(Box::new(
            crate::postgres::customscan::joinscan::build::JoinNode {
                join_type: crate::postgres::customscan::joinscan::build::JoinType::Inner,
                left: RelNode::Scan(Box::new(source_a)),
                right: RelNode::Scan(Box::new(source_b)),
                equi_keys: Vec::new(),
                filter: None,
            },
        ));

        let rule = VisibilityFilterOptimizerRule::new(JoinCSClause::new(plan_node));

        // Build two leaf plans and join them (inner join = not a barrier).
        let left = LogicalPlanBuilder::values(vec![vec![lit(1_u64)]])?
            .project(vec![
                col("column1").alias(CtidColumn::new(POS_A).to_string())
            ])?
            .build()?;
        let right = LogicalPlanBuilder::values(vec![vec![lit(2_u64)]])?
            .project(vec![
                col("column1").alias(CtidColumn::new(POS_B).to_string())
            ])?
            .build()?;

        let plan = LogicalPlanBuilder::from(left).cross_join(right)?.build()?;

        let first = rule.rewrite(plan, &config)?;
        assert!(first.transformed);
        // Both plan_positions should get visibility — single node at root covers both.
        assert_eq!(count_visibility_nodes(&first.data), 1);

        // Extract the VisibilityFilterNode and check it covers both plan_positions.
        if let LogicalPlan::Extension(ext) = &first.data {
            let vf = ext
                .node
                .as_any()
                .downcast_ref::<VisibilityFilterNode>()
                .expect("root should be VisibilityFilterNode");
            let positions: BTreeSet<usize> = vf.plan_pos_oids.iter().map(|(p, _)| *p).collect();
            assert!(positions.contains(&POS_A));
            assert!(positions.contains(&POS_B));
        } else {
            panic!("expected root to be VisibilityFilterNode");
        }

        let second = rule.rewrite(first.data.clone(), &config)?;
        assert!(!second.transformed);
        assert_eq!(count_visibility_nodes(&second.data), 1);
        Ok(())
    }

    #[pg_test]
    fn left_join_injects_per_side_visibility() -> Result<()> {
        let config = OptimizerContext::new();

        const POS_A: usize = 0;
        const POS_B: usize = 1;
        let oid_a = pg_sys::Oid::from(42);
        let oid_b = pg_sys::Oid::from(43);

        let source_a = JoinSource {
            plan_position: POS_A,
            root_id: None,
            scan_info: ScanInfo {
                heap_rti: 1,
                heaprelid: oid_a,
                ..Default::default()
            },
        };
        let source_b = JoinSource {
            plan_position: POS_B,
            root_id: None,
            scan_info: ScanInfo {
                heap_rti: 2,
                heaprelid: oid_b,
                ..Default::default()
            },
        };

        let plan_node = RelNode::Join(Box::new(
            crate::postgres::customscan::joinscan::build::JoinNode {
                join_type: crate::postgres::customscan::joinscan::build::JoinType::Inner,
                left: RelNode::Scan(Box::new(source_a)),
                right: RelNode::Scan(Box::new(source_b)),
                equi_keys: Vec::new(),
                filter: None,
            },
        ));

        let rule = VisibilityFilterOptimizerRule::new(JoinCSClause::new(plan_node));

        let left = LogicalPlanBuilder::values(vec![vec![lit(1_u64)]])?
            .project(vec![
                col("column1").alias(CtidColumn::new(POS_A).to_string())
            ])?
            .build()?;
        let right = LogicalPlanBuilder::values(vec![vec![lit(2_u64)]])?
            .project(vec![
                col("column1").alias(CtidColumn::new(POS_B).to_string())
            ])?
            .build()?;

        // LEFT join is a barrier — visibility must be injected per-side.
        let plan = LogicalPlanBuilder::from(left)
            .join_on(
                right,
                datafusion::common::JoinType::Left,
                vec![col(CtidColumn::new(POS_A).to_string())
                    .eq(col(CtidColumn::new(POS_B).to_string()))],
            )?
            .build()?;

        let first = rule.rewrite(plan, &config)?;
        assert!(first.transformed);
        // LEFT join is a barrier, so visibility filters are injected into
        // each child that has unverified plan_positions — 2 total.
        assert_eq!(count_visibility_nodes(&first.data), 2);

        let second = rule.rewrite(first.data.clone(), &config)?;
        assert!(!second.transformed);
        assert_eq!(count_visibility_nodes(&second.data), 2);
        Ok(())
    }

    #[pg_test]
    fn visibility_node_codec_roundtrip() -> Result<()> {
        use crate::scan::codec::{deserialize_logical_plan, serialize_logical_plan};
        use datafusion::execution::TaskContext;

        let plan = make_ctid_plan()?;
        let wrapped = LogicalPlan::Extension(datafusion::logical_expr::Extension {
            node: std::sync::Arc::new(VisibilityFilterNode::new(
                plan,
                vec![(TEST_PLAN_POS, pg_sys::Oid::from(42))],
            )),
        });

        let bytes =
            serialize_logical_plan(&wrapped).expect("VisibilityFilterNode should serialize");
        let ctx = TaskContext::default();
        let decoded = deserialize_logical_plan(&bytes, &ctx, None, None, None)
            .expect("VisibilityFilterNode should deserialize");

        let LogicalPlan::Extension(ext) = &decoded else {
            panic!("decoded root should be Extension");
        };
        let vis = ext
            .node
            .as_any()
            .downcast_ref::<VisibilityFilterNode>()
            .expect("decoded node should be VisibilityFilterNode");
        assert_eq!(
            vis.plan_pos_oids,
            vec![(TEST_PLAN_POS, pg_sys::Oid::from(42))]
        );
        Ok(())
    }
}

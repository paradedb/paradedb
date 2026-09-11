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
//!    `VisibilityFilterNode` → `VisibilityFilterExec`, rebuilding any immediate
//!    `TantivyDecodeExec` / `TantivyFetchExec` chain above it so visibility runs
//!    before lookup work.
//! 3. `VisibilityCtidResolverRule` (physical optimizer) — wires FFHelper from
//!    `PgSearchScanPlan` into the ctid-resolving `TantivyFetchExec` below
//!    `VisibilityFilterExec` so it can resolve packed DocAddresses to real ctids.
//! 4. `VisibilityFilterExec` (physical execution) — resolves packed DocAddresses
//!    to real ctids via FFHelper, opens heap relations, creates `VisibilityChecker`
//!    per relation, and filters batches on the resolved ctids.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use arrow_array::{Array, ArrayRef, RecordBatch, UInt64Array};
use arrow_schema::SchemaRef;
use async_trait::async_trait;
use datafusion::arrow::compute::kernels::boolean::{and, is_not_null};
use datafusion::catalog::Session;
use datafusion::common::tree_node::{Transformed, TreeNode, TreeNodeRecursion};
use datafusion::common::{DFSchemaRef, DataFusionError, Result};
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::logical_expr::physical_planning_context::PhysicalPlanningContext;
use datafusion::logical_expr::{Extension, LogicalPlan, UserDefinedLogicalNode};
use datafusion::optimizer::optimizer::ApplyOrder;
use datafusion::optimizer::{OptimizerConfig, OptimizerRule};
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::filter_pushdown::{
    FilterDescription, FilterPushdownPhase, FilterPushdownPropagation,
};
use datafusion::physical_plan::metrics::{
    BaselineMetrics, ExecutionPlanMetricsSet, MetricsSet, RecordOutput,
};
use datafusion::physical_plan::{
    ChildrenPropertiesMode, DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties,
    ReplaceChildrenOptions,
};
use datafusion::physical_planner::{ExtensionPlanner, PhysicalPlanner};
use pgrx::pg_sys;

use crate::index::fast_fields_helper::{FFHelper, for_each_segment};
use crate::postgres::customscan::joinscan::CtidColumn;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::scan::execution_plan::UnsafeSendStream;
use crate::scan::late_materialization::is_reduction_node;
use crate::scan::table_provider::{VisibilitySourceMetadata, pg_search_provider_from_scan};
use crate::scan::tantivy_decode_exec::TantivyDecodeExec;
use crate::scan::tantivy_fetch_exec::{CtidColumnLookup, TantivyFetchExec};
use arrow_select::filter::filter_record_batch;
use tantivy::DocId;

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
    /// Table names for EXPLAIN display, parallel to plan_pos_oids.
    pub table_names: Vec<String>,
    schema: DFSchemaRef,
}

impl VisibilityFilterNode {
    pub fn new(
        input: LogicalPlan,
        plan_pos_oids: Vec<(usize, pg_sys::Oid)>,
        table_names: Vec<String>,
    ) -> Self {
        let schema = input.schema().clone();
        Self {
            input,
            plan_pos_oids,
            table_names,
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
        // This node filters invisible rows and HOT-corrects the ctids, so a predicate
        // on a ctid column must run above it, not against the pre-visibility values.
        self.plan_pos_oids
            .iter()
            .map(|(plan_pos, _)| CtidColumn::new(*plan_pos).to_string())
            .collect()
    }

    fn fmt_for_explain(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "VisibilityFilter: tables=[{}]",
            self.table_names.join(", ")
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
        Ok(Self::new(
            input,
            self.plan_pos_oids.clone(),
            self.table_names.clone(),
        ))
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
#[derive(Debug, Default)]
pub struct VisibilityFilterOptimizerRule;

impl VisibilityFilterOptimizerRule {
    pub fn new() -> Self {
        Self
    }
}

fn collect_visibility_source_metadata(
    plan: &LogicalPlan,
) -> Result<BTreeMap<usize, VisibilitySourceMetadata>> {
    let mut metadata = BTreeMap::new();

    plan.apply(|node| {
        if let LogicalPlan::TableScan(scan) = node
            && let Some(provider) = pg_search_provider_from_scan(scan)
            && let Some(source_metadata) = provider.visibility_source_metadata()
            && let Some(prev_metadata) =
                metadata.insert(source_metadata.plan_position, source_metadata.clone())
            && prev_metadata != source_metadata
        {
            return Err(DataFusionError::Internal(format!(
                "VisibilityFilterInjection: conflicting metadata for plan_position {}",
                source_metadata.plan_position,
            )));
        }

        Ok(TreeNodeRecursion::Continue)
    })?;

    Ok(metadata)
}

/// Recursively traverses `plan` tracking the ancestor chain down to each `TableScan`.
/// For each `TableScan` with configured deferred ctid plan position, checks if there is
/// any intermediate reduction node on the path between the scan and the first ancestor
/// that acts as a barrier (or the root).
fn collect_beneficial_deferred_visibility_inner<'a>(
    node: &'a LogicalPlan,
    ancestors: &mut Vec<(&'a LogicalPlan, usize)>,
    beneficial: &mut BTreeSet<usize>,
) {
    if let LogicalPlan::TableScan(scan) = node {
        if let Some(provider) = pg_search_provider_from_scan(scan)
            && let Some(plan_pos) = provider.configured_deferred_ctid_plan_position()
        {
            // Like `has_reduction_before_stop`, but a stopping barrier that
            // reduces rows counts for the side that continues past it: a semi,
            // anti, or outer join is the reduction that makes deferring the
            // preserved side's check worthwhile. The checked side's filter
            // lands below that join, so nothing reduces its rows first and it
            // stays eager unless something below already did.
            //
            // A Full barrier never credits itself: it keeps every check
            // below it, so its own reduction comes after the check. Without
            // a reduction under the barrier, the wrap would cost the same
            // rows as the in-scan check plus a node.
            let mut has_reduction = false;
            for (ancestor, from_child) in ancestors.iter().rev() {
                match barrier_status(ancestor) {
                    BarrierStatus::None => {
                        if is_reduction_node(ancestor) {
                            has_reduction = true;
                        }
                    }
                    BarrierStatus::Partial(checked) if *from_child == checked => break,
                    BarrierStatus::Partial(_) => {
                        has_reduction = has_reduction || is_reduction_node(ancestor);
                        break;
                    }
                    BarrierStatus::Full => break,
                }
            }
            if has_reduction {
                beneficial.insert(plan_pos);
            }
        }
        return;
    }

    for (idx, child) in node.inputs().into_iter().enumerate() {
        ancestors.push((node, idx));
        collect_beneficial_deferred_visibility_inner(child, ancestors, beneficial);
        ancestors.pop();
    }
}

fn collect_beneficial_deferred_visibility(plan: &LogicalPlan) -> BTreeSet<usize> {
    let mut beneficial = BTreeSet::new();
    let mut ancestors = Vec::new();
    collect_beneficial_deferred_visibility_inner(plan, &mut ancestors, &mut beneficial);
    beneficial
}

fn ensure_scan_projects_ctid(
    scan: &datafusion::logical_expr::TableScan,
    plan_pos: usize,
) -> Result<datafusion::logical_expr::TableScan> {
    let ctid_name = CtidColumn::new(plan_pos).to_string();
    if scan
        .projected_schema
        .index_of_column_by_name(None, &ctid_name)
        .is_some()
    {
        return Ok(scan.clone());
    }

    let source_schema = scan.source.schema();
    let Ok(ctid_source_idx) = source_schema.index_of(&ctid_name) else {
        return Ok(scan.clone());
    };

    let mut projected_indices: Vec<usize> = scan
        .projected_schema
        .fields()
        .iter()
        .map(|f| scan.source.schema().index_of(f.name()))
        .collect::<Result<Vec<_>, _>>()?;

    projected_indices.push(ctid_source_idx);

    let projected_arrow_schema = source_schema.project(&projected_indices)?;
    let mut new_qualified_fields = Vec::new();
    for (i, field) in projected_arrow_schema.fields().iter().enumerate() {
        let qualifier = if i < scan.projected_schema.fields().len() {
            let (q, _) = scan.projected_schema.qualified_field(i);
            q.cloned()
        } else if !scan.projected_schema.fields().is_empty() {
            let (q, _) = scan.projected_schema.qualified_field(0);
            q.cloned()
        } else {
            Some(scan.table_name.clone())
        };
        new_qualified_fields.push((qualifier, field.clone()));
    }

    let mut new_scan = scan.clone();
    new_scan.projection = Some(projected_indices);
    new_scan.projected_schema = Arc::new(datafusion::common::DFSchema::new_with_metadata(
        new_qualified_fields,
        scan.projected_schema.metadata().clone(),
    )?);

    Ok(new_scan)
}

/// Returns a copy of `proj` extended with any `ctid_<n>` column that its input
/// produces for a beneficial plan position but the projection does not yet
/// output, or `None` when it already forwards them all. A projection only emits
/// the columns it lists, so a ctid bubbling up from below has to be added here
/// explicitly to reach the barrier above.
fn projection_with_ctids_added(
    proj: &datafusion::logical_expr::Projection,
    beneficial: &BTreeSet<usize>,
) -> Result<Option<datafusion::logical_expr::Projection>> {
    let mut new_exprs = proj.expr.clone();
    let mut added = false;
    for (i, field) in proj.input.schema().fields().iter().enumerate() {
        let Ok(ctid_col) = CtidColumn::try_from(field.name().as_str()) else {
            continue;
        };
        if beneficial.contains(&ctid_col.plan_position())
            && proj
                .schema
                .index_of_column_by_name(None, field.name())
                .is_none()
        {
            let (qualifier, _) = proj.input.schema().qualified_field(i);
            new_exprs.push(datafusion::logical_expr::col(
                datafusion::common::Column::new(qualifier.cloned(), field.name().clone()),
            ));
            added = true;
        }
    }
    if added {
        Ok(Some(datafusion::logical_expr::Projection::try_new(
            new_exprs,
            proj.input.clone(),
        )?))
    } else {
        Ok(None)
    }
}

/// Threads a ctid column produced below `node` up through it, so a deferred
/// scan's ctid survives every row-preserving node between the scan and the
/// barrier where `VisibilityFilterExec` reads it. The caller only invokes this
/// for non-barrier nodes.
///
/// A projection gets the ctid added to its output. A join is rebuilt with
/// `Join::try_new` because `with_new_exprs` keeps the join's cached schema; only
/// `try_new` re-runs `build_join_schema` so the bubbled ctid shows up. Every
/// other node just recomputes its schema over the rewritten children.
fn carry_ctid_columns_upward(
    node: LogicalPlan,
    beneficial: &BTreeSet<usize>,
) -> Result<Transformed<LogicalPlan>> {
    if let LogicalPlan::Projection(proj) = &node
        && let Some(new_proj) = projection_with_ctids_added(proj, beneficial)?
    {
        return Ok(Transformed::yes(LogicalPlan::Projection(new_proj)));
    }

    if let LogicalPlan::Join(join) = &node {
        let new_join = datafusion::logical_expr::logical_plan::Join::try_new(
            join.left.clone(),
            join.right.clone(),
            join.on.clone(),
            join.filter.clone(),
            join.join_type,
            join.join_constraint,
            join.null_equality,
            join.null_aware,
        )?;
        return Ok(Transformed::yes(LogicalPlan::Join(new_join)));
    }

    let new_node = node.with_new_exprs(
        node.expressions(),
        node.inputs().into_iter().cloned().collect(),
    )?;
    Ok(Transformed::yes(new_node.recompute_schema()?))
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
        let beneficial = collect_beneficial_deferred_visibility(&plan);
        if beneficial.is_empty() {
            return Ok(Transformed::no(plan));
        }

        let mut plan_pos_metadata = collect_visibility_source_metadata(&plan)?;
        plan_pos_metadata.retain(|pos, _| beneficial.contains(pos));

        if plan_pos_metadata.is_empty() {
            return Ok(Transformed::no(plan));
        }

        let prepared_plan = plan.transform_up(|node| {
            if let LogicalPlan::TableScan(scan) = &node
                && let Some(provider) = pg_search_provider_from_scan(scan)
                && let Some(plan_pos) = provider.configured_deferred_ctid_plan_position()
                && beneficial.contains(&plan_pos)
            {
                provider.enable_deferred_visibility_schema();
                let updated_scan = ensure_scan_projects_ctid(scan, plan_pos)?;
                return Ok(Transformed::yes(LogicalPlan::TableScan(updated_scan)));
            }

            if matches!(barrier_status(&node), BarrierStatus::None) {
                return carry_ctid_columns_upward(node, &beneficial);
            }

            Ok(Transformed::no(node))
        })?;

        let (result, final_state) = analyze_and_inject(prepared_plan.data, &plan_pos_metadata)?;

        // Root boundary fallback: any plan_position still unverified must be checked here.
        let unverified: BTreeSet<usize> = final_state
            .iter()
            .filter(|(_, s)| **s == VisibilityStatus::Unverified)
            .map(|(plan_pos, _)| *plan_pos)
            .collect();

        if unverified.is_empty() {
            return Ok(result);
        }

        let wrapped = wrap_with_visibility_if_needed(result.data, &unverified, &plan_pos_metadata)?;
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
    plan_pos_metadata: &BTreeMap<usize, VisibilitySourceMetadata>,
) -> Result<LogicalPlan> {
    let mut plan_pos_oids = Vec::with_capacity(plan_positions.len());
    let mut table_names = Vec::with_capacity(plan_positions.len());
    for &plan_pos in plan_positions {
        let metadata = plan_pos_metadata.get(&plan_pos).ok_or_else(|| {
            DataFusionError::Internal(format!(
                "VisibilityFilterInjection: missing source metadata for plan_position {}",
                plan_pos
            ))
        })?;
        plan_pos_oids.push((plan_pos, metadata.heap_oid));
        table_names.push(metadata.table_name.clone());
    }

    Ok(LogicalPlan::Extension(Extension {
        node: Arc::new(VisibilityFilterNode::new(input, plan_pos_oids, table_names)),
    }))
}

fn wrap_with_visibility_if_needed(
    input: LogicalPlan,
    plan_positions: &BTreeSet<usize>,
    plan_pos_metadata: &BTreeMap<usize, VisibilitySourceMetadata>,
) -> Result<Transformed<LogicalPlan>> {
    if plan_positions.is_empty() {
        return Ok(Transformed::no(input));
    }

    if let Some(existing) = existing_visibility_plan_positions(&input) {
        let missing: BTreeSet<usize> = plan_positions.difference(&existing).copied().collect();
        if missing.is_empty() {
            return Ok(Transformed::no(input));
        }
        let wrapped = wrap_with_visibility(input, &missing, plan_pos_metadata)?;
        return Ok(Transformed::yes(wrapped));
    }

    let wrapped = wrap_with_visibility(input, plan_positions, plan_pos_metadata)?;
    Ok(Transformed::yes(wrapped))
}

fn analyze_and_inject(
    plan: LogicalPlan,
    plan_pos_metadata: &BTreeMap<usize, VisibilitySourceMetadata>,
) -> Result<(Transformed<LogicalPlan>, RelationStates)> {
    let children: Vec<LogicalPlan> = plan.inputs().into_iter().cloned().collect();
    let mut new_children = Vec::with_capacity(children.len());
    let mut child_states = Vec::with_capacity(children.len());
    let mut any_modified = false;

    for child in children {
        let (result, state) = analyze_and_inject(child, plan_pos_metadata)?;
        any_modified |= result.transformed;
        new_children.push(result.data);
        child_states.push(state);
    }

    if new_children.is_empty() {
        let mut leaf_state = RelationStates::new();
        for plan_pos in extract_ctid_lineage(plan.schema()) {
            if plan_pos_metadata.contains_key(&plan_pos) {
                leaf_state.insert(plan_pos, VisibilityStatus::Unverified);
            }
        }
        return Ok((Transformed::new_transformed(plan, any_modified), leaf_state));
    }

    // Plan positions are unique per source, so child states never overlap.
    let mut merged = RelationStates::new();
    for child_state in &child_states {
        for (&plan_pos, &status) in child_state {
            let entry = merged.entry(plan_pos).or_insert(status);
            if status == VisibilityStatus::Unverified {
                *entry = VisibilityStatus::Unverified;
            }
        }
    }

    // Treat existing visibility nodes as already verified so repeated optimizer
    // passes do not keep re-wrapping.
    if let LogicalPlan::Extension(ext) = &plan
        && let Some(vf) = ext.node.as_any().downcast_ref::<VisibilityFilterNode>()
    {
        for &(plan_pos, _) in &vf.plan_pos_oids {
            merged.insert(plan_pos, VisibilityStatus::Verified);
        }
    }

    let parent_lineage: BTreeSet<usize> = extract_ctid_lineage(plan.schema())
        .into_iter()
        .filter(|plan_pos| plan_pos_metadata.contains_key(plan_pos))
        .collect();

    // If lineage appears first at this node, mark it unverified.
    for &plan_pos in &parent_lineage {
        merged
            .entry(plan_pos)
            .or_insert(VisibilityStatus::Unverified);
    }

    // Fetching the barrier status for this plan & the plan positions to force visibility checks for based on barrier status
    let barrier_status = barrier_status(&plan);
    let force_positions =
        get_force_positions(barrier_status, &merged, &child_states, &parent_lineage);

    if !force_positions.is_empty() {
        // Only wrap children that still carry one of the forced plan positions.
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
                    wrap_with_visibility_if_needed(child, &to_check, plan_pos_metadata)
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

    if any_modified {
        let new_plan = plan.with_new_exprs(plan.expressions(), new_children)?;
        Ok((Transformed::yes(new_plan), merged))
    } else {
        Ok((Transformed::no(plan), merged))
    }
}

// Returning a BTreeSet of plan positions that need to visibility checks
// Barrier nodes and lineage drops both force visibility injection here.
fn get_force_positions(
    barrier_status: BarrierStatus,
    merged: &BTreeMap<usize, VisibilityStatus>,
    child_states: &[BTreeMap<usize, VisibilityStatus>],
    parent_lineage: &BTreeSet<usize>,
) -> BTreeSet<usize> {
    match barrier_status {
        // For a Full barrier, any plan with an unverified status needs to be visibility checked
        BarrierStatus::Full => merged
            .iter()
            .filter(|(_, status)| **status == VisibilityStatus::Unverified)
            .map(|(plan_pos, _)| *plan_pos)
            .collect(),
        BarrierStatus::Partial(barrier_child) => {
            // For a Partial barrier, only the plan positions in the child specified need to be visibility checked
            merged
                .iter()
                .filter(|(_, status)| **status == VisibilityStatus::Unverified)
                .map(|(plan_pos, _)| *plan_pos)
                .filter(|plan_pos| {
                    child_states
                        .get(barrier_child)
                        .is_some_and(|cs| cs.contains_key(plan_pos))
                })
                .collect()
        }
        BarrierStatus::None => {
            // For no barrier, any plan with an unverified status that's not in any node above needs to be checked
            merged
                .iter()
                .filter(|(plan_pos, status)| {
                    **status == VisibilityStatus::Unverified && !parent_lineage.contains(plan_pos)
                })
                .map(|(plan_pos, _)| *plan_pos)
                .collect()
        }
    }
}

enum BarrierStatus {
    None,
    Partial(usize), // a barrier only on plan positions for the child specified
    Full,
}
/// Returns the "barrier status" of the given plan node (either full, partial, or no barrier) — a point where visibility
/// must be checked before proceeding.
///
/// Partial Barriers include left/left semi/left anti and right/right semi/right anti joins - the null-supplying child must have visibility checked, while
/// the preserved side should remain deferred
///
/// A consumed or null-supplying side cannot check above its join: a dead row
/// that reaches the join fabricates a semi match, suppresses an anti row, or
/// replaces a null-extension, and its ctid never reaches the plan top anyway.
/// Null support in the check is what lets the preserved side defer past the
/// join, but it is not enough to move the other side's check up: the filter
/// can skip a NULL ctid, yet it cannot re-extend the preserved row that a
/// dead match displaced.
///
/// Full Barriers include all other non-inner joins (outer, etc),
/// aggregates, distinct, window functions, and sort-with-limit.
///
/// A plain `Sort` is not a barrier because it only reorders rows; deferred ctids can
/// safely flow through it unchanged. `Sort` with `fetch` is a full barrier because Top-N
/// semantics can discard rows permanently, so visibility must be resolved first.
fn barrier_status(plan: &LogicalPlan) -> BarrierStatus {
    match plan {
        LogicalPlan::Sort(sort) => {
            if sort.fetch.is_some() {
                BarrierStatus::Full
            } else {
                BarrierStatus::None
            }
        }
        LogicalPlan::Join(join) => match join.join_type {
            datafusion::common::JoinType::Inner => BarrierStatus::None,
            // specifying that for a left/left semi/left anti join, we want to visiblity-check the right side only and vice-versa
            // note that the ordering here comes from DataFusion's `inputs` method, which returns left then right children
            datafusion::common::JoinType::Left => BarrierStatus::Partial(1),
            datafusion::common::JoinType::LeftSemi => BarrierStatus::Partial(1),
            datafusion::common::JoinType::LeftAnti => BarrierStatus::Partial(1),
            datafusion::common::JoinType::Right => BarrierStatus::Partial(0),
            datafusion::common::JoinType::RightSemi => BarrierStatus::Partial(0),
            datafusion::common::JoinType::RightAnti => BarrierStatus::Partial(0),
            _ => BarrierStatus::Full,
        },
        LogicalPlan::Limit(_) => BarrierStatus::Full,
        LogicalPlan::Aggregate(_) => BarrierStatus::Full,
        LogicalPlan::Distinct(_) => BarrierStatus::Full,
        LogicalPlan::Window(_) => BarrierStatus::Full,
        _ => BarrierStatus::None,
    }
}

// ---------------------------------------------------------------------------
// Extension Planner (Logical → Physical)
// ---------------------------------------------------------------------------

/// Converts `VisibilityFilterNode` into `VisibilityFilterExec`.
pub struct VisibilityExtensionPlanner {}

impl VisibilityExtensionPlanner {
    pub fn new() -> Self {
        Self {}
    }
}

/// Builds a `TantivyFetchExec` that resolves the given sources' `ctid_<plan_position>` columns
/// from packed doc-addresses to real ctids. Its resolvers are wired later by
/// `VisibilityCtidResolverRule`.
fn ctid_resolving_fetch(
    input: Arc<dyn ExecutionPlan>,
    plan_pos_oids: &[(usize, pg_sys::Oid)],
) -> Result<Arc<dyn ExecutionPlan>> {
    let schema = input.schema();
    let mut ctid_columns = Vec::with_capacity(plan_pos_oids.len());
    for (plan_pos, _) in plan_pos_oids {
        let name = CtidColumn::new(*plan_pos).to_string();
        let (col_idx, _) = schema.column_with_name(&name).ok_or_else(|| {
            DataFusionError::Internal(format!(
                "ctid-resolving lookup: ctid column '{name}' missing from input schema"
            ))
        })?;
        ctid_columns.push(CtidColumnLookup {
            col_idx,
            plan_position: *plan_pos,
        });
    }
    Ok(Arc::new(TantivyFetchExec::new(
        input,
        Vec::new(),
        crate::api::HashMap::default(),
        ctid_columns,
    )?))
}

fn wrap_visibility_below_lookup_chain(
    input: Arc<dyn ExecutionPlan>,
    plan_pos_oids: Vec<(usize, pg_sys::Oid)>,
    table_names: Vec<String>,
) -> Result<Arc<dyn ExecutionPlan>> {
    let mut lookups = Vec::new();
    let mut current = input;

    while current.is::<TantivyDecodeExec>() || current.is::<TantivyFetchExec>() {
        let child = Arc::clone(current.children()[0]);
        lookups.push(current);
        current = child;
    }

    // Resolve the ctid columns just below the visibility filter, so the filter (and a
    // SegmentedTopKExec that later absorbs it) consumes real ctids instead of packed addresses.
    let vf_input = ctid_resolving_fetch(current, &plan_pos_oids)?;
    let mut result = Arc::new(VisibilityFilterExec::new(
        vf_input,
        plan_pos_oids,
        table_names,
    )?) as Arc<dyn ExecutionPlan>;
    for lookup in lookups.into_iter().rev() {
        result = lookup.replace_children(
            vec![result],
            ReplaceChildrenOptions::new(ChildrenPropertiesMode::Recompute),
        )?;
    }
    Ok(result)
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
        _session: &dyn Session,
        _planning_context: &PhysicalPlanningContext,
    ) -> Result<Option<Arc<dyn ExecutionPlan>>> {
        let Some(vis_node) = node.as_any().downcast_ref::<VisibilityFilterNode>() else {
            return Ok(None);
        };

        let input = physical_inputs.first().ok_or_else(|| {
            DataFusionError::Internal("VisibilityFilterExec requires exactly one input".into())
        })?;
        let exec = wrap_visibility_below_lookup_chain(
            input.clone(),
            vis_node.plan_pos_oids.clone(),
            vis_node.table_names.clone(),
        )?;
        Ok(Some(exec))
    }
}

// ---------------------------------------------------------------------------
// Physical Execution Plan
// ---------------------------------------------------------------------------

/// The dispatch wire shape: `(plan_pos, heap_oid)` pairs and display names.
type VisibilityDispatchPayload = (Vec<(usize, pg_sys::Oid)>, Vec<String>);

/// Physical plan node that visibility-checks ctid columns and HOT-corrects them.
///
/// The ctid columns arrive already resolved to real ctids from the `TantivyFetchExec`
/// below this node. For each `(plan_position, heap_oid)` in `plan_pos_oids`, it:
/// 1. Reads the `ctid_{plan_position}` column from the batch
/// 2. Runs `VisibilityChecker::check_batch()` to determine visible rows
/// 3. Filters the batch to only visible rows
/// 4. Replaces ctid values with HOT-resolved ctids
pub struct VisibilityFilterExec {
    input: Arc<dyn ExecutionPlan>,
    /// (plan_position, heap_oid) pairs for visibility checking.
    plan_pos_oids: Vec<(usize, pg_sys::Oid)>,
    /// Table names for EXPLAIN display, parallel to plan_pos_oids.
    table_names: Vec<String>,
    properties: Arc<PlanProperties>,
    metrics: ExecutionPlanMetricsSet,
}

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
        table_names: Vec<String>,
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
            table_names,
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
        })
    }

    pub fn plan_pos_oids(&self) -> &[(usize, pg_sys::Oid)] {
        &self.plan_pos_oids
    }

    /// Serialize for leader dispatch. Visibility checking needs no live state, so only the
    /// `(plan_pos, heap_oid)` pairs and table names travel.
    pub(crate) fn encode_for_dispatch(&self) -> Result<Vec<u8>> {
        let payload = (&self.plan_pos_oids, &self.table_names);
        serde_json::to_vec(&payload).map_err(|e| {
            DataFusionError::Internal(format!("VisibilityFilterExec dispatch: serialize: {e}"))
        })
    }

    /// Rebuild from a dispatch descriptor. The ctid columns are already resolved by the
    /// `TantivyFetchExec` below, so there is nothing to re-wire here.
    pub(crate) fn decode_for_dispatch(
        buf: &[u8],
        input: Arc<dyn ExecutionPlan>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let (plan_pos_oids, table_names): VisibilityDispatchPayload = serde_json::from_slice(buf)
            .map_err(|e| {
            DataFusionError::Internal(format!("VisibilityFilterExec dispatch: deserialize: {e}"))
        })?;
        Ok(Arc::new(VisibilityFilterExec::new(
            input,
            plan_pos_oids,
            table_names,
        )?))
    }

    pub fn table_names(&self) -> &[String] {
        &self.table_names
    }
}

impl DisplayAs for VisibilityFilterExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "VisibilityFilterExec: tables=[{}]",
            self.table_names.join(", ")
        )
    }
}

impl ExecutionPlan for VisibilityFilterExec {
    fn name(&self) -> &str {
        "VisibilityFilterExec"
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

    fn apply_expressions(
        &self,
        _f: &mut dyn FnMut(
            &Arc<dyn datafusion::physical_plan::PhysicalExpr>,
        ) -> Result<TreeNodeRecursion>,
    ) -> Result<TreeNodeRecursion> {
        Ok(TreeNodeRecursion::Continue)
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
        Ok(Arc::new(VisibilityFilterExec::new(
            children.remove(0),
            self.plan_pos_oids.clone(),
            self.table_names.clone(),
        )?))
    }

    fn gather_filters_for_pushdown(
        &self,
        phase: FilterPushdownPhase,
        parent_filters: Vec<Arc<dyn datafusion::physical_expr::PhysicalExpr>>,
        _config: &datafusion::common::config::ConfigOptions,
    ) -> Result<FilterDescription> {
        if !matches!(phase, FilterPushdownPhase::Post) {
            return Ok(FilterDescription::all_unsupported(
                &parent_filters,
                &self.children(),
            ));
        }
        // VisibilityFilterExec is unary and preserves its child's schema.
        // We block ctid_* columns (this node still filters dead rows and HOT-corrects
        // them) and allow all other columns through for filter pushdown.
        let schema = self.input.schema();
        let blocked_ctid_names: std::collections::HashSet<String> = self
            .plan_pos_oids
            .iter()
            .map(|(plan_pos, _)| CtidColumn::new(*plan_pos).to_string())
            .collect();
        let allowed_indices: std::collections::HashSet<usize> = schema
            .fields()
            .iter()
            .enumerate()
            .filter(|(_, f)| !blocked_ctid_names.contains(f.name()))
            .map(|(i, _)| i)
            .collect();
        let child_desc = crate::scan::filter_pushdown::schema_preserving_child_filter_description(
            &parent_filters,
            &schema,
            Some(&allowed_indices),
        )?;
        Ok(FilterDescription::new().with_child(child_desc))
    }

    fn handle_child_pushdown_result(
        &self,
        _phase: FilterPushdownPhase,
        child_pushdown_result: datafusion::physical_plan::filter_pushdown::ChildPushdownResult,
        _config: &datafusion::common::config::ConfigOptions,
    ) -> Result<FilterPushdownPropagation<Arc<dyn ExecutionPlan>>> {
        Ok(FilterPushdownPropagation::if_all(child_pushdown_result))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let mut input_stream = self.input.execute(partition, context)?;
        let schema = self.schema();

        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        if snapshot.is_null() {
            panic!("VisibilityFilterExec requires an active Postgres snapshot");
        }

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
            let visibility = VisibilityChecker::with_rel_and_snap(&heaprel, snapshot);
            checkers.push(CtidCheckerEntry {
                col_idx,
                checker: visibility,
                ctid_input: Vec::new(),
                visibility_results: Vec::new(),
            });
        }

        let baseline_metrics = BaselineMetrics::new(&self.metrics, partition);
        let stream_schema = schema.clone();
        let stream_gen = async_stream::try_stream! {
            use futures::StreamExt;
            while let Some(batch_res) = input_stream.next().await {
                let timer = baseline_metrics.elapsed_compute().timer();
                let result = match batch_res {
                    Ok(batch) => filter_batch(&stream_schema, &mut checkers, batch),
                    Err(e) => Err(e),
                };
                timer.done();

                yield result.record_output(&baseline_metrics)?;
            }
            baseline_metrics.done();
        };

        // SAFETY: The generated stream captures VisibilityChecker instances
        // holding raw Postgres relation/snapshot pointers. These are safe because
        // we run on a single-threaded Tokio runtime within the backend process.
        let stream = unsafe { UnsafeSendStream::new(stream_gen, schema) };
        Ok(Box::pin(stream))
    }
}

// ---------------------------------------------------------------------------
// Deferred ctid materialization
// ---------------------------------------------------------------------------

/// Reusable buffers for per-segment ctid materialization.
#[derive(Default)]
pub(crate) struct DeferredCtidMaterializationState {
    resolved_ctids: Vec<Option<u64>>,
    segment_doc_ids: Vec<DocId>,
    segment_ctids: Vec<Option<u64>>,
}

/// Resolves packed DocAddresses (UInt64) to real ctids via FFHelper.
///
/// Each packed value encodes (segment_ord, doc_id). The FFHelper's ctid()
/// column is used to look up the real ctid for each document.
pub(crate) fn materialize_deferred_ctid(
    ffhelper: &FFHelper,
    doc_addr_array: &UInt64Array,
    state: &mut DeferredCtidMaterializationState,
) -> Result<ArrayRef> {
    let num_rows = doc_addr_array.len();
    let packed_iter = (0..num_rows)
        .filter(|&i| !doc_addr_array.is_null(i))
        .map(|i| (i, doc_addr_array.value(i)));

    state.resolved_ctids.clear();
    state.resolved_ctids.resize(num_rows, None);

    let num_segments = ffhelper.num_segments();
    for_each_segment(num_segments, packed_iter, |seg_ord, rows| {
        state.segment_doc_ids.clear();
        state.segment_doc_ids.extend(rows.iter().map(|(_, id)| *id));

        state.segment_ctids.clear();
        state.segment_ctids.resize(rows.len(), None);
        ffhelper
            .ctid(seg_ord)
            .as_u64s(&state.segment_doc_ids, &mut state.segment_ctids);

        for ((row_idx, _), value) in rows.into_iter().zip(state.segment_ctids.iter()) {
            state.resolved_ctids[row_idx] = *value;
        }
        Ok(())
    })?;

    Ok(uint64_array_from_options(&state.resolved_ctids))
}

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
    ctid_input: Vec<Option<u64>>,
    visibility_results: Vec<Option<u64>>,
}

/// Runs visibility check for a single relation's ctid column.
/// Returns HOT-resolved ctids (None for invisible rows).
fn check_column_visibility(entry: &mut CtidCheckerEntry, ctid_array: &UInt64Array) -> ArrayRef {
    if ctid_array.null_count() != 0 {
        panic!(
            "ctid column contains {} nulls, which indicate a planning or storage bug",
            ctid_array.null_count()
        );
    }
    entry.ctid_input.clear();
    entry
        .ctid_input
        .extend(ctid_array.values().iter().copied().map(Some));
    entry.visibility_results.clear();
    entry.visibility_results.resize(ctid_array.len(), None);
    entry
        .checker
        .check_batch(&entry.ctid_input, &mut entry.visibility_results);
    uint64_array_from_options(&entry.visibility_results)
}

fn filter_batch(
    schema: &SchemaRef,
    checkers: &mut [CtidCheckerEntry],
    batch: RecordBatch,
) -> Result<RecordBatch> {
    if batch.num_rows() == 0 {
        return Ok(batch);
    }

    let num_rows = batch.num_rows();

    // The ctid columns arrive already resolved to real ctids from the TantivyFetchExec below
    // this node, so this only checks visibility and HOT-corrects them.
    let mut columns: Vec<ArrayRef> = batch.columns().to_vec();

    let mut visible_mask = None;
    for entry in checkers.iter_mut() {
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

        let resolved = check_column_visibility(entry, ctid_array);
        let current_mask = is_not_null(resolved.as_ref())
            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
        visible_mask = Some(match visible_mask.take() {
            None => current_mask,
            Some(mask) => and(&mask, &current_mask)
                .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?,
        });
        columns[entry.col_idx] = resolved;
    }

    let visible_mask =
        visible_mask.unwrap_or_else(|| arrow_array::BooleanArray::from(vec![true; num_rows]));
    let visible_count = visible_mask
        .iter()
        .filter(|visible| matches!(visible, Some(true)))
        .count();
    let resolved_batch = RecordBatch::try_new(schema.clone(), columns)
        .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
    if visible_count == num_rows {
        return Ok(resolved_batch);
    }
    filter_record_batch(&resolved_batch, &visible_mask)
        .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::Arc;

    use datafusion::catalog::default_table_source::DefaultTableSource;
    use datafusion::common::Result;
    use datafusion::logical_expr::{LogicalPlan, LogicalPlanBuilder, col};
    use datafusion::optimizer::OptimizerRule;
    use datafusion::optimizer::optimizer::OptimizerContext;
    use pgrx::pg_sys;
    use pgrx::prelude::*;

    use crate::index::fast_fields_helper::WhichFastField;
    use crate::postgres::customscan::joinscan::CtidColumn;
    use crate::postgres::customscan::joinscan::visibility_filter::{
        VisibilityFilterNode, VisibilityFilterOptimizerRule,
    };
    use crate::scan::{PgSearchTableProvider, ScanInfo, VisibilityMode};

    const TEST_PLAN_POS: usize = 0;

    fn make_rule() -> VisibilityFilterOptimizerRule {
        VisibilityFilterOptimizerRule::new()
    }

    fn make_ctid_plan(
        plan_position: usize,
        heap_oid: pg_sys::Oid,
        alias: Option<&str>,
    ) -> Result<LogicalPlan> {
        let mut scan_info = ScanInfo::new(
            1,
            heap_oid,
            pgrx::pg_sys::InvalidOid,
            crate::scan::ScanMode::all(),
        );
        if let Some(alias) = alias {
            scan_info = scan_info.with_alias(alias);
        }
        let mut provider = PgSearchTableProvider::new(scan_info, vec![WhichFastField::Ctid], None);
        provider.configure_deferred_outputs(
            &crate::api::HashSet::default(),
            VisibilityMode::Deferred { plan_position },
        );

        LogicalPlanBuilder::scan(
            alias.unwrap_or("test_table"),
            Arc::new(DefaultTableSource::new(Arc::new(provider))),
            None,
        )?
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
    fn single_scan_without_reduction_is_not_transformed() -> Result<()> {
        let config = OptimizerContext::new();
        let rule = make_rule();
        let plan = make_ctid_plan(TEST_PLAN_POS, pg_sys::Oid::from(42), Some("test_table"))?;

        let result = rule.rewrite(plan, &config)?;
        assert!(!result.transformed);
        assert_eq!(count_visibility_nodes(&result.data), 0);
        Ok(())
    }

    #[pg_test]
    fn root_injection_is_idempotent() -> Result<()> {
        let config = OptimizerContext::new();
        let rule = make_rule();
        let plan = LogicalPlanBuilder::from(make_ctid_plan(
            TEST_PLAN_POS,
            pg_sys::Oid::from(42),
            Some("test_table"),
        )?)
        .filter(
            col(CtidColumn::new(TEST_PLAN_POS).to_string()).gt(datafusion::logical_expr::lit(10)),
        )?
        .build()?;

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
        let plan = build(
            LogicalPlanBuilder::from(make_ctid_plan(
                TEST_PLAN_POS,
                pg_sys::Oid::from(42),
                Some("test_table"),
            )?)
            .filter(
                col(CtidColumn::new(TEST_PLAN_POS).to_string())
                    .gt(datafusion::logical_expr::lit(10)),
            )?,
        )?;
        assert_barrier_injection(plan)
    }

    #[pg_test]
    fn inserts_visibility_below_limit_barrier() -> Result<()> {
        let config = OptimizerContext::new();
        let rule = make_rule();
        let plan = LogicalPlanBuilder::from(make_ctid_plan(
            TEST_PLAN_POS,
            pg_sys::Oid::from(42),
            Some("test_table"),
        )?)
        .filter(
            col(CtidColumn::new(TEST_PLAN_POS).to_string()).gt(datafusion::logical_expr::lit(10)),
        )?
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
        let plan = LogicalPlanBuilder::from(make_ctid_plan(
            TEST_PLAN_POS,
            pg_sys::Oid::from(42),
            Some("test_table"),
        )?)
        .filter(
            col(CtidColumn::new(TEST_PLAN_POS).to_string()).gt(datafusion::logical_expr::lit(10)),
        )?
        .sort(vec![
            col(CtidColumn::new(TEST_PLAN_POS).to_string()).sort(true, false),
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

        let rule = VisibilityFilterOptimizerRule::new();

        // Build two leaf scans and join them (inner join = not a barrier).
        let left = make_ctid_plan(POS_A, oid_a, Some("a"))?;
        let right = make_ctid_plan(POS_B, oid_b, Some("b"))?;

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

    /// Helper: builds a two-table join of the given type, runs the optimizer,
    /// and asserts partial-barrier structure: the forced child (index
    /// `forced_child`, 0=left, 1=right) is wrapped with visibility at the
    /// join, the preserved child is deferred to the root.
    fn assert_partial_barrier_join(
        join_type: datafusion::common::JoinType,
        forced_child: usize,
    ) -> Result<()> {
        let config = OptimizerContext::new();

        const POS_A: usize = 0;
        const POS_B: usize = 1;
        let oid_a = pg_sys::Oid::from(42);
        let oid_b = pg_sys::Oid::from(43);

        let (preserved_pos, preserved_oid, forced_pos, forced_oid) = if forced_child == 1 {
            (POS_A, oid_a, POS_B, oid_b)
        } else {
            (POS_B, oid_b, POS_A, oid_a)
        };

        let rule = VisibilityFilterOptimizerRule::new();

        let left = make_ctid_plan(POS_A, oid_a, Some("a"))?;
        let right = make_ctid_plan(POS_B, oid_b, Some("b"))?;

        let plan = LogicalPlanBuilder::from(left)
            .join_on(
                right,
                join_type,
                vec![
                    col(CtidColumn::new(POS_A).to_string())
                        .eq(col(CtidColumn::new(POS_B).to_string())),
                ],
            )?
            .build()?;

        let first = rule.rewrite(plan, &config)?;
        assert!(
            first.transformed,
            "{join_type:?}: first pass should transform"
        );
        // The checked side's filter would sit right below the join, where
        // nothing has reduced its rows yet, so it stays eager and only the
        // preserved side defers.
        assert_eq!(
            count_visibility_nodes(&first.data),
            1,
            "{join_type:?}: expected 1 visibility node"
        );

        // Root should be a VisibilityFilterNode covering the preserved side.
        let LogicalPlan::Extension(root_ext) = &first.data else {
            panic!("{join_type:?}: expected root to be VisibilityFilterNode");
        };
        let root_vf = root_ext
            .node
            .as_any()
            .downcast_ref::<VisibilityFilterNode>()
            .expect("root should be VisibilityFilterNode");
        assert_eq!(
            root_vf.plan_pos_oids,
            vec![(preserved_pos, preserved_oid)],
            "{join_type:?}: root visibility should cover preserved side"
        );

        // Under the root visibility node should be the join.
        let LogicalPlan::Join(join) = &root_vf.input else {
            panic!("{join_type:?}: expected child of root visibility to be Join");
        };

        let (forced_plan, preserved_plan) = if forced_child == 1 {
            (join.right.as_ref(), join.left.as_ref())
        } else {
            (join.left.as_ref(), join.right.as_ref())
        };

        let _ = (forced_pos, forced_oid);
        for (side, child) in [("forced", forced_plan), ("preserved", preserved_plan)] {
            assert!(
                !matches!(child, LogicalPlan::Extension(ext)
                    if ext.node.as_any().downcast_ref::<VisibilityFilterNode>().is_some()),
                "{join_type:?}: {side} child should NOT be wrapped with VisibilityFilterNode"
            );
        }

        let second = rule.rewrite(first.data.clone(), &config)?;
        assert!(
            !second.transformed,
            "{join_type:?}: second pass should be idempotent"
        );
        assert_eq!(count_visibility_nodes(&second.data), 1);
        Ok(())
    }

    /// A reduction below the checked side changes the answer: deferring past
    /// that reduction pays, so the partial barrier forces the check below the
    /// join.
    #[pg_test]
    fn left_join_reduced_checked_side_forces_visibility_below_join() -> Result<()> {
        let config = OptimizerContext::new();

        const POS_A: usize = 0;
        const POS_B: usize = 1;
        let oid_a = pg_sys::Oid::from(42);
        let oid_b = pg_sys::Oid::from(43);

        let rule = VisibilityFilterOptimizerRule::new();

        let left = make_ctid_plan(POS_A, oid_a, Some("a"))?;
        let right = LogicalPlanBuilder::from(make_ctid_plan(POS_B, oid_b, Some("b"))?)
            .filter(col(CtidColumn::new(POS_B).to_string()).gt(datafusion::logical_expr::lit(10)))?
            .build()?;

        let plan = LogicalPlanBuilder::from(left)
            .join_on(
                right,
                datafusion::common::JoinType::Left,
                vec![
                    col(CtidColumn::new(POS_A).to_string())
                        .eq(col(CtidColumn::new(POS_B).to_string())),
                ],
            )?
            .build()?;

        let first = rule.rewrite(plan, &config)?;
        assert!(first.transformed, "first pass should transform");
        assert_eq!(count_visibility_nodes(&first.data), 2);

        let LogicalPlan::Extension(root_ext) = &first.data else {
            panic!("expected root to be VisibilityFilterNode");
        };
        let root_vf = root_ext
            .node
            .as_any()
            .downcast_ref::<VisibilityFilterNode>()
            .expect("root should be VisibilityFilterNode");
        assert_eq!(
            root_vf.plan_pos_oids,
            vec![(POS_A, oid_a)],
            "root visibility should cover the preserved side"
        );

        let LogicalPlan::Join(join) = &root_vf.input else {
            panic!("expected child of root visibility to be Join");
        };
        let LogicalPlan::Extension(forced_ext) = join.right.as_ref() else {
            panic!("expected checked child to be wrapped with VisibilityFilterNode");
        };
        let forced_vf = forced_ext
            .node
            .as_any()
            .downcast_ref::<VisibilityFilterNode>()
            .expect("checked child should be VisibilityFilterNode");
        assert_eq!(forced_vf.plan_pos_oids, vec![(POS_B, oid_b)]);

        let second = rule.rewrite(first.data.clone(), &config)?;
        assert!(!second.transformed, "second pass should be idempotent");
        Ok(())
    }

    #[pg_test]
    fn left_join_forces_right_side_visibility_at_join() -> Result<()> {
        assert_partial_barrier_join(datafusion::common::JoinType::Left, 1)
    }

    #[pg_test]
    fn right_join_forces_left_side_visibility_at_join() -> Result<()> {
        assert_partial_barrier_join(datafusion::common::JoinType::Right, 0)
    }

    #[pg_test]
    fn left_semi_join_defers_preserved_side() -> Result<()> {
        assert_partial_barrier_join(datafusion::common::JoinType::LeftSemi, 1)
    }

    #[pg_test]
    fn left_anti_join_defers_preserved_side() -> Result<()> {
        assert_partial_barrier_join(datafusion::common::JoinType::LeftAnti, 1)
    }

    #[pg_test]
    fn right_semi_join_defers_preserved_side() -> Result<()> {
        assert_partial_barrier_join(datafusion::common::JoinType::RightSemi, 0)
    }

    #[pg_test]
    fn right_anti_join_defers_preserved_side() -> Result<()> {
        assert_partial_barrier_join(datafusion::common::JoinType::RightAnti, 0)
    }

    /// A Full barrier holds every check below it, so with nothing reducing
    /// under the join both scans keep their in-scan checks.
    #[pg_test]
    fn full_join_without_reduction_keeps_in_scan_checks() -> Result<()> {
        let config = OptimizerContext::new();

        const POS_A: usize = 0;
        const POS_B: usize = 1;

        let rule = VisibilityFilterOptimizerRule::new();

        let left = make_ctid_plan(POS_A, pg_sys::Oid::from(42), Some("a"))?;
        let right = make_ctid_plan(POS_B, pg_sys::Oid::from(43), Some("b"))?;

        let plan = LogicalPlanBuilder::from(left)
            .join_on(
                right,
                datafusion::common::JoinType::Full,
                vec![
                    col(CtidColumn::new(POS_A).to_string())
                        .eq(col(CtidColumn::new(POS_B).to_string())),
                ],
            )?
            .build()?;

        let result = rule.rewrite(plan, &config)?;
        assert!(!result.transformed);
        assert_eq!(count_visibility_nodes(&result.data), 0);
        Ok(())
    }

    /// A reduction under one side of a Full barrier makes deferring that
    /// side pay; the other side keeps its in-scan check.
    #[pg_test]
    fn full_join_reduced_side_checks_below_join() -> Result<()> {
        let config = OptimizerContext::new();

        const POS_A: usize = 0;
        const POS_B: usize = 1;
        let oid_b = pg_sys::Oid::from(43);

        let rule = VisibilityFilterOptimizerRule::new();

        let left = make_ctid_plan(POS_A, pg_sys::Oid::from(42), Some("a"))?;
        let right = LogicalPlanBuilder::from(make_ctid_plan(POS_B, oid_b, Some("b"))?)
            .filter(col(CtidColumn::new(POS_B).to_string()).gt(datafusion::logical_expr::lit(10)))?
            .build()?;

        let plan = LogicalPlanBuilder::from(left)
            .join_on(
                right,
                datafusion::common::JoinType::Full,
                vec![
                    col(CtidColumn::new(POS_A).to_string())
                        .eq(col(CtidColumn::new(POS_B).to_string())),
                ],
            )?
            .build()?;

        let first = rule.rewrite(plan, &config)?;
        assert!(first.transformed, "first pass should transform");
        assert_eq!(count_visibility_nodes(&first.data), 1);

        let LogicalPlan::Join(join) = &first.data else {
            panic!("expected root to be Join");
        };
        let LogicalPlan::Extension(ext) = join.right.as_ref() else {
            panic!("expected reduced child to be wrapped with VisibilityFilterNode");
        };
        let vf = ext
            .node
            .as_any()
            .downcast_ref::<VisibilityFilterNode>()
            .expect("reduced child should be VisibilityFilterNode");
        assert_eq!(vf.plan_pos_oids, vec![(POS_B, oid_b)]);

        let second = rule.rewrite(first.data.clone(), &config)?;
        assert!(!second.transformed, "second pass should be idempotent");
        Ok(())
    }

    #[pg_test]
    fn visibility_node_codec_roundtrip() -> Result<()> {
        use crate::scan::codec::{deserialize_logical_plan_with_runtime, serialize_logical_plan};
        use datafusion::execution::TaskContext;

        let plan = make_ctid_plan(TEST_PLAN_POS, pg_sys::Oid::from(42), Some("test_table"))?;
        let wrapped = LogicalPlan::Extension(datafusion::logical_expr::Extension {
            node: std::sync::Arc::new(VisibilityFilterNode::new(
                plan,
                vec![(TEST_PLAN_POS, pg_sys::Oid::from(42))],
                vec!["test_table".to_string()],
            )),
        });

        let bytes =
            serialize_logical_plan(&wrapped).expect("VisibilityFilterNode should serialize");
        let ctx = TaskContext::default();
        let decoded = deserialize_logical_plan_with_runtime(&bytes, &ctx, None, None, None, vec![])
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
        assert_eq!(vis.table_names, vec!["test_table".to_string()]);
        Ok(())
    }
}

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
//!    `TantivyLookupExec` chain above it so visibility runs before lookup work.
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
use std::sync::{Arc, Mutex};

use arrow_array::{Array, ArrayRef, RecordBatch, UInt64Array};
use arrow_schema::SchemaRef;
use async_trait::async_trait;
use datafusion::arrow::compute::kernels::boolean::{and, is_not_null};
use datafusion::catalog::default_table_source::DefaultTableSource;
use datafusion::common::tree_node::{Transformed, TreeNode, TreeNodeRecursion};
use datafusion::common::{DFSchemaRef, DataFusionError, Result};
use datafusion::execution::{SendableRecordBatchStream, SessionState, TaskContext};
use datafusion::logical_expr::{Extension, LogicalPlan, UserDefinedLogicalNode};
use datafusion::optimizer::optimizer::ApplyOrder;
use datafusion::optimizer::{OptimizerConfig, OptimizerRule};
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::filter_pushdown::{
    ChildFilterDescription, FilterDescription, FilterPushdownPhase, FilterPushdownPropagation,
};
use datafusion::physical_plan::metrics::{
    BaselineMetrics, ExecutionPlanMetricsSet, MetricsSet, RecordOutput,
};
use datafusion::physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties};
use datafusion::physical_planner::{ExtensionPlanner, PhysicalPlanner};
use pgrx::pg_sys;

use crate::index::fast_fields_helper::FFHelper;
use crate::postgres::customscan::joinscan::build::{JoinType, RelNode};
use crate::postgres::customscan::joinscan::CtidColumn;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::scan::deferred_encode::unpack_doc_address;
use crate::scan::execution_plan::UnsafeSendStream;
use crate::scan::table_provider::{PgSearchTableProvider, VisibilitySourceMetadata};
use crate::scan::tantivy_lookup_exec::TantivyLookupExec;
use arrow_select::filter::filter_record_batch;

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

fn pg_search_provider_from_scan(
    scan: &datafusion::logical_expr::TableScan,
) -> Option<&PgSearchTableProvider> {
    let source = scan.source.as_ref();
    if let Some(default_source) = source.as_any().downcast_ref::<DefaultTableSource>() {
        default_source
            .table_provider
            .as_any()
            .downcast_ref::<PgSearchTableProvider>()
    } else {
        source.as_any().downcast_ref::<PgSearchTableProvider>()
    }
}

fn collect_visibility_source_metadata(
    plan: &LogicalPlan,
) -> Result<BTreeMap<usize, VisibilitySourceMetadata>> {
    let mut metadata = BTreeMap::new();

    plan.apply(|node| {
        if let LogicalPlan::TableScan(scan) = node {
            if let Some(provider) = pg_search_provider_from_scan(scan) {
                if let Some(source_metadata) = provider.visibility_source_metadata() {
                    if let Some(prev_metadata) = metadata
                        .insert(source_metadata.plan_position, source_metadata.clone())
                    {
                        if prev_metadata != source_metadata {
                            return Err(DataFusionError::Internal(format!(
                                "VisibilityFilterInjection: conflicting metadata for plan_position {}",
                                source_metadata.plan_position,
                            )));
                        }
                    }
                }
            }
        }

        Ok(TreeNodeRecursion::Continue)
    })?;

    Ok(metadata)
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
        let plan_pos_metadata = collect_visibility_source_metadata(&plan)?;

        if plan_pos_metadata.is_empty() {
            return Ok(Transformed::no(plan));
        }

        let (result, final_state) = analyze_and_inject(plan, &plan_pos_metadata)?;

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

/// Returns the plan_positions whose ctid columns are still packed DocAddresses
/// at the output of this join subtree.
///
/// This shared barrier analysis is used while translating join-level search
/// predicates so each `SearchPredicateUDF` knows whether to emit packed or real
/// ctids at the point where it is attached.
pub fn deferred_plan_positions(node: &RelNode) -> crate::api::HashSet<usize> {
    fn collect(node: &RelNode, acc: &mut crate::api::HashSet<usize>) {
        match node {
            RelNode::Scan(source) => {
                acc.insert(source.plan_position);
            }
            RelNode::Join(join) => {
                if matches!(join.join_type, JoinType::Inner) {
                    collect(&join.left, acc);
                    collect(&join.right, acc);
                }
            }
            RelNode::Filter(filter) => collect(&filter.input, acc),
        }
    }

    let mut deferred = crate::api::HashSet::default();
    collect(node, &mut deferred);
    deferred
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
    if let LogicalPlan::Extension(ext) = &plan {
        if let Some(vf) = ext.node.as_any().downcast_ref::<VisibilityFilterNode>() {
            for &(plan_pos, _) in &vf.plan_pos_oids {
                merged.insert(plan_pos, VisibilityStatus::Verified);
            }
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

    // Barrier nodes and lineage drops both force visibility injection here.
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
        // TODO: For Left/Right/Full joins, the preserved side(s) could remain
        // deferred past the barrier. Currently all non-inner joins are treated
        // as full barriers forcing both sides to be resolved.
        LogicalPlan::Join(join) => !matches!(join.join_type, datafusion::common::JoinType::Inner),
        _ => false,
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

fn wrap_visibility_below_lookup_chain(
    input: Arc<dyn ExecutionPlan>,
    plan_pos_oids: Vec<(usize, pg_sys::Oid)>,
    table_names: Vec<String>,
) -> Result<Arc<dyn ExecutionPlan>> {
    let mut lookups = Vec::new();
    let mut current = input;

    while current
        .as_any()
        .downcast_ref::<TantivyLookupExec>()
        .is_some()
    {
        let child = Arc::clone(current.children()[0]);
        lookups.push(current);
        current = child;
    }

    let mut result = Arc::new(VisibilityFilterExec::new(
        current,
        plan_pos_oids,
        table_names,
    )?) as Arc<dyn ExecutionPlan>;
    for lookup in lookups.into_iter().rev() {
        result = lookup.with_new_children(vec![result])?;
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
        _session_state: &SessionState,
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
    /// Table names for EXPLAIN display, parallel to plan_pos_oids.
    table_names: Vec<String>,
    properties: Arc<PlanProperties>,
    metrics: ExecutionPlanMetricsSet,
    /// Per-plan_position FFHelper for resolving packed DocAddresses to real ctids.
    /// Wired by `VisibilityCtidResolverRule` after plan construction.
    /// Indexed by plan_position (0, 1, 2...).
    ctid_resolvers: Mutex<Vec<Option<Arc<FFHelper>>>>,
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
        let resolver_len = plan_pos_oids
            .iter()
            .map(|(p, _)| *p)
            .max()
            .map_or(0, |m| m + 1);
        Ok(Self {
            input,
            plan_pos_oids,
            table_names,
            properties,
            metrics: ExecutionPlanMetricsSet::new(),
            ctid_resolvers: Mutex::new(vec![None; resolver_len]),
        })
    }

    /// Wire an FFHelper for resolving packed DocAddresses to real ctids for the given plan_position.
    pub fn set_ctid_resolver(&self, plan_pos: usize, ffhelper: Arc<FFHelper>) {
        let mut resolvers = self
            .ctid_resolvers
            .lock()
            .expect("ctid_resolvers lock poisoned");
        if plan_pos >= resolvers.len() {
            resolvers.resize(plan_pos + 1, None);
        }
        resolvers[plan_pos] = Some(ffhelper);
    }

    pub fn plan_pos_oids(&self) -> &[(usize, pg_sys::Oid)] {
        &self.plan_pos_oids
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
            self.table_names.clone(),
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
            *new_resolvers = resolvers.clone();
        }
        Ok(Arc::new(new_exec))
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
        // We block ctid_* columns (packed DocAddresses below this node) and
        // allow all other columns through for filter pushdown.
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
        let child_desc = ChildFilterDescription::from_child_with_allowed_indices(
            &parent_filters,
            allowed_indices,
            &self.input,
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

        let resolvers = self
            .ctid_resolvers
            .lock()
            .expect("ctid_resolvers lock poisoned")
            .clone();
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
            let resolver = resolvers
                .get(plan_pos)
                .and_then(|r| r.clone())
                .ok_or_else(|| {
                    DataFusionError::Execution(format!(
                        "VisibilityFilterExec: no ctid resolver wired for plan_position {plan_pos}. \
                         VisibilityCtidResolverRule must run before execute."
                    ))
                })?;
            checkers.push(CtidCheckerEntry {
                col_idx,
                checker: visibility,
                resolver,
                deferred_ctid_state: DeferredCtidMaterializationState::default(),
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

#[derive(Default)]
struct DeferredCtidMaterializationState {
    requests: Vec<(u32, usize, u32)>,
    segment_doc_ids: Vec<u32>,
    segment_ctids: Vec<Option<u64>>,
    resolved_ctids: Vec<Option<u64>>,
}

/// Resolves packed DocAddresses (UInt64) to real ctids via FFHelper.
///
/// Each packed value encodes (segment_ord, doc_id). The FFHelper's ctid()
/// column is used to look up the real ctid for each document.
///
/// TODO: This request-partitioning pattern is duplicated in `materialize_deferred_column`
/// in `tantivy_lookup_exec.rs`. Both should be unified and optimized with Arrow
/// kernels where possible.
fn materialize_deferred_ctid(
    ffhelper: &FFHelper,
    doc_addr_array: &UInt64Array,
    state: &mut DeferredCtidMaterializationState,
) -> Result<ArrayRef> {
    let num_rows = doc_addr_array.len();
    state.requests.clear();
    state.resolved_ctids.clear();
    state.resolved_ctids.resize(num_rows, None);

    // Sort by segment so each fast-field column can be batch-read with a single
    // `as_u64s` call per segment.
    for i in 0..num_rows {
        if !doc_addr_array.is_null(i) {
            let (seg_ord, doc_id) = unpack_doc_address(doc_addr_array.value(i));
            state.requests.push((seg_ord, i, doc_id));
        }
    }
    state.requests.sort_unstable_by_key(|request| request.0);

    let mut offset = 0;
    while offset < state.requests.len() {
        let seg_ord = state.requests[offset].0;
        let mut end = offset + 1;
        while end < state.requests.len() && state.requests[end].0 == seg_ord {
            end += 1;
        }

        let rows = &state.requests[offset..end];
        state.segment_doc_ids.clear();
        state
            .segment_doc_ids
            .extend(rows.iter().map(|(_, _, doc_id)| *doc_id));
        if state.segment_ctids.len() < rows.len() {
            state.segment_ctids.resize(rows.len(), None);
        }
        let segment_ctids = &mut state.segment_ctids[..rows.len()];
        segment_ctids.fill(None);
        let ctid_col = ffhelper.ctid(seg_ord);
        ctid_col.as_u64s(&state.segment_doc_ids, segment_ctids);

        for ((_, row_idx, _), maybe_ctid) in rows.iter().zip(segment_ctids.iter()) {
            state.resolved_ctids[*row_idx] = *maybe_ctid;
        }
        offset = end;
    }

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
    /// Resolves packed DocAddresses to real ctids before visibility checking.
    /// Always present: `VisibilityCtidResolverRule` guarantees wiring, and
    /// `execute()` validates at runtime.
    resolver: Arc<FFHelper>,
    deferred_ctid_state: DeferredCtidMaterializationState,
    ctid_input: Vec<Option<u64>>,
    visibility_results: Vec<Option<u64>>,
}

/// Runs visibility check for a single relation's ctid column.
/// Returns HOT-resolved ctids (None for invisible rows).
fn check_column_visibility(entry: &mut CtidCheckerEntry, ctid_array: &UInt64Array) -> ArrayRef {
    if ctid_array.null_count() != 0 {
        panic!(
            "ctid column contains {} nulls — null ctids indicate a planning or storage bug",
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

    // Resolve packed DocAddresses to real ctids in place.
    let mut columns: Vec<ArrayRef> = batch.columns().to_vec();
    for entry in checkers.iter_mut() {
        let col = &columns[entry.col_idx];
        let doc_addr_array = col.as_any().downcast_ref::<UInt64Array>().ok_or_else(|| {
            DataFusionError::Execution(format!(
                "VisibilityFilterExec: ctid column (idx {}) is not UInt64 \
                 during DocAddress resolution",
                entry.col_idx
            ))
        })?;
        let resolved = materialize_deferred_ctid(
            &entry.resolver,
            doc_addr_array,
            &mut entry.deferred_ctid_state,
        )?;
        columns[entry.col_idx] = resolved;
    }

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
// Tests for the visibility filter optimizer rule, barrier injection, codec roundtrip,
// and dead-row filtering are covered by E2E integration tests in tests/tests/joins.rs.

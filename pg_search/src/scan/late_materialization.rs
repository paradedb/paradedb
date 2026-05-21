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

use std::collections::HashSet;
use std::fmt::{self, Debug};
use std::sync::Arc;

use crate::index::fast_fields_helper::CanonicalColumn;
use crate::index::fast_fields_helper::FFHelper;
use crate::scan::deferred_encode::extract_materialized_type_from_union;
use crate::scan::execution_plan::PgSearchScanPlan;
use crate::scan::table_provider::PgSearchTableProvider;
use crate::scan::tantivy_lookup_exec::TantivyLookupExec;

use async_trait::async_trait;
use datafusion::common::tree_node::{Transformed, TreeNode, TreeNodeRecursion};
use datafusion::common::{Column, DFSchemaRef, DataFusionError, Result};
use datafusion::execution::context::SessionState;
use datafusion::logical_expr::{Expr, Extension, LogicalPlan, UserDefinedLogicalNodeCore};
use datafusion::optimizer::{OptimizerConfig, OptimizerRule};
use datafusion::physical_plan::ExecutionPlan;
use datafusion::physical_planner::{ExtensionPlanner, PhysicalPlanner};

/// `LateMaterializationRule` is a logical optimizer rule that delays the decoding of
/// string/bytes dictionaries (from Tantivy's fast fields) for as long as logically possible.
///
/// **Strategy:**
/// 1. We traverse the `LogicalPlan` bottom-up using `transform_up`.
/// 2. At the base, we intercept any `TableScan` originating from a `PgSearchTableProvider`
///    that has actively deferred columns. We wrap its source in a `UnionTableSource`,
///    which safely overrides the declared schema from `Utf8View` (which the SQL planner needs)
///    to `Union(UInt64, Utf8View)` (which reflects the true physical layout).
/// 3. As we bubble up through the plan, we evaluate each node via `should_anchor`.
///    If a node (like `Projection` or a `HashJoin` not joining on the deferred column)
///    merely passes the column through without evaluating it, we let the `Union` schema
///    bubble through the node transparently (via `recompute_schema`).
///    If a node (like `Sort`, `Filter`, or `Aggregate`) natively evaluates the deferred column,
///    we *anchor* a `LateMaterializeNode` directly underneath it.
/// 4. If the `Union` successfully bubbles all the way to the root of the plan, we wrap the
///    final result in a `LateMaterializeNode` to ensure the client receives the standard
///    materialized strings.
#[derive(Debug)]
pub struct LateMaterializationRule;

/// Traverses down from the given node to find the underlying `PgSearchTableProvider`
/// to extract the list of deferred fields that must be materialized.
fn get_deferred_fields(plan: &LogicalPlan) -> Vec<DeferredField> {
    let mut fields = Vec::new();
    let _ = plan.apply(|node| {
        if let LogicalPlan::TableScan(scan) = node {
            let source = scan.source.as_ref();

            let provider = if let Some(default_source) =
                source
                    .as_any()
                    .downcast_ref::<datafusion::catalog::default_table_source::DefaultTableSource>()
            {
                default_source
                    .table_provider
                    .as_any()
                    .downcast_ref::<PgSearchTableProvider>()
            } else {
                source.as_any().downcast_ref::<PgSearchTableProvider>()
            };

            if let Some(p) = provider {
                fields.extend(p.deferred_fields());
            }
        }
        Ok(TreeNodeRecursion::Continue)
    });
    fields
}

/// Traces a column backward through a logical plan down to the originating `TableScan`.
///
/// This is necessary because DataFusion does not natively preserve custom metadata (like our
/// `DeferredField` info) alongside a logical `Column` as it is transformed by the query plan.
/// When a query contains aliases (e.g., `SELECT description AS desc`), `Projection`s, or `Join`s
/// with fully qualified names (e.g., `p.description`), the output `Column`'s name and relation
/// often change, breaking naive string matching against the base table's column name.
///
/// Instead of relying on brittle string suffix matching (`ends_with`) or unsafe positional
/// indices, this function explicitly recursively traces the `Column`'s lineage back down the
/// plan tree to find its exact root `Column` at the `TableScan` level, allowing robust exact matching.
fn trace_column(plan: &LogicalPlan, col: &Column) -> Option<Column> {
    match plan {
        LogicalPlan::TableScan(scan) => {
            if scan.projected_schema.has_column(col) {
                // If the column exists in this table scan's projected schema, we have found the base!
                let (_, field) = scan
                    .projected_schema
                    .qualified_field_from_column(col)
                    .ok()?;
                Some(Column::from_name(field.name()))
            } else {
                None
            }
        }
        LogicalPlan::Projection(proj) => {
            let idx = proj.schema.index_of_column(col).ok()?;
            let expr = &proj.expr[idx];
            let unaliased = match expr {
                Expr::Alias(alias) => alias.expr.as_ref(),
                e => e,
            };
            if let Expr::Column(c) = unaliased {
                trace_column(proj.input.as_ref(), c)
            } else {
                None
            }
        }
        LogicalPlan::Filter(filter) => trace_column(filter.input.as_ref(), col),
        LogicalPlan::Sort(sort) => trace_column(sort.input.as_ref(), col),
        LogicalPlan::Limit(limit) => trace_column(limit.input.as_ref(), col),
        LogicalPlan::Window(window) => {
            if let Ok(idx) = window.schema.index_of_column(col) {
                let input_schema = window.input.schema();
                if idx < input_schema.fields().len() {
                    let (q, f) = input_schema.qualified_field(idx);
                    trace_column(window.input.as_ref(), &Column::new(q.cloned(), f.name()))
                } else {
                    None
                }
            } else {
                None
            }
        }
        LogicalPlan::Aggregate(agg) => {
            if let Ok(idx) = agg.schema.index_of_column(col) {
                if idx < agg.group_expr.len() {
                    let expr = &agg.group_expr[idx];
                    let unaliased = match expr {
                        Expr::Alias(alias) => alias.expr.as_ref(),
                        e => e,
                    };
                    if let Expr::Column(c) = unaliased {
                        trace_column(agg.input.as_ref(), c)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        LogicalPlan::Join(join) => {
            if join.left.schema().has_column(col) {
                trace_column(join.left.as_ref(), col)
            } else if join.right.schema().has_column(col) {
                trace_column(join.right.as_ref(), col)
            } else {
                None
            }
        }
        LogicalPlan::SubqueryAlias(alias) => {
            if let Ok(idx) = alias.schema.index_of_column(col) {
                let (q, f) = alias.input.schema().qualified_field(idx);
                trace_column(alias.input.as_ref(), &Column::new(q.cloned(), f.name()))
            } else {
                None
            }
        }
        LogicalPlan::Extension(ext) => {
            if ext.node.inputs().len() == 1 {
                trace_column(ext.node.inputs()[0], col)
            } else {
                None
            }
        }
        LogicalPlan::Union(union_plan) => {
            if let Ok(idx) = union_plan.schema.index_of_column(col) {
                let (q, f) = union_plan.inputs[0].schema().qualified_field(idx);
                trace_column(
                    union_plan.inputs[0].as_ref(),
                    &Column::new(q.cloned(), f.name()),
                )
            } else {
                None
            }
        }
        _ => {
            let inputs = plan.inputs();
            if inputs.len() == 1 {
                if let Ok(idx) = plan.schema().index_of_column(col) {
                    let input_schema = inputs[0].schema();
                    if idx < input_schema.fields().len() {
                        let (q, f) = input_schema.qualified_field(idx);
                        trace_column(inputs[0], &Column::new(q.cloned(), f.name()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
    }
}

/// Helper function to check if the given plan outputs a `Union` type that corresponds
/// to a known deferred field. If it does, it returns the actively tracked deferred fields
/// mapped accurately to the plan's current output schema (accounting for relation aliases).
fn get_union_info(
    plan: &LogicalPlan,
) -> Option<(Vec<DeferredField>, datafusion::common::DFSchemaRef)> {
    let schema = plan.schema();
    let mut has_union = false;
    for field in schema.fields() {
        if matches!(field.data_type(), arrow_schema::DataType::Union(_, _)) {
            has_union = true;
            break;
        }
    }
    if !has_union {
        return None;
    }

    let mut all_deferred = get_deferred_fields(plan);
    let mut active_deferred = Vec::new();
    let mut new_fields = Vec::new();

    for (qualifier, field) in schema.iter() {
        if let arrow_schema::DataType::Union(union_fields, _) = field.data_type() {
            let materialized_type = extract_materialized_type_from_union(union_fields);

            let materialized_field = Arc::new(arrow_schema::Field::new(
                field.name(),
                materialized_type,
                field.is_nullable(),
            ));
            new_fields.push((qualifier.cloned(), materialized_field));

            // Find the matching deferred field by tracing the column lineage back to its
            // base TableScan name. Name-matching is safe here because we consume entries from
            // all_deferred one-by-one: for a self-join both sides produce an identical
            // DeferredField (same name, is_bytes, canonical), so each Union field in the schema
            // claims its own entry without duplication.
            let col =
                datafusion::common::Column::from((qualifier.cloned().as_ref(), field.as_ref()));
            if let Some(base_col) = trace_column(plan, &col) {
                if let Some(pos) = all_deferred.iter().position(|d| d.name == base_col.name) {
                    let d = all_deferred.remove(pos);
                    active_deferred.push(d);
                }
            }
        } else {
            new_fields.push((qualifier.cloned(), field.clone()));
        }
    }

    let new_schema = Arc::new(
        datafusion::common::DFSchema::new_with_metadata(new_fields, schema.metadata().clone())
            .unwrap(),
    );

    Some((active_deferred, new_schema))
}

/// Helper function to extract all column references used by the expressions inside a node.
fn get_column_refs(node: &LogicalPlan) -> HashSet<Column> {
    let mut cols = HashSet::new();
    let exprs = node.expressions();
    for expr in &exprs {
        expr.add_column_refs(&mut cols);
    }
    cols.into_iter().cloned().collect()
}

/// Determines whether a `LateMaterializeNode` must be anchored *below* the given logical plan node.
/// If `true`, the `Union` values will be materialized into standard strings before this node executes.
/// If `false`, the `Union` schema will bubble straight through it.
fn should_anchor(node: &LogicalPlan, deferred_fields: &[DeferredField]) -> bool {
    let refs = get_column_refs(node);

    let references_deferred = refs.iter().any(|c| {
        if let Some(base_col) = trace_column(node, c) {
            deferred_fields.iter().any(|df| df.name == base_col.name)
        } else {
            false
        }
    });

    let anchor = match node {
        LogicalPlan::Filter(_) => references_deferred,
        LogicalPlan::Projection(proj) => {
            // Only anchor if the projection does something other than pass through or alias the deferred column.
            // If it's just a Column or Alias(Column), the Union can safely pass through.
            let mut anchors_deferred = false;
            for expr in &proj.expr {
                let mut cols = HashSet::new();
                expr.add_column_refs(&mut cols);
                let uses_deferred = cols.iter().any(|c| {
                    if let Some(base_col) = trace_column(node, c) {
                        deferred_fields.iter().any(|df| df.name == base_col.name)
                    } else {
                        false
                    }
                });

                if uses_deferred {
                    // Check if it's a simple pass-through or alias
                    let is_simple = match expr {
                        datafusion::logical_expr::Expr::Column(_) => true,
                        datafusion::logical_expr::Expr::Alias(alias) => {
                            matches!(
                                alias.expr.as_ref(),
                                datafusion::logical_expr::Expr::Column(_)
                            )
                        }
                        _ => false,
                    };

                    if !is_simple {
                        anchors_deferred = true;
                        break;
                    }
                }
            }
            anchors_deferred
        }
        LogicalPlan::Sort(_) => {
            // A Sort node requires the actual, materialized values to perform comparisons.
            // If the Sort orders by a deferred column, we must anchor the materialization
            // immediately below it.
            references_deferred
        }
        LogicalPlan::Aggregate(_) | LogicalPlan::Window(_) => true,
        LogicalPlan::Join(join) => {
            let mut join_refs = HashSet::new();
            for (l, r) in &join.on {
                l.add_column_refs(&mut join_refs);
                r.add_column_refs(&mut join_refs);
            }
            if let Some(filter) = &join.filter {
                filter.add_column_refs(&mut join_refs);
            }
            let join_cols: HashSet<Column> = join_refs.into_iter().cloned().collect();
            join_cols.iter().any(|c| {
                if let Some(base_col) = trace_column(node, c) {
                    deferred_fields.iter().any(|df| df.name == base_col.name)
                } else {
                    false
                }
            })
        }
        LogicalPlan::TableScan(_)
        | LogicalPlan::EmptyRelation(_)
        | LogicalPlan::Extension(_)
        | LogicalPlan::Limit(_)
        | LogicalPlan::SubqueryAlias(_) => false,
        _ => true,
    };

    anchor
}

impl OptimizerRule for LateMaterializationRule {
    fn name(&self) -> &str {
        "LateMaterialization"
    }

    fn rewrite(
        &self,
        plan: LogicalPlan,
        _config: &dyn OptimizerConfig,
    ) -> Result<Transformed<LogicalPlan>> {
        let transformed_plan = plan.transform_up(|node| {
            if let LogicalPlan::TableScan(scan) = &node {
                let provider = if let Some(default_source) = scan.source.as_any().downcast_ref::<datafusion::catalog::default_table_source::DefaultTableSource>() {
                    default_source.table_provider.as_any().downcast_ref::<PgSearchTableProvider>()
                } else {
                    scan.source.as_any().downcast_ref::<PgSearchTableProvider>()
                };

                if let Some(provider) = provider {
                    let deferred_fields = provider.deferred_fields();
                    if !deferred_fields.is_empty() {
                        let is_already_union = scan.projected_schema.fields().iter().any(|f| {
                            matches!(f.data_type(), arrow_schema::DataType::Union(_, _))
                        });

                        if is_already_union {
                            return Ok(Transformed::no(node));
                        }

                        // Tell the provider to flip its schema output from Utf8View to Union
                        provider.enable_late_materialization_schema();

                        // Now the provider natively outputs the Union schema!
                        // We must reconstruct the TableScan's projected schema to reflect this new reality.
                        let mut new_scan = scan.clone();
                        let projected_indices: Result<Vec<usize>, _> = scan.projected_schema.fields().iter()
                            .map(|f| scan.source.schema().index_of(f.name()))
                            .collect();
                        let projected_indices = projected_indices?;

                        let projected_arrow_schema = new_scan.source.schema().project(&projected_indices)?;
                        let mut new_qualified_fields = Vec::new();
                        for (i, field) in projected_arrow_schema.fields().iter().enumerate() {
                            let (qualifier, _) = scan.projected_schema.qualified_field(i);
                            new_qualified_fields.push((qualifier.cloned(), field.clone()));
                        }

                        new_scan.projected_schema = Arc::new(
                            datafusion::common::DFSchema::new_with_metadata(
                                new_qualified_fields,
                                scan.projected_schema.metadata().clone()
                            )?
                        );

                        return Ok(Transformed::yes(LogicalPlan::TableScan(new_scan)));
                    }
                }
                return Ok(Transformed::no(node));
            }

            let mut needs_anchor = false;
            let mut new_inputs = Vec::new();

            for input in node.inputs() {
                let input_plan = input.clone();
                if let Some((deferred_fields, output_schema)) = get_union_info(&input_plan) {
                    if should_anchor(&node, &deferred_fields) {
                        let extension_node = LogicalPlan::Extension(Extension {
                            node: Arc::new(LateMaterializeNode {
                                input: input_plan,
                                output_schema,
                                deferred_fields,
                            }),
                        });
                        new_inputs.push(extension_node);
                        needs_anchor = true;
                    } else {
                        new_inputs.push(input_plan);
                    }
                } else {
                    new_inputs.push(input_plan);
                }
            }

            if needs_anchor {
                let new_node = node.with_new_exprs(node.expressions(), new_inputs)?;
                Ok(Transformed::yes(new_node))
            } else {
                let has_union_child = new_inputs.iter().any(|i| get_union_info(i).is_some());
                if has_union_child {
                    // Union bubbled into us. We MUST recompute our schema to reflect the Union.
                    // DataFusion's `transform_up` uses `map_children` which intentionally preserves the
                    // Join's old `schema` to avoid overhead during structural recursion. 
                    // Using `Join::try_new` is the recommended way to forcefully re-evaluate `build_join_schema` 
                    // using the mutated child schemas, guaranteeing that the `Join` node correctly 
                    // reports the bubbled `Union` types to the rest of the plan.
                    if let LogicalPlan::Join(join) = &node {
                        let new_join = datafusion::logical_expr::logical_plan::Join::try_new(
                            Arc::new(new_inputs[0].clone()),
                            Arc::new(new_inputs[1].clone()),
                            join.on.clone(),
                            join.filter.clone(),
                            join.join_type,
                            join.join_constraint,
                            join.null_equality,
                            join.null_aware,
                        )?;
                        return Ok(Transformed::yes(LogicalPlan::Join(new_join)));
                    }

                    let new_node = node.with_new_exprs(node.expressions(), new_inputs)?;
                    let recomputed_node = new_node.recompute_schema()?;
                    Ok(Transformed::yes(recomputed_node))
                } else {
                    Ok(Transformed::no(node))
                }
            }
        })?;

        let final_plan = transformed_plan.data;
        if let Some((deferred_fields, output_schema)) = get_union_info(&final_plan) {
            let root_mat = LogicalPlan::Extension(Extension {
                node: Arc::new(LateMaterializeNode {
                    input: final_plan,
                    output_schema,
                    deferred_fields,
                }),
            });
            Ok(Transformed::yes(root_mat))
        } else {
            Ok(Transformed::yes(final_plan))
        }
    }
}

#[derive(Hash, PartialEq, Eq)]
pub(crate) struct LateMaterializeNode {
    pub input: LogicalPlan,
    pub output_schema: DFSchemaRef,
    pub deferred_fields: Vec<DeferredField>,
}

impl Debug for LateMaterializeNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("LateMaterializeNode")
            .field("deferred_fields", &self.deferred_fields)
            .finish()
    }
}

impl std::cmp::PartialOrd for LateMaterializeNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let input_cmp = self.input.partial_cmp(&other.input);
        if input_cmp != Some(std::cmp::Ordering::Equal) {
            return input_cmp;
        }

        // Note: Comparing `Arc` pointers is non-deterministic across runs, but DataFusion's
        // `partial_cmp` trait bound for plans only requires it for opportunistic deduplication/caching,
        // not stable semantic ordering. Deep schema comparison is too expensive here.
        let schema_cmp =
            Arc::as_ptr(&self.output_schema).partial_cmp(&Arc::as_ptr(&other.output_schema));
        if schema_cmp != Some(std::cmp::Ordering::Equal) {
            return schema_cmp;
        }

        self.deferred_fields.partial_cmp(&other.deferred_fields)
    }
}

impl UserDefinedLogicalNodeCore for LateMaterializeNode {
    fn name(&self) -> &str {
        "LateMaterialize"
    }

    fn inputs(&self) -> Vec<&LogicalPlan> {
        vec![&self.input]
    }

    fn schema(&self) -> &DFSchemaRef {
        &self.output_schema
    }

    fn expressions(&self) -> Vec<Expr> {
        vec![]
    }

    fn necessary_children_exprs(&self, output_columns: &[usize]) -> Option<Vec<Vec<usize>>> {
        Some(vec![output_columns.to_vec()])
    }

    fn fmt_for_explain(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "LateMaterialize: decode=[{}]",
            self.deferred_fields
                .iter()
                .map(|d| d.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn with_exprs_and_inputs(
        &self,
        _exprs: Vec<Expr>,
        mut inputs: Vec<LogicalPlan>,
    ) -> Result<Self> {
        let input = inputs.swap_remove(0);
        let child_schema = input.schema();

        let mut qualified_fields = Vec::new();
        // Update deferred_fields to match the new child schema, in case columns were dropped or renamed
        let mut new_deferred_fields = Vec::new();
        let mut deferred_pool = self.deferred_fields.clone();

        for (i, field) in child_schema.fields().iter().enumerate() {
            let (qualifier, _) = child_schema.qualified_field(i);

            if let arrow_schema::DataType::Union(union_fields, _) = field.data_type() {
                // When DataFusion's `OptimizeProjections` rule rebuilds nodes, it trims the schema.
                // We must manually map the incoming `Union` types back to their materialized `T` types
                // to construct a truthful output schema, avoiding invariant panics.
                let materialized_type = extract_materialized_type_from_union(union_fields);

                qualified_fields.push((
                    qualifier.cloned(),
                    Arc::new(arrow_schema::Field::new(
                        field.name(),
                        materialized_type,
                        field.is_nullable(),
                    )),
                ));

                // Find the corresponding deferred field by tracing column lineage.
                // Name-matching is safe: for a self-join both sides produce identical
                // DeferredField structs; we consume entries one-by-one so each Union
                // column in the child schema claims its own distinct slot.
                let target_col = datafusion::common::Column::from((qualifier, field.as_ref()));
                if let Some(base_col) = trace_column(&input, &target_col) {
                    if let Some(pos) = deferred_pool.iter().position(|d| d.name == base_col.name) {
                        new_deferred_fields.push(deferred_pool.remove(pos));
                    }
                }
            } else {
                qualified_fields.push((
                    qualifier.cloned(),
                    Arc::new(arrow_schema::Field::new(
                        field.name(),
                        field.data_type().clone(),
                        field.is_nullable(),
                    )),
                ));
            }
        }

        let new_output_schema = Arc::new(datafusion::common::DFSchema::new_with_metadata(
            qualified_fields,
            child_schema.metadata().clone(),
        )?);

        Ok(Self {
            input,
            output_schema: new_output_schema,
            deferred_fields: new_deferred_fields,
        })
    }
}

pub struct LateMaterializePlanner;

fn extract_ff_helper(
    plan: &Arc<dyn ExecutionPlan>,
    helpers: &mut crate::api::HashMap<u32, Arc<FFHelper>>,
) {
    if let Some(scan) = plan.as_any().downcast_ref::<PgSearchScanPlan>() {
        if scan.has_deferred_fields() {
            if let Some(ff) = scan.ffhelper() {
                helpers.insert(scan.indexrelid, ff);
            }
        }
    }

    for child in plan.children() {
        extract_ff_helper(child, helpers);
    }
}

#[async_trait]
impl ExtensionPlanner for LateMaterializePlanner {
    async fn plan_extension(
        &self,
        _planner: &dyn PhysicalPlanner,
        node: &dyn datafusion::logical_expr::UserDefinedLogicalNode,
        _logical_inputs: &[&LogicalPlan],
        physical_inputs: &[Arc<dyn ExecutionPlan>],
        _session_state: &SessionState,
    ) -> Result<Option<Arc<dyn ExecutionPlan>>> {
        if let Some(mat_node) = node.as_any().downcast_ref::<LateMaterializeNode>() {
            let input_exec = Arc::clone(&physical_inputs[0]);

            let mut ff_helpers = crate::api::HashMap::default();
            extract_ff_helper(&input_exec, &mut ff_helpers);

            if ff_helpers.is_empty() {
                return Err(DataFusionError::Plan(
                    "Could not find PgSearchScanPlan beneath LateMaterializeNode".into(),
                ));
            }

            let child_logical_schema = mat_node.input.schema();
            let mut physical_deferred_fields = Vec::with_capacity(mat_node.deferred_fields.len());

            for deferred in &mat_node.deferred_fields {
                // Scan the child schema for the Union-typed field whose base column name
                // matches this deferred field. We iterate by physical index so that duplicate
                // column names (e.g. both sides of a self-join both called "ord") each resolve
                // to their own distinct physical slot, not the first match by name.
                let mut found_col_idx: Option<usize> = None;
                for (i, field) in child_logical_schema.fields().iter().enumerate() {
                    if !matches!(field.data_type(), arrow_schema::DataType::Union(_, _)) {
                        continue;
                    }
                    // Only consider Union columns that haven't been claimed yet.
                    if physical_deferred_fields.iter().any(
                        |p: &crate::scan::tantivy_lookup_exec::PhysicalDeferredField| {
                            p.col_idx == i
                        },
                    ) {
                        continue;
                    }
                    let (q, _) = child_logical_schema.qualified_field(i);
                    let col =
                        datafusion::common::Column::from((q.cloned().as_ref(), field.as_ref()));
                    if let Some(base_col) = trace_column(&mat_node.input, &col) {
                        if base_col.name == deferred.name {
                            found_col_idx = Some(i);
                            break;
                        }
                    }
                }

                let col_idx = found_col_idx.ok_or_else(|| {
                    DataFusionError::Internal(format!(
                        "LateMaterializePlanner: could not locate physical Union column \
                         for deferred field '{}' in child schema. \
                         Child schema fields: [{}]",
                        deferred.name,
                        child_logical_schema
                            .fields()
                            .iter()
                            .enumerate()
                            .map(|(i, f)| format!("{}:{}", i, f.name()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                })?;

                physical_deferred_fields.push(
                    crate::scan::tantivy_lookup_exec::PhysicalDeferredField {
                        col_idx,
                        display_name: deferred.name.clone(),
                        is_bytes: deferred.is_bytes,
                        canonical: deferred.canonical.clone(),
                    },
                );
            }

            let exec = TantivyLookupExec::new(input_exec, physical_deferred_fields, ff_helpers)?;

            Ok(Some(Arc::new(exec)))
        } else {
            Ok(None)
        }
    }
}

/// Tracks a deferred column's metadata through DataFusion's logical query plan.
///
/// DataFusion's logical schema engine natively tracks data types (like our `Union`)
/// as they bubble up through projections and joins. However, the schema engine does
/// *not* preserve custom metadata attached to fields.
///
/// We use `DeferredField` to manually carry the Tantivy `ff_index` and `is_bytes` metadata
/// alongside the logical column's base `name`. As columns are renamed
/// or fully qualified (e.g. from `name` to `p.name` or `col_1`), `LateMaterializationRule`
/// uses logical plan tracing to recursively discover the original table scan column,
/// allowing exact matching against this base `name`.
///
/// When the logical plan is converted to a physical plan, this is resolved into a
/// `PhysicalDeferredField`.
#[derive(
    Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct DeferredField {
    pub name: String,
    pub is_bytes: bool,
    pub canonical: CanonicalColumn,
}

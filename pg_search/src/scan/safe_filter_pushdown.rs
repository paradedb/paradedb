// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;

use datafusion::common::Result;
use datafusion::common::config::ConfigOptions;
use datafusion::common::tree_node::{Transformed, TreeNode};
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::PhysicalExpr;
use datafusion::physical_expr::expressions::Column;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_optimizer::filter_pushdown::FilterPushdown;
use datafusion::physical_plan::filter_pushdown::{
    ChildPushdownResult, FilterDescription, FilterPushdownPhase, FilterPushdownPropagation,
};
use datafusion::physical_plan::projection::ProjectionExec;
use datafusion::physical_plan::{
    ChildrenPropertiesMode, DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties,
    ReplaceChildrenOptions,
};

#[derive(Debug)]
pub struct SafeFilterPushdown {
    inner: FilterPushdown,
    static_filters: bool,
}

impl SafeFilterPushdown {
    pub fn pre() -> Self {
        Self {
            inner: FilterPushdown::new(),
            static_filters: true,
        }
    }

    pub fn post() -> Self {
        Self {
            inner: FilterPushdown::new_post_optimization(),
            static_filters: false,
        }
    }
}

impl PhysicalOptimizerRule for SafeFilterPushdown {
    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        config: &ConfigOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let guarded = rewrite_barriers(plan, true, self.static_filters)?;
        rewrite_barriers(
            self.inner.optimize(guarded, config)?,
            false,
            self.static_filters,
        )
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn schema_check(&self) -> bool {
        true
    }
}

// datafusion's remapper uses the first child field with a given name. block
// pushdown at a boundary where that lookup could select the wrong column.
fn rewrite_barriers(
    plan: Arc<dyn ExecutionPlan>,
    insert: bool,
    static_filters: bool,
) -> Result<Arc<dyn ExecutionPlan>> {
    let plan = if !insert {
        if let Some(barrier) = plan.downcast_ref::<PushdownBarrier>() {
            Arc::clone(&barrier.inner)
        } else if let Some(projection) = plan.downcast_ref::<PositionedProjection>() {
            Arc::clone(&projection.inner)
        } else {
            plan
        }
    } else {
        plan
    };

    let children = plan.children();
    let mut changed = false;
    let mut rewritten = Vec::with_capacity(children.len());
    for child in &children {
        let new_child = rewrite_barriers(Arc::clone(child), insert, static_filters)?;
        changed |= !Arc::ptr_eq(child, &new_child);
        rewritten.push(new_child);
    }
    let plan = if changed {
        plan.replace_children(
            rewritten,
            ReplaceChildrenOptions::new(ChildrenPropertiesMode::Recompute),
        )?
    } else {
        plan
    };

    let duplicate_names = |schema: &arrow_schema::Schema| {
        let mut names = HashSet::new();
        schema
            .fields()
            .iter()
            .any(|field| !names.insert(field.name()))
    };
    if insert && !static_filters && plan.is::<ProjectionExec>() && duplicate_names(&plan.schema()) {
        return Ok(Arc::new(PositionedProjection { inner: plan }));
    }
    if insert
        && (static_filters
            || plan.is::<datafusion::physical_plan::filter::FilterExec>()
            || plan.is::<datafusion::physical_plan::joins::HashJoinExec>())
        && (plan
            .children()
            .iter()
            .any(|child| duplicate_names(&child.schema()))
            || (static_filters
                && plan.is::<datafusion::physical_plan::projection::ProjectionExec>()
                && duplicate_names(&plan.schema())))
    {
        Ok(Arc::new(PushdownBarrier { inner: plan }))
    } else {
        Ok(plan)
    }
}

#[derive(Debug)]
struct PositionedProjection {
    inner: Arc<dyn ExecutionPlan>,
}

impl DisplayAs for PositionedProjection {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt_as(t, f)
    }
}

impl ExecutionPlan for PositionedProjection {
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn properties(&self) -> &Arc<PlanProperties> {
        self.inner.properties()
    }
    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        self.inner.children()
    }
    fn apply_expressions(
        &self,
        f: &mut dyn FnMut(
            &Arc<dyn PhysicalExpr>,
        ) -> Result<datafusion::common::tree_node::TreeNodeRecursion>,
    ) -> Result<datafusion::common::tree_node::TreeNodeRecursion> {
        self.inner.apply_expressions(f)
    }
    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let inner = Arc::clone(&self.inner).replace_children(
            children,
            ReplaceChildrenOptions::new(ChildrenPropertiesMode::Recompute),
        )?;
        Ok(Arc::new(Self { inner }))
    }
    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        self.inner.execute(partition, context)
    }
    fn gather_filters_for_pushdown(
        &self,
        _phase: FilterPushdownPhase,
        filters: Vec<Arc<dyn PhysicalExpr>>,
        _config: &ConfigOptions,
    ) -> Result<FilterDescription> {
        let projection = self.inner.downcast_ref::<ProjectionExec>().ok_or_else(|| {
            datafusion::common::DataFusionError::Internal("expected projection".into())
        })?;
        let output_schema = projection.schema();
        let mut rewritten = Vec::with_capacity(filters.len());
        for filter in filters {
            let mapped = filter
                .transform_down(|expr| {
                    if let Some(column) = expr.downcast_ref::<Column>() {
                        if output_schema
                            .fields()
                            .get(column.index())
                            .is_some_and(|f| f.name() == column.name())
                        {
                            return Ok(Transformed::yes(Arc::clone(
                                &projection.expr()[column.index()].expr,
                            )));
                        }
                        return Ok(Transformed::yes(Arc::new(Column::new(
                            "__invalid_projection_column__",
                            usize::MAX,
                        ))
                            as Arc<dyn PhysicalExpr>));
                    }
                    Ok(Transformed::no(expr))
                })?
                .data;
            rewritten.push(mapped);
        }
        let desc = crate::scan::filter_pushdown::schema_preserving_child_filter_description(
            &rewritten,
            &projection.input().schema(),
            None,
        )?;
        Ok(FilterDescription::new().with_child(desc))
    }
    fn handle_child_pushdown_result(
        &self,
        _phase: FilterPushdownPhase,
        result: ChildPushdownResult,
        _config: &ConfigOptions,
    ) -> Result<FilterPushdownPropagation<Arc<dyn ExecutionPlan>>> {
        Ok(FilterPushdownPropagation::if_all(result))
    }
}

#[derive(Debug)]
struct PushdownBarrier {
    inner: Arc<dyn ExecutionPlan>,
}

impl DisplayAs for PushdownBarrier {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt_as(t, f)
    }
}

// the default pushdown methods reject parent filters. the wrapper is removed
// immediately after the optimizer pass, leaving the original node in place.
impl ExecutionPlan for PushdownBarrier {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn properties(&self) -> &Arc<PlanProperties> {
        self.inner.properties()
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        self.inner.children()
    }

    fn apply_expressions(
        &self,
        f: &mut dyn FnMut(
            &Arc<dyn datafusion::physical_expr::PhysicalExpr>,
        ) -> Result<datafusion::common::tree_node::TreeNodeRecursion>,
    ) -> Result<datafusion::common::tree_node::TreeNodeRecursion> {
        self.inner.apply_expressions(f)
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let inner = Arc::clone(&self.inner).replace_children(
            children,
            ReplaceChildrenOptions::new(ChildrenPropertiesMode::Recompute),
        )?;
        Ok(Arc::new(Self { inner }))
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        self.inner.execute(partition, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_schema::{DataType, Field, Schema};
    use datafusion::arrow::array::{ArrayRef, BooleanArray, Int64Array};
    use datafusion::arrow::record_batch::RecordBatch;
    use datafusion::logical_expr::Operator;
    use datafusion::physical_expr::expressions::{BinaryExpr, DynamicFilterPhysicalExpr, lit};
    use datafusion::physical_plan::empty::EmptyExec;
    use datafusion::physical_plan::filter_pushdown::PushedDown;
    use datafusion::physical_plan::projection::ProjectionExpr;

    #[test]
    fn dynamic_projection_preserves_column_position() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("a", DataType::Int64, false),
            Field::new("b", DataType::Int64, false),
        ]));
        let input: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(schema.clone()));
        let projection: Arc<dyn ExecutionPlan> = Arc::new(
            ProjectionExec::try_new(
                vec![
                    ProjectionExpr {
                        expr: Arc::new(Column::new("a", 0)),
                        alias: "id".into(),
                    },
                    ProjectionExpr {
                        expr: Arc::new(Column::new("b", 1)),
                        alias: "id".into(),
                    },
                ],
                input,
            )
            .unwrap(),
        );
        let original: Arc<dyn PhysicalExpr> = Arc::new(Column::new("id", 0));
        let dynamic = Arc::new(DynamicFilterPhysicalExpr::new(
            vec![original.clone()],
            datafusion::physical_expr::expressions::lit(true),
        ));
        dynamic
            .update(Arc::new(BinaryExpr::new(original, Operator::Eq, lit(4i64))))
            .unwrap();
        let guarded = rewrite_barriers(projection, true, false).unwrap();
        let desc = guarded
            .gather_filters_for_pushdown(
                FilterPushdownPhase::Post,
                vec![dynamic],
                &ConfigOptions::default(),
            )
            .unwrap();
        let filters = desc.parent_filters();
        assert!(matches!(filters[0][0].discriminant, PushedDown::Yes));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(Int64Array::from(vec![1, 4])) as ArrayRef,
                Arc::new(Int64Array::from(vec![4, 5])) as ArrayRef,
            ],
        )
        .unwrap();
        let result = filters[0][0]
            .predicate
            .evaluate(&batch)
            .unwrap()
            .into_array(2)
            .unwrap();
        let result = result.as_any().downcast_ref::<BooleanArray>().unwrap();
        assert!(!result.value(0));
        assert!(result.value(1));
    }
}

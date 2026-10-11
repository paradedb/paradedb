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

use std::sync::Arc;

use datafusion::common::Result;
use datafusion::common::tree_node::Transformed;
use datafusion::logical_expr::expr_rewriter::unalias;
use datafusion::logical_expr::{Aggregate, EmptyRelation, Expr, LogicalPlan};
use datafusion::optimizer::optimizer::ApplyOrder;
use datafusion::optimizer::{OptimizerConfig, OptimizerRule};

use crate::postgres::customscan::datafusion::topk_agg::TOPK_AS_AGG_NAME;

/// Optimizer rule that propagates [`EmptyRelation`] through [`LogicalPlan::Unnest`]
/// and through the Top-K aggregate.
///
/// DataFusion's built-in `PropagateEmptyRelation` rule does not handle `LogicalPlan::Unnest`,
/// and it keeps an aggregate without group keys over an empty input, since such an
/// aggregate produces one row. Because `datafusion-proto` drops schema information when
/// serializing `EmptyRelation`, deserializing the node left on top of it then fails
/// looking for its columns in the empty schema.
///
/// Unnesting zero rows always produces zero rows with the schema of the `Unnest` node.
/// The Top-K aggregate (`topk_as_agg` with no group keys) over zero rows produces one
/// row holding an empty `__topk` list, which the `unnest` always placed above it turns
/// back into zero rows; so the aggregate can become an empty relation with its own
/// schema, and the `Unnest` and the rest of the plan empty out after it.
#[derive(Default, Debug)]
pub struct PropagateEmptyUnnestRule;

fn is_empty(input: &LogicalPlan) -> bool {
    matches!(input, LogicalPlan::EmptyRelation(empty) if !empty.produce_one_row)
}

/// The Top-K aggregate: no group keys and a `topk_as_agg` call. Its output always
/// feeds an `unnest` of the `__topk` list (see `apply_topk_as_agg`).
fn is_topk_aggregate(agg: &Aggregate) -> bool {
    agg.group_expr.is_empty()
        && agg.aggr_expr.iter().any(|e| match unalias(e.clone()) {
            Expr::AggregateFunction(f) => f.func.name() == TOPK_AS_AGG_NAME,
            _ => false,
        })
}

impl OptimizerRule for PropagateEmptyUnnestRule {
    fn name(&self) -> &str {
        "propagate_empty_unnest"
    }

    fn apply_order(&self) -> Option<ApplyOrder> {
        Some(ApplyOrder::BottomUp)
    }

    fn supports_rewrite(&self) -> bool {
        true
    }

    fn rewrite(
        &self,
        plan: LogicalPlan,
        _config: &dyn OptimizerConfig,
    ) -> Result<Transformed<LogicalPlan>> {
        let schema = match &plan {
            LogicalPlan::Unnest(unnest) if is_empty(&unnest.input) => &unnest.schema,
            LogicalPlan::Aggregate(agg) if is_topk_aggregate(agg) && is_empty(&agg.input) => {
                &agg.schema
            }
            _ => return Ok(Transformed::no(plan)),
        };
        Ok(Transformed::yes(LogicalPlan::EmptyRelation(
            EmptyRelation {
                produce_one_row: false,
                schema: Arc::clone(schema),
            },
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion::common::DFSchema;
    use datafusion::logical_expr::builder::LogicalPlanBuilder;
    use datafusion::optimizer::OptimizerContext;

    #[test]
    fn propagate_empty_unnest_rule_transforms_empty_child() -> Result<()> {
        let schema = Arc::new(DFSchema::try_from(Schema::new(vec![Field::new(
            "tags",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            true,
        )]))?);
        let empty_input = LogicalPlan::EmptyRelation(EmptyRelation {
            produce_one_row: false,
            schema: Arc::clone(&schema),
        });
        let unnest_plan = LogicalPlanBuilder::from(empty_input)
            .unnest_column("tags")?
            .build()?;

        let rule = PropagateEmptyUnnestRule;
        let config = OptimizerContext::default();
        let transformed = rule.rewrite(unnest_plan, &config)?;
        assert!(transformed.transformed);
        assert!(matches!(
            transformed.data,
            LogicalPlan::EmptyRelation(EmptyRelation {
                produce_one_row: false,
                ..
            })
        ));
        Ok(())
    }

    fn id_ctid_rows(produce_one_row: bool) -> Result<LogicalPlan> {
        let schema = Arc::new(DFSchema::try_from(Schema::new(vec![
            Field::new("id", DataType::Int64, true),
            Field::new("ctid", DataType::UInt64, true),
        ]))?);
        Ok(LogicalPlan::EmptyRelation(EmptyRelation {
            produce_one_row,
            schema,
        }))
    }

    #[test]
    fn propagate_empty_unnest_rule_transforms_topk_aggregate_over_empty_child() -> Result<()> {
        use crate::postgres::customscan::datafusion::topk_agg::{
            TOPK_AGG_ROWS_COL_NAME, topk_as_agg,
        };
        use datafusion::logical_expr::col;

        let topk = topk_as_agg(&[col("id"), col("ctid")], vec![], 3, &[1], false);
        let plan = LogicalPlanBuilder::from(id_ctid_rows(false)?)
            .aggregate(Vec::<Expr>::new(), vec![topk.alias(TOPK_AGG_ROWS_COL_NAME)])?
            .build()?;
        let schema = Arc::clone(plan.schema());

        let transformed = PropagateEmptyUnnestRule.rewrite(plan, &OptimizerContext::default())?;
        assert!(transformed.transformed);
        match transformed.data {
            LogicalPlan::EmptyRelation(EmptyRelation {
                produce_one_row: false,
                schema: empty_schema,
            }) => assert_eq!(empty_schema, schema),
            other => panic!("expected an empty relation, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn propagate_empty_unnest_rule_keeps_other_aggregates_over_empty_child() -> Result<()> {
        use datafusion::functions_aggregate::count::count_all;

        let plan = LogicalPlanBuilder::from(id_ctid_rows(false)?)
            .aggregate(Vec::<Expr>::new(), vec![count_all()])?
            .build()?;

        let transformed = PropagateEmptyUnnestRule.rewrite(plan, &OptimizerContext::default())?;
        assert!(!transformed.transformed);
        Ok(())
    }

    #[test]
    fn propagate_empty_unnest_rule_ignores_produce_one_row() -> Result<()> {
        let schema = Arc::new(DFSchema::try_from(Schema::new(vec![Field::new(
            "tags",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            true,
        )]))?);
        let empty_input = LogicalPlan::EmptyRelation(EmptyRelation {
            produce_one_row: true,
            schema: Arc::clone(&schema),
        });
        let unnest_plan = LogicalPlanBuilder::from(empty_input)
            .unnest_column("tags")?
            .build()?;

        let rule = PropagateEmptyUnnestRule;
        let config = OptimizerContext::default();
        let transformed = rule.rewrite(unnest_plan, &config)?;
        assert!(!transformed.transformed);
        Ok(())
    }
}

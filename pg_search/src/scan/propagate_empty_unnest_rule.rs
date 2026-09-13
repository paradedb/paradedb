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
use datafusion::logical_expr::{EmptyRelation, LogicalPlan};
use datafusion::optimizer::optimizer::ApplyOrder;
use datafusion::optimizer::{OptimizerConfig, OptimizerRule};

/// Optimizer rule that propagates [`EmptyRelation`] through [`LogicalPlan::Unnest`].
///
/// DataFusion's built-in `PropagateEmptyRelation` rule does not handle `LogicalPlan::Unnest`,
/// leaving `Unnest` sitting on top of `EmptyRelation`. Because `datafusion-proto` drops
/// schema information when serializing `EmptyRelation`, deserializing `Unnest` subsequently
/// fails looking for its unnest column in the empty schema.
///
/// Unnesting zero rows always produces zero rows with the schema of the `Unnest` node.
#[derive(Default, Debug)]
pub struct PropagateEmptyUnnestRule;

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
        match plan {
            LogicalPlan::Unnest(ref unnest) => {
                if let LogicalPlan::EmptyRelation(empty) = unnest.input.as_ref()
                    && !empty.produce_one_row
                {
                    Ok(Transformed::yes(LogicalPlan::EmptyRelation(
                        EmptyRelation {
                            produce_one_row: false,
                            schema: Arc::clone(&unnest.schema),
                        },
                    )))
                } else {
                    Ok(Transformed::no(plan))
                }
            }
            _ => Ok(Transformed::no(plan)),
        }
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

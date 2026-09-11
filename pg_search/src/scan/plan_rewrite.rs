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

//! A bottom-up plan rewrite that lets a node meet its new children.

use std::sync::Arc;

use datafusion::common::Result;
use datafusion::common::tree_node::Transformed;
use datafusion::physical_plan::{ExecutionPlan, replace_children_if_necessary};

/// Rewrites `plan` bottom-up, giving `f` each node together with its already-rewritten
/// children. `f` answers with [`Transformed::yes`] and the node that takes this one's place,
/// or with [`Transformed::no`] to let the walk put the new children back on its own.
///
/// `TreeNode::transform_up` puts the new children back before `f` runs. A node that checks
/// its input against its own state then fails while `f` is still one step away from fixing
/// it: a `TantivyDecodeExec` refuses a child that already decoded the column the rewrite is
/// about to take away from it. Handing `f` the children first keeps the two in one step, and
/// every node `f` leaves alone is put back together the way `transform_up` would.
pub fn transform_up_with_children<F>(
    plan: Arc<dyn ExecutionPlan>,
    f: &mut F,
) -> Result<Transformed<Arc<dyn ExecutionPlan>>>
where
    F: FnMut(
        Arc<dyn ExecutionPlan>,
        &[Arc<dyn ExecutionPlan>],
    ) -> Result<Transformed<Arc<dyn ExecutionPlan>>>,
{
    let mut children = Vec::with_capacity(plan.children().len());
    let mut children_changed = false;
    for child in plan.children() {
        let rewritten = transform_up_with_children(Arc::clone(child), f)?;
        children_changed |= rewritten.transformed;
        children.push(rewritten.data);
    }

    let answered = f(Arc::clone(&plan), &children)?;
    if answered.transformed {
        return Ok(answered);
    }
    let node = replace_children_if_necessary(answered.data, children)?;
    Ok(if children_changed {
        Transformed::yes(node)
    } else {
        Transformed::no(node)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_schema::{DataType, Field, Schema, SchemaRef};
    use datafusion::physical_expr::expressions::Column;
    use datafusion::physical_expr::projection::ProjectionExpr;
    use datafusion::physical_plan::empty::EmptyExec;
    use datafusion::physical_plan::projection::ProjectionExec;

    fn leaf(ty: DataType) -> Arc<dyn ExecutionPlan> {
        let schema: SchemaRef = Arc::new(Schema::new(vec![Field::new("a", ty, true)]));
        Arc::new(EmptyExec::new(schema))
    }

    fn projection(input: Arc<dyn ExecutionPlan>) -> Arc<dyn ExecutionPlan> {
        Arc::new(
            ProjectionExec::try_new(
                vec![ProjectionExpr::new(Arc::new(Column::new("a", 0)), "a")],
                input,
            )
            .unwrap(),
        )
    }

    #[test]
    fn an_untouched_plan_comes_back_as_it_went_in() {
        let plan = projection(leaf(DataType::UInt64));
        let mut step =
            |node: Arc<dyn ExecutionPlan>, _: &[Arc<dyn ExecutionPlan>]| Ok(Transformed::no(node));
        let rewritten = transform_up_with_children(Arc::clone(&plan), &mut step).unwrap();
        assert!(!rewritten.transformed);
        assert!(Arc::ptr_eq(&plan, &rewritten.data));
    }

    /// The point of the utility: a parent sees the new child before the walk puts it back, so
    /// it can answer with a node that suits the child's new type.
    #[test]
    fn a_parent_meets_its_new_child_before_the_walk_puts_it_back() {
        let plan = projection(leaf(DataType::UInt64));
        let mut step = |node: Arc<dyn ExecutionPlan>, children: &[Arc<dyn ExecutionPlan>]| {
            if node.is::<EmptyExec>() {
                return Ok(Transformed::yes(leaf(DataType::Utf8View)));
            }
            assert_eq!(
                children[0].schema().field(0).data_type(),
                &DataType::Utf8View,
                "the parent must see the retyped child"
            );
            Ok(Transformed::yes(projection(Arc::clone(&children[0]))))
        };
        let rewritten = transform_up_with_children(plan, &mut step).unwrap();
        assert!(rewritten.transformed);
        assert_eq!(
            rewritten.data.schema().field(0).data_type(),
            &DataType::Utf8View
        );
    }

    #[test]
    fn a_node_left_alone_still_gets_its_new_children() {
        let plan = projection(leaf(DataType::UInt64));
        let mut step = |node: Arc<dyn ExecutionPlan>, _: &[Arc<dyn ExecutionPlan>]| {
            if node.is::<EmptyExec>() {
                return Ok(Transformed::yes(leaf(DataType::Int32)));
            }
            Ok(Transformed::no(node))
        };
        let rewritten = transform_up_with_children(plan, &mut step).unwrap();
        assert!(rewritten.transformed);
        assert_eq!(
            rewritten.data.children()[0].schema().field(0).data_type(),
            &DataType::Int32
        );
    }
}

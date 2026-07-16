use datafusion::common::Result;
use datafusion::config::ConfigOptions;
use datafusion::physical_expr::expressions::Column;
use datafusion::physical_plan::aggregates::{AggregateExec, AggregateMode};
use std::sync::Arc;

use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::ExecutionPlan;

use crate::scan::deferred_resolve_exec::DeferredResolveExec;
use crate::scan::tantivy_lookup_exec::TantivyLookupExec;

#[derive(Debug)]
pub struct DeferredAggregateRule;

impl PhysicalOptimizerRule for DeferredAggregateRule {
    fn name(&self) -> &str {
        "DeferredAggregateRule"
    }

    fn schema_check(&self) -> bool {
        true
    }

    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        if !crate::gucs::enable_segmented_topk() {
            // We can reuse the topk flag or create a new one. For now just proceed.
            // Let's just proceed.
        }
        rewrite_plan(plan)
    }
}

fn rewrite_plan(plan: Arc<dyn ExecutionPlan>) -> Result<Arc<dyn ExecutionPlan>> {
    let children = plan.children();
    let mut new_children = Vec::with_capacity(children.len());
    let mut children_changed = false;
    for child in &children {
        let new_child = rewrite_plan(Arc::clone(child))?;
        if !Arc::ptr_eq(child, &new_child) {
            children_changed = true;
        }
        new_children.push(new_child);
    }

    let current_plan = if children_changed {
        plan.clone().with_new_children(new_children)?
    } else {
        plan.clone()
    };

    if let Some(agg) = current_plan.downcast_ref::<AggregateExec>() {
        if *agg.mode() == AggregateMode::Partial {
            let agg_children = agg.children();
            if agg_children.len() == 1 {
                if let Some(lookup) = agg_children[0].downcast_ref::<TantivyLookupExec>() {
                    let lookup_child = Arc::clone(lookup.children()[0]);

                    let deferred_fields = lookup.deferred_fields().to_vec();
                    let ffhelpers = lookup.ffhelpers().clone();

                    // 1. Inject DeferredResolveExec below AggregateExec
                    let resolved_input = Arc::new(DeferredResolveExec::try_new(
                        lookup_child,
                        ffhelpers.clone(),
                        deferred_fields.clone(),
                    )?) as Arc<dyn ExecutionPlan>;

                    // 2. Create new AggregateExec
                    // group_by expressions are still valid because the column indices don't change.
                    // But we need to update the AggregateExec to use resolved_input.
                    // We can just use with_new_children!
                    // Wait, with_new_children might check the schema.
                    // Let's try with_new_children.
                    let new_agg = current_plan.with_new_children(vec![resolved_input])?;

                    // 3. Place TantivyLookupExec above AggregateExec
                    // We need to map the deferred_fields col_idx to their new positions in the AggregateExec output schema.
                    // In a Partial Aggregate, the output schema starts with the group by keys.
                    // If a group by key is one of our deferred fields, we can find its new index.

                    let mut hoisted_deferred_fields = Vec::new();

                    if let Some(new_agg_typed) = new_agg.downcast_ref::<AggregateExec>() {
                        for (i, (expr, _name)) in
                            new_agg_typed.group_expr().expr().iter().enumerate()
                        {
                            if let Some(col) = expr.as_ref().downcast_ref::<Column>() {
                                // If this group key references a deferred field, its new index in the output is 'i'.
                                if let Some(field) =
                                    deferred_fields.iter().find(|d| d.col_idx == col.index())
                                {
                                    let mut new_field = field.clone();
                                    new_field.col_idx = i;
                                    hoisted_deferred_fields.push(new_field);
                                }
                            }
                        }
                    }

                    if hoisted_deferred_fields.is_empty() {
                        return Ok(new_agg);
                    }

                    let new_lookup = Arc::new(TantivyLookupExec::new(
                        new_agg,
                        hoisted_deferred_fields,
                        ffhelpers,
                    )?) as Arc<dyn ExecutionPlan>;

                    return Ok(new_lookup);
                }
            }
        }
    }

    Ok(current_plan)
}

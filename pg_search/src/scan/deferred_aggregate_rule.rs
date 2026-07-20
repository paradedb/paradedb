use datafusion::common::Result;
use datafusion::config::ConfigOptions;
use datafusion::physical_expr::expressions::Column;
use datafusion::physical_plan::aggregates::{AggregateExec, AggregateMode};
use std::sync::Arc;

use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::ExecutionPlan;

use crate::scan::deferred_encode::DeferMode;
use crate::scan::deferred_resolve_exec::DeferredResolveExec;
use crate::scan::execution_plan::PgSearchScanPlan;
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
        let is_partial = *agg.mode() == AggregateMode::Partial;
        let is_single = *agg.mode() == AggregateMode::Single;

        if is_partial || is_single {
            let agg_children = agg.children();
            if agg_children.len() == 1 {
                if let Some(lookup) = agg_children[0].downcast_ref::<TantivyLookupExec>() {
                    let lookup_child = Arc::clone(lookup.children()[0]);

                    let deferred_fields = lookup.deferred_fields().to_vec();
                    let ffhelpers = lookup.ffhelpers().clone();

                    // 1. Try to push DeferMode::TermOrdinal down to PgSearchScanPlan
                    // It could be directly under us, or under a single CooperativeExec.
                    let mut resolved_input: Option<Arc<dyn ExecutionPlan>> = None;
                    if let Some(lookup_child) = lookup.children().first() {
                        if let Some(coop) = lookup_child
                            .downcast_ref::<datafusion::physical_plan::coop::CooperativeExec>(
                        ) {
                            if let Some(scan) =
                                coop.children()[0].downcast_ref::<PgSearchScanPlan>()
                            {
                                if let Ok(new_scan) = scan.with_defer_mode(DeferMode::TermOrdinal) {
                                    if let Ok(new_coop) = Arc::clone(lookup_child)
                                        .with_new_children(vec![Arc::new(new_scan)])
                                    {
                                        resolved_input = Some(new_coop);
                                    }
                                }
                            }
                        } else if let Some(scan) = lookup_child.downcast_ref::<PgSearchScanPlan>() {
                            if let Ok(new_scan) = scan.with_defer_mode(DeferMode::TermOrdinal) {
                                resolved_input = Some(Arc::new(new_scan));
                            }
                        }
                    }

                    // 1b. Fallback to injecting DeferredResolveExec if we couldn't push it down to the scan
                    let resolved_input = if let Some(resolved) = resolved_input {
                        resolved
                    } else {
                        Arc::new(DeferredResolveExec::try_new(
                            lookup_child,
                            ffhelpers.clone(),
                            deferred_fields.clone(),
                        )?) as Arc<dyn ExecutionPlan>
                    };

                    // 2. Extract group and aggregate expressions
                    let mut new_group_exprs = agg.group_expr().expr().to_vec();
                    let mut extra_group_keys = 0;

                    for aggr in agg.aggr_expr().iter() {
                        let fun_name = aggr.fun().name();
                        if fun_name.eq_ignore_ascii_case("max")
                            || fun_name.eq_ignore_ascii_case("min")
                        {
                            if let Some(arg) = aggr.expressions().first() {
                                if let Some(col) = arg.as_ref().downcast_ref::<Column>() {
                                    if let Some(field) =
                                        deferred_fields.iter().find(|d| d.col_idx == col.index())
                                    {
                                        // It's a MAX/MIN on a deferred field. Add seg_id to group keys!
                                        let seg_id_name = format!("{}_seg_id", field.display_name);
                                        if let Ok(seg_id_idx) =
                                            resolved_input.schema().index_of(&seg_id_name)
                                        {
                                            let new_group_key =
                                                Arc::new(Column::new(&seg_id_name, seg_id_idx))
                                                    as Arc<
                                                        dyn datafusion::physical_expr::PhysicalExpr,
                                                    >;
                                            new_group_exprs.push((new_group_key, seg_id_name));
                                            extra_group_keys += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 3. Unconditionally construct the Partial Aggregate over the resolved_input
                    use datafusion::physical_plan::aggregates::PhysicalGroupBy;
                    let new_group_by = PhysicalGroupBy::new_single(new_group_exprs.clone());
                    let partial_agg_exec = AggregateExec::try_new(
                        AggregateMode::Partial,
                        new_group_by,
                        agg.aggr_expr().to_vec(),
                        agg.filter_expr().to_vec(),
                        Arc::clone(&resolved_input),
                        resolved_input.schema(),
                    )?;
                    let partial_agg = Arc::new(partial_agg_exec) as Arc<dyn ExecutionPlan>;

                    // 4. Map the deferred_fields col_idx to their new positions in the Partial Aggregate output schema.
                    let partial_agg_typed = partial_agg.downcast_ref::<AggregateExec>().unwrap();
                    let num_group_keys = partial_agg_typed.group_expr().expr().len();
                    let num_aggr_keys = partial_agg_typed.aggr_expr().len();

                    let mut hoisted_deferred_fields = Vec::new();

                    for (i, (expr, _name)) in
                        partial_agg_typed.group_expr().expr().iter().enumerate()
                    {
                        if let Some(col) = expr.as_ref().downcast_ref::<Column>() {
                            if let Some(field) =
                                deferred_fields.iter().find(|d| d.col_idx == col.index())
                            {
                                let mut new_field = field.clone();
                                new_field.col_idx = i;
                                hoisted_deferred_fields.push(new_field);
                            }
                        }
                    }

                    // Map MAX/MIN aggregates to deferred fields for decoding
                    if extra_group_keys > 0 {
                        for (i, aggr) in partial_agg_typed.aggr_expr().iter().enumerate() {
                            let fun_name = aggr.fun().name();
                            if fun_name.eq_ignore_ascii_case("max")
                                || fun_name.eq_ignore_ascii_case("min")
                            {
                                if let Some(arg) = aggr.expressions().first() {
                                    if let Some(col) = arg.as_ref().downcast_ref::<Column>() {
                                        if let Some(field) = deferred_fields
                                            .iter()
                                            .find(|d| d.col_idx == col.index())
                                        {
                                            let mut new_field = field.clone();
                                            new_field.col_idx = num_group_keys + i;
                                            hoisted_deferred_fields.push(new_field);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if hoisted_deferred_fields.is_empty() {
                        return Ok(current_plan);
                    }

                    // 5. Place TantivyLookupExec above Partial AggregateExec
                    let new_lookup = Arc::new(TantivyLookupExec::new(
                        partial_agg,
                        hoisted_deferred_fields,
                        ffhelpers,
                    )?) as Arc<dyn ExecutionPlan>;

                    let mut current_top = new_lookup;

                    if extra_group_keys > 0 {
                        // Project away the extra segment_id group keys
                        use datafusion::physical_plan::projection::ProjectionExec;
                        let mut proj_exprs = Vec::new();
                        let lookup_schema = current_top.schema();
                        let orig_num_group_keys = num_group_keys - extra_group_keys;

                        // 1. Keep original group keys
                        for i in 0..orig_num_group_keys {
                            proj_exprs.push((
                                Arc::new(Column::new(lookup_schema.field(i).name(), i))
                                    as Arc<dyn datafusion::physical_expr::PhysicalExpr>,
                                lookup_schema.field(i).name().clone(),
                            ));
                        }
                        // 2. Skip extra_group_keys
                        // 3. Keep aggregates
                        for i in 0..num_aggr_keys {
                            let src_idx = num_group_keys + i;
                            proj_exprs.push((
                                Arc::new(Column::new(lookup_schema.field(src_idx).name(), src_idx))
                                    as Arc<dyn datafusion::physical_expr::PhysicalExpr>,
                                lookup_schema.field(src_idx).name().clone(),
                            ));
                        }
                        current_top = Arc::new(ProjectionExec::try_new(proj_exprs, current_top)?)
                            as Arc<dyn ExecutionPlan>;
                    }

                    // 6. If original aggregate was Single, wrap with Final aggregate to merge segment-local groupings!
                    if is_single {
                        let mut final_group_exprs = Vec::new();
                        let top_schema = current_top.schema();
                        let orig_num_group_keys = num_group_keys - extra_group_keys;

                        for i in 0..orig_num_group_keys {
                            let name = top_schema.field(i).name().clone();
                            final_group_exprs.push((
                                Arc::new(Column::new(&name, i))
                                    as Arc<dyn datafusion::physical_expr::PhysicalExpr>,
                                name,
                            ));
                        }

                        let final_group_by = PhysicalGroupBy::new_single(final_group_exprs);
                        let final_agg_exec = AggregateExec::try_new(
                            AggregateMode::Final,
                            final_group_by,
                            agg.aggr_expr().to_vec(),
                            vec![None; agg.aggr_expr().len()], // filter_expr must match aggr_expr length!
                            Arc::clone(&current_top),
                            current_top.schema(),
                        )?;
                        return Ok(Arc::new(final_agg_exec) as Arc<dyn ExecutionPlan>);
                    }

                    return Ok(current_top);
                }
            }
        }
    }

    Ok(current_plan)
}

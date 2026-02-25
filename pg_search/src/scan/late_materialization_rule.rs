use datafusion::common::config::ConfigOptions;
use datafusion::common::Result;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::ExecutionPlan;
use std::sync::Arc;

use crate::index::fast_fields_helper::FFHelper;
use crate::scan::execution_plan::PgSearchScanPlan;
use crate::scan::tantivy_lookup_exec::{DeferredField, TantivyLookupExec};

#[derive(Debug)]
pub struct LateMaterializationRule;

impl PhysicalOptimizerRule for LateMaterializationRule {
    fn name(&self) -> &str {
        "LateMaterialization"
    }
    fn schema_check(&self) -> bool {
        false
    }

    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let (rewritten, maybe_pending) = rewrite(plan)?;
        if let Some(pending) = maybe_pending {
            inject(rewritten, pending)
        } else {
            Ok(rewritten)
        }
    }
}

struct PendingLookup {
    deferred_fields: Vec<DeferredField>,
    ffhelper: Arc<FFHelper>,
}

fn rewrite(
    plan: Arc<dyn ExecutionPlan>,
) -> Result<(Arc<dyn ExecutionPlan>, Option<PendingLookup>)> {
    if let Some(scan) = plan.as_any().downcast_ref::<PgSearchScanPlan>() {
        if !scan.deferred_fields().is_empty() {
            return Ok((
                plan.clone(),
                Some(PendingLookup {
                    deferred_fields: scan.deferred_fields().to_vec(),
                    ffhelper: Arc::clone(scan.ffhelper()),
                }),
            ));
        }
    }

    let children = plan.children();
    let mut new_children = Vec::with_capacity(children.len());
    let mut pending_tokens = Vec::new();

    for (idx, child) in children.into_iter().enumerate() {
        let (rewritten_child, maybe_pending) = rewrite(Arc::clone(child))?;
        new_children.push(rewritten_child);
        if let Some(p) = maybe_pending {
            pending_tokens.push((idx, p));
        }
    }

    let plan = if new_children.is_empty() {
        plan
    } else {
        plan.with_new_children(new_children)?
    };

    if pending_tokens.is_empty() {
        return Ok((plan, None));
    }

    if is_anchor(plan.as_ref()) || pending_tokens.len() > 1 {
        let mut result = plan;
        for (_, pending) in pending_tokens {
            result = inject(result, pending)?;
        }
        Ok((result, None))
    } else {
        let (_, pending) = pending_tokens.remove(0);
        Ok((plan, Some(pending)))
    }
}

fn is_anchor(plan: &dyn ExecutionPlan) -> bool {
    matches!(
        plan.name(),
        "GlobalLimitExec"
            | "LocalLimitExec"
            | "HashJoinExec"
            | "NestedLoopJoinExec"
            | "CrossJoinExec"
            | "SortMergeJoinExec"
            | "VisibilityFilterExec"
    )
}

fn inject(plan: Arc<dyn ExecutionPlan>, pending: PendingLookup) -> Result<Arc<dyn ExecutionPlan>> {
    Ok(Arc::new(TantivyLookupExec::new(
        plan,
        pending.deferred_fields,
        pending.ffhelper,
    )?))
}

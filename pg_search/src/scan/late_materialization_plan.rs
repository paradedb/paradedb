use datafusion::common::Result;
use datafusion::config::ConfigOptions;
use datafusion::logical_expr::{Extension, LogicalPlan};
use datafusion::optimizer::analyzer::AnalyzerRule;
use std::sync::Arc;

use crate::scan::logical_tantivy_lookup::LogicalTantivyLookup;
use crate::scan::table_provider::PgSearchTableProvider;
use crate::scan::tantivy_lookup_exec::DeferredField;

#[derive(Debug)]
pub struct LateMaterializationRule;

impl AnalyzerRule for LateMaterializationRule {
    fn name(&self) -> &str {
        "LateMaterialization"
    }

    fn analyze(&self, plan: LogicalPlan, _config: &ConfigOptions) -> Result<LogicalPlan> {
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
}

fn rewrite(plan: LogicalPlan) -> Result<(LogicalPlan, Option<PendingLookup>)> {
    if let LogicalPlan::TableScan(scan) = &plan {
        if let Some(provider) = scan.source.as_any().downcast_ref::<PgSearchTableProvider>() {
            let deferred = provider.deferred_fields();
            if !deferred.is_empty() {
                return Ok((
                    plan.clone(),
                    Some(PendingLookup {
                        deferred_fields: deferred,
                    }),
                ));
            }
        }
    }

    let inputs = plan.inputs();
    if inputs.is_empty() {
        return Ok((plan, None));
    }

    let mut new_inputs = Vec::with_capacity(inputs.len());
    let mut pending_tokens = Vec::new();

    for (idx, child) in inputs.iter().enumerate() {
        let (rewritten_child, maybe_pending) = rewrite((*child).clone())?;
        new_inputs.push(rewritten_child);
        if let Some(p) = maybe_pending {
            pending_tokens.push((idx, p));
        }
    }

    let plan = if new_inputs.is_empty() {
        plan
    } else {
        plan.with_new_exprs(plan.expressions(), new_inputs)?
    };

    if pending_tokens.is_empty() {
        return Ok((plan, None));
    }

    if is_anchor(&plan) || pending_tokens.len() > 1 {
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

fn is_anchor(plan: &LogicalPlan) -> bool {
    matches!(
        plan,
        LogicalPlan::Limit(_)
            | LogicalPlan::Join(_)
            | LogicalPlan::Aggregate(_)
            | LogicalPlan::Sort(_)
    )
}
fn inject(plan: LogicalPlan, pending: PendingLookup) -> Result<LogicalPlan> {
    let lookup = LogicalTantivyLookup::new(Arc::new(plan), pending.deferred_fields)?;
    Ok(LogicalPlan::Extension(Extension {
        node: Arc::new(lookup),
    }))
}

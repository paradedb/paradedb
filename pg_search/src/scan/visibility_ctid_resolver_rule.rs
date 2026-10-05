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

//! Physical optimizer rule that wires FFHelper instances from PgSearchScanPlan
//! into the VisibilityFilterExec that resolves ctid columns.
//!
//! Visibility checking needs real ctids, but when deferred visibility is enabled
//! the ctid columns hold packed DocAddresses. This rule finds the PgSearchScanPlan
//! that owns each ctid column and wires its FFHelper into the `VisibilityFilterExec` that
//! resolves them.
//!
//! This is interior mutation only (Mutex-based wiring), with no structural plan changes.

use std::sync::Arc;

use datafusion::common::config::ConfigOptions;
use datafusion::common::{DataFusionError, Result};
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::ExecutionPlan;

use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;
use crate::scan::execution_plan::find_ctid_resolver_for_plan_position;

#[derive(Debug)]
pub struct VisibilityCtidResolverRule;

impl PhysicalOptimizerRule for VisibilityCtidResolverRule {
    fn name(&self) -> &str {
        "VisibilityCtidResolver"
    }

    fn schema_check(&self) -> bool {
        // Interior mutation only — no schema changes.
        true
    }

    fn optimize(
        &self,
        plan: Arc<dyn ExecutionPlan>,
        _config: &ConfigOptions,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        walk_plan(&plan)?;
        Ok(plan)
    }
}

/// Walk the plan tree. When we find a VisibilityFilterExec,
/// wire FFHelpers from matching PgSearchScanPlans in the subtree.
fn walk_plan(plan: &Arc<dyn ExecutionPlan>) -> Result<()> {
    // VisibilityFilterExec owns ctid resolution for its plan positions.
    if let Some(vf) = plan.downcast_ref::<VisibilityFilterExec>() {
        for &(plan_pos, _) in vf.plan_pos_oids() {
            let resolver =
                find_ctid_resolver_for_plan_position(plan, plan_pos).ok_or_else(|| {
                    DataFusionError::Internal(format!(
                        "VisibilityCtidResolverRule: no PgSearchScanPlan found \
                     for VisibilityFilterExec deferred ctid plan_position {plan_pos}"
                    ))
                })?;
            vf.set_ctid_resolver(plan_pos, resolver);
        }
    }
    for child in plan.children() {
        walk_plan(child)?;
    }
    Ok(())
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::VisibilityCtidResolverRule;
    use std::sync::Arc;

    use arrow_schema::{Schema, SchemaRef};
    use pgrx::prelude::*;

    use crate::index::fast_fields_helper::FFHelper;
    use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;
    use crate::query::SearchQueryInput;
    use crate::scan::execution_plan::PgSearchScanPlan;

    fn sort_schema() -> SchemaRef {
        use arrow_schema::{DataType, Field};
        Arc::new(Schema::new(vec![Field::new(
            "sort_col",
            DataType::Int64,
            true,
        )]))
    }

    #[pg_test]
    fn wires_ctid_resolver_to_visibility_filter_exec() {
        use datafusion::physical_optimizer::PhysicalOptimizerRule;
        use pgrx::pg_sys;

        let plan_pos = 2_usize;
        let schema = sort_schema();
        let ffhelper_scan = Arc::new(FFHelper::empty());
        let scan = Arc::new(PgSearchScanPlan::new(
            None,
            schema.clone(),
            SearchQueryInput::All,
            None,
            Vec::new(),
            Some(ffhelper_scan.clone()),
            42,
            Some(plan_pos),
            1,
            None,
            None,
        ));

        let vf = Arc::new(
            VisibilityFilterExec::new(
                scan,
                vec![(plan_pos, pg_sys::Oid::INVALID)],
                vec!["test_table".to_string()],
            )
            .expect("VisibilityFilterExec::new should succeed"),
        );

        let rule = VisibilityCtidResolverRule;
        let config = datafusion::common::config::ConfigOptions::default();
        let optimized = rule
            .optimize(vf.clone(), &config)
            .expect("optimize should succeed");

        let vf_opt = optimized
            .downcast_ref::<VisibilityFilterExec>()
            .expect("optimized node should still be VisibilityFilterExec");
        assert_eq!(vf_opt.plan_pos_oids().len(), 1);
        assert_eq!(vf_opt.plan_pos_oids()[0].0, plan_pos);
    }
}

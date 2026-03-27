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
//! into VisibilityFilterExec for ctid resolution.
//!
//! VisibilityFilterExec needs real ctids to check visibility, but when deferred
//! visibility is enabled, ctid columns hold packed DocAddresses. This rule
//! finds the PgSearchScanPlan that owns each ctid column and wires its FFHelper
//! into the VisibilityFilterExec so it can resolve the packed addresses itself.
//!
//! This is interior mutation only (Mutex-based wiring) — no structural plan changes.

use std::sync::Arc;

use datafusion::common::config::ConfigOptions;
use datafusion::common::{DataFusionError, Result};
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::ExecutionPlan;

use crate::index::fast_fields_helper::FFHelper;
use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;
use crate::postgres::customscan::joinscan::CtidColumn;
use crate::scan::execution_plan::PgSearchScanPlan;

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

/// Walk the plan tree. When we find a VisibilityFilterExec, wire FFHelpers
/// from matching PgSearchScanPlans in its subtree.
fn walk_plan(plan: &Arc<dyn ExecutionPlan>) -> Result<()> {
    if let Some(vis_exec) = plan.as_any().downcast_ref::<VisibilityFilterExec>() {
        for &(plan_pos, _) in vis_exec.plan_pos_oids() {
            let ctid_field_name = CtidColumn::new(plan_pos).to_string();
            let ffhelper =
                find_ffhelper_for_ctid(plan.as_ref(), &ctid_field_name).ok_or_else(|| {
                    DataFusionError::Internal(format!(
                        "VisibilityCtidResolverRule: no PgSearchScanPlan found \
                         with deferred ctid field '{ctid_field_name}'"
                    ))
                })?;
            vis_exec.set_ctid_resolver(plan_pos, ffhelper);
        }
    }

    for child in plan.children() {
        walk_plan(child)?;
    }
    Ok(())
}

/// Search the subtree for a PgSearchScanPlan whose deferred ctid alias matches
/// the given ctid field name. Returns its FFHelper if found.
fn find_ffhelper_for_ctid(
    plan: &dyn ExecutionPlan,
    ctid_field_name: &str,
) -> Option<Arc<FFHelper>> {
    if let Some(scan) = plan.as_any().downcast_ref::<PgSearchScanPlan>() {
        if scan.deferred_ctid_alias() == Some(ctid_field_name) {
            return scan.ffhelper();
        }
    }

    for child in plan.children() {
        if let Some(helper) = find_ffhelper_for_ctid(child.as_ref(), ctid_field_name) {
            return Some(helper);
        }
    }

    None
}

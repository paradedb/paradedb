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
use crate::scan::execution_plan::PgSearchScanPlan;
use crate::scan::segmented_topk_exec::SegmentedTopKExec;

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

/// Walk the plan tree. When we find a VisibilityFilterExec or a
/// SegmentedTopKExec that has absorbed one, wire FFHelpers from matching
/// PgSearchScanPlans in the subtree.
fn walk_plan(plan: &Arc<dyn ExecutionPlan>) -> Result<()> {
    if let Some(vis_exec) = plan.as_any().downcast_ref::<VisibilityFilterExec>() {
        for &(plan_pos, _) in vis_exec.plan_pos_oids() {
            let ffhelper =
                find_ffhelper_for_plan_position(plan.as_ref(), plan_pos).ok_or_else(|| {
                    DataFusionError::Internal(format!(
                        "VisibilityCtidResolverRule: no PgSearchScanPlan found \
                         for deferred ctid plan_position {plan_pos}"
                    ))
                })?;
            vis_exec.set_ctid_resolver(plan_pos, ffhelper);
        }
    }

    // SegmentedTopKExec may have absorbed a VisibilityFilterExec and now
    // owns ctid resolution for the same plan positions.
    if let Some(stk) = plan.as_any().downcast_ref::<SegmentedTopKExec>() {
        for &(plan_pos, _) in stk.plan_pos_oids() {
            let ffhelper =
                find_ffhelper_for_plan_position(plan.as_ref(), plan_pos).ok_or_else(|| {
                    DataFusionError::Internal(format!(
                        "VisibilityCtidResolverRule: no PgSearchScanPlan found \
                         for SegmentedTopKExec deferred ctid plan_position {plan_pos}"
                    ))
                })?;
            stk.set_ctid_resolver(plan_pos, ffhelper);
        }
    }

    for child in plan.children() {
        walk_plan(child)?;
    }
    Ok(())
}

/// Search the subtree for a PgSearchScanPlan whose deferred ctid metadata matches
/// the given plan position. Returns its FFHelper if found.
fn find_ffhelper_for_plan_position(
    plan: &dyn ExecutionPlan,
    plan_position: usize,
) -> Option<Arc<FFHelper>> {
    if let Some(scan) = plan.as_any().downcast_ref::<PgSearchScanPlan>() {
        if scan.deferred_ctid_plan_position() == Some(plan_position) {
            return scan.ffhelper();
        }
    }

    for child in plan.children() {
        if let Some(helper) = find_ffhelper_for_plan_position(child.as_ref(), plan_position) {
            return Some(helper);
        }
    }

    None
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::find_ffhelper_for_plan_position;
    use std::sync::Arc;

    use arrow_schema::{Schema, SchemaRef};
    use pgrx::prelude::*;

    use crate::index::fast_fields_helper::FFHelper;
    use crate::query::SearchQueryInput;
    use crate::scan::execution_plan::PgSearchScanPlan;

    fn empty_schema() -> SchemaRef {
        Arc::new(Schema::empty())
    }

    #[pg_test]
    fn matches_scan_by_deferred_ctid_plan_position() {
        let ffhelper = Arc::new(FFHelper::empty());
        let scan = PgSearchScanPlan::new(
            vec![],
            empty_schema(),
            SearchQueryInput::All,
            None,
            Vec::new(),
            Some(ffhelper.clone()),
            0,
            Some(7),
        );

        let found = find_ffhelper_for_plan_position(&scan, 7)
            .expect("matching plan_position should find ffhelper");
        assert!(Arc::ptr_eq(&found, &ffhelper));
        assert!(find_ffhelper_for_plan_position(&scan, 6).is_none());
    }

    fn sort_schema() -> SchemaRef {
        use arrow_schema::{DataType, Field};
        Arc::new(Schema::new(vec![Field::new(
            "sort_col",
            DataType::Int64,
            true,
        )]))
    }

    fn dummy_lex_ordering(schema: &SchemaRef) -> datafusion::physical_expr::LexOrdering {
        use arrow_schema::SortOptions;
        use datafusion::physical_expr::expressions::Column;
        use datafusion::physical_expr::{LexOrdering, PhysicalSortExpr};

        let sort_expr = PhysicalSortExpr {
            expr: Arc::new(Column::new("sort_col", 0)),
            options: SortOptions::default(),
        };
        // We need the schema to create EquivalenceProperties — not used here
        let _ = schema;
        LexOrdering::new(vec![sort_expr]).expect("single-element LexOrdering must succeed")
    }

    #[pg_test]
    fn stk_plan_pos_oids_returns_empty_without_visibility_data() {
        use crate::scan::segmented_topk_exec::SegmentedTopKExec;

        let schema = sort_schema();
        let ffhelper = Arc::new(FFHelper::empty());
        let scan = PgSearchScanPlan::new(
            vec![],
            schema.clone(),
            SearchQueryInput::All,
            None,
            Vec::new(),
            Some(ffhelper.clone()),
            0,
            None,
        );
        let stk = SegmentedTopKExec::new(
            Arc::new(scan),
            dummy_lex_ordering(&schema),
            vec![],
            ffhelper,
            5,
            None,
        );

        // No visibility data absorbed → plan_pos_oids must be empty.
        assert!(
            stk.plan_pos_oids().is_empty(),
            "plan_pos_oids should be empty when no VisibilityFilterExec was absorbed"
        );
    }

    #[pg_test]
    fn stk_plan_pos_oids_returns_absorbed_data() {
        use crate::scan::segmented_topk_exec::{AbsorbedVisibilityData, SegmentedTopKExec};
        use pgrx::pg_sys;

        let plan_pos = 3_usize;
        let schema = sort_schema();
        let ffhelper_scan = Arc::new(FFHelper::empty());
        let scan = PgSearchScanPlan::new(
            vec![],
            schema.clone(),
            SearchQueryInput::All,
            None,
            Vec::new(),
            Some(ffhelper_scan.clone()),
            0,
            Some(plan_pos),
        );

        let vis_data = Arc::new(AbsorbedVisibilityData::new(
            vec![(plan_pos, pg_sys::Oid::INVALID)],
            vec!["test_table".to_string()],
        ));
        let stk = SegmentedTopKExec::new(
            Arc::new(scan),
            dummy_lex_ordering(&schema),
            vec![],
            Arc::new(FFHelper::empty()),
            5,
            Some(vis_data),
        );

        let pos_oids = stk.plan_pos_oids();
        assert_eq!(pos_oids.len(), 1, "one plan_pos_oid should be present");
        assert_eq!(pos_oids[0].0, plan_pos, "plan_position should match");

        // Wire a resolver and verify no panic.
        stk.set_ctid_resolver(plan_pos, ffhelper_scan);
    }
}

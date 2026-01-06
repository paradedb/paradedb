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

use crate::customscan::aggregatescan::exec::AggregationResultsRow;
use crate::customscan::aggregatescan::AggregateCSClause;
use crate::postgres::customscan::solve_expr::SolvePostgresExpressions;
use crate::postgres::customscan::CustomScanState;
use crate::postgres::PgSearchRelation;

use pgrx::pg_sys;

#[derive(Default)]
pub enum ExecutionState {
    #[default]
    NotStarted,
    Emitting(std::vec::IntoIter<AggregationResultsRow>),
    Completed,
}

#[derive(Default)]
pub struct AggregateScanState {
    pub state: ExecutionState,
    pub indexrelid: pg_sys::Oid,
    pub indexrel: Option<(pg_sys::LOCKMODE, PgSearchRelation)>,
    pub execution_rti: pg_sys::Index,
    pub aggregate_clause: AggregateCSClause,

    /// Target list with FuncExpr placeholders replaced by Const nodes.
    /// Used for expression projection when aggregates are wrapped in functions.
    /// The Const nodes are mutated with actual aggregate values before each
    /// ExecBuildProjectionInfo call, which bakes the current values into the
    /// compiled projection. This follows the pdbscan pattern.
    pub placeholder_targetlist: Option<*mut pg_sys::List>,

    /// Pointers to Const nodes in placeholder_targetlist, indexed by target entry position.
    /// These are mutated with aggregate values before each projection build.
    /// Indexed by target entry position (0-based), None for entries without Const nodes.
    pub const_agg_nodes: Vec<Option<*mut pg_sys::Const>>,

    /// Reusable tuple slot for aggregate result rows
    /// Created once during begin_custom_scan and cleared/reused for each row
    /// to avoid per-row memory allocation and leaks
    pub scan_slot: Option<*mut pg_sys::TupleTableSlot>,
}

impl AggregateScanState {
    pub fn open_relations(&mut self, lockmode: pg_sys::LOCKMODE) {
        self.indexrel = Some((
            lockmode,
            PgSearchRelation::with_lock(self.indexrelid, lockmode),
        ));
    }

    #[inline(always)]
    pub fn indexrel(&self) -> &PgSearchRelation {
        self.indexrel
            .as_ref()
            .map(|(_, rel)| rel)
            .expect("PdbScanState: indexrel should be initialized")
    }
}

impl CustomScanState for AggregateScanState {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState) {
        // TODO: Unused currently. See the comment on `trait CustomScanState` regarding making this
        // more useful.
    }
}

impl SolvePostgresExpressions for AggregateScanState {
    fn has_heap_filters(&mut self) -> bool {
        self.aggregate_clause.query_mut().has_heap_filters()
            || self
                .aggregate_clause
                .aggregates_mut()
                .any(|agg| agg.has_heap_filters())
    }

    fn has_postgres_expressions(&mut self) -> bool {
        self.aggregate_clause.query_mut().has_postgres_expressions()
            || self
                .aggregate_clause
                .aggregates_mut()
                .any(|agg| agg.has_postgres_expressions())
    }

    fn init_postgres_expressions(&mut self, planstate: *mut pg_sys::PlanState) {
        self.aggregate_clause
            .query_mut()
            .init_postgres_expressions(planstate);
        self.aggregate_clause
            .aggregates_mut()
            .for_each(|agg| agg.init_postgres_expressions(planstate));
    }

    fn solve_postgres_expressions(&mut self, expr_context: *mut pg_sys::ExprContext) {
        self.aggregate_clause
            .query_mut()
            .solve_postgres_expressions(expr_context);
        self.aggregate_clause
            .aggregates_mut()
            .for_each(|agg| agg.solve_postgres_expressions(expr_context));
    }
}

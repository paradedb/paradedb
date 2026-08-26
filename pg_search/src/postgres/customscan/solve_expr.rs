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

use crate::api::operator::searchqueryinput_typoid;
use crate::query::heap_field_filter::TidBitmapSet;
use crate::query::{PostgresExpression, SearchQueryInput};
use pgrx::{PgMemoryContexts, pg_sys};
use std::sync::Arc;

impl SearchQueryInput {
    /// The bitmap set attached to this query's HeapFilters, if any.
    pub fn tid_bitmap_set(&self) -> Option<Arc<TidBitmapSet>> {
        let mut found = None;
        self.visit_ref(&mut |sqi| {
            if let SearchQueryInput::HeapFilter {
                tid_bitmap_set: Some(set),
                ..
            } = sqi
                && found.is_none()
            {
                found = Some(Arc::clone(set));
            }
        });
        found
    }

    /// Attach the execution-time bitmap set to every HeapFilter that was flagged at
    /// plan time as covered by an external index's bitmap.
    pub fn attach_tid_bitmap_set(&mut self, set: &Arc<TidBitmapSet>) {
        self.visit(&mut |sqi| {
            if let SearchQueryInput::HeapFilter {
                uses_tid_bitmap: true,
                tid_bitmap_set,
                ..
            } = sqi
            {
                *tid_bitmap_set = Some(Arc::clone(set));
            }
        });
    }

    pub fn has_heap_filters(&self) -> bool {
        let mut found = false;
        self.visit_ref(&mut |sqi| {
            if let SearchQueryInput::HeapFilter { .. } = sqi {
                found = true;
            }
        });
        found
    }

    pub fn has_postgres_expressions(&self) -> bool {
        let mut found = false;
        self.visit_ref(&mut |sqi| {
            if let SearchQueryInput::PostgresExpression { .. } = sqi {
                found = true;
            }
        });
        found
    }

    pub fn has_parameters(&self) -> bool {
        let mut found = false;
        self.visit_ref(&mut |sqi| {
            if let SearchQueryInput::HeapFilter {
                always_filters,
                recheck_filters,
                ..
            } = sqi
                && always_filters
                    .iter()
                    .chain(recheck_filters.iter())
                    .any(|f| f.has_parameters())
            {
                found = true;
            }
        });
        found
    }

    /// Collects raw `Expr*` pointers for every PostgreSQL expression referenced
    /// by heap filters or PostgresExpression variants in this query tree.
    ///
    /// Used to populate `CustomScan.custom_exprs` so `finalize_plan`'s
    /// param-dependency walker sees InitPlan references (issue #5727).
    pub fn collect_expression_nodes(&mut self) -> Vec<*mut pg_sys::Node> {
        let mut nodes = Vec::new();
        self.visit(&mut |sqi| match sqi {
            SearchQueryInput::HeapFilter {
                always_filters,
                recheck_filters,
                ..
            } => {
                for filter in always_filters.iter().chain(recheck_filters.iter()) {
                    let node = unsafe { filter.get_expression_node() };
                    if !node.is_null() {
                        nodes.push(node);
                    }
                }
            }
            SearchQueryInput::PostgresExpression { expr } => {
                let node = expr.node();
                if !node.is_null() {
                    nodes.push(node);
                }
            }
            _ => {}
        });
        nodes
    }

    pub fn init_postgres_expressions(&mut self, planstate: *mut pg_sys::PlanState) -> usize {
        let mut cnt = 0;
        self.visit(&mut |sqi| {
            if let SearchQueryInput::PostgresExpression { expr } = sqi {
                expr.init(planstate);
                cnt += 1;
            }
        });
        cnt
    }

    pub fn solve_postgres_expressions(&mut self, expr_context: *mut pg_sys::ExprContext) {
        assert!(
            !expr_context.is_null(),
            "expr_context was never initialized"
        );
        unsafe {
            pg_sys::MemoryContextReset((*expr_context).ecxt_per_tuple_memory);
            self.solve_postgres_expressions_no_reset(expr_context);
        }
    }

    /// Same as `solve_postgres_expressions`, but does not reset
    /// `ecxt_per_tuple_memory` first. Callers solving several `SearchQueryInput`s
    /// against the same `ExprContext` in one pass (e.g. `JoinCSClause`, which visits
    /// multiple sources' queries) must reset the context once themselves before the
    /// first call, then use this variant for every subsequent one in the pass —
    /// otherwise each call's reset frees the rewritten expression tree solved by the
    /// call before it, and anything reading the earlier tree afterward (e.g. rebaking
    /// the logical plan) sees a dangling pointer.
    pub fn solve_postgres_expressions_no_reset(&mut self, expr_context: *mut pg_sys::ExprContext) {
        assert!(
            !expr_context.is_null(),
            "expr_context was never initialized"
        );
        unsafe {
            PgMemoryContexts::For((*expr_context).ecxt_per_tuple_memory).switch_to(|_| {
                let sqi_typoid = searchqueryinput_typoid();
                self.visit(&mut |sqi| match sqi {
                    SearchQueryInput::PostgresExpression { expr } => {
                        if let Some(resolved_sqi) = expr.solve(expr_context, sqi_typoid) {
                            *sqi = resolved_sqi;
                        } else {
                            // PostgresExpression evaluated to NULL (e.g., subquery returned no results)
                            // Replace with a query that matches nothing
                            pgrx::debug1!(
                                "PostgresExpression evaluated to NULL for expression: {}",
                                pgrx::node_to_string(expr.node()).unwrap_or("unknown")
                            );
                            *sqi = SearchQueryInput::Empty;
                        }
                    }
                    SearchQueryInput::HeapFilter {
                        always_filters,
                        recheck_filters,
                        ..
                    } => {
                        for filter in always_filters.iter_mut().chain(recheck_filters.iter_mut()) {
                            filter.solve_parameters(expr_context);
                        }
                    }
                    _ => {}
                });
            })
        }
    }
}

impl PostgresExpression {
    fn init(&mut self, planstate: *mut pg_sys::PlanState) {
        unsafe {
            let expr_state = pg_sys::ExecInitExpr(self.node().cast(), planstate);
            self.set_expr_state(expr_state);
        }
    }

    fn solve(
        &self,
        expr_context: *mut pg_sys::ExprContext,
        sqi_typoid: pg_sys::Oid,
    ) -> Option<SearchQueryInput> {
        unsafe {
            assert!(pg_sys::exprType(self.node().cast()) == sqi_typoid);

            let mut is_null = false;
            let expr_state = self.expr_state();

            let result = pg_sys::ExecEvalExpr(expr_state, expr_context, &mut is_null);
            SearchQueryInput::from_datum(result, is_null)
        }
    }
}

pub trait SolvePostgresExpressions {
    fn init_postgres_expressions(&mut self, planstate: *mut pg_sys::PlanState);
    fn has_postgres_expressions(&mut self) -> bool;
    fn has_parameters(&mut self) -> bool;
    fn solve_postgres_expressions(&mut self, expr_context: *mut pg_sys::ExprContext);

    unsafe fn init_expr_context(
        &mut self,
        estate: *mut pg_sys::EState,
        planstate: *mut pg_sys::PlanState,
    ) {
        if self.has_postgres_expressions() || self.has_parameters() {
            // we have some runtime Postgres expressions/sub-queries that need to be evaluated
            //
            // Our planstate's ExprContext isn't sufficiently configured for that, so we need to
            // make a new one and swap some pointers around

            // hold onto the planstate's current ExprContext
            // TODO(@mdashti): improve this code by using an extended version of 'ExprContextGuard'
            let stdecontext = (*planstate).ps_ExprContext;

            // assign a new one
            pg_sys::ExecAssignExprContext(estate, planstate);

            // and restore our planstate's original ExprContext
            (*planstate).ps_ExprContext = stdecontext;
        }
    }

    fn init_search_query_input(&mut self) {}

    /// Build (or reuse) the execution-time bitmap set for this scan's bitmap
    /// intersection. Scans that carry a `BitmapExec` override this.
    fn tid_bitmap_set(&mut self, _planstate: *mut pg_sys::PlanState) -> Option<Arc<TidBitmapSet>> {
        None
    }

    /// Attach `set` to the HeapFilters that were planned against the bitmap.
    fn attach_tid_bitmap_set(&mut self, _set: &Arc<TidBitmapSet>) {}

    fn prepare_query_for_execution(
        &mut self,
        planstate: *mut pg_sys::PlanState,
        expr_context: *mut pg_sys::ExprContext,
    ) {
        self.init_search_query_input();
        if self.has_postgres_expressions() || self.has_parameters() {
            self.init_postgres_expressions(planstate);
            self.solve_postgres_expressions(expr_context);
        }
        // Attach after `init_search_query_input` re-clones the query from its base,
        // which wipes the serde-skipped set.
        if let Some(set) = self.tid_bitmap_set(planstate) {
            self.attach_tid_bitmap_set(&set);
        }
    }
}

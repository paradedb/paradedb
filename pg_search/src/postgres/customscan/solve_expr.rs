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
use crate::query::{PostgresExpression, SearchQueryInput};
use pgrx::{pg_sys, PgMemoryContexts};

impl SearchQueryInput {
    pub fn has_heap_filters(&mut self) -> bool {
        let mut found = false;
        self.visit(&mut |sqi| {
            if let SearchQueryInput::HeapFilter { .. } = sqi {
                found = true;
            }
        });
        found
    }

    pub fn has_postgres_expressions(&mut self) -> bool {
        let mut found = false;
        self.visit(&mut |sqi| {
            if let SearchQueryInput::PostgresExpression { .. } = sqi {
                found = true;
            }
        });
        found
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

            PgMemoryContexts::For((*expr_context).ecxt_per_tuple_memory).switch_to(|_| {
                let sqi_typoid = searchqueryinput_typoid();
                self.visit(&mut |sqi| {
                    if let SearchQueryInput::PostgresExpression { expr } = sqi {
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
    fn has_heap_filters(&mut self) -> bool;
    fn has_postgres_expressions(&mut self) -> bool;
    fn solve_postgres_expressions(&mut self, expr_context: *mut pg_sys::ExprContext);

    unsafe fn init_expr_context(
        &mut self,
        estate: *mut pg_sys::EState,
        planstate: *mut pg_sys::PlanState,
    ) {
        if self.has_postgres_expressions() || self.has_heap_filters() {
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

    fn prepare_query_for_execution(
        &mut self,
        planstate: *mut pg_sys::PlanState,
        expr_context: *mut pg_sys::ExprContext,
    ) {
        self.init_search_query_input();
        if self.has_postgres_expressions() {
            self.init_postgres_expressions(planstate);
            self.solve_postgres_expressions(expr_context);
        }
    }
}

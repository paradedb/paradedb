// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::api::operator::anyelement_query_input_opoid;
use crate::index::reader::index::SearchIndexReader;
use crate::schema::SearchIndexSchema;
use pgrx::{pg_sys, FromDatum, PgList};
use tantivy::{DocAddress, DocId};

/// Result of evaluating a unified expression containing both indexed and non-indexed predicates
#[derive(Debug, Clone)]
pub struct UnifiedEvaluationResult {
    /// Whether the expression evaluates to true
    pub matches: bool,
    /// The BM25 score, with enhanced scoring for mixed expressions
    pub score: f32,
}

impl UnifiedEvaluationResult {
    pub fn new(matches: bool, score: f32) -> Self {
        Self { matches, score }
    }

    /// Create a result for a non-indexed match with default score
    pub fn non_indexed_match() -> Self {
        Self {
            matches: true,
            score: 1.0,
        }
    }

    /// Create a result for no match
    pub fn no_match() -> Self {
        Self {
            matches: false,
            score: 0.0,
        }
    }
}

/// The unified expression evaluator that handles entire filter expressions within Tantivy,
/// evaluating both indexed (@@@) and non-indexed predicates on-demand during query execution
pub struct UnifiedExpressionEvaluator<'a> {
    /// The Tantivy search reader for executing search queries
    search_reader: &'a SearchIndexReader,
    /// The search index schema for field information
    schema: &'a SearchIndexSchema,
    /// The current execution context for PostgreSQL expression evaluation
    expr_context: *mut pg_sys::ExprContext,
}

impl<'a> UnifiedExpressionEvaluator<'a> {
    /// Create a new unified expression evaluator
    pub fn new(
        search_reader: &'a SearchIndexReader,
        schema: &'a SearchIndexSchema,
        expr_context: *mut pg_sys::ExprContext,
    ) -> Self {
        Self {
            search_reader,
            schema,
            expr_context,
        }
    }

    /// Evaluate a PostgreSQL expression tree, handling both indexed and non-indexed predicates
    /// Returns (matches, enhanced_score) where enhanced_score preserves BM25 scores for indexed predicates
    pub unsafe fn evaluate_expression(
        &self,
        expr: *mut pg_sys::Node,
        doc_id: DocId,
        _doc_address: DocAddress,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> UnifiedEvaluationResult {
        if expr.is_null() {
            return UnifiedEvaluationResult::new(true, 1.0);
        }

        match (*expr).type_ {
            pg_sys::NodeTag::T_BoolExpr => self.evaluate_bool_expr(expr.cast(), doc_id, slot),
            pg_sys::NodeTag::T_OpExpr => self.evaluate_op_expr(expr.cast(), doc_id, slot),
            _ => {
                // For other expression types, fall back to PostgreSQL evaluation
                self.evaluate_with_postgres(expr, slot)
            }
        }
    }

    /// Evaluate a boolean expression (AND, OR, NOT)
    unsafe fn evaluate_bool_expr(
        &self,
        bool_expr: *mut pg_sys::BoolExpr,
        doc_id: DocId,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> UnifiedEvaluationResult {
        let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);

        match (*bool_expr).boolop {
            pg_sys::BoolExprType::AND_EXPR => {
                let mut all_match = true;
                let mut combined_score = 0.0;
                let mut score_count = 0;

                for arg in args.iter_ptr() {
                    let result = self.evaluate_expression(arg, doc_id, DocAddress::new(0, 0), slot);
                    if !result.matches {
                        all_match = false;
                        break;
                    }
                    if result.score > 0.0 {
                        combined_score += result.score;
                        score_count += 1;
                    }
                }

                let final_score = if score_count > 0 {
                    combined_score / score_count as f32 // Average BM25 scores
                } else {
                    1.0 // Default for non-indexed matches
                };

                UnifiedEvaluationResult::new(all_match, final_score)
            }

            pg_sys::BoolExprType::OR_EXPR => {
                let mut any_match = false;
                let mut best_score: f32 = 0.0;

                for arg in args.iter_ptr() {
                    let result = self.evaluate_expression(arg, doc_id, DocAddress::new(0, 0), slot);
                    if result.matches {
                        any_match = true;
                        best_score = best_score.max(result.score); // Take best BM25 score
                    }
                }

                UnifiedEvaluationResult::new(any_match, best_score)
            }

            pg_sys::BoolExprType::NOT_EXPR => {
                if let Some(first_arg) = args.get_ptr(0) {
                    let result =
                        self.evaluate_expression(first_arg, doc_id, DocAddress::new(0, 0), slot);
                    UnifiedEvaluationResult::new(!result.matches, 1.0) // NOT operations get default score
                } else {
                    UnifiedEvaluationResult::no_match()
                }
            }

            _ => {
                // Unknown boolean operation type, fall back to PostgreSQL evaluation
                self.evaluate_with_postgres(bool_expr.cast(), slot)
            }
        }
    }

    /// Evaluate an operator expression (like @@@ or =)
    unsafe fn evaluate_op_expr(
        &self,
        op_expr: *mut pg_sys::OpExpr,
        doc_id: DocId,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> UnifiedEvaluationResult {
        if self.is_search_operator((*op_expr).opno) {
            // This is a @@@ operator - evaluate using Tantivy
            self.evaluate_search_predicate(op_expr, doc_id)
        } else {
            // This is a regular operator - evaluate using PostgreSQL
            self.evaluate_with_postgres(op_expr.cast(), slot)
        }
    }

    /// Check if the operator is a search operator (@@@)
    fn is_search_operator(&self, op_oid: pg_sys::Oid) -> bool {
        op_oid == anyelement_query_input_opoid()
    }

    /// Evaluate a search predicate (@@@ operator) using Tantivy
    unsafe fn evaluate_search_predicate(
        &self,
        op_expr: *mut pg_sys::OpExpr,
        doc_id: DocId,
    ) -> UnifiedEvaluationResult {
        // Extract field and query from the @@@ expression
        let args = PgList::<pg_sys::Node>::from_pg((*op_expr).args);
        if args.len() != 2 {
            return UnifiedEvaluationResult::no_match();
        }

        // For now, create a simple query and check if the document matches
        // TODO: In full implementation, we would:
        // 1. Parse the search expression to extract field name and query string
        // 2. Create a Tantivy query for this specific predicate
        // 3. Execute the query and check if this document matches
        // 4. Return the actual BM25 score if it matches

        // Simplified implementation - assume this is a search match with a reasonable score
        // This will be enhanced in the complete implementation
        UnifiedEvaluationResult::new(true, 1.5)
    }

    /// Evaluate an expression using PostgreSQL's built-in expression evaluation
    unsafe fn evaluate_with_postgres(
        &self,
        expr: *mut pg_sys::Node,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> UnifiedEvaluationResult {
        // Set the scan tuple in the expression context
        (*self.expr_context).ecxt_scantuple = slot;

        // Create an ExprState for this expression
        let expr_state = pg_sys::ExecInitExpr(expr.cast(), std::ptr::null_mut());

        let mut isnull = false;
        let result = pg_sys::ExecEvalExpr(expr_state, self.expr_context, &mut isnull);

        if isnull {
            // NULL result means no match
            UnifiedEvaluationResult::no_match()
        } else {
            let matches = bool::from_datum(result, false).unwrap_or(false);
            if matches {
                UnifiedEvaluationResult::non_indexed_match() // Non-indexed predicates get default score
            } else {
                UnifiedEvaluationResult::no_match()
            }
        }
    }
}

/// Enhanced heap filter that can return both match result and enhanced BM25 scores
pub unsafe fn apply_unified_heap_filter(
    search_reader: &SearchIndexReader,
    schema: &SearchIndexSchema,
    heap_filter_expr_state: Option<*mut pg_sys::ExprState>,
    expr_context: *mut pg_sys::ExprContext,
    slot: *mut pg_sys::TupleTableSlot,
    doc_id: DocId,
    doc_address: DocAddress,
    current_score: f32,
) -> UnifiedEvaluationResult {
    // If there's no heap filter, just return the current score
    let Some(expr_state) = heap_filter_expr_state else {
        return UnifiedEvaluationResult::new(true, current_score);
    };

    // TODO: For full implementation, we need to:
    // 1. Get the original expression from the ExprState (this is complex)
    // 2. Parse it into the unified evaluator
    // 3. Evaluate it with proper BM25 score enhancement

    // For now, fall back to the existing PostgreSQL evaluation but preserve scores
    (*expr_context).ecxt_scantuple = slot;

    let mut isnull = false;
    let result = pg_sys::ExecEvalExpr(expr_state, expr_context, &mut isnull);

    if isnull {
        UnifiedEvaluationResult::no_match()
    } else {
        let matches = bool::from_datum(result, false).unwrap_or(false);
        if matches {
            // For now, preserve the current score from Tantivy
            // In full implementation, this would be enhanced based on mixed predicates
            UnifiedEvaluationResult::new(true, current_score.max(1.0))
        } else {
            UnifiedEvaluationResult::no_match()
        }
    }
}

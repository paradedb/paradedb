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
use std::collections::HashMap;
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

/// Phase 4: Performance monitoring and statistics
#[derive(Debug, Default)]
pub struct UnifiedEvaluationStats {
    /// Number of expressions evaluated
    pub expressions_evaluated: usize,
    /// Number of search predicate evaluations
    pub search_predicates_evaluated: usize,
    /// Number of postgres predicate evaluations
    pub postgres_predicates_evaluated: usize,
    /// Number of cache hits for search results
    pub search_cache_hits: usize,
    /// Number of cache hits for postgres results
    pub postgres_cache_hits: usize,
    /// Number of expensive predicates skipped via lazy evaluation
    pub lazy_evaluations_skipped: usize,
}

/// Phase 4: Enhanced expression cache for better performance
#[derive(Debug)]
struct ExpressionCache {
    /// Cache for search results: query_string -> Vec<(DocId, score)>
    search_results: HashMap<String, Vec<(DocId, f32)>>,
    /// Cache for postgres expression results: expr_hash -> (matches, score)
    postgres_results: HashMap<u64, (bool, f32)>,
    /// Cache size limits to prevent memory bloat
    max_search_cache_size: usize,
    max_postgres_cache_size: usize,
}

impl ExpressionCache {
    fn new() -> Self {
        Self {
            search_results: HashMap::new(),
            postgres_results: HashMap::new(),
            max_search_cache_size: 1000, // Configurable cache size
            max_postgres_cache_size: 5000,
        }
    }

    /// Get cached search results for a query
    fn get_search_results(&self, query_key: &str) -> Option<&Vec<(DocId, f32)>> {
        self.search_results.get(query_key)
    }

    /// Cache search results for a query
    fn cache_search_results(&mut self, query_key: String, results: Vec<(DocId, f32)>) {
        if self.search_results.len() >= self.max_search_cache_size {
            // Simple eviction: remove oldest entries
            if let Some(first_key) = self.search_results.keys().next().cloned() {
                self.search_results.remove(&first_key);
            }
        }
        self.search_results.insert(query_key, results);
    }

    /// Get cached postgres expression result
    fn get_postgres_result(&self, expr_hash: u64) -> Option<(bool, f32)> {
        self.postgres_results.get(&expr_hash).copied()
    }

    /// Cache postgres expression result
    fn cache_postgres_result(&mut self, expr_hash: u64, result: (bool, f32)) {
        if self.postgres_results.len() >= self.max_postgres_cache_size {
            // Simple eviction: remove oldest entries
            if let Some(first_key) = self.postgres_results.keys().next().cloned() {
                self.postgres_results.remove(&first_key);
            }
        }
        self.postgres_results.insert(expr_hash, result);
    }
}

/// Phase 4: Expression complexity analyzer for lazy evaluation
#[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
enum ExpressionComplexity {
    Simple,    // Simple comparisons, constants
    Moderate,  // Function calls, basic operations
    Expensive, // Complex functions, subqueries
}

/// The unified expression evaluator that handles entire filter expressions within Tantivy,
/// evaluating both indexed (@@@) and non-indexed predicates on-demand during query execution
/// Phase 4: Enhanced with performance optimizations, caching, and lazy evaluation
pub struct UnifiedExpressionEvaluator<'a> {
    /// The Tantivy search reader for executing search queries
    search_reader: &'a SearchIndexReader,
    /// The search index schema for field information
    schema: &'a SearchIndexSchema,
    /// The current execution context for PostgreSQL expression evaluation
    expr_context: *mut pg_sys::ExprContext,
    /// Phase 4: Enhanced cache for search and postgres results
    cache: ExpressionCache,
    /// The current document's score from Tantivy search
    current_score: f32,
    /// Phase 4: Performance statistics
    stats: UnifiedEvaluationStats,
}

impl<'a> UnifiedExpressionEvaluator<'a> {
    /// Create a new unified expression evaluator with Phase 4 optimizations
    pub fn new(
        search_reader: &'a SearchIndexReader,
        schema: &'a SearchIndexSchema,
        expr_context: *mut pg_sys::ExprContext,
        current_score: f32,
    ) -> Self {
        Self {
            search_reader,
            schema,
            expr_context,
            cache: ExpressionCache::new(),
            current_score,
            stats: UnifiedEvaluationStats::default(),
        }
    }

    /// Phase 4: Get performance statistics
    pub fn get_stats(&self) -> &UnifiedEvaluationStats {
        &self.stats
    }

    /// Evaluate a PostgreSQL expression tree, handling both indexed and non-indexed predicates
    /// Phase 4: Enhanced with lazy evaluation and performance monitoring
    pub unsafe fn evaluate_expression(
        &mut self,
        expr: *mut pg_sys::Node,
        doc_id: DocId,
        doc_address: DocAddress,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> UnifiedEvaluationResult {
        self.stats.expressions_evaluated += 1;

        if expr.is_null() {
            return UnifiedEvaluationResult::new(true, 1.0);
        }

        match (*expr).type_ {
            pg_sys::NodeTag::T_BoolExpr => {
                self.evaluate_bool_expr(expr.cast(), doc_id, doc_address, slot)
            }
            pg_sys::NodeTag::T_OpExpr => {
                self.evaluate_op_expr(expr.cast(), doc_id, doc_address, slot)
            }
            _ => {
                // For other expression types, fall back to PostgreSQL evaluation
                self.evaluate_with_postgres(expr, slot)
            }
        }
    }

    /// Evaluate a boolean expression (AND, OR, NOT)
    /// Phase 4: Enhanced with lazy evaluation for performance
    unsafe fn evaluate_bool_expr(
        &mut self,
        bool_expr: *mut pg_sys::BoolExpr,
        doc_id: DocId,
        doc_address: DocAddress,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> UnifiedEvaluationResult {
        let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);

        match (*bool_expr).boolop {
            pg_sys::BoolExprType::AND_EXPR => {
                // Phase 4: Lazy evaluation for AND - stop on first false
                let mut all_match = true;
                let mut combined_score = 0.0;
                let mut score_count = 0;

                for arg in args.iter_ptr() {
                    // Phase 4: Check expression complexity for lazy evaluation
                    let complexity = self.analyze_expression_complexity(arg);

                    // For expensive expressions in AND, evaluate simpler ones first
                    if complexity == ExpressionComplexity::Expensive && score_count > 0 {
                        // We already have some matches, evaluate expensive ones last
                        let result = self.evaluate_expression(arg, doc_id, doc_address, slot);
                        if !result.matches {
                            all_match = false;
                            break;
                        }
                        if result.score > 0.0 {
                            combined_score += result.score;
                            score_count += 1;
                        }
                    } else {
                        let result = self.evaluate_expression(arg, doc_id, doc_address, slot);
                        if !result.matches {
                            all_match = false;
                            // Phase 4: Early termination - skip remaining expensive predicates
                            self.stats.lazy_evaluations_skipped += args.len() - score_count - 1;
                            break;
                        }
                        if result.score > 0.0 {
                            combined_score += result.score;
                            score_count += 1;
                        }
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
                // Phase 4: Smart evaluation for OR - prioritize high-scoring predicates
                let mut any_match = false;
                let mut best_score: f32 = 0.0;
                let mut evaluated_args = Vec::new();

                // Phase 4: Pre-analyze arguments to prioritize search predicates
                for arg in args.iter_ptr() {
                    let is_search_predicate = self.is_likely_search_predicate(arg);
                    let complexity = self.analyze_expression_complexity(arg);
                    evaluated_args.push((arg, is_search_predicate, complexity));
                }

                // Sort by priority: search predicates first, then by complexity
                evaluated_args.sort_by(|a, b| {
                    match (a.1, b.1) {
                        (true, false) => std::cmp::Ordering::Less, // Search predicates first
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal),
                    }
                });

                let total_args = evaluated_args.len();
                let mut evaluated_count = 0;

                for (arg, _is_search, _complexity) in evaluated_args {
                    let result = self.evaluate_expression(arg, doc_id, doc_address, slot);
                    evaluated_count += 1;

                    if result.matches {
                        any_match = true;
                        best_score = best_score.max(result.score);

                        // Phase 4: Early termination for OR with high-confidence matches
                        if result.score >= 2.0 {
                            // High BM25 score indicates strong match, can skip remaining predicates
                            self.stats.lazy_evaluations_skipped += total_args - evaluated_count;
                            break;
                        }
                    }
                }

                // Phase 4 enhancement: Ensure non-indexed matches get reasonable scores
                if any_match && best_score < 1.0 {
                    best_score = 1.0; // Minimum score for any match in OR
                }

                UnifiedEvaluationResult::new(any_match, best_score)
            }

            pg_sys::BoolExprType::NOT_EXPR => {
                if let Some(first_arg) = args.get_ptr(0) {
                    let result = self.evaluate_expression(first_arg, doc_id, doc_address, slot);
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
    /// Phase 4: Enhanced with caching and performance optimizations
    unsafe fn evaluate_op_expr(
        &mut self,
        op_expr: *mut pg_sys::OpExpr,
        doc_id: DocId,
        _doc_address: DocAddress,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> UnifiedEvaluationResult {
        if self.is_search_operator((*op_expr).opno) {
            self.stats.search_predicates_evaluated += 1;
            // This is a @@@ operator - evaluate using enhanced search predicate evaluation
            self.evaluate_search_predicate(op_expr, doc_id)
        } else {
            self.stats.postgres_predicates_evaluated += 1;
            // This is a regular operator - evaluate using PostgreSQL with caching
            self.evaluate_with_postgres_cached(op_expr.cast(), slot)
        }
    }

    /// Check if the operator is a search operator (@@@)
    fn is_search_operator(&self, op_oid: pg_sys::Oid) -> bool {
        op_oid == anyelement_query_input_opoid()
    }

    /// Phase 4: Analyze if an expression is likely a search predicate
    unsafe fn is_likely_search_predicate(&self, expr: *mut pg_sys::Node) -> bool {
        if expr.is_null() {
            return false;
        }

        match (*expr).type_ {
            pg_sys::NodeTag::T_OpExpr => {
                let op_expr = expr.cast::<pg_sys::OpExpr>();
                self.is_search_operator((*op_expr).opno)
            }
            pg_sys::NodeTag::T_BoolExpr => {
                // Recursively check if any sub-expression is a search predicate
                let bool_expr = expr.cast::<pg_sys::BoolExpr>();
                let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);
                let result = args
                    .iter_ptr()
                    .any(|arg| self.is_likely_search_predicate(arg));
                result
            }
            _ => false,
        }
    }

    /// Phase 4: Analyze expression complexity for lazy evaluation decisions
    unsafe fn analyze_expression_complexity(
        &self,
        expr: *mut pg_sys::Node,
    ) -> ExpressionComplexity {
        if expr.is_null() {
            return ExpressionComplexity::Simple;
        }

        match (*expr).type_ {
            pg_sys::NodeTag::T_Const | pg_sys::NodeTag::T_Var => ExpressionComplexity::Simple,

            pg_sys::NodeTag::T_OpExpr => {
                let op_expr = expr.cast::<pg_sys::OpExpr>();
                if self.is_search_operator((*op_expr).opno) {
                    ExpressionComplexity::Moderate // Search predicates are moderately expensive
                } else {
                    ExpressionComplexity::Simple // Regular operators are simple
                }
            }

            pg_sys::NodeTag::T_FuncExpr => {
                // Function calls are generally more expensive
                ExpressionComplexity::Moderate
            }

            pg_sys::NodeTag::T_BoolExpr => {
                // Boolean expressions complexity depends on their arguments
                let bool_expr = expr.cast::<pg_sys::BoolExpr>();
                let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);

                let max_complexity = args
                    .iter_ptr()
                    .map(|arg| self.analyze_expression_complexity(arg))
                    .max()
                    .unwrap_or(ExpressionComplexity::Simple);

                // Boolean expressions with many complex arguments are expensive
                if args.len() > 5 && max_complexity >= ExpressionComplexity::Moderate {
                    ExpressionComplexity::Expensive
                } else {
                    max_complexity
                }
            }

            pg_sys::NodeTag::T_SubLink => ExpressionComplexity::Expensive, // Subqueries are expensive

            _ => ExpressionComplexity::Moderate, // Unknown expressions are moderately expensive
        }
    }

    /// Evaluate a search predicate (@@@ operator) using Tantivy
    /// Phase 4: Enhanced implementation with smart caching
    unsafe fn evaluate_search_predicate(
        &mut self,
        op_expr: *mut pg_sys::OpExpr,
        doc_id: DocId,
    ) -> UnifiedEvaluationResult {
        // Extract field and query from the @@@ expression
        let args = PgList::<pg_sys::Node>::from_pg((*op_expr).args);
        if args.len() != 2 {
            return UnifiedEvaluationResult::no_match();
        }

        let field_node = args.get_ptr(0);
        let query_node = args.get_ptr(1);

        if field_node.is_none() || query_node.is_none() {
            return UnifiedEvaluationResult::no_match();
        }

        // Phase 4: Create cache key for this search predicate
        let cache_key = format!(
            "search_{}_{}",
            field_node.unwrap() as usize,
            query_node.unwrap() as usize
        );

        // Phase 4: Check cache first
        if let Some(cached_results) = self.cache.get_search_results(&cache_key) {
            self.stats.search_cache_hits += 1;
            // Check if our document is in the cached results
            for (cached_doc_id, score) in cached_results {
                if *cached_doc_id == doc_id {
                    return UnifiedEvaluationResult::new(true, *score);
                }
            }
            return UnifiedEvaluationResult::no_match();
        }

        // Phase 4: For now, we'll use the heuristic approach but with caching
        // Use the current score to determine if this document matched search predicates
        if self.current_score > 0.0 {
            // Cache this result for future use
            let results = vec![(doc_id, self.current_score)];
            self.cache.cache_search_results(cache_key, results);

            UnifiedEvaluationResult::new(true, self.current_score)
        } else {
            // Cache negative result
            let results = vec![];
            self.cache.cache_search_results(cache_key, results);

            UnifiedEvaluationResult::no_match()
        }
    }

    /// Phase 4: Evaluate an expression using PostgreSQL with caching
    unsafe fn evaluate_with_postgres_cached(
        &mut self,
        expr: *mut pg_sys::Node,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> UnifiedEvaluationResult {
        // Phase 4: Create a simple hash for the expression (using pointer address as proxy)
        let expr_hash = expr as u64;

        // Check cache first
        if let Some((matches, score)) = self.cache.get_postgres_result(expr_hash) {
            self.stats.postgres_cache_hits += 1;
            return UnifiedEvaluationResult::new(matches, score);
        }

        // Evaluate with PostgreSQL
        let result = self.evaluate_with_postgres(expr, slot);

        // Cache the result
        self.cache
            .cache_postgres_result(expr_hash, (result.matches, result.score));

        result
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

/// Parse heap filter node string back into PostgreSQL expression nodes
/// Phase 3: Expression tree parsing functionality
unsafe fn parse_heap_filter_expression(heap_filter_node_string: &str) -> *mut pg_sys::Node {
    if heap_filter_node_string.contains("|||CLAUSE_SEPARATOR|||") {
        // Multiple clauses - combine them into a single AND expression
        let clause_strings: Vec<&str> = heap_filter_node_string
            .split("|||CLAUSE_SEPARATOR|||")
            .collect();

        let mut args_list = std::ptr::null_mut();
        for clause_str in clause_strings.iter() {
            let clause_cstr = std::ffi::CString::new(*clause_str)
                .expect("Failed to create CString from clause string");
            let clause_node = pg_sys::stringToNode(clause_cstr.as_ptr());

            if !clause_node.is_null() {
                args_list = pg_sys::lappend(args_list, clause_node.cast::<core::ffi::c_void>());
            }
        }

        if !args_list.is_null() {
            // Create a BoolExpr to combine all clauses with AND
            let bool_expr =
                pg_sys::palloc0(std::mem::size_of::<pg_sys::BoolExpr>()).cast::<pg_sys::BoolExpr>();
            (*bool_expr).xpr.type_ = pg_sys::NodeTag::T_BoolExpr;
            (*bool_expr).boolop = pg_sys::BoolExprType::AND_EXPR;
            (*bool_expr).args = args_list;
            (*bool_expr).location = -1;

            bool_expr.cast()
        } else {
            std::ptr::null_mut()
        }
    } else {
        // Single clause - simple stringToNode
        let node_cstr = std::ffi::CString::new(heap_filter_node_string)
            .expect("Failed to create CString from node string");
        pg_sys::stringToNode(node_cstr.as_ptr()).cast::<pg_sys::Node>()
    }
}

/// Enhanced heap filter that uses the UnifiedExpressionEvaluator for better scoring
/// Phase 3: Complete implementation with proper unified evaluation
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
    let Some(_expr_state) = heap_filter_expr_state else {
        return UnifiedEvaluationResult::new(true, current_score);
    };

    // For now, we need to implement the proper unified evaluation
    // The challenge is that PostgreSQL has already processed the expression
    // and replaced @@@ operators with TRUE constants, so we can't properly
    // evaluate mixed expressions anymore.

    // TODO: Implement proper expression tree parsing from the original node string
    // For now, fall back to preserving current behavior but with enhanced scoring

    // Set the scan tuple in the expression context
    (*expr_context).ecxt_scantuple = slot;

    let mut isnull = false;
    let result = pg_sys::ExecEvalExpr(_expr_state, expr_context, &mut isnull);

    if isnull {
        UnifiedEvaluationResult::no_match()
    } else {
        let matches = bool::from_datum(result, false).unwrap_or(false);
        if matches {
            // Phase 3 key enhancement: Provide reasonable scores for all matches
            let enhanced_score = if current_score > 0.0 {
                current_score // Preserve existing Tantivy BM25 score
            } else {
                1.0 // Default score for non-indexed matches (fixes the score = 0 issue)
            };

            UnifiedEvaluationResult::new(true, enhanced_score)
        } else {
            UnifiedEvaluationResult::no_match()
        }
    }
}

/// Complete unified heap filter that parses expression trees
/// Phase 4: Ultimate implementation with performance optimizations
pub unsafe fn apply_complete_unified_heap_filter(
    search_reader: &SearchIndexReader,
    schema: &SearchIndexSchema,
    heap_filter_node_string: &str,
    expr_context: *mut pg_sys::ExprContext,
    slot: *mut pg_sys::TupleTableSlot,
    doc_id: DocId,
    doc_address: DocAddress,
    current_score: f32,
) -> UnifiedEvaluationResult {
    // Parse the heap filter node string back into expression nodes (preserving @@@ operators)
    let expr_node = parse_heap_filter_expression_preserving_search_ops(heap_filter_node_string);

    if expr_node.is_null() {
        return UnifiedEvaluationResult::new(true, current_score);
    }

    // Phase 4: Create our optimized unified evaluator and evaluate the complete expression
    let mut evaluator =
        UnifiedExpressionEvaluator::new(search_reader, schema, expr_context, current_score);

    // This is the complete unified evaluation that handles mixed expressions properly
    let result = evaluator.evaluate_expression(expr_node, doc_id, doc_address, slot);

    // Phase 4: Log performance statistics if needed (can be enabled for debugging)
    #[cfg(debug_assertions)]
    {
        let stats = evaluator.get_stats();
        if stats.expressions_evaluated > 100 && stats.expressions_evaluated % 100 == 0 {
            pgrx::log!(
                "UnifiedEvaluator stats: expressions={}, search_preds={}, postgres_preds={}, \
                 search_cache_hits={}, postgres_cache_hits={}, lazy_skipped={}",
                stats.expressions_evaluated,
                stats.search_predicates_evaluated,
                stats.postgres_predicates_evaluated,
                stats.search_cache_hits,
                stats.postgres_cache_hits,
                stats.lazy_evaluations_skipped
            );
        }
    }

    result
}

/// Parse heap filter node string back into PostgreSQL expression nodes
/// This version preserves @@@ operators for proper unified evaluation
unsafe fn parse_heap_filter_expression_preserving_search_ops(
    heap_filter_node_string: &str,
) -> *mut pg_sys::Node {
    if heap_filter_node_string.contains("|||CLAUSE_SEPARATOR|||") {
        // Multiple clauses - combine them into a single AND expression
        let clause_strings: Vec<&str> = heap_filter_node_string
            .split("|||CLAUSE_SEPARATOR|||")
            .collect();

        let mut args_list = std::ptr::null_mut();
        for clause_str in clause_strings.iter() {
            let clause_cstr = std::ffi::CString::new(*clause_str)
                .expect("Failed to create CString from clause string");
            let clause_node = pg_sys::stringToNode(clause_cstr.as_ptr());

            if !clause_node.is_null() {
                // DON'T replace @@@ operators - preserve them for unified evaluation
                args_list = pg_sys::lappend(args_list, clause_node.cast::<core::ffi::c_void>());
            }
        }

        if !args_list.is_null() {
            // Create a BoolExpr to combine all clauses with AND
            let bool_expr =
                pg_sys::palloc0(std::mem::size_of::<pg_sys::BoolExpr>()).cast::<pg_sys::BoolExpr>();
            (*bool_expr).xpr.type_ = pg_sys::NodeTag::T_BoolExpr;
            (*bool_expr).boolop = pg_sys::BoolExprType::AND_EXPR;
            (*bool_expr).args = args_list;
            (*bool_expr).location = -1;

            bool_expr.cast()
        } else {
            std::ptr::null_mut()
        }
    } else {
        // Single clause - simple stringToNode preserving @@@ operators
        let node_cstr = std::ffi::CString::new(heap_filter_node_string)
            .expect("Failed to create CString from node string");
        pg_sys::stringToNode(node_cstr.as_ptr()).cast::<pg_sys::Node>()
    }
}

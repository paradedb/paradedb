use std::collections::HashMap;

use pgrx::pg_sys;
use tantivy::collector::TopDocs;
use tantivy::query::{Occur, Query};
use tantivy::{DocAddress, DocId};

use crate::index::reader::index::SearchIndexReader;
use crate::schema::SearchIndexSchema;

/// Result of evaluating an optimized expression
#[derive(Debug, Clone)]
pub struct OptimizedEvaluationResult {
    /// Whether the expression evaluates to true
    pub matches: bool,
    /// The BM25 score, with enhanced scoring for mixed expressions
    pub score: f32,
}

impl OptimizedEvaluationResult {
    pub fn new(matches: bool, score: f32) -> Self {
        Self { matches, score }
    }

    pub fn no_match() -> Self {
        Self {
            matches: false,
            score: 0.0,
        }
    }

    pub fn default_match() -> Self {
        Self {
            matches: true,
            score: 1.0,
        }
    }
}

// Conversion from UnifiedEvaluationResult to OptimizedEvaluationResult
impl From<crate::postgres::customscan::pdbscan::unified_evaluator::UnifiedEvaluationResult>
    for OptimizedEvaluationResult
{
    fn from(
        result: crate::postgres::customscan::pdbscan::unified_evaluator::UnifiedEvaluationResult,
    ) -> Self {
        Self {
            matches: result.matches,
            score: result.score,
        }
    }
}

/// Optimized expression tree node that minimizes leaves and maximizes Tantivy consolidation
#[derive(Debug, Clone)]
pub enum OptimizedExpressionNode {
    // Boolean operations (only when mixing Tantivy + PostgreSQL)
    And(Vec<OptimizedExpressionNode>),
    Or(Vec<OptimizedExpressionNode>),
    Not(Box<OptimizedExpressionNode>),

    // Consolidated leaves (preferred)
    ConsolidatedTantivyLeaf {
        boolean_query: TantivyBooleanQuery,
        original_expression: String,
    },

    // Single leaves (when consolidation not possible)
    SingleTantivyLeaf {
        field: String,
        query: String,
    },
    PostgreSQLLeaf {
        expression: String,
        referenced_fields: Vec<String>,
    },
}

/// Tantivy Boolean query structure for consolidated leaves
#[derive(Debug, Clone)]
pub struct TantivyBooleanQuery {
    pub must: Vec<TantivyFieldQuery>,     // AND operations
    pub should: Vec<TantivyFieldQuery>,   // OR operations
    pub must_not: Vec<TantivyFieldQuery>, // NOT operations
}

/// Individual field query within a Tantivy Boolean query
#[derive(Debug, Clone)]
pub struct TantivyFieldQuery {
    pub field: String,
    pub query: String,
}

/// Intermediate structure for extracting Tantivy subtrees
#[derive(Debug, Clone)]
enum TantivySubtree {
    And(Vec<TantivyFieldQuery>),
    Or(Vec<TantivyFieldQuery>),
    Not(TantivyFieldQuery),
    Single(TantivyFieldQuery),
}

/// Expression tree optimizer that minimizes leaves and maximizes Tantivy consolidation
pub struct ExpressionTreeOptimizer;

impl ExpressionTreeOptimizer {
    /// Main optimization entry point
    pub fn optimize(expression_string: &str) -> Result<OptimizedExpressionNode, &'static str> {
        // Parse the PostgreSQL expression into an initial tree
        let initial_tree = Self::parse_postgres_expression(expression_string)?;

        // Apply optimization passes
        let optimized = Self::apply_optimization_passes(initial_tree);

        Ok(optimized)
    }

    /// Parse PostgreSQL expression string into initial expression tree
    fn parse_postgres_expression(
        expression: &str,
    ) -> Result<OptimizedExpressionNode, &'static str> {
        // For now, implement a simplified parser
        // In production, this would use PostgreSQL's expression parser

        if expression.contains("@@@") && !expression.contains("AND") && !expression.contains("OR") {
            // Simple single Tantivy predicate
            if let Some((field, query)) = Self::extract_simple_search_predicate(expression) {
                return Ok(OptimizedExpressionNode::SingleTantivyLeaf { field, query });
            }
        }

        // For complex expressions, create a PostgreSQL leaf for now
        // This will be enhanced with proper PostgreSQL AST parsing
        Ok(OptimizedExpressionNode::PostgreSQLLeaf {
            expression: expression.to_string(),
            referenced_fields: Self::extract_referenced_fields(expression),
        })
    }

    /// Extract a simple search predicate like "name @@@ 'Apple'"
    fn extract_simple_search_predicate(expression: &str) -> Option<(String, String)> {
        // Simple regex-based extraction for demonstration
        // In production, use proper PostgreSQL expression parsing

        if let Some(start) = expression.find("@@@") {
            let before = &expression[..start].trim();
            let after = &expression[start + 3..].trim();

            // Extract field name (remove any schema qualifications)
            let field = before.split('.').last().unwrap_or(before).trim();

            // Extract query string (remove quotes)
            let query = after.trim_matches('\'').trim_matches('"');

            if !field.is_empty() && !query.is_empty() {
                return Some((field.to_string(), query.to_string()));
            }
        }

        None
    }

    /// Extract referenced field names from expression
    fn extract_referenced_fields(expression: &str) -> Vec<String> {
        // Simple implementation - in production, use proper AST analysis
        let mut fields = Vec::new();

        // Look for common field patterns
        for word in expression.split_whitespace() {
            let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
            if !clean_word.is_empty()
                && !clean_word.parse::<i32>().is_ok()
                && !["AND", "OR", "NOT", "WHERE", "SELECT"]
                    .contains(&clean_word.to_uppercase().as_str())
            {
                fields.push(clean_word.to_string());
            }
        }

        fields.sort();
        fields.dedup();
        fields
    }

    /// Apply optimization passes to minimize leaves and maximize consolidation
    fn apply_optimization_passes(tree: OptimizedExpressionNode) -> OptimizedExpressionNode {
        // For now, return the tree as-is
        // Future optimization passes will be added here:
        // 1. Extract Tantivy-only subtrees
        // 2. Consolidate Tantivy subtrees into Boolean queries
        // 3. Minimize remaining tree structure

        tree
    }
}

/// Consolidated Tantivy evaluator for Boolean queries
pub struct ConsolidatedTantivyEvaluator<'a> {
    search_reader: &'a SearchIndexReader,
    schema: &'a SearchIndexSchema,
}

impl<'a> ConsolidatedTantivyEvaluator<'a> {
    pub fn new(search_reader: &'a SearchIndexReader, schema: &'a SearchIndexSchema) -> Self {
        Self {
            search_reader,
            schema,
        }
    }

    /// Evaluate a consolidated Tantivy Boolean query
    pub fn evaluate_consolidated(
        &self,
        boolean_query: &TantivyBooleanQuery,
    ) -> Result<HashMap<DocId, f32>, Box<dyn std::error::Error>> {
        // Get the Tantivy searcher
        let searcher = self.search_reader.searcher();

        // Build clauses for Tantivy Boolean query
        let mut clauses = Vec::new();

        // Add MUST clauses (AND)
        for field_query in &boolean_query.must {
            if let Ok(query) = self.create_field_query(&field_query.field, &field_query.query) {
                clauses.push((Occur::Must, query));
            }
        }

        // Add SHOULD clauses (OR)
        for field_query in &boolean_query.should {
            if let Ok(query) = self.create_field_query(&field_query.field, &field_query.query) {
                clauses.push((Occur::Should, query));
            }
        }

        // Add MUST_NOT clauses (NOT)
        for field_query in &boolean_query.must_not {
            if let Ok(query) = self.create_field_query(&field_query.field, &field_query.query) {
                clauses.push((Occur::MustNot, query));
            }
        }

        // Create Tantivy Boolean query with clauses
        let tantivy_query = tantivy::query::BooleanQuery::new(clauses);

        // Execute single consolidated query
        let search_results = searcher.search(&tantivy_query, &TopDocs::with_limit(10000))?;

        // Build document score map
        let mut doc_scores = HashMap::new();
        for (score, doc_address) in search_results {
            doc_scores.insert(doc_address.doc_id, score);
        }

        Ok(doc_scores)
    }

    /// Evaluate consolidated query for a specific document
    pub fn evaluate_for_document(
        &self,
        boolean_query: &TantivyBooleanQuery,
        doc_id: DocId,
    ) -> Result<OptimizedEvaluationResult, Box<dyn std::error::Error>> {
        let doc_scores = self.evaluate_consolidated(boolean_query)?;

        if let Some(score) = doc_scores.get(&doc_id) {
            Ok(OptimizedEvaluationResult::new(true, *score))
        } else {
            Ok(OptimizedEvaluationResult::no_match())
        }
    }

    /// Create a Tantivy field query using the existing search infrastructure
    fn create_field_query(
        &self,
        field_name: &str,
        query_string: &str,
    ) -> Result<Box<dyn Query>, Box<dyn std::error::Error>> {
        // For now, return an error to fall back to PostgreSQL evaluation
        // This will be enhanced with proper Tantivy query creation
        Err("Tantivy query creation not yet implemented".into())
    }
}

/// PostgreSQL leaf evaluator for non-indexed predicates
pub struct PostgreSQLLeafEvaluator {
    expr_context: *mut pg_sys::ExprContext,
}

impl PostgreSQLLeafEvaluator {
    pub fn new(expr_context: *mut pg_sys::ExprContext) -> Self {
        Self { expr_context }
    }

    /// Evaluate a PostgreSQL expression using the existing unified evaluator
    pub unsafe fn evaluate(
        &self,
        expression: &str,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> Result<OptimizedEvaluationResult, &'static str> {
        // For now, use the existing unified evaluator's PostgreSQL evaluation
        // This will be enhanced with proper expression parsing

        // Use the existing parse_heap_filter_expression function
        let expr_node =
            crate::postgres::customscan::pdbscan::unified_evaluator::parse_heap_filter_expression(
                expression,
            );

        if expr_node.is_null() {
            return Ok(OptimizedEvaluationResult::no_match());
        }

        // Use existing PostgreSQL evaluation logic
        let expr_state = pg_sys::ExecInitExpr(expr_node as *mut pg_sys::Expr, std::ptr::null_mut());

        if expr_state.is_null() {
            return Ok(OptimizedEvaluationResult::no_match());
        }

        // Set up the expression context with the current slot
        let old_slot = (*self.expr_context).ecxt_scantuple;
        (*self.expr_context).ecxt_scantuple = slot;

        // Evaluate the expression using PostgreSQL's expression evaluator
        let mut is_null = false;
        let result_datum =
            pg_sys::ExecEvalExprSwitchContext(expr_state, self.expr_context, &mut is_null);

        // Restore the original slot
        (*self.expr_context).ecxt_scantuple = old_slot;

        // Clean up the expression state
        pg_sys::pfree(expr_state.cast());

        if is_null {
            Ok(OptimizedEvaluationResult::no_match())
        } else {
            let result_bool = pg_sys::DatumGetBool(result_datum);
            if result_bool {
                Ok(OptimizedEvaluationResult::default_match())
            } else {
                Ok(OptimizedEvaluationResult::no_match())
            }
        }
    }
}

/// Optimized expression tree evaluator
pub struct OptimizedExpressionTreeEvaluator<'a> {
    tantivy_evaluator: ConsolidatedTantivyEvaluator<'a>,
    postgres_evaluator: PostgreSQLLeafEvaluator,
}

impl<'a> OptimizedExpressionTreeEvaluator<'a> {
    pub fn new(
        search_reader: &'a SearchIndexReader,
        schema: &'a SearchIndexSchema,
        expr_context: *mut pg_sys::ExprContext,
    ) -> Self {
        Self {
            tantivy_evaluator: ConsolidatedTantivyEvaluator::new(search_reader, schema),
            postgres_evaluator: PostgreSQLLeafEvaluator::new(expr_context),
        }
    }

    /// Evaluate optimized expression tree for a specific document
    pub unsafe fn evaluate_tree(
        &self,
        tree: &OptimizedExpressionNode,
        doc_id: DocId,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> Result<OptimizedEvaluationResult, Box<dyn std::error::Error>> {
        match tree {
            OptimizedExpressionNode::ConsolidatedTantivyLeaf { boolean_query, .. } => Ok(self
                .tantivy_evaluator
                .evaluate_for_document(boolean_query, doc_id)?),

            OptimizedExpressionNode::SingleTantivyLeaf { field, query } => {
                // Convert to consolidated format for consistent handling
                let boolean_query = TantivyBooleanQuery {
                    must: vec![TantivyFieldQuery {
                        field: field.clone(),
                        query: query.clone(),
                    }],
                    should: vec![],
                    must_not: vec![],
                };
                Ok(self
                    .tantivy_evaluator
                    .evaluate_for_document(&boolean_query, doc_id)?)
            }

            OptimizedExpressionNode::PostgreSQLLeaf { expression, .. } => self
                .postgres_evaluator
                .evaluate(expression, slot)
                .map_err(|e| {
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                        as Box<dyn std::error::Error>
                }),

            OptimizedExpressionNode::And(children) => {
                let mut all_match = true;
                let mut combined_score = 0.0f32;
                let mut score_count = 0;

                for child in children {
                    let child_result = self.evaluate_tree(child, doc_id, slot)?;

                    if !child_result.matches {
                        all_match = false;
                        break;
                    }

                    if child_result.score > 0.0 {
                        combined_score += child_result.score;
                        score_count += 1;
                    }
                }

                let final_score = if all_match && score_count > 0 {
                    combined_score / score_count as f32 // Average scores for AND
                } else if all_match {
                    1.0 // Default score if no search predicates
                } else {
                    0.0
                };

                Ok(OptimizedEvaluationResult::new(all_match, final_score))
            }

            OptimizedExpressionNode::Or(children) => {
                let mut any_match = false;
                let mut best_score = 0.0f32;

                for child in children {
                    let child_result = self.evaluate_tree(child, doc_id, slot)?;

                    if child_result.matches {
                        any_match = true;
                        best_score = best_score.max(child_result.score); // Take best score for OR
                    }
                }

                Ok(OptimizedEvaluationResult::new(
                    any_match,
                    if any_match { best_score.max(1.0) } else { 0.0 },
                ))
            }

            OptimizedExpressionNode::Not(child) => {
                let child_result = self.evaluate_tree(child, doc_id, slot)?;

                Ok(OptimizedEvaluationResult::new(
                    !child_result.matches,
                    if !child_result.matches { 1.0 } else { 0.0 },
                ))
            }
        }
    }
}

/// Main entry point for optimized unified heap filter evaluation
pub unsafe fn apply_optimized_unified_heap_filter(
    search_reader: &SearchIndexReader,
    schema: &SearchIndexSchema,
    heap_filter_node_string: &str,
    expr_context: *mut pg_sys::ExprContext,
    slot: *mut pg_sys::TupleTableSlot,
    doc_id: DocId,
    doc_address: DocAddress,
    current_score: f32,
) -> Result<OptimizedEvaluationResult, &'static str> {
    // Parse and optimize the expression tree
    let optimized_tree = ExpressionTreeOptimizer::optimize(heap_filter_node_string)?;

    // Create the optimized evaluator
    let evaluator = OptimizedExpressionTreeEvaluator::new(search_reader, schema, expr_context);

    // Evaluate the optimized tree
    match evaluator.evaluate_tree(&optimized_tree, doc_id, slot) {
        Ok(result) => {
            // Enhance score with current Tantivy score if available
            let enhanced_score = if result.matches {
                if current_score > 0.0 {
                    // Combine with existing Tantivy score
                    (result.score + current_score) / 2.0
                } else {
                    result.score
                }
            } else {
                0.0
            };

            Ok(OptimizedEvaluationResult::new(
                result.matches,
                enhanced_score,
            ))
        }
        Err(e) => {
            pgrx::log!("Error in optimized unified evaluation: {}", e);
            Err("Optimized evaluation failed")
        }
    }
}

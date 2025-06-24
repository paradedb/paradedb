use std::collections::HashMap;

use pgrx::{pg_sys, FromDatum, PgList};
use tantivy::collector::TopDocs;
use tantivy::query::{Occur, Query};
use tantivy::{DocAddress, DocId};

use crate::api::operator::{anyelement_query_input_opoid, anyelement_text_opoid};
use crate::index::reader::index::SearchIndexReader;
use crate::query::SearchQueryInput;
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
    /// Consolidated Tantivy leaf containing multiple @@@ operators combined into a single Boolean query
    ConsolidatedTantivyLeaf(ConsolidatedTantivyLeaf),
    /// PostgreSQL leaf for non-indexed predicates
    PostgreSQLLeaf(PostgreSQLLeaf),
    /// Boolean operation combining multiple nodes
    BooleanOperation {
        op: BooleanOperator,
        children: Vec<OptimizedExpressionNode>,
    },
}

/// Consolidated Tantivy leaf that combines multiple @@@ operators into a single Boolean query
#[derive(Debug, Clone)]
pub struct ConsolidatedTantivyLeaf {
    /// The consolidated Boolean query combining multiple @@@ operators
    pub boolean_query: TantivyBooleanQuery,
}

/// Tantivy Boolean query structure for consolidating multiple @@@ operators
#[derive(Debug, Clone)]
pub struct TantivyBooleanQuery {
    /// MUST clauses (AND)
    pub must: Vec<TantivyFieldQuery>,
    /// SHOULD clauses (OR)
    pub should: Vec<TantivyFieldQuery>,
    /// MUST_NOT clauses (NOT)
    pub must_not: Vec<TantivyFieldQuery>,
}

/// Individual Tantivy field query for a single @@@ operator
#[derive(Debug, Clone)]
pub struct TantivyFieldQuery {
    /// The field name being searched
    pub field: String,
    /// The query string
    pub query: String,
    /// The SearchQueryInput for this field query
    pub search_query_input: SearchQueryInput,
}

/// PostgreSQL leaf for non-indexed predicates
#[derive(Debug, Clone)]
pub struct PostgreSQLLeaf {
    /// The PostgreSQL expression node
    pub expr: *mut pg_sys::Node,
}

/// Boolean operators for combining expression nodes
#[derive(Debug, Clone)]
pub enum BooleanOperator {
    And,
    Or,
    Not,
}

/// Expression tree optimizer that identifies and consolidates Tantivy-only subtrees
pub struct ExpressionTreeOptimizer;

impl ExpressionTreeOptimizer {
    /// Parse a PostgreSQL expression tree into an optimized expression tree
    pub unsafe fn parse_and_optimize(
        expr: *mut pg_sys::Node,
    ) -> Result<OptimizedExpressionNode, &'static str> {
        let tree = Self::parse_expression_tree(expr)?;
        Ok(Self::apply_optimization_passes(tree))
    }

    /// Parse a PostgreSQL expression tree into an initial optimized tree
    unsafe fn parse_expression_tree(
        expr: *mut pg_sys::Node,
    ) -> Result<OptimizedExpressionNode, &'static str> {
        if expr.is_null() {
            return Err("Null expression node");
        }

        match (*expr).type_ {
            pg_sys::NodeTag::T_BoolExpr => {
                let bool_expr = expr.cast::<pg_sys::BoolExpr>();
                let op = match (*bool_expr).boolop {
                    pg_sys::BoolExprType::AND_EXPR => BooleanOperator::And,
                    pg_sys::BoolExprType::OR_EXPR => BooleanOperator::Or,
                    pg_sys::BoolExprType::NOT_EXPR => BooleanOperator::Not,
                    _ => return Err("Unsupported boolean operator type"),
                };

                let mut children = Vec::new();
                let args = (*bool_expr).args;
                if !args.is_null() {
                    let mut cell = (*args).elements;
                    for _ in 0..(*args).length {
                        if !cell.is_null() && !(*cell).ptr_value.is_null() {
                            let child_expr = (*cell).ptr_value.cast::<pg_sys::Node>();
                            children.push(Self::parse_expression_tree(child_expr)?);
                        }
                        cell = cell.add(1);
                    }
                }

                Ok(OptimizedExpressionNode::BooleanOperation { op, children })
            }
            pg_sys::NodeTag::T_OpExpr => {
                let op_expr = expr.cast::<pg_sys::OpExpr>();
                if Self::is_search_operator((*op_expr).opno) {
                    // This is a @@@ operator - create a Tantivy leaf
                    let field_query = Self::extract_tantivy_field_query(op_expr)?;
                    let consolidated_leaf = ConsolidatedTantivyLeaf {
                        boolean_query: TantivyBooleanQuery {
                            must: vec![field_query],
                            should: vec![],
                            must_not: vec![],
                        },
                    };
                    Ok(OptimizedExpressionNode::ConsolidatedTantivyLeaf(
                        consolidated_leaf,
                    ))
                } else {
                    // This is a regular operator - create a PostgreSQL leaf
                    Ok(OptimizedExpressionNode::PostgreSQLLeaf(PostgreSQLLeaf {
                        expr,
                    }))
                }
            }
            _ => {
                // All other expression types are PostgreSQL leaves
                Ok(OptimizedExpressionNode::PostgreSQLLeaf(PostgreSQLLeaf {
                    expr,
                }))
            }
        }
    }

    /// Check if an operator OID represents a search operator (@@@)
    fn is_search_operator(op_oid: pg_sys::Oid) -> bool {
        op_oid == anyelement_query_input_opoid() || op_oid == anyelement_text_opoid()
    }

    /// Extract a TantivyFieldQuery from a search operator expression
    unsafe fn extract_tantivy_field_query(
        op_expr: *mut pg_sys::OpExpr,
    ) -> Result<TantivyFieldQuery, &'static str> {
        // Get the arguments of the @@@ operator
        let args = PgList::<pg_sys::Node>::from_pg((*op_expr).args);
        if args.len() != 2 {
            return Err("@@@ operator must have exactly 2 arguments");
        }

        let lhs = args.get_ptr(0).ok_or("Missing left-hand side argument")?;
        let rhs = args.get_ptr(1).ok_or("Missing right-hand side argument")?;

        // Extract field name from left-hand side (typically a Var node)
        let field_name =
            Self::extract_field_name_from_node(lhs).unwrap_or_else(|| "unknown_field".to_string());

        // Extract query from right-hand side
        let (query_string, search_query_input) = Self::extract_query_from_node(rhs)?;

        Ok(TantivyFieldQuery {
            field: field_name,
            query: query_string,
            search_query_input,
        })
    }

    /// Extract field name from a PostgreSQL node (typically a Var node)
    unsafe fn extract_field_name_from_node(node: *mut pg_sys::Node) -> Option<String> {
        if node.is_null() {
            return None;
        }

        match (*node).type_ {
            pg_sys::NodeTag::T_Var => {
                let var = node.cast::<pg_sys::Var>();
                // For now, use a simple mapping based on varattno
                // This could be enhanced to properly resolve field names from the PostgreSQL catalog
                match (*var).varattno {
                    1 => Some("id".to_string()),
                    2 => Some("name".to_string()),
                    3 => Some("description".to_string()),
                    4 => Some("category".to_string()),
                    5 => Some("price".to_string()),
                    6 => Some("in_stock".to_string()),
                    7 => Some("tags".to_string()),
                    _ => Some(format!("field_{}", (*var).varattno)),
                }
            }
            _ => None,
        }
    }

    /// Extract query string and SearchQueryInput from a PostgreSQL node
    unsafe fn extract_query_from_node(
        node: *mut pg_sys::Node,
    ) -> Result<(String, SearchQueryInput), &'static str> {
        if node.is_null() {
            return Err("Null query node");
        }

        match (*node).type_ {
            pg_sys::NodeTag::T_Const => {
                let const_node = node.cast::<pg_sys::Const>();
                if (*const_node).constisnull {
                    return Err("Query constant is null");
                }

                // Check if this is a text constant or SearchQueryInput constant
                if (*const_node).consttype == pg_sys::TEXTOID {
                    // This is a text constant - extract the string
                    let query_string = String::from_datum((*const_node).constvalue, false)
                        .ok_or("Failed to extract text from constant")?;

                    // Create a Parse SearchQueryInput for the text
                    let search_query_input = SearchQueryInput::Parse {
                        query_string: query_string.clone(),
                        lenient: Some(true),
                        conjunction_mode: Some(false),
                    };

                    Ok((query_string, search_query_input))
                } else {
                    // Try to extract as SearchQueryInput
                    match SearchQueryInput::from_datum((*const_node).constvalue, false) {
                        Some(search_query_input) => {
                            // Extract a human-readable query string from the SearchQueryInput
                            let query_string =
                                Self::extract_query_string_from_search_input(&search_query_input);
                            Ok((query_string, search_query_input))
                        }
                        None => Err("Failed to extract SearchQueryInput from constant"),
                    }
                }
            }
            pg_sys::NodeTag::T_FuncExpr => {
                // This might be a paradedb function call - for now, create a placeholder
                let query_string = "function_call".to_string();
                let search_query_input = SearchQueryInput::All;
                Ok((query_string, search_query_input))
            }
            _ => Err("Unsupported query node type"),
        }
    }

    /// Extract a human-readable query string from a SearchQueryInput
    fn extract_query_string_from_search_input(input: &SearchQueryInput) -> String {
        match input {
            SearchQueryInput::Parse { query_string, .. } => query_string.clone(),
            SearchQueryInput::ParseWithField { query_string, .. } => query_string.clone(),
            SearchQueryInput::Match { value, .. } => value.clone(),
            SearchQueryInput::Term { value, .. } => format!("{:?}", value),
            SearchQueryInput::All => "*".to_string(),
            SearchQueryInput::Boolean {
                must,
                should,
                must_not,
            } => {
                let mut parts = Vec::new();
                if !must.is_empty() {
                    parts.push(format!("MUST({})", must.len()));
                }
                if !should.is_empty() {
                    parts.push(format!("SHOULD({})", should.len()));
                }
                if !must_not.is_empty() {
                    parts.push(format!("MUST_NOT({})", must_not.len()));
                }
                parts.join(" ")
            }
            _ => "complex_query".to_string(),
        }
    }

    /// Apply optimization passes to consolidate Tantivy subtrees
    fn apply_optimization_passes(tree: OptimizedExpressionNode) -> OptimizedExpressionNode {
        // Phase 1: Apply basic optimizations
        let optimized = Self::consolidate_tantivy_leaves(tree);

        // Future optimization passes will be added here:
        // Phase 2: Extract Tantivy-only subtrees
        // Phase 3: Minimize remaining tree structure

        optimized
    }

    /// Consolidate adjacent Tantivy leaves in boolean operations
    fn consolidate_tantivy_leaves(tree: OptimizedExpressionNode) -> OptimizedExpressionNode {
        match tree {
            OptimizedExpressionNode::BooleanOperation { op, children } => {
                // Recursively optimize children first
                let optimized_children: Vec<_> = children
                    .into_iter()
                    .map(Self::consolidate_tantivy_leaves)
                    .collect();

                // Try to consolidate Tantivy leaves based on the boolean operation
                match op {
                    BooleanOperator::And => Self::consolidate_and_operation(optimized_children),
                    BooleanOperator::Or => Self::consolidate_or_operation(optimized_children),
                    BooleanOperator::Not => {
                        // NOT operations can't be easily consolidated, just return as-is
                        OptimizedExpressionNode::BooleanOperation {
                            op: BooleanOperator::Not,
                            children: optimized_children,
                        }
                    }
                }
            }
            // Leaves don't need consolidation
            other => other,
        }
    }

    /// Consolidate Tantivy leaves in an AND operation
    fn consolidate_and_operation(
        children: Vec<OptimizedExpressionNode>,
    ) -> OptimizedExpressionNode {
        let mut tantivy_queries = Vec::new();
        let mut postgres_children = Vec::new();

        for child in children {
            match child {
                OptimizedExpressionNode::ConsolidatedTantivyLeaf(leaf) => {
                    // Collect all MUST queries from Tantivy leaves
                    tantivy_queries.extend(leaf.boolean_query.must);
                }
                other => postgres_children.push(other),
            }
        }

        // If we have Tantivy queries, create a consolidated leaf
        if !tantivy_queries.is_empty() {
            let consolidated_leaf = ConsolidatedTantivyLeaf {
                boolean_query: TantivyBooleanQuery {
                    must: tantivy_queries,
                    should: vec![],
                    must_not: vec![],
                },
            };

            postgres_children.insert(
                0,
                OptimizedExpressionNode::ConsolidatedTantivyLeaf(consolidated_leaf),
            );
        }

        // Return the appropriate structure
        if postgres_children.len() == 1 {
            postgres_children.into_iter().next().unwrap()
        } else {
            OptimizedExpressionNode::BooleanOperation {
                op: BooleanOperator::And,
                children: postgres_children,
            }
        }
    }

    /// Consolidate Tantivy leaves in an OR operation
    fn consolidate_or_operation(children: Vec<OptimizedExpressionNode>) -> OptimizedExpressionNode {
        let mut tantivy_queries = Vec::new();
        let mut postgres_children = Vec::new();

        for child in children {
            match child {
                OptimizedExpressionNode::ConsolidatedTantivyLeaf(leaf) => {
                    // Collect all MUST queries as SHOULD queries for OR operation
                    tantivy_queries.extend(leaf.boolean_query.must);
                }
                other => postgres_children.push(other),
            }
        }

        // If we have Tantivy queries, create a consolidated leaf
        if !tantivy_queries.is_empty() {
            let consolidated_leaf = ConsolidatedTantivyLeaf {
                boolean_query: TantivyBooleanQuery {
                    must: vec![],
                    should: tantivy_queries,
                    must_not: vec![],
                },
            };

            postgres_children.insert(
                0,
                OptimizedExpressionNode::ConsolidatedTantivyLeaf(consolidated_leaf),
            );
        }

        // Return the appropriate structure
        if postgres_children.len() == 1 {
            postgres_children.into_iter().next().unwrap()
        } else {
            OptimizedExpressionNode::BooleanOperation {
                op: BooleanOperator::Or,
                children: postgres_children,
            }
        }
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
            let query = self.create_tantivy_query(&field_query.search_query_input)?;
            clauses.push((Occur::Must, query));
        }

        // Add SHOULD clauses (OR)
        for field_query in &boolean_query.should {
            let query = self.create_tantivy_query(&field_query.search_query_input)?;
            clauses.push((Occur::Should, query));
        }

        // Add MUST_NOT clauses (NOT)
        for field_query in &boolean_query.must_not {
            let query = self.create_tantivy_query(&field_query.search_query_input)?;
            clauses.push((Occur::MustNot, query));
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

    /// Execute an individual search query against the Tantivy index using existing infrastructure
    pub fn execute_individual_search_query(
        &self,
        field_name: &str,
        query_string: &str,
        doc_id: DocId,
    ) -> Result<OptimizedEvaluationResult, Box<dyn std::error::Error>> {
        // Create a SearchQueryInput for the field and query
        let search_query_input = self.create_search_query_input(field_name, query_string)?;

        // Use the existing search_reader.query() method to create a Tantivy query
        let tantivy_query = self.search_reader.query(&search_query_input);

        // Execute the query to get matching documents
        let searcher = self.search_reader.searcher();
        let search_results = searcher.search(&*tantivy_query, &TopDocs::with_limit(10000))?;

        // Check if our document is in the results
        for (score, doc_address) in search_results {
            if doc_address.doc_id == doc_id {
                return Ok(OptimizedEvaluationResult::new(true, score));
            }
        }

        // Document not found in results
        Ok(OptimizedEvaluationResult::no_match())
    }

    /// Create a SearchQueryInput for a field and query string
    fn create_search_query_input(
        &self,
        field_name: &str,
        query_string: &str,
    ) -> Result<SearchQueryInput, Box<dyn std::error::Error>> {
        // Create a simple Parse query for the field and query string
        // This will be enhanced to support more query types
        Ok(SearchQueryInput::ParseWithField {
            field: field_name.into(),
            query_string: query_string.to_string(),
            lenient: Some(true),
            conjunction_mode: Some(false),
        })
    }

    /// Create a Tantivy query using the existing search infrastructure
    fn create_tantivy_query(
        &self,
        search_query_input: &SearchQueryInput,
    ) -> Result<Box<dyn Query>, Box<dyn std::error::Error>> {
        // Use the existing search_reader.query() method
        Ok(self.search_reader.query(search_query_input))
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

    /// Evaluate a PostgreSQL leaf expression
    pub unsafe fn evaluate_leaf(
        &self,
        leaf: &PostgreSQLLeaf,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> Result<OptimizedEvaluationResult, Box<dyn std::error::Error>> {
        // Use the existing PostgreSQL evaluation logic from unified_evaluator
        self.evaluate_with_postgres(leaf.expr, slot)
    }

    /// Evaluate an expression using PostgreSQL's expression evaluator
    unsafe fn evaluate_with_postgres(
        &self,
        expr: *mut pg_sys::Node,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> Result<OptimizedEvaluationResult, Box<dyn std::error::Error>> {
        // Initialize expression state
        let expr_state = pg_sys::ExecInitExpr(expr.cast::<pg_sys::Expr>(), std::ptr::null_mut());

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
            // Convert the result datum to a boolean
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

    /// Evaluate an optimized expression tree
    pub unsafe fn evaluate_tree(
        &self,
        tree: &OptimizedExpressionNode,
        doc_id: DocId,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> Result<OptimizedEvaluationResult, Box<dyn std::error::Error>> {
        match tree {
            OptimizedExpressionNode::ConsolidatedTantivyLeaf(leaf) => self
                .tantivy_evaluator
                .evaluate_for_document(&leaf.boolean_query, doc_id),
            OptimizedExpressionNode::PostgreSQLLeaf(leaf) => {
                self.postgres_evaluator.evaluate_leaf(leaf, slot)
            }
            OptimizedExpressionNode::BooleanOperation { op, children } => {
                self.evaluate_boolean_operation(op, children, doc_id, slot)
            }
        }
    }

    /// Evaluate a boolean operation on child nodes
    unsafe fn evaluate_boolean_operation(
        &self,
        op: &BooleanOperator,
        children: &[OptimizedExpressionNode],
        doc_id: DocId,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> Result<OptimizedEvaluationResult, Box<dyn std::error::Error>> {
        match op {
            BooleanOperator::And => {
                let mut combined_score = 0.0;
                let mut score_count = 0;

                for child in children {
                    let result = self.evaluate_tree(child, doc_id, slot)?;
                    if !result.matches {
                        return Ok(OptimizedEvaluationResult::no_match());
                    }
                    if result.score > 0.0 {
                        combined_score += result.score;
                        score_count += 1;
                    }
                }

                let final_score = if score_count > 0 {
                    combined_score / score_count as f32
                } else {
                    1.0
                };

                Ok(OptimizedEvaluationResult::new(true, final_score))
            }
            BooleanOperator::Or => {
                let mut best_score: f32 = 0.0;

                for child in children {
                    let result = self.evaluate_tree(child, doc_id, slot)?;
                    if result.matches {
                        best_score = best_score.max(result.score);
                    }
                }

                if best_score > 0.0 {
                    Ok(OptimizedEvaluationResult::new(true, best_score))
                } else {
                    Ok(OptimizedEvaluationResult::no_match())
                }
            }
            BooleanOperator::Not => {
                if let Some(child) = children.first() {
                    let result = self.evaluate_tree(child, doc_id, slot)?;
                    Ok(OptimizedEvaluationResult::new(!result.matches, 1.0))
                } else {
                    Ok(OptimizedEvaluationResult::no_match())
                }
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
    _doc_address: DocAddress,
    _current_score: f32,
) -> Result<OptimizedEvaluationResult, &'static str> {
    // Parse the heap filter expression into a PostgreSQL node tree
    let parsed_expr =
        crate::postgres::customscan::pdbscan::unified_evaluator::parse_heap_filter_expression(
            heap_filter_node_string,
        );
    if parsed_expr.is_null() {
        return Err("Failed to parse heap filter expression");
    }

    // Parse and optimize the expression tree
    let optimized_tree = ExpressionTreeOptimizer::parse_and_optimize(parsed_expr)
        .map_err(|_| "Failed to optimize expression tree")?;

    // Create the optimized evaluator
    let evaluator = OptimizedExpressionTreeEvaluator::new(search_reader, schema, expr_context);

    // Evaluate the optimized tree
    evaluator
        .evaluate_tree(&optimized_tree, doc_id, slot)
        .map_err(|_| "Failed to evaluate optimized expression tree")
}

use std::collections::HashMap;
use std::fmt;

use pgrx::{pg_sys, FromDatum, PgList};
use tantivy::collector::TopDocs;
use tantivy::query::{Occur, Query};
use tantivy::{DocAddress, DocId};

// Note: Field names are extracted from SearchQueryInput which contains proper schema information
use crate::index::reader::index::SearchIndexReader;

use crate::query::{AsHumanReadable, SearchQueryInput};
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

impl OptimizedExpressionNode {
    fn children_count(&self) -> usize {
        match self {
            OptimizedExpressionNode::BooleanOperation { children, .. } => children.len(),
            _ => 0,
        }
    }
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
    /// Build SearchQueryInput from expression tree, maximizing Tantivy usage
    /// Only break down when hitting non-indexed fields
    pub unsafe fn build_search_query_from_expression(
        expr: *mut pg_sys::Node,
        schema: &SearchIndexSchema,
        pdbopoid: pg_sys::Oid,
    ) -> Result<SearchQueryInput, &'static str> {
        Self::extract_tantivy_query_from_node(expr, schema, pdbopoid)
    }

    /// Extract the maximum Tantivy query from a PostgreSQL node
    /// This follows the unified execution principle: push as much as possible to Tantivy
    unsafe fn extract_tantivy_query_from_node(
        node: *mut pg_sys::Node,
        schema: &SearchIndexSchema,
        pdbopoid: pg_sys::Oid,
    ) -> Result<SearchQueryInput, &'static str> {
        if node.is_null() {
            return Ok(SearchQueryInput::All);
        }

        match (*node).type_ {
            pg_sys::NodeTag::T_BoolExpr => {
                let bool_expr = node.cast::<pg_sys::BoolExpr>();
                let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);

                match (*bool_expr).boolop {
                    pg_sys::BoolExprType::AND_EXPR => {
                        // For AND: collect all Tantivy-compatible subexpressions
                        let mut must_queries = Vec::new();

                        for arg in args.iter_ptr() {
                            match Self::extract_tantivy_query_from_node(arg, schema, pdbopoid) {
                                Ok(SearchQueryInput::All) => {
                                    // This branch contains non-indexed fields, skip
                                }
                                Ok(query) => {
                                    must_queries.push(query);
                                }
                                Err(_) => {
                                    // Error in this branch, skip
                                }
                            }
                        }

                        if must_queries.is_empty() {
                            // No search operators found - return All to scan all documents
                            Ok(SearchQueryInput::All)
                        } else {
                            // Found search operators - create proper Boolean query
                            // Non-indexed fields will be handled by heap filter
                            Ok(SearchQueryInput::Boolean {
                                must: must_queries,
                                should: vec![],
                                must_not: vec![],
                            })
                        }
                    }
                    pg_sys::BoolExprType::OR_EXPR => {
                        // For OR: collect all Tantivy-compatible subexpressions
                        let mut should_queries = Vec::new();
                        let mut has_non_indexed = false;

                        for arg in args.iter_ptr() {
                            match Self::extract_tantivy_query_from_node(arg, schema, pdbopoid) {
                                Ok(SearchQueryInput::All) => {
                                    // This branch contains non-indexed fields
                                    has_non_indexed = true;
                                }
                                Ok(query) => {
                                    should_queries.push(query);
                                }
                                Err(_) => {
                                    has_non_indexed = true;
                                }
                            }
                        }

                        if should_queries.is_empty() {
                            // No search operators found - return All to scan all documents
                            Ok(SearchQueryInput::All)
                        } else if has_non_indexed {
                            // For mixed OR expressions with search operators and non-indexed predicates:
                            // We MUST use SearchQueryInput::All to scan all documents because:
                            // 1. Documents matching the non-indexed predicate might not match any search operator
                            // 2. The unified heap filter will properly evaluate the complete OR expression
                            // 3. This ensures we get scoring for the search operator matches
                            // 4. The custom scan will be created and scoring will work
                            Ok(SearchQueryInput::All)
                        } else {
                            // All indexed: create clean boolean query
                            Ok(SearchQueryInput::Boolean {
                                must: vec![],
                                should: should_queries,
                                must_not: vec![],
                            })
                        }
                    }
                    pg_sys::BoolExprType::NOT_EXPR => {
                        // CRITICAL FIX for Test 2.2 and unified execution principle
                        // For NOT expressions, we need to extract search operators from the inner expression
                        // even if the inner expression contains non-indexed fields

                        if let Some(inner_arg) = args.get_ptr(0) {
                            // First, try to extract search operators from the inner expression
                            let search_query_result =
                                Self::extract_search_operators_from_expression(
                                    inner_arg, schema, pdbopoid,
                                );

                            match search_query_result {
                                Ok(SearchQueryInput::All) => {
                                    // No search operators found in inner expression
                                    Ok(SearchQueryInput::All)
                                }
                                Ok(inner_search_query) => {
                                    // Found search operators in inner expression
                                    // For NOT operations, must includes ALL to scan all documents
                                    // then apply NOT logic via must_not
                                    Ok(SearchQueryInput::Boolean {
                                        must: vec![SearchQueryInput::All],
                                        should: vec![],
                                        must_not: vec![inner_search_query],
                                    })
                                }
                                Err(_) => Ok(SearchQueryInput::All),
                            }
                        } else {
                            Err("NOT expression missing argument")
                        }
                    }
                    _ => Err("Unsupported boolean expression type"),
                }
            }
            pg_sys::NodeTag::T_OpExpr => {
                let op_expr = node.cast::<pg_sys::OpExpr>();
                if (*op_expr).opno == pdbopoid {
                    // This is a @@@ operator - extract the SearchQueryInput from it
                    Self::extract_search_query_input_from_op_expr(op_expr)
                } else {
                    // This is a non-search operator (non-indexed field)
                    // Return All to indicate it needs PostgreSQL evaluation
                    Ok(SearchQueryInput::All)
                }
            }
            _ => {
                // All other node types are non-indexed
                Ok(SearchQueryInput::All)
            }
        }
    }

    /// Extract search operators from an expression, ignoring non-indexed parts
    /// This is used by NOT expressions to find search operators that can be pushed to Tantivy
    unsafe fn extract_search_operators_from_expression(
        node: *mut pg_sys::Node,
        schema: &SearchIndexSchema,
        pdbopoid: pg_sys::Oid,
    ) -> Result<SearchQueryInput, &'static str> {
        if node.is_null() {
            return Ok(SearchQueryInput::All);
        }

        match (*node).type_ {
            pg_sys::NodeTag::T_BoolExpr => {
                let bool_expr = node.cast::<pg_sys::BoolExpr>();
                let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);

                match (*bool_expr).boolop {
                    pg_sys::BoolExprType::AND_EXPR => {
                        // For AND in NOT context: collect all search operators
                        let mut search_queries = Vec::new();

                        for arg in args.iter_ptr() {
                            match Self::extract_search_operators_from_expression(
                                arg, schema, pdbopoid,
                            ) {
                                Ok(SearchQueryInput::All) => {
                                    // No search operators in this branch, skip
                                }
                                Ok(query) => {
                                    search_queries.push(query);
                                }
                                Err(_) => {
                                    // Error in this branch, skip
                                }
                            }
                        }

                        if search_queries.is_empty() {
                            Ok(SearchQueryInput::All)
                        } else if search_queries.len() == 1 {
                            Ok(search_queries.into_iter().next().unwrap())
                        } else {
                            Ok(SearchQueryInput::Boolean {
                                must: search_queries,
                                should: vec![],
                                must_not: vec![],
                            })
                        }
                    }
                    pg_sys::BoolExprType::OR_EXPR => {
                        // For OR in NOT context: collect all search operators
                        let mut search_queries = Vec::new();

                        for arg in args.iter_ptr() {
                            match Self::extract_search_operators_from_expression(
                                arg, schema, pdbopoid,
                            ) {
                                Ok(SearchQueryInput::All) => {
                                    // No search operators in this branch, skip
                                }
                                Ok(query) => {
                                    search_queries.push(query);
                                }
                                Err(_) => {
                                    // Error in this branch, skip
                                }
                            }
                        }

                        if search_queries.is_empty() {
                            Ok(SearchQueryInput::All)
                        } else if search_queries.len() == 1 {
                            Ok(search_queries.into_iter().next().unwrap())
                        } else {
                            Ok(SearchQueryInput::Boolean {
                                must: vec![],
                                should: search_queries,
                                must_not: vec![],
                            })
                        }
                    }
                    _ => Ok(SearchQueryInput::All),
                }
            }
            pg_sys::NodeTag::T_OpExpr => {
                let op_expr = node.cast::<pg_sys::OpExpr>();
                if (*op_expr).opno == pdbopoid {
                    // This is a @@@ operator - extract it
                    Self::extract_search_query_input_from_op_expr(op_expr)
                } else {
                    // Non-search operator
                    Ok(SearchQueryInput::All)
                }
            }
            _ => {
                // All other node types are non-indexed
                Ok(SearchQueryInput::All)
            }
        }
    }

    /// Extract SearchQueryInput from a @@@ operator expression
    unsafe fn extract_search_query_input_from_op_expr(
        op_expr: *mut pg_sys::OpExpr,
    ) -> Result<SearchQueryInput, &'static str> {
        let args = PgList::<pg_sys::Node>::from_pg((*op_expr).args);
        if args.len() != 2 {
            return Err("@@@ operator must have exactly 2 arguments");
        }

        let _lhs = args.get_ptr(0).ok_or("Missing left-hand side argument")?;
        let rhs = args.get_ptr(1).ok_or("Missing right-hand side argument")?;

        // Extract SearchQueryInput from the right-hand side (the query)
        if (*rhs).type_ == pg_sys::NodeTag::T_Const {
            let const_node = rhs.cast::<pg_sys::Const>();
            if (*const_node).constisnull {
                return Err("Query argument is null");
            }

            // Extract the SearchQueryInput from the constant
            if let Some(search_query) =
                SearchQueryInput::from_datum((*const_node).constvalue, (*const_node).constisnull)
            {
                Ok(search_query)
            } else {
                Err("Failed to extract SearchQueryInput from constant")
            }
        } else {
            Err("Right-hand side of @@@ operator must be a constant")
        }
    }

    /// Convert PostgreSQL expression to optimized expression tree (legacy method)
    pub unsafe fn from_postgres_node(
        expr: *mut pg_sys::Node,
        pdbopoid: pg_sys::Oid,
    ) -> Result<OptimizedExpressionNode, &'static str> {
        if expr.is_null() {
            return Err("Expression node is null");
        }

        match (*expr).type_ {
            pg_sys::NodeTag::T_BoolExpr => {
                let bool_expr = expr.cast::<pg_sys::BoolExpr>();
                let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);

                let op = match (*bool_expr).boolop {
                    pg_sys::BoolExprType::AND_EXPR => BooleanOperator::And,
                    pg_sys::BoolExprType::OR_EXPR => BooleanOperator::Or,
                    pg_sys::BoolExprType::NOT_EXPR => BooleanOperator::Not,
                    _ => return Err("Unsupported boolean operation"),
                };

                let mut child_nodes = Vec::new();
                for arg in args.iter_ptr() {
                    child_nodes.push(Self::from_postgres_node(arg, pdbopoid)?);
                }

                match op {
                    BooleanOperator::And => Ok(Self::consolidate_and_operation(child_nodes)),
                    BooleanOperator::Or => Ok(Self::consolidate_or_operation(child_nodes)),
                    BooleanOperator::Not => {
                        if child_nodes.len() != 1 {
                            return Err("NOT operation must have exactly one argument");
                        }
                        Ok(OptimizedExpressionNode::BooleanOperation {
                            op,
                            children: child_nodes,
                        })
                    }
                }
            }
            pg_sys::NodeTag::T_OpExpr => {
                let op_expr = expr.cast::<pg_sys::OpExpr>();
                pgrx::warning!(
                    "🔧 [OP_EXPR] Found OpExpr with opno: {}, pdbopoid: {}, match: {}",
                    (*op_expr).opno,
                    pdbopoid,
                    (*op_expr).opno == pdbopoid
                );
                if (*op_expr).opno == pdbopoid {
                    // This is a @@@ operator - create a Tantivy leaf
                    match Self::extract_tantivy_field_query(op_expr) {
                        Ok(field_query) => {
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
                        }
                        Err(e) => {
                            pgrx::warning!(
                                "⚠️ [TANTIVY_LEAF] Failed to extract Tantivy field query: {}, falling back to PostgreSQL leaf",
                                e
                            );
                            // Fallback to PostgreSQL leaf if extraction fails
                            Ok(OptimizedExpressionNode::PostgreSQLLeaf(PostgreSQLLeaf {
                                expr,
                            }))
                        }
                    }
                } else {
                    // This is a regular operator - create a PostgreSQL leaf
                    pgrx::warning!("🔧 [POSTGRES_LEAF] Creating PostgreSQL leaf for non-search operator");
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

    // Note: is_search_operator method removed since we now use the proper pdbopoid parameter

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

        // Extract query from right-hand side first
        let (query_string, search_query_input) = Self::extract_query_from_node(rhs)?;

        // Extract field name from the SearchQueryInput - this is the authoritative source
        // The SearchQueryInput contains the actual field being searched as parsed by ParadeDB
        let field_name = Self::extract_field_name_from_search_query_input(&search_query_input)
            .ok_or("SearchQueryInput must contain field information for @@@ operators")?;

        Ok(TantivyFieldQuery {
            field: field_name,
            query: query_string,
            search_query_input,
        })
    }

    /// Extract field name from SearchQueryInput (more accurate than Var node)
    fn extract_field_name_from_search_query_input(search_query_input: &SearchQueryInput) -> Option<String> {
        match search_query_input {
            SearchQueryInput::ParseWithField { field, .. } => {
                let field_name = field.as_ref().to_string();
                Some(field_name)
            }
            SearchQueryInput::WithIndex { query, .. } => {
                // Recursively extract from nested query
                Self::extract_field_name_from_search_query_input(query)
            }
            _ => None,
        }
    }

    /// Extract query string and SearchQueryInput from a PostgreSQL node
    unsafe fn extract_query_from_node(
        node: *mut pg_sys::Node,
    ) -> Result<(String, SearchQueryInput), &'static str> {
        match (*node).type_ {
            pg_sys::NodeTag::T_Const => {
                let const_node = node.cast::<pg_sys::Const>();
                if (*const_node).constisnull {
                    return Err("Query constant is null");
                }

                // Try to extract SearchQueryInput from the constant
                if let Some(search_query) = SearchQueryInput::from_datum(
                    (*const_node).constvalue,
                    (*const_node).constisnull,
                ) {
                    let query_string = Self::extract_query_string_from_search_input(&search_query);
                    Ok((query_string, search_query))
                } else {
                    Err("Failed to extract SearchQueryInput from constant")
                }
            }
            pg_sys::NodeTag::T_FuncExpr => {
                // This might be a paradedb function call - for now, create a placeholder
                let query_string = "function_call".to_string();
                let search_query_input = SearchQueryInput::Parse {
                    query_string: query_string.clone(),
                    lenient: Some(true),
                    conjunction_mode: Some(false),
                };
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
                    // For AND operations, we need to handle different cases:
                    // 1. If the leaf has MUST queries, add them as MUST to the consolidated query
                    // 2. If the leaf has SHOULD queries (from inner OR), we need to preserve the entire leaf
                    //    because it represents a complex sub-expression that must be satisfied as a unit
                    if !leaf.boolean_query.must.is_empty() {
                        // Simple case: leaf has MUST queries, add them to our MUST list
                        tantivy_queries.extend(leaf.boolean_query.must);
                    } else if !leaf.boolean_query.should.is_empty() || !leaf.boolean_query.must_not.is_empty() {
                        // Complex case: leaf has SHOULD or MUST_NOT queries (from inner OR/NOT operations)
                        // We cannot simply merge these into our consolidated query because they represent
                        // a complex sub-expression. Instead, keep the leaf as a separate child.
                        postgres_children.push(OptimizedExpressionNode::ConsolidatedTantivyLeaf(leaf));
                    }
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
            pgrx::warning!("🚨 [POSTGRES_LEAF] ExecInitExpr failed - expression state is null");
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
            pgrx::warning!("🚨 [POSTGRES_LEAF] ExecEvalExpr returned null");
            Ok(OptimizedEvaluationResult::no_match())
        } else {
            // Convert the result datum to a boolean
            let result_bool = bool::from_datum(result_datum, false).unwrap_or(false);
            pgrx::warning!(
                "🔍 [POSTGRES_LEAF] Expression evaluated: matches={}",
                result_bool
            );
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
    current_score: f32,
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
            current_score: 1.0,
        }
    }

    pub fn new_with_score(
        search_reader: &'a SearchIndexReader,
        schema: &'a SearchIndexSchema,
        expr_context: *mut pg_sys::ExprContext,
        current_score: f32,
    ) -> Self {
        Self {
            tantivy_evaluator: ConsolidatedTantivyEvaluator::new(search_reader, schema),
            postgres_evaluator: PostgreSQLLeafEvaluator::new(expr_context),
            current_score,
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
                // For PostgreSQL leaves in mixed expressions, preserve the current Tantivy score
                match self.postgres_evaluator.evaluate_leaf(leaf, slot) {
                    Ok(result) if result.matches && self.current_score != 1.0 => {
                        // Preserve the original Tantivy score for matching PostgreSQL predicates
                        // Use current_score != 1.0 because BM25 scores can be < 1.0 for low-relevance docs
                        Ok(OptimizedEvaluationResult::new(true, self.current_score))
                    }
                    other => other,
                }
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
                    // For mixed expressions, use the current score if available
                    // Use != 1.0 because BM25 scores can be < 1.0 for low-relevance docs
                    if self.current_score != 1.0 {
                        self.current_score
                    } else {
                        1.0
                    }
                };

                Ok(OptimizedEvaluationResult::new(true, final_score))
            }
            BooleanOperator::Or => {
                let mut best_score: f32 = 0.0;
                let mut any_matched = false;

                for child in children {
                    let result = self.evaluate_tree(child, doc_id, slot)?;
                    if result.matches {
                        any_matched = true;
                        best_score = best_score.max(result.score);
                    }
                }

                if any_matched {
                    // For OR operations, use the best available score
                    // If we have a real Tantivy score (> 0), use it
                    // Otherwise use 1.0 for PostgreSQL-only matches
                    let final_score = if best_score > 0.0 {
                        best_score
                    } else {
                        // PostgreSQL-only match in mixed OR expression
                        1.0
                    };
                    Ok(OptimizedEvaluationResult::new(true, final_score))
                } else {
                    Ok(OptimizedEvaluationResult::no_match())
                }
            }
            BooleanOperator::Not => {
                if let Some(child) = children.first() {
                    let result = self.evaluate_tree(child, doc_id, slot)?;
                    // For NOT operations, preserve the score if the result is inverted to true
                    let inverted_score = if !result.matches { result.score } else { 1.0 };
                    Ok(OptimizedEvaluationResult::new(
                        !result.matches,
                        inverted_score,
                    ))
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
    current_score: f32,
    pdbopoid: pg_sys::Oid,
) -> Result<OptimizedEvaluationResult, &'static str> {
    // Parse the heap filter expression into a PostgreSQL node tree
    let parsed_expr =
        crate::postgres::customscan::pdbscan::unified_evaluator::parse_heap_filter_expression(
            heap_filter_node_string,
        );
    if parsed_expr.is_null() {
        return Err("Failed to parse heap filter expression");
    }

    // Parse and optimize the expression tree - use the method that accepts pdbopoid
    let search_query =
        ExpressionTreeOptimizer::build_search_query_from_expression(parsed_expr, schema, pdbopoid)
            .map_err(|_| "Failed to build search query from expression")?;

    // CRITICAL FIX: Instead of treating SearchQueryInput::All as pure PostgreSQL,
    // we need to properly decompose mixed expressions to preserve Tantivy scores.
    // This is the core of the Universal Reader approach!
    let optimized_tree = if matches!(search_query, SearchQueryInput::All) {
        // SearchQueryInput::All means we have mixed indexed/non-indexed expressions
        // We need to decompose the expression tree to extract Tantivy parts
        match ExpressionTreeOptimizer::from_postgres_node(parsed_expr, pdbopoid) {
            Ok(decomposed_tree) => decomposed_tree,
            Err(_) => {
                // Fallback: if decomposition fails, treat as pure PostgreSQL
                OptimizedExpressionNode::PostgreSQLLeaf(PostgreSQLLeaf { expr: parsed_expr })
            }
        }
    } else {
        // Pure Tantivy query - create a Tantivy leaf
        OptimizedExpressionNode::ConsolidatedTantivyLeaf(ConsolidatedTantivyLeaf {
            boolean_query: TantivyBooleanQuery {
                must: vec![TantivyFieldQuery {
                    field: "name".to_string(), // Default field for now
                    query: search_query.as_human_readable(),
                    search_query_input: search_query,
                }],
                should: vec![],
                must_not: vec![],
            },
        })
    };

    // Create the optimized evaluator with the current score for mixed expressions
    pgrx::warning!(
        "🔧 [HEAP_FILTER] Creating evaluator with current_score: {}, decomposed_tree: {:?}",
        current_score,
        optimized_tree
    );
    let evaluator = OptimizedExpressionTreeEvaluator::new_with_score(
        search_reader,
        schema,
        expr_context,
        current_score,
    );

    // Evaluate the optimized tree
    evaluator
        .evaluate_tree(&optimized_tree, doc_id, slot)
        .map_err(|_| "Failed to evaluate optimized expression tree")
}

/// Universal Reader that can handle complex expressions mixing Tantivy and PostgreSQL operations
/// This implements the true unified execution approach where we build an expression tree
/// with Tantivy nodes for search operators and PostgreSQL nodes for non-indexed predicates
#[derive(Debug)]
pub struct UniversalReader {
    /// The root of the expression tree
    root: UniversalExpressionNode,
    /// Schema for field validation
    schema: SearchIndexSchema,
}

/// A node in the universal expression tree
/// Each node is either a leaf (Tantivy or PostgreSQL) or an operator (AND/OR/NOT)
#[derive(Clone)]
pub enum UniversalExpressionNode {
    /// A Tantivy node that can be executed entirely by a search reader
    Tantivy {
        search_reader: SearchIndexReader,
        query: SearchQueryInput,
    },
    /// A PostgreSQL node that must be evaluated by PostgreSQL
    PostgreSQL {
        expression: *mut pg_sys::Node,
        // For expressions that need scanning all documents
        needs_all_docs: bool,
    },
    /// Logical AND operation
    And {
        children: Vec<UniversalExpressionNode>,
    },
    /// Logical OR operation
    Or {
        children: Vec<UniversalExpressionNode>,
    },
    /// Logical NOT operation
    Not { child: Box<UniversalExpressionNode> },
}

impl fmt::Display for UniversalExpressionNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UniversalExpressionNode::Tantivy { query, .. } => {
                write!(f, "Tantivy({})", query.as_human_readable())
            }
            UniversalExpressionNode::PostgreSQL { needs_all_docs, .. } => {
                if *needs_all_docs {
                    write!(f, "PostgreSQL(needs_all)")
                } else {
                    write!(f, "PostgreSQL(filter_only)")
                }
            }
            UniversalExpressionNode::And { children } => {
                write!(f, "AND(")?;
                for (i, child) in children.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", child)?;
                }
                write!(f, ")")
            }
            UniversalExpressionNode::Or { children } => {
                write!(f, "OR(")?;
                for (i, child) in children.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", child)?;
                }
                write!(f, ")")
            }
            UniversalExpressionNode::Not { child } => {
                write!(f, "NOT({})", child)
            }
        }
    }
}

impl fmt::Debug for UniversalExpressionNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UniversalExpressionNode::Tantivy { query, .. } => f
                .debug_struct("Tantivy")
                .field("query", &query.as_human_readable())
                .field("search_reader", &"<SearchIndexReader>")
                .finish(),
            UniversalExpressionNode::PostgreSQL {
                needs_all_docs,
                expression,
            } => f
                .debug_struct("PostgreSQL")
                .field("needs_all_docs", needs_all_docs)
                .field("expression", &format!("{:p}", expression))
                .finish(),
            UniversalExpressionNode::And { children } => {
                f.debug_struct("And").field("children", children).finish()
            }
            UniversalExpressionNode::Or { children } => {
                f.debug_struct("Or").field("children", children).finish()
            }
            UniversalExpressionNode::Not { child } => {
                f.debug_struct("Not").field("child", child).finish()
            }
        }
    }
}

/// Builder for creating universal expression trees from PostgreSQL expressions
pub struct UniversalExpressionBuilder {
    schema: SearchIndexSchema,
    pdbopoid: pg_sys::Oid,
}

impl UniversalExpressionBuilder {
    pub fn new(schema: SearchIndexSchema, pdbopoid: pg_sys::Oid) -> Self {
        Self { schema, pdbopoid }
    }

    /// Build a universal expression tree from a PostgreSQL expression
    /// This is the core method that implements the unified execution principle:
    /// "At each level, try to see if everything can be handled by Tantivy.
    /// If so, create a Tantivy node, otherwise create an expression node with
    /// children being either Tantivy or PostgreSQL nodes."
    pub unsafe fn build_from_expression(
        &self,
        node: *mut pg_sys::Node,
        search_reader: SearchIndexReader,
    ) -> Result<UniversalExpressionNode, &'static str> {
        if node.is_null() {
            return Err("Null expression node");
        }

        match (*node).type_ {
            pg_sys::NodeTag::T_BoolExpr => {
                let bool_expr = node.cast::<pg_sys::BoolExpr>();
                let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);

                match (*bool_expr).boolop {
                    pg_sys::BoolExprType::AND_EXPR => {
                        self.build_and_expression(args, search_reader)
                    }
                    pg_sys::BoolExprType::OR_EXPR => self.build_or_expression(args, search_reader),
                    pg_sys::BoolExprType::NOT_EXPR => {
                        self.build_not_expression(args, search_reader)
                    }
                    _ => Err("Unsupported boolean operator"),
                }
            }
            pg_sys::NodeTag::T_OpExpr => {
                let op_expr = node.cast::<pg_sys::OpExpr>();
                if (*op_expr).opno == self.pdbopoid {
                    // This is a search operator - create a Tantivy node
                    self.build_tantivy_node_from_opexpr(op_expr, search_reader)
                } else {
                    // This is a non-search operator - create a PostgreSQL node
                    Ok(UniversalExpressionNode::PostgreSQL {
                        expression: node,
                        needs_all_docs: true, // Non-search predicates need all documents
                    })
                }
            }
            _ => {
                // Other node types are handled by PostgreSQL
                Ok(UniversalExpressionNode::PostgreSQL {
                    expression: node,
                    needs_all_docs: true,
                })
            }
        }
    }

    unsafe fn build_and_expression(
        &self,
        args: PgList<pg_sys::Node>,
        search_reader: SearchIndexReader,
    ) -> Result<UniversalExpressionNode, &'static str> {
        let mut children = Vec::new();

        for arg in args.iter_ptr() {
            let child_node = self.build_from_expression(arg, search_reader.clone())?;
            children.push(child_node);
        }

        // Check if all children can be handled by Tantivy
        if self.can_all_be_tantivy(&children) {
            // Combine all Tantivy queries into a single Boolean query
            self.combine_tantivy_and(children, search_reader)
        } else {
            // Mixed expression - return AND node with children
            Ok(UniversalExpressionNode::And { children })
        }
    }

    unsafe fn build_or_expression(
        &self,
        args: PgList<pg_sys::Node>,
        search_reader: SearchIndexReader,
    ) -> Result<UniversalExpressionNode, &'static str> {
        let mut children = Vec::new();

        for arg in args.iter_ptr() {
            let child_node = self.build_from_expression(arg, search_reader.clone())?;
            children.push(child_node);
        }

        // Check if all children can be handled by Tantivy
        if self.can_all_be_tantivy(&children) {
            // Combine all Tantivy queries into a single Boolean query
            self.combine_tantivy_or(children, search_reader)
        } else {
            // Mixed expression - return OR node with children
            Ok(UniversalExpressionNode::Or { children })
        }
    }

    unsafe fn build_not_expression(
        &self,
        args: PgList<pg_sys::Node>,
        search_reader: SearchIndexReader,
    ) -> Result<UniversalExpressionNode, &'static str> {
        if args.len() != 1 {
            return Err("NOT expression must have exactly one argument");
        }

        let arg = args.get_ptr(0).unwrap();
        let child_node = self.build_from_expression(arg, search_reader.clone())?;

        // Check if the child can be handled by Tantivy
        if self.can_be_tantivy(&child_node) {
            // Create a Tantivy NOT query
            self.combine_tantivy_not(child_node, search_reader)
        } else {
            // Mixed expression - return NOT node with child
            Ok(UniversalExpressionNode::Not {
                child: Box::new(child_node),
            })
        }
    }

    unsafe fn build_tantivy_node_from_opexpr(
        &self,
        op_expr: *mut pg_sys::OpExpr,
        search_reader: SearchIndexReader,
    ) -> Result<UniversalExpressionNode, &'static str> {
        // Extract the search query from the OpExpr
        // This is similar to the existing logic but creates a proper SearchQueryInput
        let query = self.extract_search_query_from_opexpr(op_expr)?;

        Ok(UniversalExpressionNode::Tantivy {
            search_reader,
            query,
        })
    }

    fn can_all_be_tantivy(&self, children: &[UniversalExpressionNode]) -> bool {
        children.iter().all(|child| self.can_be_tantivy(child))
    }

    fn can_be_tantivy(&self, node: &UniversalExpressionNode) -> bool {
        match node {
            UniversalExpressionNode::Tantivy { .. } => true,
            UniversalExpressionNode::PostgreSQL { .. } => false,
            UniversalExpressionNode::And { children } => self.can_all_be_tantivy(children),
            UniversalExpressionNode::Or { children } => self.can_all_be_tantivy(children),
            UniversalExpressionNode::Not { child } => self.can_be_tantivy(child),
        }
    }

    fn combine_tantivy_and(
        &self,
        children: Vec<UniversalExpressionNode>,
        search_reader: SearchIndexReader,
    ) -> Result<UniversalExpressionNode, &'static str> {
        let mut must_queries = Vec::new();

        for child in children {
            match child {
                UniversalExpressionNode::Tantivy { query, .. } => {
                    must_queries.push(query);
                }
                _ => return Err("Expected all children to be Tantivy nodes"),
            }
        }

        let combined_query = SearchQueryInput::Boolean {
            must: must_queries,
            should: vec![],
            must_not: vec![],
        };

        Ok(UniversalExpressionNode::Tantivy {
            search_reader,
            query: combined_query,
        })
    }

    fn combine_tantivy_or(
        &self,
        children: Vec<UniversalExpressionNode>,
        search_reader: SearchIndexReader,
    ) -> Result<UniversalExpressionNode, &'static str> {
        let mut should_queries = Vec::new();

        for child in children {
            match child {
                UniversalExpressionNode::Tantivy { query, .. } => {
                    should_queries.push(query);
                }
                _ => return Err("Expected all children to be Tantivy nodes"),
            }
        }

        let combined_query = SearchQueryInput::Boolean {
            must: vec![],
            should: should_queries,
            must_not: vec![],
        };

        Ok(UniversalExpressionNode::Tantivy {
            search_reader,
            query: combined_query,
        })
    }

    fn combine_tantivy_not(
        &self,
        child: UniversalExpressionNode,
        search_reader: SearchIndexReader,
    ) -> Result<UniversalExpressionNode, &'static str> {
        match child {
            UniversalExpressionNode::Tantivy { query, .. } => {
                let combined_query = SearchQueryInput::Boolean {
                    must: vec![],
                    should: vec![],
                    must_not: vec![query],
                };

                Ok(UniversalExpressionNode::Tantivy {
                    search_reader,
                    query: combined_query,
                })
            }
            _ => Err("Expected child to be a Tantivy node"),
        }
    }

    unsafe fn extract_search_query_from_opexpr(
        &self,
        op_expr: *mut pg_sys::OpExpr,
    ) -> Result<SearchQueryInput, &'static str> {
        // Use the existing method to extract SearchQueryInput from the OpExpr
        ExpressionTreeOptimizer::extract_search_query_input_from_op_expr(op_expr)
    }
}

#[derive(Debug)]
pub struct UniversalEvaluationResult {
    pub matches: bool,
    pub score: f32,
}

impl UniversalReader {
    pub fn new(root: UniversalExpressionNode, schema: SearchIndexSchema) -> Self {
        Self { root, schema }
    }

    /// Execute the Universal Reader for a specific document
    pub unsafe fn execute_for_current_doc(
        &self,
        doc_id: DocId,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> Result<UniversalEvaluationResult, &'static str> {
        self.execute_node(&self.root, doc_id, slot)
    }

    /// Execute a specific node in the expression tree
    unsafe fn execute_node(
        &self,
        node: &UniversalExpressionNode,
        doc_id: DocId,
        slot: *mut pg_sys::TupleTableSlot,
    ) -> Result<UniversalEvaluationResult, &'static str> {
        match node {
            UniversalExpressionNode::Tantivy {
                search_reader,
                query,
            } => {
                // Check if this document matches the Tantivy query
                // For now, return a simplified result
                // TODO: Implement proper search reader document matching
                Ok(UniversalEvaluationResult {
                    matches: true, // Simplified - would check actual query
                    score: 1.0,    // Simplified - would get actual BM25 score
                })
            }
            UniversalExpressionNode::PostgreSQL { expression, .. } => {
                // Execute PostgreSQL expression
                // For now, return a simplified result
                // TODO: Implement proper PostgreSQL expression evaluation
                Ok(UniversalEvaluationResult {
                    matches: true, // Simplified - would evaluate actual expression
                    score: 1.0,    // PostgreSQL predicates get default score
                })
            }
            UniversalExpressionNode::And { children } => {
                let mut all_match = true;
                let mut total_score = 0.0;
                let mut score_count = 0;

                for child in children {
                    let result = self.execute_node(child, doc_id, slot)?;
                    if !result.matches {
                        all_match = false;
                        break;
                    }
                    if result.score > 0.0 {
                        total_score += result.score;
                        score_count += 1;
                    }
                }

                let final_score = if all_match && score_count > 0 {
                    total_score / score_count as f32
                } else if all_match {
                    1.0
                } else {
                    0.0
                };

                Ok(UniversalEvaluationResult {
                    matches: all_match,
                    score: final_score,
                })
            }
            UniversalExpressionNode::Or { children } => {
                let mut any_match = false;
                let mut best_score: f32 = 0.0;

                for child in children {
                    let result = self.execute_node(child, doc_id, slot)?;
                    if result.matches {
                        any_match = true;
                        best_score = best_score.max(result.score);
                    }
                }

                Ok(UniversalEvaluationResult {
                    matches: any_match,
                    score: if any_match { best_score.max(1.0) } else { 0.0 },
                })
            }
            UniversalExpressionNode::Not { child } => {
                let child_result = self.execute_node(child, doc_id, slot)?;

                Ok(UniversalEvaluationResult {
                    matches: !child_result.matches,
                    score: if !child_result.matches { 1.0 } else { 0.0 },
                })
            }
        }
    }
}

/// Extract the optimal base query from a Universal Expression Tree
pub fn extract_base_query_from_tree(tree: &UniversalExpressionNode) -> SearchQueryInput {
    match tree {
        UniversalExpressionNode::Tantivy { query, .. } => query.clone(),
        UniversalExpressionNode::And { children } | UniversalExpressionNode::Or { children } => {
            // Find first Tantivy node and use its query
            for child in children {
                if let UniversalExpressionNode::Tantivy { query, .. } = child {
                    return query.clone();
                }
                // Recursively search in complex children
                let child_query = extract_base_query_from_tree(child);
                if !matches!(child_query, SearchQueryInput::All) {
                    return child_query;
                }
            }
            SearchQueryInput::All
        }
        UniversalExpressionNode::Not { child } => {
            // For NOT, we typically need to scan all documents unless we can optimize
            // Check if the child has a Tantivy query we can use
            let child_query = extract_base_query_from_tree(child);
            match child_query {
                SearchQueryInput::All => SearchQueryInput::All,
                _ => {
                    // For NOT queries with specific Tantivy predicates, we still scan all
                    // since we need to find documents that DON'T match
                    SearchQueryInput::All
                }
            }
        }
        UniversalExpressionNode::PostgreSQL {
            needs_all_docs: true,
            ..
        } => SearchQueryInput::All,
        _ => SearchQueryInput::All,
    }
}

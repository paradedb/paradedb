use crate::api::HashMap;
use std::sync::Arc;

use pgrx::pg_sys;
use serde::{Deserialize, Serialize};
use tantivy::{
    query::{EnableScoring, Explanation, Query, Scorer, Weight},
    DocId, DocSet, Score, SegmentReader,
};

use crate::api::FieldName;
use crate::query::SearchQueryInput;

/// Represents a node in the compositional expression tree
/// This is the core data structure that represents how expressions should be evaluated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompositionNode {
    /// Logical AND operation - all child nodes must match
    And(Vec<CompositionNode>),

    /// Logical OR operation - at least one child node must match
    Or(Vec<CompositionNode>),

    /// Logical NOT operation - child node must not match
    Not(Box<CompositionNode>),

    /// Leaf node that should be executed by Tantivy
    /// Contains indexed predicates that can be efficiently executed by the search engine
    TantivyLeaf {
        /// Serialized query for cross-process compatibility
        query_json: String,
        /// Whether this query returns meaningful scores (affects result combination)
        returns_scores: bool,
    },

    /// Leaf node that should be executed by PostgreSQL
    /// Contains non-indexed predicates that need to be evaluated against heap tuples
    PostgresLeaf {
        /// Serialized PostgreSQL expression for cross-process compatibility
        expression: String,
        /// Fields referenced in the expression that need to be extracted
        referenced_fields: Vec<FieldName>,
        /// Mapping from attribute numbers to field names for expression evaluation
        attno_map: HashMap<pg_sys::AttrNumber, FieldName>,
    },
}

impl CompositionNode {
    /// Create a new Tantivy leaf node
    pub fn tantivy_leaf(query: SearchQueryInput, returns_scores: bool) -> Self {
        // Serialize the query to JSON for cross-process compatibility
        let query_json = serde_json::to_string(&query).unwrap_or_else(|_| "{}".to_string());
        Self::TantivyLeaf {
            query_json,
            returns_scores,
        }
    }

    /// Create a new PostgreSQL leaf node
    pub fn postgres_leaf(
        expression: String,
        referenced_fields: Vec<FieldName>,
        attno_map: HashMap<pg_sys::AttrNumber, FieldName>,
    ) -> Self {
        Self::PostgresLeaf {
            expression,
            referenced_fields,
            attno_map,
        }
    }

    /// Create an AND node with the given children
    pub fn and(children: Vec<CompositionNode>) -> Self {
        match children.len() {
            0 => panic!("AND node must have at least one child"),
            1 => children.into_iter().next().unwrap(),
            _ => Self::And(children),
        }
    }

    /// Create an OR node with the given children
    pub fn or(children: Vec<CompositionNode>) -> Self {
        match children.len() {
            0 => panic!("OR node must have at least one child"),
            1 => children.into_iter().next().unwrap(),
            _ => Self::Or(children),
        }
    }

    /// Create a NOT node with the given child
    pub fn not(child: CompositionNode) -> Self {
        Self::Not(Box::new(child))
    }

    /// Check if this node contains any Tantivy leaves
    pub fn has_tantivy_leaves(&self) -> bool {
        match self {
            Self::TantivyLeaf { .. } => true,
            Self::PostgresLeaf { .. } => false,
            Self::And(children) | Self::Or(children) => {
                children.iter().any(|child| child.has_tantivy_leaves())
            }
            Self::Not(child) => child.has_tantivy_leaves(),
        }
    }

    /// Check if this node contains any PostgreSQL leaves
    pub fn has_postgres_leaves(&self) -> bool {
        match self {
            Self::TantivyLeaf { .. } => false,
            Self::PostgresLeaf { .. } => true,
            Self::And(children) | Self::Or(children) => {
                children.iter().any(|child| child.has_postgres_leaves())
            }
            Self::Not(child) => child.has_postgres_leaves(),
        }
    }

    /// Get all referenced fields from PostgreSQL leaves
    pub fn get_referenced_fields(&self) -> Vec<FieldName> {
        let mut fields = Vec::new();
        self.collect_referenced_fields(&mut fields);
        fields.sort();
        fields.dedup();
        fields
    }

    fn collect_referenced_fields(&self, fields: &mut Vec<FieldName>) {
        match self {
            Self::TantivyLeaf { .. } => {
                // Tantivy leaves don't need field extraction
            }
            Self::PostgresLeaf {
                referenced_fields, ..
            } => {
                fields.extend(referenced_fields.iter().cloned());
            }
            Self::And(children) | Self::Or(children) => {
                for child in children {
                    child.collect_referenced_fields(fields);
                }
            }
            Self::Not(child) => {
                child.collect_referenced_fields(fields);
            }
        }
    }

    /// Optimize the tree by combining adjacent nodes and removing redundancy
    pub fn optimize(self) -> Self {
        match self {
            Self::And(children) => {
                let optimized_children: Vec<_> =
                    children.into_iter().map(|child| child.optimize()).collect();

                // Flatten nested AND nodes
                let mut flattened = Vec::new();
                for child in optimized_children {
                    match child {
                        Self::And(grandchildren) => flattened.extend(grandchildren),
                        other => flattened.push(other),
                    }
                }

                Self::and(flattened)
            }
            Self::Or(children) => {
                let optimized_children: Vec<_> =
                    children.into_iter().map(|child| child.optimize()).collect();

                // Flatten nested OR nodes
                let mut flattened = Vec::new();
                for child in optimized_children {
                    match child {
                        Self::Or(grandchildren) => flattened.extend(grandchildren),
                        other => flattened.push(other),
                    }
                }

                Self::or(flattened)
            }
            Self::Not(child) => {
                let optimized_child = child.optimize();

                // Apply De Morgan's laws for double negation
                match optimized_child {
                    Self::Not(grandchild) => *grandchild, // NOT NOT A = A
                    other => Self::not(other),
                }
            }
            leaf @ (Self::TantivyLeaf { .. } | Self::PostgresLeaf { .. }) => leaf,
        }
    }
}

/// A Tantivy query that executes a compositional expression tree
/// This is the main entry point for executing compositional queries
#[derive(Clone)]
pub struct CompositionQuery {
    /// The root of the expression tree
    root: CompositionNode,
    /// PostgreSQL context for expression evaluation (set during execution)
    postgres_context: Option<Arc<PostgresContext>>,
}

/// PostgreSQL context needed for expression evaluation
/// This contains the runtime information needed to evaluate PostgreSQL expressions
#[derive(Debug)]
pub struct PostgresContext {
    /// PostgreSQL planstate for expression initialization
    pub planstate: *mut pg_sys::PlanState,
    /// PostgreSQL expression context for evaluation
    pub expr_context: *mut pg_sys::ExprContext,
    /// Heap relation for tuple access
    pub heap_relation: pg_sys::Relation,
    /// Heap relation OID for tuple access
    pub heap_relation_oid: pg_sys::Oid,
}

// Safety: PostgresContext is only used within PostgreSQL's single-threaded execution context
unsafe impl Send for PostgresContext {}
unsafe impl Sync for PostgresContext {}

impl std::fmt::Debug for CompositionQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositionQuery")
            .field("root", &self.root)
            .field("has_postgres_context", &self.postgres_context.is_some())
            .finish()
    }
}

impl CompositionQuery {
    /// Create a new composition query with the given root node
    pub fn new(root: CompositionNode) -> Self {
        Self {
            root: root.optimize(), // Optimize the tree during construction
            postgres_context: None,
        }
    }

    /// Set the PostgreSQL context for expression evaluation
    /// This should be called during query setup in the custom scan
    pub fn with_postgres_context(mut self, context: PostgresContext) -> Self {
        self.postgres_context = Some(Arc::new(context));
        self
    }

    /// Get the root node of the expression tree
    pub fn root(&self) -> &CompositionNode {
        &self.root
    }

    /// Check if this query needs PostgreSQL expression evaluation
    pub fn needs_postgres_evaluation(&self) -> bool {
        self.root.has_postgres_leaves()
    }

    /// Check if this query has Tantivy components
    pub fn has_tantivy_components(&self) -> bool {
        self.root.has_tantivy_leaves()
    }

    /// Get all fields that need to be extracted for PostgreSQL evaluation
    pub fn get_referenced_fields(&self) -> Vec<FieldName> {
        self.root.get_referenced_fields()
    }
}

impl Query for CompositionQuery {
    fn weight(&self, enable_scoring: EnableScoring) -> tantivy::Result<Box<dyn Weight>> {
        Ok(Box::new(CompositionWeight::new(
            self.root.clone(),
            self.postgres_context.clone(),
            enable_scoring,
        )))
    }
}

/// Weight implementation for compositional queries
/// This handles the creation of scorers for the expression tree
struct CompositionWeight {
    root: CompositionNode,
    postgres_context: Option<Arc<PostgresContext>>,
    enable_scoring_flag: bool,
}

impl CompositionWeight {
    fn new(
        root: CompositionNode,
        postgres_context: Option<Arc<PostgresContext>>,
        enable_scoring: EnableScoring,
    ) -> Self {
        let enable_scoring_flag = matches!(enable_scoring, EnableScoring::Enabled { .. });
        Self {
            root,
            postgres_context,
            enable_scoring_flag,
        }
    }
}

impl Weight for CompositionWeight {
    fn scorer(&self, reader: &SegmentReader, boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
        let evaluator = NodeEvaluator::new(
            &self.root,
            reader,
            self.postgres_context.clone(),
            self.enable_scoring_flag,
            boost,
        )?;

        Ok(Box::new(CompositionScorer::new(
            evaluator,
            reader.max_doc(),
        )))
    }

    fn explain(&self, reader: &SegmentReader, doc: DocId) -> tantivy::Result<Explanation> {
        // Create a basic explanation for the compositional query
        let mut explanation = Explanation::new("CompositionQuery", 1.0);
        explanation.add_detail(Explanation::new("doc", doc as f32));
        explanation.add_detail(Explanation::new("tree", 1.0));
        Ok(explanation)
    }
}

/// Scorer implementation for compositional queries
/// This handles the actual document iteration and scoring
pub struct CompositionScorer {
    evaluator: NodeEvaluator,
    current_doc: DocId,
    max_doc: DocId,
}

impl CompositionScorer {
    fn new(evaluator: NodeEvaluator, max_doc: DocId) -> Self {
        Self {
            evaluator,
            current_doc: 0,
            max_doc,
        }
    }
}

impl Scorer for CompositionScorer {
    fn score(&mut self) -> Score {
        self.evaluator.score(self.current_doc)
    }
}

impl DocSet for CompositionScorer {
    fn advance(&mut self) -> DocId {
        while self.current_doc < self.max_doc {
            if self.evaluator.matches(self.current_doc) {
                let doc = self.current_doc;
                self.current_doc += 1;
                return doc;
            }
            self.current_doc += 1;
        }
        tantivy::TERMINATED
    }

    fn doc(&self) -> DocId {
        self.current_doc
    }

    fn size_hint(&self) -> u32 {
        // Conservative estimate - we don't know how many documents will match
        (self.max_doc - self.current_doc).min(1000)
    }
}

/// Evaluator for individual nodes in the expression tree
/// This handles the recursive evaluation of the tree structure
enum NodeEvaluator {
    And(Vec<NodeEvaluator>),
    Or(Vec<NodeEvaluator>),
    Not(Box<NodeEvaluator>),
    TantivyLeaf(Box<dyn Scorer>),
    PostgresLeaf(PostgresEvaluator),
}

impl NodeEvaluator {
    fn new(
        node: &CompositionNode,
        reader: &SegmentReader,
        postgres_context: Option<Arc<PostgresContext>>,
        enable_scoring_flag: bool,
        boost: Score,
    ) -> tantivy::Result<Self> {
        match node {
            CompositionNode::And(children) => {
                let child_evaluators: tantivy::Result<Vec<_>> = children
                    .iter()
                    .map(|child| {
                        Self::new(
                            child,
                            reader,
                            postgres_context.clone(),
                            enable_scoring_flag,
                            boost,
                        )
                    })
                    .collect();
                Ok(Self::And(child_evaluators?))
            }
            CompositionNode::Or(children) => {
                let child_evaluators: tantivy::Result<Vec<_>> = children
                    .iter()
                    .map(|child| {
                        Self::new(
                            child,
                            reader,
                            postgres_context.clone(),
                            enable_scoring_flag,
                            boost,
                        )
                    })
                    .collect();
                Ok(Self::Or(child_evaluators?))
            }
            CompositionNode::Not(child) => {
                let child_evaluator =
                    Self::new(child, reader, postgres_context, enable_scoring_flag, boost)?;
                Ok(Self::Not(Box::new(child_evaluator)))
            }
            CompositionNode::TantivyLeaf { query_json, .. } => {
                pgrx::warning!("🔥 TantivyLeaf: Using placeholder implementation for Phase 2");

                // For Phase 2, we'll use a simple AllQuery placeholder
                // In Phase 3, we'll implement proper Tantivy query deserialization and execution
                // This requires access to QueryParser, Searcher, and index_oid which we don't have here

                use tantivy::query::AllQuery;
                let all_query = AllQuery;
                let enable_scoring = EnableScoring::disabled_from_schema(reader.schema());
                let weight = all_query.weight(enable_scoring)?;
                let scorer = weight.scorer(reader, boost)?;
                Ok(Self::TantivyLeaf(scorer))
            }
            CompositionNode::PostgresLeaf {
                expression,
                referenced_fields,
                attno_map,
            } => {
                let postgres_evaluator = PostgresEvaluator::new(
                    expression.clone(),
                    referenced_fields.clone(),
                    attno_map.clone(),
                    postgres_context,
                    reader,
                )?;
                Ok(Self::PostgresLeaf(postgres_evaluator))
            }
        }
    }

    /// Check if the given document matches this node
    fn matches(&mut self, doc: DocId) -> bool {
        match self {
            Self::And(children) => children.iter_mut().all(|child| child.matches(doc)),
            Self::Or(children) => children.iter_mut().any(|child| child.matches(doc)),
            Self::Not(child) => !child.matches(doc),
            Self::TantivyLeaf(scorer) => {
                // Advance the scorer to the target document
                loop {
                    let current_doc = scorer.doc();
                    if current_doc == doc {
                        return true;
                    } else if current_doc > doc {
                        return false;
                    } else {
                        let next_doc = scorer.advance();
                        if next_doc == tantivy::TERMINATED || next_doc > doc {
                            return false;
                        }
                    }
                }
            }
            Self::PostgresLeaf(evaluator) => evaluator.matches(doc),
        }
    }

    /// Get the score for the given document
    fn score(&mut self, doc: DocId) -> Score {
        match self {
            Self::And(children) => {
                // For AND: return the maximum score from children that have scores
                let mut max_score = 0.0;
                for child in children {
                    if child.matches(doc) {
                        let child_score = child.score(doc);
                        if child_score > max_score {
                            max_score = child_score;
                        }
                    } else {
                        return 0.0; // AND requires all children to match
                    }
                }
                max_score
            }
            Self::Or(children) => {
                // For OR: return the maximum score from matching children
                let mut max_score = 0.0;
                let mut any_match = false;
                for child in children {
                    if child.matches(doc) {
                        any_match = true;
                        let child_score = child.score(doc);
                        if child_score > max_score {
                            max_score = child_score;
                        }
                    }
                }
                if any_match {
                    max_score
                } else {
                    0.0
                }
            }
            Self::Not(child) => {
                // NOT nodes don't contribute to scoring, just filter
                if child.matches(doc) {
                    0.0
                } else {
                    1.0
                }
            }
            Self::TantivyLeaf(scorer) => {
                // Check if the document matches first, then get score
                // We need to avoid borrowing conflicts by checking matches separately
                let matches = loop {
                    let current_doc = scorer.doc();
                    if current_doc == doc {
                        break true;
                    } else if current_doc > doc {
                        break false;
                    } else {
                        let next_doc = scorer.advance();
                        if next_doc == tantivy::TERMINATED || next_doc > doc {
                            break false;
                        }
                    }
                };

                if matches {
                    scorer.score()
                } else {
                    0.0
                }
            }
            Self::PostgresLeaf(_) => {
                // PostgreSQL leaves don't contribute to scoring
                if self.matches(doc) {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}

/// Evaluator for PostgreSQL leaf nodes
/// This handles the evaluation of PostgreSQL expressions against heap tuples
struct PostgresEvaluator {
    expression: String,
    referenced_fields: Vec<FieldName>,
    attno_map: HashMap<pg_sys::AttrNumber, FieldName>,
    postgres_context: Option<Arc<PostgresContext>>,
    reader: SegmentReader,
    // TODO: Add expression state and other PostgreSQL-specific state
}

impl PostgresEvaluator {
    fn new(
        expression: String,
        referenced_fields: Vec<FieldName>,
        attno_map: HashMap<pg_sys::AttrNumber, FieldName>,
        postgres_context: Option<Arc<PostgresContext>>,
        reader: &SegmentReader,
    ) -> tantivy::Result<Self> {
        Ok(Self {
            expression,
            referenced_fields,
            attno_map,
            postgres_context,
            reader: reader.clone(),
        })
    }

    fn matches(&self, doc: DocId) -> bool {
        // For Phase 2, we'll use a simple placeholder implementation
        // In Phase 3, we'll implement proper PostgreSQL expression evaluation using the callback system
        pgrx::warning!(
            "🔥 PostgresEvaluator: Using placeholder implementation for Phase 2: {}",
            &self.expression[..std::cmp::min(50, self.expression.len())]
        );

        // For now, let's implement some basic pattern matching for demo purposes
        if self.expression.contains("IS NULL") {
            // Simple IS NULL test - for demo purposes, return false (no nulls)
            false
        } else if self.expression.contains("IS NOT NULL") {
            // Simple IS NOT NULL test - for demo purposes, return true (all values exist)
            true
        } else if self.expression.contains("=") {
            // Simple equality test - for demo purposes, return true for half the documents
            doc % 2 == 0
        } else {
            // Unknown expression - return false as safe default
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composition_node_creation() {
        let tantivy_leaf = CompositionNode::tantivy_leaf(SearchQueryInput::All, true);

        let postgres_leaf = CompositionNode::postgres_leaf(
            "category_name = 'Electronics'".to_string(),
            vec![FieldName::from("category_name")],
            HashMap::default(),
        );

        let and_node = CompositionNode::and(vec![tantivy_leaf.clone(), postgres_leaf.clone()]);
        let or_node = CompositionNode::or(vec![tantivy_leaf, postgres_leaf]);

        assert!(matches!(and_node, CompositionNode::And(_)));
        assert!(matches!(or_node, CompositionNode::Or(_)));
    }

    #[test]
    fn test_tree_optimization() {
        // Test flattening of nested AND nodes
        let leaf1 = CompositionNode::tantivy_leaf(SearchQueryInput::All, true);
        let leaf2 = CompositionNode::tantivy_leaf(SearchQueryInput::All, true);
        let leaf3 = CompositionNode::tantivy_leaf(SearchQueryInput::All, true);

        let inner_and = CompositionNode::and(vec![leaf1, leaf2]);
        let outer_and = CompositionNode::and(vec![inner_and, leaf3]);

        let optimized = outer_and.optimize();

        if let CompositionNode::And(children) = optimized {
            assert_eq!(children.len(), 3); // Should be flattened
        } else {
            panic!("Expected AND node after optimization");
        }
    }

    #[test]
    fn test_double_negation_elimination() {
        let leaf = CompositionNode::tantivy_leaf(SearchQueryInput::All, true);
        let not_not_leaf = CompositionNode::not(CompositionNode::not(leaf.clone()));

        let optimized = not_not_leaf.optimize();

        // Should eliminate double negation
        assert!(matches!(optimized, CompositionNode::TantivyLeaf { .. }));
    }

    #[test]
    fn test_referenced_fields_collection() {
        let postgres_leaf1 = CompositionNode::postgres_leaf(
            "category_name = 'Electronics'".to_string(),
            vec![FieldName::from("category_name")],
            HashMap::default(),
        );

        let postgres_leaf2 = CompositionNode::postgres_leaf(
            "price > 100".to_string(),
            vec![FieldName::from("price")],
            HashMap::default(),
        );

        let and_node = CompositionNode::and(vec![postgres_leaf1, postgres_leaf2]);
        let fields = and_node.get_referenced_fields();

        assert_eq!(fields.len(), 2);
        assert!(fields.contains(&FieldName::from("category_name")));
        assert!(fields.contains(&FieldName::from("price")));
    }
}

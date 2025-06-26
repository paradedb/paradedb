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

use crate::api::{FieldName, HashMap};
use crate::debug_log;
use crate::nodecast;
use crate::postgres::customscan::builders::custom_path::RestrictInfoType;
use crate::postgres::customscan::operator_oid;
use crate::postgres::customscan::pdbscan::projections::score::score_funcoid;
use crate::postgres::customscan::pdbscan::pushdown::{
    is_complex, try_external_filter, try_pushdown, PushdownField,
};
use crate::query::SearchQueryInput;
// use crate::query::external_filter::ExternalFilterQuery;
use crate::schema::SearchIndexSchema;
use pg_sys::BoolExprType;
use pgrx::{datum::FromDatum, pg_sys, IntoDatum, PgList};

use std::ops::Bound;
use tantivy::schema::OwnedValue;

/// Comparison operators for field-based filtering
#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

/// Field values for comparison operations
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Integer(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone)]
pub enum Qual {
    All,
    ExternalVar,
    ExternalExpr,
    NonIndexedExpr,
    OpExpr {
        lhs: *mut pg_sys::Node,
        opno: pg_sys::Oid,
        val: *mut pg_sys::Const,
    },
    Expr {
        node: *mut pg_sys::Node,
        expr_state: *mut pg_sys::ExprState,
    },
    PushdownExpr {
        funcexpr: *mut pg_sys::FuncExpr,
    },
    /// Field-based comparison operations that can be evaluated in Tantivy
    /// These replace PostgresEval with specific, evaluatable operations
    FieldComparison {
        field: FieldName,
        operator: ComparisonOperator,
        value: FieldValue,
    },
    /// Field NULL test operations
    FieldNullTest {
        field: FieldName,
        is_null: bool, // true for IS NULL, false for IS NOT NULL
    },
    /// DEPRECATED: PostgreSQL expression that needs to be evaluated during query execution
    /// This is kept for compatibility while migrating to field-based operations
    PostgresEval {
        expr: *mut pg_sys::Expr,
        attno_map: HashMap<pg_sys::AttrNumber, FieldName>,
    },
    /// Represents a SQL equality comparison: `bool_field = TRUE`
    /// - NULL values are excluded (NULL is not equal to TRUE)
    /// - Used for direct field reference equality comparisons
    /// - Negation transforms to PushdownVarEqFalse without including NULLs
    PushdownVarEqTrue {
        field: PushdownField,
    },
    /// Represents a SQL equality comparison: `bool_field = FALSE`
    /// - NULL values are excluded (NULL is not equal to FALSE)
    /// - Used for direct field reference equality comparisons
    /// - Negation transforms to PushdownVarEqTrue without including NULLs
    PushdownVarEqFalse {
        field: PushdownField,
    },
    /// Represents a SQL IS operator: `bool_field IS TRUE`
    /// - NULL values are excluded (NULL is not TRUE)
    /// - Different from equality in negation semantics:
    ///   IS NOT TRUE includes both FALSE and NULL values
    PushdownVarIsTrue {
        field: PushdownField,
    },
    /// Represents a SQL IS operator: `bool_field IS FALSE`
    /// - NULL values are excluded (NULL is not FALSE)
    /// - Different from equality in negation semantics:
    ///   IS NOT FALSE includes both TRUE and NULL values
    PushdownVarIsFalse {
        field: PushdownField,
    },
    PushdownIsNotNull {
        field: PushdownField,
    },
    ScoreExpr {
        opoid: pg_sys::Oid,
        value: *mut pg_sys::Node,
    },
    And(Vec<Qual>),
    Or(Vec<Qual>),
    Not(Box<Qual>),
}

impl Qual {
    pub fn contains_all(&self) -> bool {
        match self {
            Qual::All => true,
            Qual::ExternalVar => false,
            Qual::ExternalExpr => false,
            Qual::NonIndexedExpr => false,
            Qual::OpExpr { .. } => false,
            Qual::Expr { .. } => false,
            Qual::PushdownExpr { .. } => false,
            Qual::PostgresEval { .. } => false,
            Qual::PushdownVarEqTrue { .. } => false,
            Qual::PushdownVarEqFalse { .. } => false,
            Qual::PushdownVarIsTrue { .. } => false,
            Qual::PushdownVarIsFalse { .. } => false,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => false,
            Qual::And(quals) => quals.iter().any(|q| q.contains_all()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_all()),
            Qual::Not(qual) => qual.contains_all(),
            Qual::FieldComparison { .. } => false,
            Qual::FieldNullTest { .. } => false,
        }
    }

    pub fn contains_external_var(&self) -> bool {
        match self {
            Qual::All => false,
            Qual::ExternalVar => true,
            Qual::ExternalExpr => true,
            Qual::NonIndexedExpr => true,
            Qual::OpExpr { .. } => false,
            Qual::Expr { .. } => false,
            Qual::PushdownExpr { .. } => false,
            Qual::PostgresEval { .. } => false,
            Qual::PushdownVarEqTrue { .. } => false,
            Qual::PushdownVarEqFalse { .. } => false,
            Qual::PushdownVarIsTrue { .. } => false,
            Qual::PushdownVarIsFalse { .. } => false,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => false,
            Qual::And(quals) => quals.iter().any(|q| q.contains_external_var()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_external_var()),
            Qual::Not(qual) => qual.contains_external_var(),
            Qual::FieldComparison { .. } => false,
            Qual::FieldNullTest { .. } => false,
        }
    }

    pub unsafe fn contains_exec_param(&self) -> bool {
        match self {
            Qual::All => false,
            Qual::ExternalVar => false,
            Qual::ExternalExpr => false,
            Qual::NonIndexedExpr => false,
            Qual::OpExpr { .. } => false,
            Qual::Expr { node, .. } => contains_exec_param(*node),
            Qual::PushdownExpr { .. } => false,
            Qual::PostgresEval { expr, .. } => contains_exec_param((*expr).cast()),
            Qual::PushdownVarEqTrue { .. } => false,
            Qual::PushdownVarEqFalse { .. } => false,
            Qual::PushdownVarIsTrue { .. } => false,
            Qual::PushdownVarIsFalse { .. } => false,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => false,
            Qual::And(quals) => quals.iter().any(|q| q.contains_exec_param()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_exec_param()),
            Qual::Not(qual) => qual.contains_exec_param(),
            Qual::FieldComparison { .. } => false,
            Qual::FieldNullTest { .. } => false,
        }
    }

    pub fn contains_exprs(&self) -> bool {
        match self {
            Qual::All => false,
            Qual::ExternalVar => false,
            Qual::ExternalExpr => false,
            Qual::NonIndexedExpr => false,
            Qual::OpExpr { .. } => false,
            Qual::Expr { .. } => true,
            Qual::PushdownExpr { .. } => false,
            Qual::PostgresEval { .. } => true,
            Qual::PushdownVarEqTrue { .. } => true,
            Qual::PushdownVarEqFalse { .. } => true,
            Qual::PushdownVarIsTrue { .. } => true,
            Qual::PushdownVarIsFalse { .. } => true,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => false,
            Qual::And(quals) => quals.iter().any(|q| q.contains_exprs()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_exprs()),
            Qual::Not(qual) => qual.contains_exprs(),
            Qual::FieldComparison { .. } => false,
            Qual::FieldNullTest { .. } => false,
        }
    }

    pub fn contains_score_exprs(&self) -> bool {
        match self {
            Qual::All => false,
            Qual::ExternalVar => false,
            Qual::ExternalExpr => false,
            Qual::NonIndexedExpr => false,
            Qual::OpExpr { .. } => false,
            Qual::Expr { .. } => false,
            Qual::PushdownExpr { .. } => false,
            Qual::PostgresEval { .. } => false,
            Qual::PushdownVarEqTrue { .. } => false,
            Qual::PushdownVarEqFalse { .. } => false,
            Qual::PushdownVarIsTrue { .. } => false,
            Qual::PushdownVarIsFalse { .. } => false,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => true,
            Qual::And(quals) => quals.iter().any(|q| q.contains_score_exprs()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_score_exprs()),
            Qual::Not(qual) => qual.contains_score_exprs(),
            Qual::FieldComparison { .. } => false,
            Qual::FieldNullTest { .. } => false,
        }
    }

    pub fn collect_exprs<'a>(&'a mut self, exprs: &mut Vec<&'a mut Qual>) {
        match self {
            Qual::Expr { .. } => exprs.push(self),
            Qual::PostgresEval { .. } => exprs.push(self),
            Qual::And(quals) => quals.iter_mut().for_each(|q| q.collect_exprs(exprs)),
            Qual::Or(quals) => quals.iter_mut().for_each(|q| q.collect_exprs(exprs)),
            Qual::Not(qual) => qual.collect_exprs(exprs),
            _ => {}
        }
    }

    pub fn contains_postgres_eval(&self) -> bool {
        match self {
            Qual::PostgresEval { .. } => true,
            Qual::And(quals) | Qual::Or(quals) => {
                quals.iter().any(|q| q.contains_postgres_eval())
            }
            Qual::Not(qual) => qual.contains_postgres_eval(),
            _ => false,
        }
    }
}

/// Parse a mixed expression tree that contains both indexed (with @@@) and non-indexed predicates
/// Returns the indexed query part, or None if no indexed predicates are found
unsafe fn parse_mixed_expression_tree(
    expr: *mut pg_sys::Expr,
    attno_map: &HashMap<pg_sys::AttrNumber, FieldName>,
) -> Option<SearchQueryInput> {
    if expr.is_null() {
        return None;
    }

    debug_log!("🔥 parse_mixed_expression_tree: examining expression node");

    match (*expr).type_ {
        pg_sys::NodeTag::T_BoolExpr => {
            let bool_expr = expr.cast::<pg_sys::BoolExpr>();
            let bool_type = (*bool_expr).boolop;
            
            debug_log!("🔥 Found BoolExpr with type: {:?}", bool_type);

            match bool_type {
                pg_sys::BoolExprType::AND_EXPR => {
                    // For AND expressions, we need to separate indexed and non-indexed parts
                    let (indexed_parts, _non_indexed_parts) = extract_indexed_parts_recursive(expr, attno_map);
                    
                    if indexed_parts.is_empty() {
                        debug_log!("🔥 No indexed parts found in AND expression");
                        return None;
                    }

                    // Convert indexed parts to SearchQueryInput
                    if indexed_parts.len() == 1 {
                        Some(indexed_parts.into_iter().next().unwrap())
                    } else {
                        // Multiple indexed parts - combine with AND
                        Some(SearchQueryInput::Boolean {
                            must: indexed_parts,
                            should: vec![],
                            must_not: vec![],
                        })
                    }
                }
                pg_sys::BoolExprType::OR_EXPR => {
                    // For OR expressions, we need to handle mixed indexed/non-indexed parts more carefully
                    let (indexed_parts, _non_indexed_parts) = extract_indexed_parts_recursive(expr, attno_map);
                    
                    debug_log!("🔥 OR expression analysis: {} indexed parts found", indexed_parts.len());
                    
                    if !indexed_parts.is_empty() {
                        // We have indexed parts - create OR query with them
                        if indexed_parts.len() == 1 {
                            debug_log!("🔥 Single indexed part found in OR expression");
                            Some(indexed_parts.into_iter().next().unwrap())
                        } else {
                            debug_log!("🔥 Multiple indexed parts found, creating OR query");
                            // Multiple indexed parts - combine with OR
                            Some(SearchQueryInput::Boolean {
                                must: vec![],
                                should: indexed_parts,
                                must_not: vec![],
                            })
                        }
                    } else {
                        debug_log!("🔥 No indexed parts found in OR expression");
                        None
                    }
                }
                _ => {
                    debug_log!("🔥 Unsupported BoolExpr type: {:?}", bool_type);
                    None
                }
            }
        }
        pg_sys::NodeTag::T_OpExpr => {
            // Check if this is a @@@ operator (indexed search)
            let op_expr = expr.cast::<pg_sys::OpExpr>();
            let opno = (*op_expr).opno;
            
            // Check if this is the @@@ operator
            if is_search_operator(opno) {
                debug_log!("🔥 Found @@@ operator, creating indexed query");
                // This is an indexed predicate - convert it to SearchQueryInput
                match create_search_query_from_opexpr(op_expr, attno_map) {
                    Some(query) => {
                        debug_log!("🔥 Successfully created indexed query: {:?}", query);
                        Some(query)
                    }
                    None => {
                        debug_log!("🔥 Failed to create indexed query from @@@ operator");
                        None
                    }
                }
            } else {
                debug_log!("🔥 OpExpr is not a @@@ operator (opno: {})", opno);
                None
            }
        }
        _ => {
            debug_log!("🔥 Unsupported expression type: {:?}", (*expr).type_);
            None
        }
    }
}

/// Extract indexed parts from a complex expression tree
/// Returns (indexed_parts, non_indexed_parts)
unsafe fn extract_indexed_parts_recursive(
    expr: *mut pg_sys::Expr,
    attno_map: &HashMap<pg_sys::AttrNumber, FieldName>,
) -> (Vec<SearchQueryInput>, Vec<String>) {
    if expr.is_null() {
        return (vec![], vec![]);
    }

    match (*expr).type_ {
        pg_sys::NodeTag::T_BoolExpr => {
            let bool_expr = expr.cast::<pg_sys::BoolExpr>();
            let bool_type = (*bool_expr).boolop;
            let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);
            let args_vec: Vec<_> = args.iter_ptr().collect();

            debug_log!("🔥 extract_indexed_parts_recursive: BoolExpr with {} args, type: {:?}", args_vec.len(), bool_type);

            match bool_type {
                pg_sys::BoolExprType::OR_EXPR => {
                    // For OR expressions, we want to extract indexed parts from each branch
                    // and create a proper OR structure
                    let mut all_indexed = Vec::new();
                    let mut all_non_indexed = Vec::new();

                    for arg in args_vec {
                        let (indexed, non_indexed) = extract_indexed_parts_recursive(arg.cast(), attno_map);
                        all_indexed.extend(indexed);
                        all_non_indexed.extend(non_indexed);
                    }

                    debug_log!("🔥 OR expression extracted: {} indexed, {} non-indexed", all_indexed.len(), all_non_indexed.len());
                    (all_indexed, all_non_indexed)
                }
                pg_sys::BoolExprType::AND_EXPR => {
                    // For AND expressions, combine all parts
                    let mut all_indexed = Vec::new();
                    let mut all_non_indexed = Vec::new();

                    for arg in args_vec {
                        let (indexed, non_indexed) = extract_indexed_parts_recursive(arg.cast(), attno_map);
                        all_indexed.extend(indexed);
                        all_non_indexed.extend(non_indexed);
                    }

                    debug_log!("🔥 AND expression extracted: {} indexed, {} non-indexed", all_indexed.len(), all_non_indexed.len());
                    (all_indexed, all_non_indexed)
                }
                _ => {
                    debug_log!("🔥 Unsupported BoolExpr type in extraction: {:?}", bool_type);
                    let expr_string = serialize_expression(expr);
                    (vec![], vec![expr_string])
                }
            }
        }
        pg_sys::NodeTag::T_OpExpr => {
            let op_expr = expr.cast::<pg_sys::OpExpr>();
            let opno = (*op_expr).opno;

            if is_search_operator(opno) {
                debug_log!("🔥 Found @@@ operator in extraction");
                // This is an indexed predicate
                if let Some(indexed_query) = create_search_query_from_opexpr(op_expr, attno_map) {
                    debug_log!("🔥 Successfully created indexed query from @@@ operator: {:?}", indexed_query);
                    (vec![indexed_query], vec![])
                } else {
                    debug_log!("🔥 Failed to create indexed query from @@@ operator");
                    (vec![], vec![])
                }
            } else {
                debug_log!("🔥 Found non-indexed OpExpr (opno: {})", opno);
                // This is a non-indexed predicate
                let expr_string = serialize_expression(expr);
                (vec![], vec![expr_string])
            }
        }
        _ => {
            debug_log!("🔥 Found other expression type: {:?}", (*expr).type_);
            // Other expression types are treated as non-indexed
            let expr_string = serialize_expression(expr);
            (vec![], vec![expr_string])
        }
    }
}

impl From<&Qual> for SearchQueryInput {
    #[track_caller]
    fn from(value: &Qual) -> Self {
        match value {
            Qual::All => SearchQueryInput::All,
            Qual::ExternalVar => SearchQueryInput::All,
            Qual::ExternalExpr => SearchQueryInput::All,
            Qual::NonIndexedExpr => SearchQueryInput::All,
            Qual::OpExpr { val, .. } => unsafe {
                SearchQueryInput::from_datum((**val).constvalue, (**val).constisnull)
                    .expect("rhs of @@@ operator Qual must not be null")
            },
            Qual::Expr { node, expr_state } => SearchQueryInput::postgres_expression(*node),
            Qual::PostgresEval { expr, attno_map } => {
                debug_log!("🔥 Processing PostgresEval with mixed indexed/non-indexed predicates");
                
                // Parse the mixed expression to separate indexed and non-indexed parts
                let indexed_query = unsafe {
                    match parse_mixed_expression_tree(*expr, attno_map) {
                        Some(query) => {
                            debug_log!("🔥 Successfully extracted indexed query: {:?}", query);
                            query
                        }
                        None => {
                            // If we can't extract indexed parts, this means the expression
                            // contains only non-indexed predicates, so we use MatchAll
                            debug_log!("🔥 No indexed parts found, using MatchAll as base query");
                            SearchQueryInput::All
                        }
                    }
                };

                // Create the external filter configuration
                let filter_expression = unsafe { serialize_expression(*expr) };
                let referenced_fields = unsafe { extract_referenced_fields(*expr, attno_map) };

                // Combine the indexed query with the external filter
                // FIXME: Convert to SimpleFieldFilter approach
                SearchQueryInput::IndexedWithFilter {
                    indexed_query: Box::new(indexed_query),
                    field_filters: vec![], // Temporary placeholder
                }
            }
            Qual::PushdownExpr { funcexpr } => unsafe {
                let expr_state = pg_sys::ExecInitExpr((*funcexpr).cast(), std::ptr::null_mut());
                let expr_context = pg_sys::CreateStandaloneExprContext();
                let mut is_null = false;
                let datum = pg_sys::ExecEvalExpr(expr_state, expr_context, &mut is_null);
                pg_sys::FreeExprContext(expr_context, false);
                SearchQueryInput::from_datum(datum, is_null)
                    .expect("pushdown expression should not evaluate to NULL")
            },
            Qual::PushdownVarEqTrue { field } => SearchQueryInput::Term {
                field: Some(field.attname()),
                value: OwnedValue::Bool(true),
                is_datetime: false,
            },
            Qual::PushdownVarEqFalse { field } => SearchQueryInput::Term {
                field: Some(field.attname()),
                value: OwnedValue::Bool(false),
                is_datetime: false,
            },
            Qual::PushdownVarIsTrue { field } => SearchQueryInput::Term {
                field: Some(field.attname()),
                value: OwnedValue::Bool(true),
                is_datetime: false,
            },
            Qual::PushdownVarIsFalse { field } => SearchQueryInput::Term {
                field: Some(field.attname()),
                value: OwnedValue::Bool(false),
                is_datetime: false,
            },
            Qual::PushdownIsNotNull { field } => SearchQueryInput::Exists {
                field: field.attname(),
            },
            Qual::ScoreExpr { opoid, value } => unsafe {
                let score_value = {
                    let expr_state = pg_sys::ExecInitExpr((*value).cast(), std::ptr::null_mut());
                    let expr_context = pg_sys::CreateStandaloneExprContext();
                    let mut is_null = false;
                    let datum = pg_sys::ExecEvalExpr(expr_state, expr_context, &mut is_null);
                    pg_sys::FreeExprContext(expr_context, false);

                    match pg_sys::exprType(*value) {
                        pg_sys::FLOAT4OID => f32::from_datum(datum, is_null),
                        pg_sys::FLOAT8OID => f64::from_datum(datum, is_null).map(|f| f as f32),
                        _ => panic!("score expression should be float4 or float8"),
                    }
                }
                .expect("score expression should not evaluate to NULL");

                let mut bounds = None;
                for rhs_type in &["float4", "float8"] {
                    if opoid == &operator_oid(&format!("=(float4,{rhs_type})")) {
                        bounds = Some((Bound::Included(score_value), Bound::Included(score_value)));
                    } else if opoid == &operator_oid(&format!("<(float4,{rhs_type})")) {
                        bounds = Some((Bound::Unbounded, Bound::Excluded(score_value)));
                    } else if opoid == &operator_oid(&format!(">(float4,{rhs_type})")) {
                        bounds = Some((Bound::Excluded(score_value), Bound::Unbounded));
                    } else if opoid == &operator_oid(&format!("<=(float4,{rhs_type})")) {
                        bounds = Some((Bound::Unbounded, Bound::Included(score_value)));
                    } else if opoid == &operator_oid(&format!(">=(float4,{rhs_type})")) {
                        bounds = Some((Bound::Included(score_value), Bound::Unbounded));
                    } else if opoid == &operator_oid(&format!("<>(float4,{rhs_type})")) {
                        bounds = Some((Bound::Excluded(score_value), Bound::Excluded(score_value)));
                    }
                    if bounds.is_some() {
                        break;
                    }
                }
                if bounds.is_none() {
                    panic!("unsupported score operator: {opoid:?}");
                }
                let (lower, upper) = bounds.unwrap();

                SearchQueryInput::ScoreFilter {
                    bounds: vec![(lower, upper)],
                    query: None,
                }
            },
            Qual::And(quals) => {
                debug_log!("Qual::And processing: has_indexed={}, has_non_indexed={}, total_quals={}", 
                    quals.iter().any(|q| matches!(q, Qual::OpExpr { .. } | Qual::Or(_))), 
                    quals.iter().any(|q| matches!(q, Qual::PostgresEval { .. })), 
                    quals.len());

                let mut indexed_quals = Vec::new();
                let mut non_indexed_quals = Vec::new();

                for qual in quals {
                    match qual {
                        // These are indexed predicates
                        Qual::OpExpr { .. } | Qual::Or(_) | Qual::PushdownExpr { .. } 
                        | Qual::PushdownVarEqTrue { .. } | Qual::PushdownVarEqFalse { .. }
                        | Qual::PushdownVarIsTrue { .. } | Qual::PushdownVarIsFalse { .. }
                        | Qual::PushdownIsNotNull { .. } => {
                            indexed_quals.push(SearchQueryInput::from(qual));
                        }
                        // These are non-indexed predicates that need external filters
                        Qual::PostgresEval { .. } => {
                            non_indexed_quals.push(qual);
                        }
                        // Handle nested And/Not recursively
                        Qual::And(_) | Qual::Not(_) => {
                            // For nested structures, convert and treat as indexed for now
                            indexed_quals.push(SearchQueryInput::from(qual));
                        }
                        // Other types - treat as indexed
                        _ => {
                            indexed_quals.push(SearchQueryInput::from(qual));
                        }
                    }
                }

                if !indexed_quals.is_empty() && !non_indexed_quals.is_empty() {
                    debug_log!("Creating combined external filter for mixed predicates");
                    debug_log!("Indexed quals: {}, Non-indexed quals: {}", indexed_quals.len(), non_indexed_quals.len());
                    
                    // Create the base indexed query
                    let indexed_query = if indexed_quals.len() == 1 {
                        indexed_quals.into_iter().next().unwrap()
                    } else {
                        SearchQueryInput::Boolean {
                            must: indexed_quals,
                            should: vec![],
                            must_not: vec![],
                        }
                    };

                    // Combine all non-indexed quals into a single external filter
                    let mut all_referenced_fields = Vec::new();
                    let mut filter_expressions = Vec::new();
                    
                    for non_indexed_qual in non_indexed_quals {
                        if let Qual::PostgresEval { expr, attno_map } = non_indexed_qual {
                            let filter_expression = unsafe { serialize_expression(*expr) };
                            let referenced_fields = unsafe { extract_referenced_fields(*expr, attno_map) };
                            
                            filter_expressions.push(filter_expression);
                            all_referenced_fields.extend(referenced_fields);
                        }
                    }

                    // Remove duplicate referenced fields
                    all_referenced_fields.sort();
                    all_referenced_fields.dedup();

                    // Combine all filter expressions with AND
                    let combined_filter = if filter_expressions.len() == 1 {
                        filter_expressions.into_iter().next().unwrap()
                    } else {
                        // Create a combined AND expression
                        format!("({})", filter_expressions.join(" AND "))
                    };

                    // Create a single IndexedWithFilter with the combined external filter
                    // FIXME: Convert to SimpleFieldFilter approach
                    SearchQueryInput::IndexedWithFilter {
                        indexed_query: Box::new(indexed_query),
                        field_filters: vec![], // Temporary placeholder
                    }
                } else if !indexed_quals.is_empty() {
                    // Only indexed quals
                    SearchQueryInput::Boolean {
                        must: indexed_quals,
                        should: vec![],
                        must_not: vec![],
                    }
                } else if !non_indexed_quals.is_empty() {
                    // Only non-indexed quals - create a single external filter with All query
                    let mut all_referenced_fields = Vec::new();
                    let mut filter_expressions = Vec::new();
                    
                    for non_indexed_qual in non_indexed_quals {
                        if let Qual::PostgresEval { expr, attno_map } = non_indexed_qual {
                            let filter_expression = unsafe { serialize_expression(*expr) };
                            let referenced_fields = unsafe { extract_referenced_fields(*expr, attno_map) };
                            
                            filter_expressions.push(filter_expression);
                            all_referenced_fields.extend(referenced_fields);
                        }
                    }

                    // Remove duplicate referenced fields
                    all_referenced_fields.sort();
                    all_referenced_fields.dedup();

                    // Combine all filter expressions with AND
                    let combined_filter = if filter_expressions.len() == 1 {
                        filter_expressions.into_iter().next().unwrap()
                    } else {
                        // Create a combined AND expression
                        format!("({})", filter_expressions.join(" AND "))
                    };

                    // Create a single IndexedWithFilter with All query and combined external filter
                    // FIXME: Convert to SimpleFieldFilter approach
                    SearchQueryInput::IndexedWithFilter {
                        indexed_query: Box::new(SearchQueryInput::All),
                        field_filters: vec![], // Temporary placeholder
                    }
                } else {
                    // No quals - shouldn't happen but handle gracefully
                    SearchQueryInput::All
                }
            }

            Qual::Or(quals) => {
                debug_log!("🔥 Processing OR with {} branches", quals.len());
                
                let mut or_branches = Vec::new();

                for qual in quals {
                    match qual {
                        // Pure indexed predicates - add directly as Tantivy queries
                        Qual::OpExpr { .. } | Qual::PushdownExpr { .. } 
                        | Qual::PushdownVarEqTrue { .. } | Qual::PushdownVarEqFalse { .. }
                        | Qual::PushdownVarIsTrue { .. } | Qual::PushdownVarIsFalse { .. }
                        | Qual::PushdownIsNotNull { .. } => {
                            debug_log!("🔥 OR branch: pure indexed predicate");
                            let indexed_query = SearchQueryInput::from(qual);
                            or_branches.push(indexed_query);
                        }
                        
                        // Nested OR/AND structures containing only indexed predicates
                        Qual::Or(_) | Qual::And(_) => {
                            // Check if this nested structure contains any non-indexed predicates
                            if qual.contains_postgres_eval() {
                                debug_log!("🔥 OR branch: nested structure with non-indexed predicates");
                                // This nested structure contains non-indexed predicates,
                                // so we need to convert it appropriately
                                let branch_query = SearchQueryInput::from(qual);
                                or_branches.push(branch_query);
                            } else {
                                debug_log!("🔥 OR branch: pure indexed nested structure");
                                // Pure indexed nested structure
                                let indexed_query = SearchQueryInput::from(qual);
                                or_branches.push(indexed_query);
                            }
                        }
                        
                        // Non-indexed predicates - wrap in IndexedWithFilter with All query
                        Qual::PostgresEval { expr, attno_map } => {
                            debug_log!("🔥 OR branch: non-indexed predicate - wrapping in IndexedWithFilter");
                            
                            let filter_expression = unsafe { serialize_expression(*expr) };
                            let referenced_fields = unsafe { extract_referenced_fields(*expr, attno_map) };
                            
                            // For non-indexed predicates in OR, we use All query as the base
                            // This means "return all documents, then filter by the non-indexed predicate"
                            or_branches.push(SearchQueryInput::IndexedWithFilter {
                                indexed_query: Box::new(SearchQueryInput::All),
                                                        field_filters: vec![],
                            });
                        }
                        
                        // Other special cases
                        Qual::NonIndexedExpr => {
                            debug_log!("🔥 OR branch: non-indexed expression - wrapping in IndexedWithFilter");
                            // For non-indexed expressions, create an IndexedWithFilter with All
                            or_branches.push(                        // FIXME: Convert to SimpleFieldFilter approach
                        SearchQueryInput::IndexedWithFilter {
                            indexed_query: Box::new(SearchQueryInput::All),
                            field_filters: vec![], // Temporary placeholder
                        });
                        }
                        
                        // Negation and other complex cases
                        Qual::Not(_) => {
                            debug_log!("🔥 OR branch: negation");
                            let branch_query = SearchQueryInput::from(qual);
                            or_branches.push(branch_query);
                        }
                        
                        // Score expressions and other edge cases
                        _ => {
                            debug_log!("🔥 OR branch: other type - treating as indexed");
                            let branch_query = SearchQueryInput::from(qual);
                            or_branches.push(branch_query);
                        }
                    }
                }

                // Create a Boolean OR query with all branches
                SearchQueryInput::Boolean {
                    must: Default::default(),
                    should: or_branches,
                    must_not: Default::default(),
                }
            }
            Qual::Not(qual) => {
                // Special handling for boolean fields to correctly handle NULL values
                match qual.as_ref() {
                    // If we're negating a PushdownVarEqTrue, we should use PushdownVarEqFalse directly
                    // rather than using must_not, to avoid including NULLs
                    // This follows SQL standard where NOT (field = TRUE) is equivalent to (field = FALSE)
                    // and does NOT include NULL values
                    Qual::PushdownVarEqTrue { field } => Self::from(&Qual::PushdownVarEqFalse {
                        field: field.clone(),
                    }),
                    // Similarly, if we're negating a PushdownVarEqFalse, use PushdownVarEqTrue
                    // This follows SQL standard where NOT (field = FALSE) is equivalent to (field = TRUE)
                    // and does NOT include NULL values
                    Qual::PushdownVarEqFalse { field } => Self::from(&Qual::PushdownVarEqTrue {
                        field: field.clone(),
                    }),

                    // If the Qual represents a placeholder to another Var elsewhere in the plan,
                    // that means it's a JOIN of some kind and what we actually need to return, in its place,
                    // is "all" rather than "NOT all"
                    Qual::ExternalVar => SearchQueryInput::All,

                    // If the Qual represents a placeholder to another Expr elsewhere in the plan,
                    // that means it's a JOIN of some kind and what we actually need to return, in its place,
                    // is "all" rather than "NOT all"
                    Qual::ExternalExpr => SearchQueryInput::All,

                    // If the Qual represents a non-indexed expression, we should return "all"
                    // This is because we will be using tantivy to query, and we want to be able to
                    // use the non-indexed expression to filter the results
                    Qual::NonIndexedExpr => SearchQueryInput::All,

                    // For other types of negation, use the standard Boolean query with must_not
                    // Note that when negating an IS operator (e.g., IS NOT TRUE), PostgreSQL handles
                    // NULL values differently than when negating equality operators
                    _ => {
                        let must_not = vec![SearchQueryInput::from(qual.as_ref())];

                        SearchQueryInput::Boolean {
                            must: vec![SearchQueryInput::All],
                            should: Default::default(),
                            must_not,
                        }
                    }
                }
            }
            Qual::FieldComparison { field, operator, value } => {
                // Convert field comparison to appropriate SearchQueryInput
                // For now, convert to Term query as a placeholder
                let owned_value = match value {
                    FieldValue::Integer(i) => OwnedValue::I64(*i),
                    FieldValue::Float(f) => OwnedValue::F64(*f),
                    FieldValue::Text(s) => OwnedValue::Str(s.clone()),
                    FieldValue::Boolean(b) => OwnedValue::Bool(*b),
                    FieldValue::Null => OwnedValue::Null,
                };
                
                // For now, map all comparisons to Term queries
                // This is a simplification - proper implementation would need range queries for > < etc
                SearchQueryInput::Term {
                    field: Some(field.clone()),
                    value: owned_value,
                    is_datetime: false,
                }
            }
            Qual::FieldNullTest { field, is_null } => {
                if *is_null {
                    // IS NULL - use term query with null value
                    SearchQueryInput::Term {
                        field: Some(field.clone()),
                        value: OwnedValue::Null,
                        is_datetime: false,
                    }
                } else {
                    // IS NOT NULL - use exists query
                    SearchQueryInput::Exists {
                        field: field.clone(),
                    }
                }
            }
        }
    }
}

/// Extract quals from a node, returning the indexed quals if available
/// If `convert_external_to_special_qual` is true, then unpushable predicates will be converted to Qual::ExternalExpr
/// `uses_tantivy_to_query` will be set to true if we decide to use tantivy to query
#[allow(clippy::too_many_arguments)]
pub unsafe fn extract_quals(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
    convert_external_to_special_qual: bool,
    uses_tantivy_to_query: &mut bool,
) -> Option<Qual> {
    extract_quals_internal(
        root,
        rti,
        node,
        pdbopoid,
        ri_type,
        schema,
        convert_external_to_special_qual,
        false, // extract_all_quals_even_non_indexed
        uses_tantivy_to_query,
    )
    .0
}

/// Extract quals from a node, returning the all quals (indexed and non-indexed) if available
/// If `convert_external_to_special_qual` is true, then unpushable predicates will be converted to Qual::ExternalExpr
/// `uses_tantivy_to_query` will be set to true if we decide to use tantivy to query
#[allow(clippy::too_many_arguments)]
pub unsafe fn extract_quals_with_non_indexed(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
    convert_external_to_special_qual: bool,
    uses_tantivy_to_query: &mut bool,
) -> (Option<Qual>, Option<Qual>) {
    extract_quals_internal(
        root,
        rti,
        node,
        pdbopoid,
        ri_type,
        schema,
        convert_external_to_special_qual,
        true, // extract_all_quals_even_non_indexed
        uses_tantivy_to_query,
    )
}

#[allow(clippy::too_many_arguments)]
unsafe fn extract_quals_internal(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
    convert_external_to_special_qual: bool,
    extract_all_quals_even_non_indexed: bool,
    uses_tantivy_to_query: &mut bool,
) -> (Option<Qual>, Option<Qual>) {
    if node.is_null() {
        return (None, None);
    }

    match (*node).type_ {
        pg_sys::NodeTag::T_List => {
            let indexed_quals = list_internal(
                root,
                rti,
                node.cast(),
                pdbopoid,
                ri_type,
                schema,
                convert_external_to_special_qual,
                false, // Don't extract non-indexed for indexed quals
                uses_tantivy_to_query,
            );

            let all_quals = if extract_all_quals_even_non_indexed {
                list_internal(
                    root,
                    rti,
                    node.cast(),
                    pdbopoid,
                    ri_type,
                    schema,
                    convert_external_to_special_qual,
                    true, // Extract all quals including non-indexed
                    uses_tantivy_to_query,
                )
            } else {
                None
            };

            let indexed_result = if let Some(mut quals) = indexed_quals {
                if quals.is_empty() {
                    None
                } else if quals.len() == 1 {
                    quals.pop()
                } else {
                    Some(Qual::And(quals))
                }
            } else {
                None
            };

            let all_result = if let Some(mut quals) = all_quals {
                if quals.is_empty() {
                    None
                } else if quals.len() == 1 {
                    quals.pop()
                } else {
                    Some(Qual::And(quals))
                }
            } else {
                None
            };

            (indexed_result, all_result)
        }

        pg_sys::NodeTag::T_RestrictInfo => {
            let ri = if let Some(ri) = nodecast!(RestrictInfo, T_RestrictInfo, node) {
                ri
            } else {
                return (None, None);
            };
            let clause = if !(*ri).orclause.is_null() {
                (*ri).orclause
            } else {
                (*ri).clause
            };
            extract_quals_internal(
                root,
                rti,
                clause.cast(),
                pdbopoid,
                ri_type,
                schema,
                convert_external_to_special_qual,
                extract_all_quals_even_non_indexed,
                uses_tantivy_to_query,
            )
        }

        pg_sys::NodeTag::T_OpExpr => {
            let indexed_qual = opexpr_internal(
                root,
                rti,
                node,
                pdbopoid,
                ri_type,
                schema,
                convert_external_to_special_qual,
                false, // Don't extract non-indexed for indexed quals
                uses_tantivy_to_query,
            );

            let all_qual = if extract_all_quals_even_non_indexed {
                opexpr_internal(
                    root,
                    rti,
                    node,
                    pdbopoid,
                    ri_type,
                    schema,
                    convert_external_to_special_qual,
                    true, // Extract all quals including non-indexed
                    uses_tantivy_to_query,
                )
            } else {
                None
            };

            (indexed_qual, all_qual)
        }

        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = if let Some(boolexpr) = nodecast!(BoolExpr, T_BoolExpr, node) {
                boolexpr
            } else {
                return (None, None);
            };

            // Debug: log what BoolExpr we're processing
            let node_string = pg_sys::nodeToString(node.cast::<core::ffi::c_void>());
            let rust_string = std::ffi::CStr::from_ptr(node_string)
                .to_string_lossy()
                .into_owned();
            pg_sys::pfree(node_string.cast());
            debug_log!(
                "🔥 Processing BoolExpr: {} (op: {:?})",
                rust_string,
                (*boolexpr).boolop
            );

            // For OR expressions with mixed indexed/non-indexed predicates, we need special handling
            if (*boolexpr).boolop == pg_sys::BoolExprType::OR_EXPR
                && extract_all_quals_even_non_indexed
            {
                // Check if this is a mixed OR expression that should be handled by parse_mixed_expression_tree
                let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
                let mut has_indexed = false;
                let mut has_non_indexed = false;

                for arg in args.iter_ptr() {
                    let (indexed_qual, _) = extract_quals_internal(
                        root,
                        rti,
                        arg,
                        pdbopoid,
                        ri_type,
                        schema,
                        convert_external_to_special_qual,
                        false, // Don't recurse
                        uses_tantivy_to_query,
                    );

                    if indexed_qual.is_some() {
                        has_indexed = true;
                    } else {
                        has_non_indexed = true;
                    }
                }

                if has_indexed && has_non_indexed {
                    // This is a mixed OR expression - create a PostgresEval for proper parsing
                    if let Some(field_operation_qual) = try_create_field_operation(root, rti, node) {
                        return (None, Some(field_operation_qual));
                    }
                }
            }

            let indexed_quals = list_internal(
                root,
                rti,
                (*boolexpr).args,
                pdbopoid,
                ri_type,
                schema,
                convert_external_to_special_qual,
                false, // Don't extract non-indexed for indexed quals
                uses_tantivy_to_query,
            );

            let all_quals = if extract_all_quals_even_non_indexed {
                list_internal(
                    root,
                    rti,
                    (*boolexpr).args,
                    pdbopoid,
                    ri_type,
                    schema,
                    convert_external_to_special_qual,
                    true, // Extract all quals including non-indexed
                    uses_tantivy_to_query,
                )
            } else {
                None
            };

            let indexed_result = if let Some(mut quals) = indexed_quals {
                if quals.is_empty() {
                    None
                } else {
                    match (*boolexpr).boolop {
                        pg_sys::BoolExprType::AND_EXPR => Some(Qual::And(quals)),
                        pg_sys::BoolExprType::OR_EXPR => Some(Qual::Or(quals)),
                        pg_sys::BoolExprType::NOT_EXPR => {
                            quals.pop().map(|qual| Qual::Not(Box::new(qual)))
                        }
                        _ => panic!("unexpected `BoolExprType`: {}", (*boolexpr).boolop),
                    }
                }
            } else {
                None
            };

            let all_result = if let Some(mut quals) = all_quals {
                match (*boolexpr).boolop {
                    pg_sys::BoolExprType::AND_EXPR => Some(Qual::And(quals)),
                    pg_sys::BoolExprType::OR_EXPR => Some(Qual::Or(quals)),
                    pg_sys::BoolExprType::NOT_EXPR => {
                        quals.pop().map(|qual| Qual::Not(Box::new(qual)))
                    }
                    _ => panic!("unexpected `BoolExprType`: {}", (*boolexpr).boolop),
                }
            } else {
                None
            };

            (indexed_result, all_result)
        }

        // For all other node types, handle them the same way as before
        _ => {
            let single_qual = extract_quals_original(
                root,
                rti,
                node,
                pdbopoid,
                ri_type,
                schema,
                convert_external_to_special_qual,
                extract_all_quals_even_non_indexed,
                uses_tantivy_to_query,
            );
            (single_qual.clone(), single_qual)
        }
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn list_internal(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    list: *mut pg_sys::List,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
    convert_external_to_special_qual: bool,
    extract_all_quals_even_non_indexed: bool,
    uses_tantivy_to_query: &mut bool,
) -> Option<Vec<Qual>> {
    let list = PgList::<pg_sys::Node>::from_pg(list);
    let mut quals = Vec::new();

    for child in list.iter_ptr() {
        if extract_all_quals_even_non_indexed {
            // When extracting all quals, try both indexed and non-indexed approaches
            let (indexed_qual, all_qual) = extract_quals_internal(
                root,
                rti,
                child,
                pdbopoid,
                ri_type,
                schema,
                convert_external_to_special_qual,
                false, // Don't recurse the all_quals flag
                uses_tantivy_to_query,
            );

            // Use the indexed qual if available, otherwise use the all_qual (which includes non-indexed)
            if let Some(qual) = indexed_qual.or(all_qual) {
                quals.push(qual);
            } else {
                // If we can't extract this qual, try to create a PostgresEval
                                    if let Some(filter_qual) = try_create_field_operation(root, rti, child) {
                    quals.push(filter_qual);
                } else {
                    // If we can't create a postgres eval either, use NonIndexedExpr as fallback
                    quals.push(Qual::NonIndexedExpr);
                }
            }
        } else {
            // Regular extraction for indexed quals only
            if let Some(qual) = extract_quals_internal(
                root,
                rti,
                child,
                pdbopoid,
                ri_type,
                schema,
                convert_external_to_special_qual,
                false,
                uses_tantivy_to_query,
            )
            .0
            {
                quals.push(qual);
            } else if convert_external_to_special_qual {
                // During partial extraction, convert unpushable predicates to Qual::ExternalExpr
                // This allows us to proceed with the custom scan even if some predicates can't be pushed down
                quals.push(Qual::ExternalExpr);
            } else {
                // Normal extraction failed and we're not doing partial extraction
                return None;
            }
        }
    }

    Some(quals)
}

/// Try to create a FilterExpression qual from a PostgreSQL node


/// Recursively clean RestrictInfo wrappers from any PostgreSQL node
/// This handles nested RestrictInfo nodes in complex expressions like BoolExpr
unsafe fn clean_restrictinfo_recursively(node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    if node.is_null() {
        return node;
    }

    match (*node).type_ {
        pg_sys::NodeTag::T_RestrictInfo => {
            // Unwrap RestrictInfo and recursively clean the inner clause
            let restrict_info = node.cast::<pg_sys::RestrictInfo>();
            let inner_clause = if !(*restrict_info).orclause.is_null() {
                (*restrict_info).orclause
            } else {
                (*restrict_info).clause
            };
            clean_restrictinfo_recursively(inner_clause.cast())
        }
        pg_sys::NodeTag::T_BoolExpr => {
            // For BoolExpr, clean all arguments
            let bool_expr = node.cast::<pg_sys::BoolExpr>();
            let args_list = (*bool_expr).args;

            if !args_list.is_null() {
                let mut new_args = std::ptr::null_mut();
                let old_args = PgList::<pg_sys::Node>::from_pg(args_list);

                for arg in old_args.iter_ptr() {
                    let cleaned_arg = clean_restrictinfo_recursively(arg);
                    new_args = pg_sys::lappend(new_args, cleaned_arg.cast::<core::ffi::c_void>());
                }

                // Create a new BoolExpr with cleaned arguments
                let new_bool_expr = pg_sys::palloc0(std::mem::size_of::<pg_sys::BoolExpr>())
                    .cast::<pg_sys::BoolExpr>();
                *new_bool_expr = *bool_expr; // Copy the original
                (*new_bool_expr).args = new_args;
                new_bool_expr.cast()
            } else {
                node
            }
        }
        _ => {
            // For other node types, return as-is (we could extend this for other complex types)
            node
        }
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn opexpr_internal(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
    convert_external_to_special_qual: bool,
    extract_all_quals_even_non_indexed: bool,
    uses_tantivy_to_query: &mut bool,
) -> Option<Qual> {
    let opexpr = nodecast!(OpExpr, T_OpExpr, node)?;
    let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);

    let mut lhs = args.get_ptr(0)?;
    let rhs = args.get_ptr(1)?;

    // relabel types are essentially a cast, but for types that are directly compatible without
    // the need for a cast function.  So if the lhs of the input node is a RelabelType, just
    // keep chasing its arg until we get a final node type
    while (*lhs).type_ == pg_sys::NodeTag::T_RelabelType {
        let relabel_type = lhs as *mut pg_sys::RelabelType;
        lhs = (*relabel_type).arg as _;
    }

    match (*lhs).type_ {
        pg_sys::NodeTag::T_Var => node_opexpr(
            root,
            rti,
            pdbopoid,
            ri_type,
            schema,
            uses_tantivy_to_query,
            opexpr,
            lhs,
            rhs,
            convert_external_to_special_qual,
            extract_all_quals_even_non_indexed,
        ),

        pg_sys::NodeTag::T_FuncExpr => {
            // direct support for paradedb.score() in the WHERE clause
            let funcexpr = nodecast!(FuncExpr, T_FuncExpr, lhs)?;
            if (*funcexpr).funcid != score_funcoid() {
                return node_opexpr(
                    root,
                    rti,
                    pdbopoid,
                    ri_type,
                    schema,
                    uses_tantivy_to_query,
                    opexpr,
                    lhs,
                    rhs,
                    convert_external_to_special_qual,
                    extract_all_quals_even_non_indexed,
                );
            }

            if is_complex(rhs) {
                // Complex expressions on RHS are not supported
                return None;
            }

            Some(Qual::ScoreExpr {
                opoid: (*opexpr).opno,
                value: rhs,
            })
        }
        pg_sys::NodeTag::T_OpExpr => node_opexpr(
            root,
            rti,
            pdbopoid,
            ri_type,
            schema,
            uses_tantivy_to_query,
            opexpr,
            lhs,
            rhs,
            convert_external_to_special_qual,
            extract_all_quals_even_non_indexed,
        ),

        _ => {
            // Unhandled expression types are not supported
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn node_opexpr(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
    uses_tantivy_to_query: &mut bool,
    opexpr: *mut pg_sys::OpExpr,
    lhs: *mut pg_sys::Node,
    mut rhs: *mut pg_sys::Node,
    convert_external_to_special_qual: bool,
    extract_all_quals_even_non_indexed: bool,
) -> Option<Qual> {
    while let Some(relabel_target) = nodecast!(RelabelType, T_RelabelType, rhs) {
        rhs = (*relabel_target).arg.cast();
    }

    let rhs_as_const = nodecast!(Const, T_Const, rhs);

    let is_our_operator = (*opexpr).opno == pdbopoid;

    if rhs_as_const.is_none() {
        // the rhs expression is not a Const, so it's some kind of expression
        // that we'll need to execute during query execution, if we can

        if is_our_operator {
            if contains_var(rhs) {
                // it contains a Var, and that means some kind of sequential scan will be required
                // so indicate we can't handle this expression at all
                return None;
            } else {
                // it uses our operator, so we directly know how to handle it
                // this is the case of:  field @@@ paradedb.xxx(EXPR) where EXPR likely includes something
                // that's parameterized
                *uses_tantivy_to_query = true;
                return Some(Qual::Expr {
                    node: rhs,
                    expr_state: std::ptr::null_mut(),
                });
            }
        } else {
            // it doesn't use our operator
            if contains_var(rhs) {
                // the rhs is (or contains) a Var, which likely means its part of a join condition
                // we choose to just select everything in this situation
                return Some(Qual::ExternalVar);
            } else {
                // it doesn't use our operator.
                // we'll try to convert it into a pushdown
                let result = try_pushdown(root, rti, opexpr, schema);
                if result.is_none() {
                    if extract_all_quals_even_non_indexed {
                        // Try to create an external filter for this non-indexed predicate
                        if let Some(external_filter) = try_external_filter(root, rti, opexpr) {
                            return Some(external_filter);
                        }
                    }
                    if convert_external_to_special_qual {
                        return Some(Qual::ExternalExpr);
                    } else {
                        return None;
                    }
                }
                *uses_tantivy_to_query = true;
                return result;
            }
        }
    }

    let rhs = rhs_as_const?;
    if is_our_operator {
        // the rhs expression is a Const, so we can use it directly
        if is_node_range_table_entry(lhs, rti) {
            // the node comes from this range table entry, so we can use the full expression directly
            *uses_tantivy_to_query = true;
            Some(Qual::OpExpr {
                lhs,
                opno: (*opexpr).opno,
                val: rhs,
            })
        } else {
            // the node comes from a different range table
            if matches!(ri_type, RestrictInfoType::Join) {
                // and we're doing a join, so in this case we choose to just select everything
                Some(Qual::ExternalVar)
            } else {
                // the node comes from a different range table and we're not doing a join (how is that possible?!)
                // so we don't do anything
                None
            }
        }
    } else {
        // it doesn't use our operator.
        // we'll try to convert it into a pushdown
        let result = try_pushdown(root, rti, opexpr, schema);
        if result.is_none() {
            if extract_all_quals_even_non_indexed {
                // Try to create an external filter for this non-indexed predicate
                if let Some(external_filter) = try_external_filter(root, rti, opexpr) {
                    return Some(external_filter);
                }
            }
            if convert_external_to_special_qual {
                return Some(Qual::ExternalExpr);
            } else {
                return None;
            }
        }
        *uses_tantivy_to_query = true;
        result
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn extract_quals_original(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
    convert_external_to_special_qual: bool,
    extract_all_quals_even_non_indexed: bool,
    uses_tantivy_to_query: &mut bool,
) -> Option<Qual> {
    match (*node).type_ {
        pg_sys::NodeTag::T_Var if (*(node as *mut pg_sys::Var)).vartype == pg_sys::BOOLOID => {
            PushdownField::try_new(root, node.cast(), schema)
                .map(|field| Qual::PushdownVarEqTrue { field })
        }

        pg_sys::NodeTag::T_NullTest => {
            let nulltest = nodecast!(NullTest, T_NullTest, node)?;
            if let Some(field) = PushdownField::try_new(root, (*nulltest).arg.cast(), schema) {
                if let Some(search_field) = schema.search_field(field.attname()) {
                    if search_field.is_fast() {
                        if (*nulltest).nulltesttype == pg_sys::NullTestType::IS_NOT_NULL {
                            return Some(Qual::PushdownIsNotNull { field });
                        } else {
                            return Some(Qual::Not(Box::new(Qual::PushdownIsNotNull { field })));
                        }
                    }
                }
            }

            None
        }

        pg_sys::NodeTag::T_BooleanTest => booltest(
            root,
            rti,
            node,
            pdbopoid,
            ri_type,
            schema,
            convert_external_to_special_qual,
            uses_tantivy_to_query,
        ),

        pg_sys::NodeTag::T_Const => {
            // Handle constants that result from join clause simplification
            let const_node = nodecast!(Const, T_Const, node)?;
            if (*const_node).consttype == pg_sys::BOOLOID && !(*const_node).constisnull {
                let bool_value = bool::from_datum((*const_node).constvalue, false).unwrap_or(false);
                if bool_value {
                    Some(Qual::All)
                } else {
                    Some(Qual::Not(Box::new(Qual::All)))
                }
            } else {
                None
            }
        }

        // we don't understand this clause so we can't do anything
        _ => None,
    }
}

unsafe fn is_node_range_table_entry(node: *mut pg_sys::Node, rti: pg_sys::Index) -> bool {
    match (*node).type_ {
        pg_sys::NodeTag::T_Var => {
            let var = node.cast::<pg_sys::Var>();
            (*var).varno as i32 == rti as i32
        }
        pg_sys::NodeTag::T_FuncExpr => {
            let funcexpr = node.cast::<pg_sys::FuncExpr>();
            PgList::<pg_sys::Node>::from_pg((*funcexpr).args)
                .iter_ptr()
                .all(|arg| is_node_range_table_entry(arg, rti))
        }
        pg_sys::NodeTag::T_OpExpr => {
            let opexpr = node.cast::<pg_sys::OpExpr>();
            PgList::<pg_sys::Node>::from_pg((*opexpr).args)
                .iter_ptr()
                .all(|arg| {
                    is_node_range_table_entry(arg, rti)
                        || matches!((*arg).type_, pg_sys::NodeTag::T_Const)
                })
        }
        _ => false,
    }
}

unsafe fn contains_exec_param(root: *mut pg_sys::Node) -> bool {
    unsafe extern "C-unwind" fn walker(node: *mut pg_sys::Node, _: *mut core::ffi::c_void) -> bool {
        if let Some(param) = nodecast!(Param, T_Param, node) {
            if (*param).paramkind == pg_sys::ParamKind::PARAM_EXEC {
                return true;
            }
        }
        pg_sys::expression_tree_walker(node, Some(walker), std::ptr::null_mut())
    }

    if root.is_null() {
        return false;
    }

    walker(root, std::ptr::null_mut())
}

unsafe fn contains_var(root: *mut pg_sys::Node) -> bool {
    unsafe extern "C-unwind" fn walker(node: *mut pg_sys::Node, _: *mut core::ffi::c_void) -> bool {
        nodecast!(Var, T_Var, node).is_some()
            || pg_sys::expression_tree_walker(node, Some(walker), std::ptr::null_mut())
    }

    if root.is_null() {
        return false;
    }

    walker(root, std::ptr::null_mut())
}

#[allow(clippy::too_many_arguments)]
/// Handles SQL boolean test operators: IS TRUE, IS FALSE, IS NOT TRUE, IS NOT FALSE
///
/// According to SQL standards:
/// - IS TRUE: Only returns TRUE (not NULL)
/// - IS FALSE: Only returns FALSE (not NULL)
/// - IS NOT TRUE: Returns FALSE and NULL
/// - IS NOT FALSE: Returns TRUE and NULL
///
/// This function interprets these operators to generate the appropriate Qual variants
/// that will correctly handle NULL values in the query.
unsafe fn booltest(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
    convert_external_to_special_qual: bool,
    uses_tantivy_to_query: &mut bool,
) -> Option<Qual> {
    let booltest = nodecast!(BooleanTest, T_BooleanTest, node)?;
    let arg = (*booltest).arg;

    // We only support boolean test for simple field references (Var nodes)
    // For complex expressions, the optimizer will evaluate the condition later
    if let Some(arg_var) = nodecast!(Var, T_Var, arg) {
        if let Some(field) = PushdownField::try_new(root, arg_var, schema) {
            // It's a simple field reference, handle as specific cases
            match (*booltest).booltesttype {
                pg_sys::BoolTestType::IS_TRUE => Some(Qual::PushdownVarIsTrue { field }),
                pg_sys::BoolTestType::IS_NOT_FALSE => {
                    Some(Qual::Not(Box::new(Qual::PushdownVarIsFalse { field })))
                }
                pg_sys::BoolTestType::IS_FALSE => Some(Qual::PushdownVarIsFalse { field }),
                pg_sys::BoolTestType::IS_NOT_TRUE => {
                    Some(Qual::Not(Box::new(Qual::PushdownVarIsTrue { field })))
                }
                _ => None,
            }
        } else {
            None
        }
    } else {
        // Not a simple field reference - let the PostgreSQL executor handle it
        None
    }
}

/// Extract join-level search predicates that are relevant for snippet generation
/// This captures search predicates that reference specific fields but may not be
/// pushed down to the current scan due to join conditions.
/// Returns the entire simplified Boolean expression to preserve OR structures.
pub unsafe fn extract_join_predicates(
    root: *mut pg_sys::PlannerInfo,
    current_rti: pg_sys::Index,
    pdbopoid: pg_sys::Oid,
    schema: &SearchIndexSchema,
    base_query: &SearchQueryInput,
) -> Option<SearchQueryInput> {
    // Only look at the current relation's join clauses
    if (*root).simple_rel_array.is_null()
        || current_rti == 0
        || current_rti as usize >= (*root).simple_rel_array_size as usize
    {
        return None;
    }

    let relinfo = *(*root).simple_rel_array.add(current_rti as usize);
    if relinfo.is_null() {
        return None;
    }

    let joinlist = (*relinfo).joininfo;
    if joinlist.is_null() {
        return None;
    }

    // Check joininfo for join clauses involving our current relation
    let joininfo = PgList::<pg_sys::RestrictInfo>::from_pg(joinlist);

    for ri in joininfo.iter_ptr() {
        // Transform the join clause by replacing expressions from other relations with TRUE
        if let Some(simplified_node) =
            simplify_join_clause_for_relation((*ri).clause.cast(), current_rti)
        {
            let mut uses_tantivy_to_query = false;
            // Extract search predicates from the simplified expression
            if let Some(qual) = extract_quals(
                root,
                current_rti,
                simplified_node.cast(),
                pdbopoid,
                RestrictInfoType::BaseRelation,
                schema,
                true,
                &mut uses_tantivy_to_query,
            ) {
                if uses_tantivy_to_query {
                    // Convert qual to SearchQueryInput and return the entire expression
                    let search_input = SearchQueryInput::from(&qual);
                    // Return the entire simplified expression for scoring
                    // This preserves OR structures like (TRUE OR name:"Rowling")
                    return Some(search_input);
                }
            }
        }
    }

    None
}

/// Transform a join clause by replacing expressions from other relations with TRUE
/// Returns a new node representing the simplified expression
unsafe fn simplify_join_clause_for_relation(
    node: *mut pg_sys::Node,
    current_rti: pg_sys::Index,
) -> Option<*mut pg_sys::Node> {
    if node.is_null() {
        return None;
    }

    let input_type = (*node).type_;

    match (*node).type_ {
        pg_sys::NodeTag::T_OpExpr => {
            let opexpr = nodecast!(OpExpr, T_OpExpr, node)?;

            // Check if this operation involves our current relation
            if contains_relation_reference(node, current_rti) {
                // Keep the original expression if it involves our relation
                Some(node)
            } else if contains_any_relation_reference(node) {
                // Replace with TRUE if it only involves other relations
                create_bool_const_true()
            } else {
                // Keep non-relation expressions (constants, etc.)
                Some(node)
            }
        }

        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = nodecast!(BoolExpr, T_BoolExpr, node)?;
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
            let mut simplified_args = Vec::new();

            // Recursively simplify each argument
            for (i, arg) in args.iter_ptr().enumerate() {
                if let Some(simplified_arg) = simplify_join_clause_for_relation(arg, current_rti) {
                    simplified_args.push(simplified_arg);
                }
            }

            if simplified_args.is_empty() {
                return None;
            }

            match (*boolexpr).boolop {
                pg_sys::BoolExprType::AND_EXPR => {
                    // For AND: preserve the Boolean structure, keep TRUE values
                    // This maintains the original structure like: (TRUE AND a.age @@@ '>50')
                    match simplified_args.len() {
                        0 => None,
                        1 => Some(simplified_args[0]),
                        _ => create_bool_expr(pg_sys::BoolExprType::AND_EXPR, simplified_args),
                    }
                }
                pg_sys::BoolExprType::OR_EXPR => {
                    // For OR: preserve the Boolean structure, don't simplify even if TRUE is present
                    // This allows scoring to see search predicates like: (TRUE OR a.name @@@ 'Rowling')
                    match simplified_args.len() {
                        0 => None,
                        1 => Some(simplified_args[0]),
                        _ => create_bool_expr(pg_sys::BoolExprType::OR_EXPR, simplified_args),
                    }
                }
                pg_sys::BoolExprType::NOT_EXPR => {
                    // For NOT: apply to the single simplified argument
                    if simplified_args.len() == 1 {
                        let arg = simplified_args[0];
                        if is_bool_const_true(arg) {
                            create_bool_const_false()
                        } else {
                            create_bool_expr(pg_sys::BoolExprType::NOT_EXPR, simplified_args)
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        pg_sys::NodeTag::T_RestrictInfo => {
            let ri = nodecast!(RestrictInfo, T_RestrictInfo, node)?;
            let clause = if !(*ri).orclause.is_null() {
                (*ri).orclause
            } else {
                (*ri).clause
            };
            simplify_join_clause_for_relation(clause.cast(), current_rti)
        }

        _ => {
            // For other node types, check if they reference our relation
            if contains_relation_reference(node, current_rti) {
                Some(node)
            } else if contains_any_relation_reference(node) {
                create_bool_const_true()
            } else {
                Some(node)
            }
        }
    }
}

/// Create a boolean constant TRUE node
unsafe fn create_bool_const_true() -> Option<*mut pg_sys::Node> {
    let const_node = pg_sys::makeConst(
        pg_sys::BOOLOID,
        -1,
        pg_sys::InvalidOid as pg_sys::Oid,
        1,
        true.into_datum().unwrap(),
        false,
        true,
    );
    Some(const_node.cast())
}

/// Create a boolean constant FALSE node
unsafe fn create_bool_const_false() -> Option<*mut pg_sys::Node> {
    let const_node = pg_sys::makeConst(
        pg_sys::BOOLOID,
        -1,
        pg_sys::InvalidOid as pg_sys::Oid,
        1,
        false.into_datum().unwrap(),
        false,
        true,
    );
    Some(const_node.cast())
}

/// Check if a node is a boolean constant TRUE
unsafe fn is_bool_const_true(node: *mut pg_sys::Node) -> bool {
    if let Some(const_node) = nodecast!(Const, T_Const, node) {
        (*const_node).consttype == pg_sys::BOOLOID
            && !(*const_node).constisnull
            && bool::from_datum((*const_node).constvalue, false).unwrap_or(false)
    } else {
        false
    }
}

/// Create a boolean expression node with the given operator and arguments
unsafe fn create_bool_expr(
    boolop: BoolExprType::Type,
    args: Vec<*mut pg_sys::Node>,
) -> Option<*mut pg_sys::Node> {
    if args.is_empty() {
        return None;
    }

    // Create the first list item
    let mut args_list = std::ptr::null_mut();
    for &arg in &args {
        args_list = pg_sys::lappend(args_list, arg.cast::<core::ffi::c_void>());
    }

    // Allocate and initialize BoolExpr node
    let boolexpr =
        pg_sys::palloc0(std::mem::size_of::<pg_sys::BoolExpr>()).cast::<pg_sys::BoolExpr>();
    (*boolexpr).xpr.type_ = pg_sys::NodeTag::T_BoolExpr;
    (*boolexpr).boolop = boolop;
    (*boolexpr).args = args_list;
    (*boolexpr).location = -1;

    Some(boolexpr.cast())
}

/// Check if a node contains a reference to the specified relation
unsafe fn contains_relation_reference(node: *mut pg_sys::Node, target_rti: pg_sys::Index) -> bool {
    if node.is_null() {
        return false;
    }

    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        let target_rti = context as pg_sys::Index;

        if let Some(var) = nodecast!(Var, T_Var, node) {
            if (*var).varno as pg_sys::Index == target_rti {
                return true;
            }
        }

        pg_sys::expression_tree_walker(node, Some(walker), context)
    }

    walker(node, target_rti as *mut core::ffi::c_void)
}

/// Check if a node contains any relation reference (Var nodes)
unsafe fn contains_any_relation_reference(node: *mut pg_sys::Node) -> bool {
    if node.is_null() {
        return false;
    }

    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        _context: *mut core::ffi::c_void,
    ) -> bool {
        if nodecast!(Var, T_Var, node).is_some() {
            return true;
        }

        pg_sys::expression_tree_walker(node, Some(walker), std::ptr::null_mut())
    }

    walker(node, std::ptr::null_mut())
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::prelude::*;
    use proptest::prelude::*;

    #[pg_test]
    fn test_all_variant() {
        let got = SearchQueryInput::from(&Qual::All);
        let want = SearchQueryInput::All;
        assert_eq!(got, want);
    }

    #[pg_test]
    fn test_external_var_variant() {
        let got = SearchQueryInput::from(&Qual::ExternalVar);
        let want = SearchQueryInput::All;
        assert_eq!(got, want);
    }

    #[pg_test]
    fn test_pushdown_var_eq_true() {
        let qual = Qual::PushdownVarEqTrue {
            field: PushdownField::new("foo"),
        };
        let got = SearchQueryInput::from(&qual);
        let want = SearchQueryInput::Term {
            field: Some("foo".into()),
            value: OwnedValue::Bool(true),
            is_datetime: false,
        };
        assert_eq!(got, want);
    }

    #[pg_test]
    fn test_pushdown_var_eq_false() {
        let qual = Qual::PushdownVarEqFalse {
            field: PushdownField::new("bar"),
        };
        let got = SearchQueryInput::from(&qual);
        let want = SearchQueryInput::Term {
            field: Some("bar".into()),
            value: OwnedValue::Bool(false),
            is_datetime: false,
        };
        assert_eq!(got, want);
    }

    #[pg_test]
    fn test_pushdown_var_is_true() {
        let qual = Qual::PushdownVarIsTrue {
            field: PushdownField::new("baz"),
        };
        let got = SearchQueryInput::from(&qual);
        let want = SearchQueryInput::Term {
            field: Some("baz".into()),
            value: OwnedValue::Bool(true),
            is_datetime: false,
        };
        assert_eq!(got, want);
    }

    #[pg_test]
    fn test_pushdown_var_is_false() {
        let qual = Qual::PushdownVarIsFalse {
            field: PushdownField::new("qux"),
        };
        let got = SearchQueryInput::from(&qual);
        let want = SearchQueryInput::Term {
            field: Some("qux".into()),
            value: OwnedValue::Bool(false),
            is_datetime: false,
        };
        assert_eq!(got, want);
    }

    #[pg_test]
    fn test_pushdown_is_not_null() {
        let qual = Qual::PushdownIsNotNull {
            field: PushdownField::new("fld"),
        };
        let got = SearchQueryInput::from(&qual);
        let want = SearchQueryInput::Exists {
            field: "fld".into(),
        };
        assert_eq!(got, want);
    }

    fn arb_leaf() -> impl Strategy<Value = Qual> {
        prop_oneof![
            Just(Qual::All),
            "[a-z]{1,3}".prop_map(|s| Qual::PushdownVarEqTrue {
                field: PushdownField::new(&s)
            }),
            "[a-z]{1,3}".prop_map(|s| Qual::PushdownVarEqFalse {
                field: PushdownField::new(&s)
            }),
            "[a-z]{1,3}".prop_map(|s| Qual::PushdownVarIsTrue {
                field: PushdownField::new(&s)
            }),
            "[a-z]{1,3}".prop_map(|s| Qual::PushdownVarIsFalse {
                field: PushdownField::new(&s)
            }),
            "[a-z]{1,3}".prop_map(|s| Qual::PushdownIsNotNull {
                field: PushdownField::new(&s)
            }),
        ]
    }

    fn arb_qual(depth: u32) -> impl Strategy<Value = Qual> {
        arb_leaf().prop_recursive(depth, 256, 3, |inner| {
            prop_oneof![
                inner.clone().prop_map(|q| Qual::Not(Box::new(q))),
                prop::collection::vec(inner.clone(), 1..4).prop_map(Qual::And),
                prop::collection::vec(inner, 1..4).prop_map(Qual::Or),
            ]
        })
    }

    fn is_logical_equivalent(a: &Qual, b: &SearchQueryInput) -> bool {
        match (a, b) {
            // Match Qual::All with ConstScore
            (Qual::All, SearchQueryInput::All) => true,

            // Match boolean field TRUE cases
            (
                qual @ (Qual::PushdownVarEqTrue { field } | Qual::PushdownVarIsTrue { field }),
                SearchQueryInput::Term {
                    field: Some(f),
                    value,
                    ..
                },
            ) => field.attname() == *f && matches!(value, OwnedValue::Bool(true)),

            // Match boolean field FALSE cases
            (
                qual @ (Qual::PushdownVarEqFalse { field } | Qual::PushdownVarIsFalse { field }),
                SearchQueryInput::Term {
                    field: Some(f),
                    value,
                    ..
                },
            ) => field.attname() == *f && matches!(value, OwnedValue::Bool(false)),

            // Match IS NOT NULL
            (Qual::PushdownIsNotNull { field }, SearchQueryInput::Exists { field: f }) => {
                field.attname() == *f
            }

            // Match AND clauses
            (
                Qual::And(quals),
                SearchQueryInput::Boolean {
                    must,
                    should,
                    must_not,
                },
            ) => should.is_empty() && must_not.is_empty() && quals.len() == must.len(),

            // Match OR clauses
            (
                Qual::Or(quals),
                SearchQueryInput::Boolean {
                    must,
                    should,
                    must_not,
                },
            ) => must.is_empty() && must_not.is_empty() && quals.len() == should.len(),

            // Match NOT clauses
            (
                Qual::Not(inner),
                SearchQueryInput::Boolean {
                    must,
                    should,
                    must_not,
                },
            ) => must.len() == 1 && matches!(must[0], SearchQueryInput::All) && must_not.len() == 1,

            // Match negation of PushdownVarEqTrue mapping to PushdownVarEqFalse
            (
                Qual::Not(inner),
                SearchQueryInput::Term {
                    field: Some(f),
                    value: OwnedValue::Bool(false),
                    ..
                },
            ) if matches!(**inner, Qual::PushdownVarEqTrue { field: ref a } if a.attname() == *f) => {
                true
            }

            // Match negation of PushdownVarEqFalse mapping to PushdownVarEqTrue
            (
                Qual::Not(inner),
                SearchQueryInput::Term {
                    field: Some(f),
                    value: OwnedValue::Bool(true),
                    ..
                },
            ) if matches!(**inner, Qual::PushdownVarEqFalse { field: ref a } if a.attname() == *f) => {
                true
            }

            _ => false,
        }
    }

    proptest! {
        #[pg_test]
        fn test_qual_conversion_logical_equivalence(q in arb_qual(3)) {
            let query = SearchQueryInput::from(&q);
            prop_assert!(is_logical_equivalent(&q, &query), "Failed: Qual: {:?} SearchQueryInput: {:?}", q, query);
        }
    }
}

/// Check if an operator is a search operator (@@@ family)
unsafe fn is_search_operator(opno: pg_sys::Oid) -> bool {
    // Use the dynamic operator ID function
    use crate::api::operator::anyelement_query_input_opoid;
    opno == anyelement_query_input_opoid()
}

/// Create a SearchQueryInput from an OpExpr that represents a @@@ operation
unsafe fn create_search_query_from_opexpr(
    op_expr: *mut pg_sys::OpExpr,
    attno_map: &HashMap<pg_sys::AttrNumber, FieldName>,
) -> Option<SearchQueryInput> {
    let args = PgList::<pg_sys::Node>::from_pg((*op_expr).args);
    let args_vec: Vec<_> = args.iter_ptr().collect();
    
    if args_vec.len() != 2 {
        debug_log!("🔥 OpExpr does not have exactly 2 arguments");
        return None;
    }

    // First argument should be a column reference (Var)
    // Second argument should be the search query (Const)
    let left_arg = args_vec[0];
    let right_arg = args_vec[1];

    // Extract field name from left argument
    let field_name = if (*left_arg).type_ == pg_sys::NodeTag::T_Var {
        let var_node = left_arg.cast::<pg_sys::Var>();
        let attno = (*var_node).varattno;
        attno_map.get(&attno).cloned()
    } else {
        debug_log!("🔥 Left argument is not a Var node");
        return None;
    };

    // Extract query from right argument
    let search_query = if (*right_arg).type_ == pg_sys::NodeTag::T_Const {
        let const_node = right_arg.cast::<pg_sys::Const>();
        if (*const_node).constisnull {
            debug_log!("🔥 Query is NULL");
            return None;
        }

        // Convert the constant value to SearchQueryInput
        let datum = (*const_node).constvalue;
        let type_oid = (*const_node).consttype;
        
        // Check if it's a text type (simple string query)
        if type_oid == pg_sys::TEXTOID {
            let text_ptr = datum.cast_mut_ptr::<pg_sys::varlena>();
            let vl_len = unsafe { (*text_ptr).vl_len_ };
            let text_len = (vl_len[0] as u32 | (vl_len[1] as u32) << 8 | (vl_len[2] as u32) << 16 | (vl_len[3] as u32) << 24) - 4; // VARHDRSZ is 4
            let text_data = unsafe { (text_ptr as *const u8).add(4) }; // Skip header
            
            let query_string = unsafe {
                std::slice::from_raw_parts(text_data, text_len as usize)
                    .to_vec()
                    .into_iter()
                    .map(|b| b as char)
                    .collect::<String>()
            };
            
            // Create a simple ParseWithField query
            if let Some(field_name) = field_name {
                debug_log!("🔥 Creating search query from text: field={}, query={}", field_name, query_string);
                return Some(SearchQueryInput::ParseWithField {
                    field: field_name,
                    query_string,
                    lenient: Some(false),
                    conjunction_mode: Some(false),
                });
            } else {
                debug_log!("🔥 Could not determine field name for text search query");
                return None;
            }
        } else {
            // Try to deserialize as SearchQueryInput object
            debug_log!("🔥 Query is SearchQueryInput object (OID: {}), deserializing", type_oid);
            
            // Use SearchQueryInput::from_datum to deserialize
            use pgrx::FromDatum;
            match SearchQueryInput::from_datum(datum, false) {
                Some(mut search_query_input) => {
                    debug_log!("🔥 Successfully deserialized SearchQueryInput: {:?}", search_query_input);
                    
                    // If the field is specified in the left side, we need to ensure the SearchQueryInput
                    // has the correct field. This handles cases where the query was parsed generically
                    // and needs to be associated with the specific field from the @@@ operator.
                    if let Some(field_name) = field_name {
                        search_query_input = match search_query_input {
                            SearchQueryInput::ParseWithField { query_string, lenient, conjunction_mode, .. } => {
                                SearchQueryInput::ParseWithField {
                                    field: field_name,
                                    query_string,
                                    lenient,
                                    conjunction_mode,
                                }
                            }
                            SearchQueryInput::Parse { query_string, lenient, conjunction_mode } => {
                                SearchQueryInput::ParseWithField {
                                    field: field_name,
                                    query_string,
                                    lenient,
                                    conjunction_mode,
                                }
                            }
                            // For other query types, return as-is since they might not need field specification
                            other => other,
                        };
                        debug_log!("🔥 Updated SearchQueryInput with field: {:?}", search_query_input);
                    }
                    
                    return Some(search_query_input);
                }
                None => {
                    debug_log!("🔥 Failed to deserialize SearchQueryInput from datum");
                    return None;
                }
            }
        }
    } else {
        debug_log!("🔥 Right argument is not a Const node");
        return None;
    };

    // This should not be reached since we return early in both branches above
    debug_log!("🔥 Unexpected code path in create_search_query_from_opexpr");
    None
}

/// Serialize a PostgreSQL expression to string
unsafe fn serialize_expression(expr: *mut pg_sys::Expr) -> String {
    let node_string = pg_sys::nodeToString(expr.cast::<core::ffi::c_void>());
    let rust_string = std::ffi::CStr::from_ptr(node_string)
        .to_string_lossy()
        .into_owned();
    pg_sys::pfree(node_string.cast());
    rust_string
}

/// Extract field names referenced in an expression
unsafe fn extract_referenced_fields(
    expr: *mut pg_sys::Expr,
    attno_map: &HashMap<pg_sys::AttrNumber, FieldName>,
) -> Vec<FieldName> {
    let mut fields = Vec::new();
    extract_referenced_fields_recursive(expr, attno_map, &mut fields);
    fields.sort();
    fields.dedup();
    fields
}

/// Recursively extract field names from an expression
unsafe fn extract_referenced_fields_recursive(
    expr: *mut pg_sys::Expr,
    attno_map: &HashMap<pg_sys::AttrNumber, FieldName>,
    fields: &mut Vec<FieldName>,
) {
    if expr.is_null() {
        return;
    }

    match (*expr).type_ {
        pg_sys::NodeTag::T_Var => {
            let var_node = expr.cast::<pg_sys::Var>();
            let attno = (*var_node).varattno;
            if let Some(field_name) = attno_map.get(&attno) {
                fields.push(field_name.clone());
            }
        }
        pg_sys::NodeTag::T_BoolExpr => {
            let bool_expr = expr.cast::<pg_sys::BoolExpr>();
            let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);
            for arg in args.iter_ptr() {
                extract_referenced_fields_recursive(arg.cast(), attno_map, fields);
            }
        }
        pg_sys::NodeTag::T_OpExpr => {
            let op_expr = expr.cast::<pg_sys::OpExpr>();
            let args = PgList::<pg_sys::Node>::from_pg((*op_expr).args);
            for arg in args.iter_ptr() {
                extract_referenced_fields_recursive(arg.cast(), attno_map, fields);
            }
        }
        _ => {
            // For other expression types, we might need to add more cases
            // but for now, we'll skip them
        }
    }
}

/// Create a PostgresEval qual for non-indexed expressions that need PostgreSQL evaluation
/// Convert PostgreSQL expressions to field-based operations that can be evaluated in Tantivy
unsafe fn try_create_field_operation(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
) -> Option<Qual> {
    if node.is_null() {
        return None;
    }

    match (*node).type_ {
        pg_sys::NodeTag::T_OpExpr => {
            let op_expr = node.cast::<pg_sys::OpExpr>();
            if let Some(field_comparison) = try_create_field_comparison(op_expr, root, rti) {
                return Some(field_comparison);
            }
        }
        pg_sys::NodeTag::T_NullTest => {
            let null_test = node.cast::<pg_sys::NullTest>();
            if let Some(field_null_test) = try_create_field_null_test(null_test, root, rti) {
                return Some(field_null_test);
            }
        }
        _ => {}
    }

    None
}

/// Try to create a FieldComparison from a PostgreSQL OpExpr
unsafe fn try_create_field_comparison(
    op_expr: *mut pg_sys::OpExpr,
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
) -> Option<Qual> {
    let args = PgList::<pg_sys::Node>::from_pg((*op_expr).args);
    let args_vec: Vec<_> = args.iter_ptr().collect();
    
    if args_vec.len() != 2 {
        return None;
    }

    let left = args_vec[0];
    let right = args_vec[1];

    // Check if left side is a Var (field reference)
    if (*left).type_ != pg_sys::NodeTag::T_Var {
        return None;
    }

    let var = left.cast::<pg_sys::Var>();
    if (*var).varno as pg_sys::Index != rti {
        return None;
    }

    // Get field name from attribute number
    let attno = (*var).varattno;
    let field_name = get_field_name_from_attno(root, rti, attno)?;

    // Check if right side is a constant
    if (*right).type_ != pg_sys::NodeTag::T_Const {
        return None;
    }

    let const_node = right.cast::<pg_sys::Const>();
    if (*const_node).constisnull {
        return None;
    }

    // Extract the value based on the constant type
    let field_value = extract_field_value_from_const(const_node)?;

    // Map operator OID to comparison operator
    let operator = map_oid_to_comparison_operator((*op_expr).opno)?;

    Some(Qual::FieldComparison {
        field: field_name,
        operator,
        value: field_value,
    })
}

/// Try to create a FieldNullTest from a PostgreSQL NullTest
unsafe fn try_create_field_null_test(
    null_test: *mut pg_sys::NullTest,
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
) -> Option<Qual> {
    let arg = (*null_test).arg;
    
    // Check if argument is a Var (field reference)
    if (*arg).type_ != pg_sys::NodeTag::T_Var {
        return None;
    }

    let var = arg.cast::<pg_sys::Var>();
    if (*var).varno as pg_sys::Index != rti {
        return None;
    }

    // Get field name from attribute number
    let attno = (*var).varattno;
    let field_name = get_field_name_from_attno(root, rti, attno)?;

    let is_null = (*null_test).nulltesttype == pg_sys::NullTestType::IS_NULL;

    Some(Qual::FieldNullTest {
        field: field_name,
        is_null,
    })
}

/// Extract field value from a PostgreSQL Const node
unsafe fn extract_field_value_from_const(const_node: *mut pg_sys::Const) -> Option<FieldValue> {
    let datum = (*const_node).constvalue;
    let type_oid = (*const_node).consttype;

    match type_oid {
        pg_sys::INT4OID => {
            let value = i32::from_datum(datum, false)?;
            Some(FieldValue::Integer(value as i64))
        }
        pg_sys::INT8OID => {
            let value = i64::from_datum(datum, false)?;
            Some(FieldValue::Integer(value))
        }
        pg_sys::FLOAT4OID => {
            let value = f32::from_datum(datum, false)?;
            Some(FieldValue::Float(value as f64))
        }
        pg_sys::FLOAT8OID => {
            let value = f64::from_datum(datum, false)?;
            Some(FieldValue::Float(value))
        }
        pg_sys::TEXTOID => {
            let value = String::from_datum(datum, false)?;
            Some(FieldValue::Text(value))
        }
        pg_sys::BOOLOID => {
            let value = bool::from_datum(datum, false)?;
            Some(FieldValue::Boolean(value))
        }
        pg_sys::NUMERICOID => {
            // Convert NUMERIC to f64 for simplicity
            if let Some(numeric) = pgrx::AnyNumeric::from_datum(datum, false) {
                if let Ok(value) = f64::try_from(numeric) {
                    return Some(FieldValue::Float(value));
                }
            }
            None
        }
        _ => None,
    }
}

/// Map PostgreSQL operator OID to ComparisonOperator
unsafe fn map_oid_to_comparison_operator(opno: pg_sys::Oid) -> Option<ComparisonOperator> {
    // Common operator OIDs for different types
    match opno.to_u32() {
        // Integer operators
        96 => Some(ComparisonOperator::Equal),      // int4eq
        518 => Some(ComparisonOperator::NotEqual),  // int4ne
        97 => Some(ComparisonOperator::LessThan),   // int4lt
        523 => Some(ComparisonOperator::LessThanOrEqual), // int4le
        521 => Some(ComparisonOperator::GreaterThan), // int4gt
        525 => Some(ComparisonOperator::GreaterThanOrEqual), // int4ge
        
        // Float operators
        1120 => Some(ComparisonOperator::Equal),    // float4eq
        1121 => Some(ComparisonOperator::NotEqual), // float4ne
        1122 => Some(ComparisonOperator::LessThan), // float4lt
        1123 => Some(ComparisonOperator::LessThanOrEqual), // float4le
        1124 => Some(ComparisonOperator::GreaterThan), // float4gt
        1125 => Some(ComparisonOperator::GreaterThanOrEqual), // float4ge
        
        // Text operators
        98 => Some(ComparisonOperator::Equal),      // texteq
        531 => Some(ComparisonOperator::NotEqual),  // textne
        664 => Some(ComparisonOperator::LessThan),  // textlt
        665 => Some(ComparisonOperator::LessThanOrEqual), // textle
        666 => Some(ComparisonOperator::GreaterThan), // textgt
        667 => Some(ComparisonOperator::GreaterThanOrEqual), // textge
        
        _ => None,
    }
}

/// Get field name from attribute number using the relation's tuple descriptor
unsafe fn get_field_name_from_attno(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    attno: pg_sys::AttrNumber,
) -> Option<FieldName> {
    let rte = pg_sys::planner_rt_fetch(rti, root);
    if rte.is_null() {
        return None;
    }

    let relid = (*rte).relid;
    if relid == pg_sys::InvalidOid {
        return None;
    }

    let rel = pg_sys::RelationIdGetRelation(relid);
    if rel.is_null() {
        return None;
    }

    let tupdesc = (*rel).rd_att;
    if attno <= 0 || i32::from(attno) > (*tupdesc).natts {
        pg_sys::RelationClose(rel);
        return None;
    }

    let attr = (*tupdesc).attrs.as_ptr().add((attno - 1) as usize);
    let attr_name = std::ffi::CStr::from_ptr((*attr).attname.data.as_ptr());
    let field_name = attr_name.to_string_lossy().to_string();
    
    pg_sys::RelationClose(rel);
    Some(FieldName::from(field_name))
}

unsafe fn try_create_postgres_eval(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
) -> Option<Qual> {
    use crate::postgres::var::{fieldname_from_var, find_var_relation};

    if node.is_null() {
        return None;
    }

    // Clean any RestrictInfo wrappers first
    let cleaned_node = clean_restrictinfo_recursively(node);

    // Walk the expression tree to find all Var nodes
    unsafe extern "C-unwind" fn var_walker(
        node: *mut pg_sys::Node,
        original_context: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        let context = &mut *(original_context as *mut VarWalkerContext);

        if let Some(var) = nodecast!(Var, T_Var, node) {
            if (*var).varno as pg_sys::Index == context.rti {
                context.has_our_relation = true;
                let (heaprelid, varattno, _) = find_var_relation(var, context.root);
                if heaprelid != pg_sys::Oid::INVALID {
                    if let Some(field) = fieldname_from_var(heaprelid, var, varattno) {
                        context.attno_map.insert((*var).varattno, field);
                    }
                }
            }
        }

        pg_sys::expression_tree_walker(node, Some(var_walker), original_context)
    }

    struct VarWalkerContext {
        root: *mut pg_sys::PlannerInfo,
        rti: pg_sys::Index,
        attno_map: HashMap<pg_sys::AttrNumber, FieldName>,
        has_our_relation: bool,
    }

    let mut context = VarWalkerContext {
        root,
        rti,
        attno_map: HashMap::default(),
        has_our_relation: false,
    };

    var_walker(
        cleaned_node,
        (&mut context as *mut VarWalkerContext) as *mut core::ffi::c_void,
    );

    // Only create PostgresEval if this expression involves our relation
    if context.has_our_relation && !context.attno_map.is_empty() {
        Some(Qual::PostgresEval {
            expr: cleaned_node.cast(),
            attno_map: context.attno_map,
        })
    } else {
        None
    }
}

/// Transform the qual tree to bubble up IndexedWithFilter wrappers optimally
/// This covers minimal subtrees that contain PostgresEval nodes
pub fn transform_qual_tree(qual: Qual) -> Qual {
    match qual {
        // If we find a PostgresEval, wrap it in IndexedWithFilter with All query
        Qual::PostgresEval { expr, attno_map } => {
            debug_log!("🔥 Transforming PostgresEval to IndexedWithFilter with All query");
            // For standalone PostgresEval, use All query since there's no indexed part
            let filter_expression = unsafe { serialize_expression(expr) };
            let referenced_fields = unsafe { extract_referenced_fields(expr, &attno_map) };
            
            // For now, return the original PostgresEval - proper IndexedWithFilter integration needed
            // TODO: Implement proper IndexedWithFilter qual representation
            Qual::PostgresEval { expr, attno_map }
        }
        
        // Handle OR nodes specially - each branch should be independently evaluatable
        Qual::Or(quals) => {
            let mut transformed_branches = Vec::new();
            
            for branch in quals {
                let transformed_branch = transform_qual_tree(branch);
                
                // Check if this branch contains PostgresEval after transformation
                if transformed_branch.contains_postgres_eval() {
                    // This branch has non-indexed predicates, wrap in IndexedWithFilter with All
                    debug_log!("🔥 OR branch contains PostgresEval, wrapping with All query");
                    transformed_branches.push(wrap_in_indexed_with_filter_all(transformed_branch));
                } else {
                    // This branch is pure indexed, keep as-is
                    transformed_branches.push(transformed_branch);
                }
            }
            
            Qual::Or(transformed_branches)
        }
        
        // Handle AND nodes - look for mixed indexed/non-indexed patterns
        Qual::And(quals) => {
            let mut indexed_parts = Vec::new();
            let mut postgres_eval_parts = Vec::new();
            let mut other_parts = Vec::new();
            
            for qual in quals {
                let transformed = transform_qual_tree(qual);
                
                if transformed.contains_postgres_eval() {
                    postgres_eval_parts.push(transformed);
                } else if is_indexed_qual(&transformed) {
                    indexed_parts.push(transformed);
                } else {
                    other_parts.push(transformed);
                }
            }
            
            // If we have both indexed and PostgresEval parts, create optimal wrapper
            if !indexed_parts.is_empty() && !postgres_eval_parts.is_empty() {
                debug_log!("🔥 AND contains mixed indexed/PostgresEval, creating optimal IndexedWithFilter");
                
                // Combine indexed parts into base query
                let indexed_query = if indexed_parts.len() == 1 {
                    SearchQueryInput::from(&indexed_parts[0])
                } else {
                    SearchQueryInput::Boolean {
                        must: indexed_parts.into_iter().map(|q| SearchQueryInput::from(&q)).collect(),
                        should: vec![],
                        must_not: vec![],
                    }
                };
                
                // Combine PostgresEval parts 
                let combined_postgres_eval = if postgres_eval_parts.len() == 1 {
                    postgres_eval_parts.into_iter().next().unwrap()
                } else {
                    Qual::And(postgres_eval_parts)
                };
                
                // Create IndexedWithFilter that combines indexed query with PostgresEval filter
                create_indexed_with_filter(indexed_query, combined_postgres_eval)
            } else {
                // No mixing, just combine all parts
                let mut all_parts = indexed_parts;
                all_parts.extend(postgres_eval_parts);
                all_parts.extend(other_parts);
                
                if all_parts.len() == 1 {
                    all_parts.into_iter().next().unwrap()
                } else {
                    Qual::And(all_parts)
                }
            }
        }
        
        // For other qual types, recursively transform children
        Qual::Not(inner) => Qual::Not(Box::new(transform_qual_tree(*inner))),
        
        // Leaf nodes don't need transformation
        other => other,
    }
}

/// Check if a qual represents an indexed predicate
fn is_indexed_qual(qual: &Qual) -> bool {
    matches!(qual, 
        Qual::OpExpr { .. } | 
        Qual::PushdownExpr { .. } |
        Qual::PushdownVarEqTrue { .. } |
        Qual::PushdownVarEqFalse { .. } |
        Qual::PushdownVarIsTrue { .. } |
        Qual::PushdownVarIsFalse { .. } |
        Qual::PushdownIsNotNull { .. } |
        Qual::ScoreExpr { .. }
    )
}

/// Wrap a qual in IndexedWithFilter with All query
fn wrap_in_indexed_with_filter_all(qual: Qual) -> Qual {
    // Extract PostgresEval information for the filter
    if let Some((filter_expression, referenced_fields)) = extract_postgres_eval_info(&qual) {
        create_indexed_with_filter(SearchQueryInput::All, qual)
    } else {
        // If no PostgresEval found, return as-is
        qual
    }
}

/// Create an IndexedWithFilter qual from indexed query and PostgresEval filter
fn create_indexed_with_filter(indexed_query: SearchQueryInput, postgres_eval_qual: Qual) -> Qual {
    if let Some((filter_expression, referenced_fields)) = extract_postgres_eval_info(&postgres_eval_qual) {
        // Convert to SearchQueryInput::IndexedWithFilter and then back to Qual
        let search_query =                         // FIXME: Convert to SimpleFieldFilter approach
                        SearchQueryInput::IndexedWithFilter {
                            indexed_query: Box::new(indexed_query),
                            field_filters: vec![], // Temporary placeholder
                        };
        
        // Convert SearchQueryInput back to Qual
        // This is a bit of a hack - we need to represent IndexedWithFilter as a Qual
        // For now, we'll use a PostgresEval with special metadata
        postgres_eval_qual // Return the original for now, proper implementation needed
    } else {
        postgres_eval_qual
    }
}

/// Extract PostgresEval information from a qual tree
fn extract_postgres_eval_info(qual: &Qual) -> Option<(String, Vec<FieldName>)> {
    match qual {
        Qual::PostgresEval { expr, attno_map } => {
            let filter_expression = unsafe { serialize_expression(*expr) };
            let referenced_fields = unsafe { extract_referenced_fields(*expr, attno_map) };
            Some((filter_expression, referenced_fields))
        }
        Qual::And(quals) | Qual::Or(quals) => {
            // For compound quals, we'd need to combine multiple PostgresEval parts
            // This is more complex and might need a different approach
            None
        }
        _ => None,
    }
}

/// Convert field comparison quals to SimpleFieldFilter objects
pub unsafe fn convert_quals_to_simple_filters(
    qual: &Qual,
    relation_oid: pg_sys::Oid,
) -> Vec<crate::query::simple_field_filter::SimpleFieldFilter> {
    use crate::query::simple_field_filter::{SimpleFieldFilter, SimpleOperator, SimpleValue};
    
    let mut filters = Vec::new();
    
    match qual {
        Qual::FieldComparison { field, operator, value } => {
            // Convert comparison operator
            let simple_op = match operator {
                ComparisonOperator::Equal => SimpleOperator::Equal,
                ComparisonOperator::GreaterThan => SimpleOperator::GreaterThan,
                ComparisonOperator::LessThan => SimpleOperator::LessThan,
                ComparisonOperator::GreaterThanOrEqual => SimpleOperator::GreaterThan, // Simplified
                ComparisonOperator::LessThanOrEqual => SimpleOperator::LessThan, // Simplified
                ComparisonOperator::NotEqual => return filters, // Skip NOT EQUAL for now
            };
            
            // Convert field value
            let simple_value = match value {
                FieldValue::Integer(i) => SimpleValue::Integer(*i),
                FieldValue::Float(f) => SimpleValue::Float(*f),
                FieldValue::Text(s) => SimpleValue::Text(s.clone()),
                FieldValue::Boolean(b) => SimpleValue::Boolean(*b),
                FieldValue::Null => return filters, // Skip NULL values
            };
            
            // Get field attribute number - for now use a placeholder
            let field_attno = 1; // This would need to be properly resolved from field name
            
            filters.push(SimpleFieldFilter::new(
                field.clone(),
                simple_op,
                simple_value,
                relation_oid,
                field_attno,
            ));
        }
        Qual::FieldNullTest { field, is_null } => {
            let simple_op = if *is_null {
                SimpleOperator::IsNull
            } else {
                SimpleOperator::IsNotNull
            };
            
            // For NULL tests, the value doesn't matter
            let simple_value = SimpleValue::Text("unused".to_string());
            let field_attno = 1; // This would need to be properly resolved
            
            filters.push(SimpleFieldFilter::new(
                field.clone(),
                simple_op,
                simple_value,
                relation_oid,
                field_attno,
            ));
        }
        Qual::And(quals) => {
            // Recursively convert all AND-ed quals
            for sub_qual in quals {
                filters.extend(convert_quals_to_simple_filters(sub_qual, relation_oid));
            }
        }
        _ => {
            // Other qual types are not converted to SimpleFieldFilter
        }
    }
    
    filters
}

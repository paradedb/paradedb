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

use super::opexpr::OpExpr;
use crate::nodecast;
use crate::postgres::customscan::builders::custom_path::RestrictInfoType;
use crate::postgres::customscan::operator_oid;
use crate::postgres::customscan::pdbscan::projections::score::score_funcoid;
use crate::postgres::customscan::pdbscan::pushdown::{is_complex, try_pushdown, PushdownField};
use crate::query::SearchQueryInput;
use crate::query::heap_field_filter::{HeapFieldFilter, HeapOperand, HeapOperator, HeapValue};
use crate::api::FieldName;
use crate::schema::SearchIndexSchema;
use crate::debug_log;
use pg_sys::BoolExprType;
use pgrx::{pg_sys, FromDatum, IntoDatum, PgList};
use std::ops::Bound;
use tantivy::schema::OwnedValue;

#[derive(Debug, Clone)]
pub enum Qual {
    All,
    ExternalVar,
    ExternalExpr,
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
    /// Heap-based expression evaluation for non-indexed predicates
    /// Contains an underlying search query that must be executed first
    HeapExpr {
        /// The PostgreSQL expression node to evaluate
        expr_node: *mut pg_sys::Node,
        /// Description of the expression for debugging
        expr_description: String,
        /// The search query to execute before applying the heap filter
        /// Can be All (scan whole relation) or a more specific query
        search_query_input: Box<SearchQueryInput>,
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
            Qual::OpExpr { .. } => false,
            Qual::Expr { .. } => false,
            Qual::PushdownExpr { .. } => false,
            Qual::PushdownVarEqTrue { .. } => false,
            Qual::PushdownVarEqFalse { .. } => false,
            Qual::PushdownVarIsTrue { .. } => false,
            Qual::PushdownVarIsFalse { .. } => false,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => false,
            Qual::HeapExpr { search_query_input, .. } => matches!(**search_query_input, SearchQueryInput::All),
            Qual::And(quals) => quals.iter().any(|q| q.contains_all()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_all()),
            Qual::Not(qual) => qual.contains_all(),
        }
    }

    pub fn contains_external_var(&self) -> bool {
        match self {
            Qual::All => false,
            Qual::ExternalVar => true,
            Qual::ExternalExpr => true,
            Qual::OpExpr { .. } => false,
            Qual::Expr { .. } => false,
            Qual::PushdownExpr { .. } => false,
            Qual::PushdownVarEqTrue { .. } => false,
            Qual::PushdownVarEqFalse { .. } => false,
            Qual::PushdownVarIsTrue { .. } => false,
            Qual::PushdownVarIsFalse { .. } => false,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => false,
            Qual::HeapExpr { .. } => false,
            Qual::And(quals) => quals.iter().any(|q| q.contains_external_var()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_external_var()),
            Qual::Not(qual) => qual.contains_external_var(),
        }
    }

    pub unsafe fn contains_exec_param(&self) -> bool {
        match self {
            Qual::All => false,
            Qual::ExternalVar => false,
            Qual::ExternalExpr => false,
            Qual::OpExpr { .. } => false,
            Qual::Expr { node, .. } => contains_exec_param(*node),
            Qual::PushdownExpr { .. } => false,
            Qual::PushdownVarEqTrue { .. } => false,
            Qual::PushdownVarEqFalse { .. } => false,
            Qual::PushdownVarIsTrue { .. } => false,
            Qual::PushdownVarIsFalse { .. } => false,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => false,
            Qual::HeapExpr { .. } => false,
            Qual::And(quals) => quals.iter().any(|q| q.contains_exec_param()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_exec_param()),
            Qual::Not(qual) => qual.contains_exec_param(),
        }
    }

    pub fn contains_exprs(&self) -> bool {
        match self {
            Qual::All => false,
            Qual::ExternalVar => false,
            Qual::ExternalExpr => false,
            Qual::OpExpr { .. } => false,
            Qual::Expr { .. } => true,
            Qual::PushdownExpr { .. } => false,
            Qual::PushdownVarEqTrue { .. } => true,
            Qual::PushdownVarEqFalse { .. } => true,
            Qual::PushdownVarIsTrue { .. } => true,
            Qual::PushdownVarIsFalse { .. } => true,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => false,
            Qual::HeapExpr { .. } => true,
            Qual::And(quals) => quals.iter().any(|q| q.contains_exprs()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_exprs()),
            Qual::Not(qual) => qual.contains_exprs(),
        }
    }

    pub fn contains_score_exprs(&self) -> bool {
        match self {
            Qual::All => false,
            Qual::ExternalVar => false,
            Qual::ExternalExpr => false,
            Qual::OpExpr { .. } => false,
            Qual::Expr { .. } => false,
            Qual::PushdownExpr { .. } => false,
            Qual::PushdownVarEqTrue { .. } => false,
            Qual::PushdownVarEqFalse { .. } => false,
            Qual::PushdownVarIsTrue { .. } => false,
            Qual::PushdownVarIsFalse { .. } => false,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => true,
            Qual::HeapExpr { .. } => false,
            Qual::And(quals) => quals.iter().any(|q| q.contains_score_exprs()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_score_exprs()),
            Qual::Not(qual) => qual.contains_score_exprs(),
        }
    }

    pub fn collect_exprs<'a>(&'a mut self, exprs: &mut Vec<&'a mut Qual>) {
        match self {
            Qual::Expr { .. } => exprs.push(self),
            Qual::HeapExpr { .. } => exprs.push(self),
            Qual::And(quals) => quals.iter_mut().for_each(|q| q.collect_exprs(exprs)),
            Qual::Or(quals) => quals.iter_mut().for_each(|q| q.collect_exprs(exprs)),
            Qual::Not(qual) => qual.collect_exprs(exprs),
            _ => {}
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
            Qual::OpExpr { val, .. } => unsafe {
                SearchQueryInput::from_datum((**val).constvalue, (**val).constisnull)
                    .expect("rhs of @@@ operator Qual must not be null")
            },
            Qual::Expr { node, expr_state } => SearchQueryInput::postgres_expression(*node),
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
            Qual::HeapExpr { expr_node, expr_description, search_query_input } => {
                // Create HeapFieldFilter from the PostgreSQL expression
                let field_filters = vec![unsafe { HeapFieldFilter::new(*expr_node, expr_description.clone()) }];
                
                SearchQueryInput::IndexedWithFilter {
                    indexed_query: search_query_input.clone(),
                    field_filters,
                }
            },
            Qual::And(quals) => {
                let mut must = quals.iter().map(SearchQueryInput::from).collect::<Vec<_>>();
                let popscore = |vec: &mut Vec<SearchQueryInput>| -> Option<SearchQueryInput> {
                    for i in 0..vec.len() {
                        if matches!(vec[i], SearchQueryInput::ScoreFilter { .. }) {
                            return Some(vec.remove(i));
                        }
                    }
                    None
                };

                // pull out any ScoreFilters from the `must` clauses
                let mut must_scores = vec![];
                while let Some(score_filter) = popscore(&mut must) {
                    must_scores.push(score_filter);
                }

                // make the Boolean clause we intend to return (or wrap)
                let mut boolean = SearchQueryInput::Boolean {
                    must,
                    should: vec![],
                    must_not: vec![],
                };

                // wrap the basic boolean query, iteratively, in each of the extracted ScoreFilters
                while let Some(SearchQueryInput::ScoreFilter { bounds, query }) = must_scores.pop()
                {
                    boolean = SearchQueryInput::ScoreFilter {
                        bounds,
                        query: Some(Box::new(boolean)),
                    }
                }

                boolean
            }

            Qual::Or(quals) => {
                let should = quals
                    .iter()
                    .map(SearchQueryInput::from)
                    // any dangling ScoreFilter clauses are non-sensical, so we'll just pretend they don't exist
                    .filter(|query| !matches!(query, SearchQueryInput::ScoreFilter { .. }))
                    .collect::<Vec<_>>();

                match should.len() {
                    0 => SearchQueryInput::Boolean {
                        must: Default::default(),
                        should: Default::default(),
                        must_not: Default::default(),
                    },
                    _ => SearchQueryInput::Boolean {
                        must: Default::default(),
                        should,
                        must_not: Default::default(),
                    },
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
        }
    }
}

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
    // Add debug logging to see what node types we're processing
    let node_tag = (*node).type_;
    debug_log!("extract_quals called with node type: {:?}", node_tag);
    
    if node.is_null() {
        return None;
    }

    match (*node).type_ {
        pg_sys::NodeTag::T_List => {
            let mut quals = list(
                root,
                rti,
                node.cast(),
                pdbopoid,
                ri_type,
                schema,
                convert_external_to_special_qual,
                uses_tantivy_to_query,
            )?;
            if quals.len() == 1 {
                quals.pop()
            } else {
                Some(Qual::And(quals))
            }
        }

        pg_sys::NodeTag::T_RestrictInfo => {
            let ri = nodecast!(RestrictInfo, T_RestrictInfo, node)?;
            let clause = if !(*ri).orclause.is_null() {
                (*ri).orclause
            } else {
                (*ri).clause
            };
            extract_quals(
                root,
                rti,
                clause.cast(),
                pdbopoid,
                ri_type,
                schema,
                convert_external_to_special_qual,
                uses_tantivy_to_query,
            )
        }

        pg_sys::NodeTag::T_OpExpr => {
            debug_log!("Processing OpExpr node");
            opexpr(
                root,
                rti,
                OpExpr::from_single(node)?,
                pdbopoid,
                ri_type,
                schema,
                convert_external_to_special_qual,
                uses_tantivy_to_query,
            )
        }

        pg_sys::NodeTag::T_ScalarArrayOpExpr => opexpr(
            root,
            rti,
            OpExpr::from_array(node)?,
            pdbopoid,
            ri_type,
            schema,
            convert_external_to_special_qual,
            uses_tantivy_to_query,
        ),

        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = nodecast!(BoolExpr, T_BoolExpr, node)?;
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
            let mut quals = list(
                root,
                rti,
                (*boolexpr).args,
                pdbopoid,
                ri_type,
                schema,
                convert_external_to_special_qual,
                uses_tantivy_to_query,
            )?;

            match (*boolexpr).boolop {
                pg_sys::BoolExprType::AND_EXPR => Some(Qual::And(quals)),
                pg_sys::BoolExprType::OR_EXPR => Some(Qual::Or(quals)),
                pg_sys::BoolExprType::NOT_EXPR => Some(Qual::Not(Box::new(quals.pop()?))),
                _ => panic!("unexpected `BoolExprType`: {}", (*boolexpr).boolop),
            }
        }

        pg_sys::NodeTag::T_Var => {
            debug_log!("Processing T_Var node - this might be a boolean field reference");
            let var_node = nodecast!(Var, T_Var, node)?;
            
            // Check if this is a boolean field reference to our relation
            if (*var_node).varno as pg_sys::Index == rti {
                debug_log!("Found boolean field reference: varno={}, attno={}", (*var_node).varno, (*var_node).varattno);
                
                // Try to create a HeapExpr for boolean field = true
                if convert_external_to_special_qual {
                    debug_log!("Attempting HeapExpr creation for boolean field");
                    
                    // Check if root and parse are valid
                    if root.is_null() || (*root).parse.is_null() {
                        debug_log!("Invalid root or parse pointer");
                        return None;
                    }
                    
                    let rte = pg_sys::rt_fetch(rti, (*(*root).parse).rtable);
                    if rte.is_null() {
                        debug_log!("Failed to fetch range table entry");
                        return None;
                    }
                    
                    let relation_oid = (*rte).relid;
                    debug_log!("Got relation_oid: {}", relation_oid);
                    
                    // Get the field name
                    let attno = (*var_node).varattno;
                    if let Some(field_name) = get_field_name_from_attno(relation_oid, attno) {
                        debug_log!("Creating HeapExpr for boolean field: {}", field_name.root());
                        
                        // Create HeapExpr using the new expression-based approach
                        let expr_description = format!("Boolean field {} = true", field_name.root());
                        let heap_expr = Qual::HeapExpr {
                            expr_node: node, // Use the original T_Var node
                            expr_description,
                            search_query_input: Box::new(SearchQueryInput::All),
                        };
                        
                        debug_log!("Successfully created HeapExpr for boolean field");
                        *uses_tantivy_to_query = true;
                        return Some(heap_expr);
                    } else {
                        debug_log!("Failed to get field name for attno: {}", attno);
                    }
                } else {
                    debug_log!("convert_external_to_special_qual is false, attempting HeapExpr creation anyway for boolean field");
                    
                    // Even if convert_external_to_special_qual is false, we should still try to create HeapExpr
                    // for boolean fields since this is a common case
                    if root.is_null() || (*root).parse.is_null() {
                        debug_log!("Invalid root or parse pointer");
                        return None;
                    }
                    
                    let rte = pg_sys::rt_fetch(rti, (*(*root).parse).rtable);
                    if rte.is_null() {
                        debug_log!("Failed to fetch range table entry");
                        return None;
                    }
                    
                    let relation_oid = (*rte).relid;
                    debug_log!("Got relation_oid: {}", relation_oid);
                    
                    // Get the field name
                    let attno = (*var_node).varattno;
                    if let Some(field_name) = get_field_name_from_attno(relation_oid, attno) {
                        debug_log!("Creating HeapExpr for boolean field (even with convert_external_to_special_qual=false): {}", field_name.root());
                        
                        // Create HeapExpr using the new expression-based approach
                        let expr_description = format!("Boolean field {} = true", field_name.root());
                        let heap_expr = Qual::HeapExpr {
                            expr_node: node, // Use the original T_Var node
                            expr_description,
                            search_query_input: Box::new(SearchQueryInput::All),
                        };
                        
                        debug_log!("Successfully created HeapExpr for boolean field");
                        *uses_tantivy_to_query = true;
                        return Some(heap_expr);
                    } else {
                        debug_log!("Failed to get field name for attno: {}", attno);
                    }
                }
                
                // If HeapExpr creation failed, return None
                debug_log!("HeapExpr creation failed for boolean field, returning None");
                None
            } else {
                debug_log!("T_Var node does not reference our relation");
                None
            }
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
                    } else {
                        // Field is not fast, try creating HeapExpr if requested
                        if convert_external_to_special_qual {
                            debug_log!("Field is not fast, attempting HeapExpr creation for NULL test");
                            let rte = pg_sys::rt_fetch(rti, (*(*root).parse).rtable);
                            let relation_oid = (*rte).relid;
                            if let Some(heap_expr) = try_create_heap_expr_from_null_test(nulltest, rti) {
                                debug_log!("Successfully created HeapExpr for NULL test");
                                *uses_tantivy_to_query = true;
                                Some(heap_expr)
                            } else {
                                debug_log!("HeapExpr creation failed for NULL test");
                                None
                            }
                        } else {
                            None
                        }
                    }
                } else {
                    // Field not found in schema, try creating HeapExpr if requested
                    if convert_external_to_special_qual {
                        debug_log!("Field not found in schema, attempting HeapExpr creation for NULL test");
                        let rte = pg_sys::rt_fetch(rti, (*(*root).parse).rtable);
                        let relation_oid = (*rte).relid;
                        if let Some(heap_expr) = try_create_heap_expr_from_null_test(nulltest, rti) {
                            debug_log!("Successfully created HeapExpr for NULL test");
                            *uses_tantivy_to_query = true;
                            Some(heap_expr)
                        } else {
                            debug_log!("HeapExpr creation failed for NULL test");
                            None
                        }
                    } else {
                        None
                    }
                }
            } else if convert_external_to_special_qual {
                // Try to create a HeapExpr for non-indexed field NULL tests
                debug_log!("No pushdown field, attempting HeapExpr creation for NULL test");
                let rte = pg_sys::rt_fetch(rti, (*(*root).parse).rtable);
                let relation_oid = (*rte).relid;
                if let Some(heap_expr) = try_create_heap_expr_from_null_test(nulltest, rti) {
                    debug_log!("Successfully created HeapExpr for NULL test");
                    *uses_tantivy_to_query = true;
                    Some(heap_expr)
                } else {
                    debug_log!("HeapExpr creation failed for NULL test");
                    None
                }
            } else {
                None
            }
        }

        pg_sys::NodeTag::T_BooleanTest => {
            debug_log!("Processing BooleanTest node");
            booltest(
                root,
                rti,
                node,
                pdbopoid,
                ri_type,
                schema,
                convert_external_to_special_qual,
                uses_tantivy_to_query,
            )
        }

        pg_sys::NodeTag::T_Const => {
            debug_log!("Processing T_Const node - this might be a boolean constant");
            let const_node = nodecast!(Const, T_Const, node)?;
            
            // Check if this is a boolean constant
            if (*const_node).consttype == pg_sys::BOOLOID {
                debug_log!("Found boolean constant");
                
                if convert_external_to_special_qual {
                    // Convert boolean constant to HeapExpr: constant = true
                    if let Some(heap_value) = postgres_const_to_heap_value(const_node) {
                        debug_log!("Creating HeapExpr for boolean constant: {:?}", heap_value);
                        
                        // Create HeapExpr using the new expression-based approach
                        let expr_description = format!("Boolean constant = true");
                        let heap_expr = Qual::HeapExpr {
                            expr_node: node, // Use the original T_Const node
                            expr_description,
                            search_query_input: Box::new(SearchQueryInput::All),
                        };
                        
                        debug_log!("Successfully created HeapExpr for boolean constant");
                        *uses_tantivy_to_query = true;
                        return Some(heap_expr);
                    } else {
                        debug_log!("Failed to convert boolean constant to HeapValue");
                    }
                } else {
                    // Handle constants that result from join clause simplification (original logic)
                    if !(*const_node).constisnull {
                        let bool_value = bool::from_datum((*const_node).constvalue, false).unwrap_or(false);
                        if bool_value {
                            return Some(Qual::All);
                        } else {
                            return Some(Qual::Not(Box::new(Qual::All)));
                        }
                    }
                }
            } else {
                debug_log!("T_Const is not a boolean type: {}", (*const_node).consttype);
            }
            
            None
        }

        // we don't understand this clause so we can't do anything
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn list(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    list: *mut pg_sys::List,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
    convert_external_to_special_qual: bool,
    uses_tantivy_to_query: &mut bool,
) -> Option<Vec<Qual>> {
    let args = PgList::<pg_sys::Node>::from_pg(list);
    let mut quals = Vec::new();
    for child in args.iter_ptr() {
        if let Some(qual) = extract_quals(
            root,
            rti,
            child,
            pdbopoid,
            ri_type,
            schema,
            convert_external_to_special_qual,
            uses_tantivy_to_query,
        ) {
            quals.push(qual);
        }
        // If extract_quals returns None, we just skip this child instead of failing
    }
    
    // Only return None if we couldn't extract any quals at all
    if quals.is_empty() {
        None
    } else {
        Some(quals)
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn opexpr(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    opexpr: OpExpr,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
    convert_external_to_special_qual: bool,
    uses_tantivy_to_query: &mut bool,
) -> Option<Qual> {
    let args = opexpr.args();
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
                );
            }

            if is_complex(rhs) {
                return None;
            }

            Some(Qual::ScoreExpr {
                opoid: opexpr.opno(),
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
        ),

        _ => None,
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
    opexpr: OpExpr,
    lhs: *mut pg_sys::Node,
    mut rhs: *mut pg_sys::Node,
    convert_external_to_special_qual: bool,
) -> Option<Qual> {
    while let Some(relabel_target) = nodecast!(RelabelType, T_RelabelType, rhs) {
        rhs = (*relabel_target).arg.cast();
    }

    let rhs_as_const = nodecast!(Const, T_Const, rhs);

    let is_our_operator = opexpr.opno() == pdbopoid;

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
                opno: opexpr.opno(),
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
        // Save the operator OID and node pointer before the move
        let opno = opexpr.opno();
        
        // Save the node pointer before the move so we can recreate the OpExpr later
        let opexpr_node = match &opexpr {
            OpExpr::Array(expr) => *expr as *mut pg_sys::Node,
            OpExpr::Single(expr) => *expr as *mut pg_sys::Node,
        };
        
        // we'll try to convert it into a pushdown
        let pushdown_result = try_pushdown(root, rti, opexpr, schema);
        if pushdown_result.is_none() {
            debug_log!("try_pushdown failed for opexpr, attempting HeapExpr creation");
            
            // Try to create a HeapExpr for non-indexed field comparisons
            // We need the relation OID - get it from the range table
            let rte = pg_sys::rt_fetch(rti, (*(*root).parse).rtable);
            let relation_oid = (*rte).relid;
            
            // Create a description for debugging
            let expr_description = format!("OpExpr with operator OID {}", opno);
            
            // Check if this expression references our relation
            if contains_relation_reference(opexpr_node, rti) {
                debug_log!("Creating HeapExpr with PostgreSQL expression: {}", expr_description);
                
                let heap_expr = Qual::HeapExpr {
                    expr_node: opexpr_node,
                    expr_description,
                    search_query_input: Box::new(SearchQueryInput::All),
                };
                
                debug_log!("Successfully created HeapExpr for non-indexed predicate");
                *uses_tantivy_to_query = true; // We do use search (with heap filtering)
                Some(heap_expr)
            } else if convert_external_to_special_qual {
                debug_log!("OpExpr doesn't reference our relation, falling back to ExternalExpr");
                Some(Qual::ExternalExpr)
            } else {
                debug_log!("OpExpr doesn't reference our relation, returning None");
                None
            }
        } else {
            *uses_tantivy_to_query = true;
            pushdown_result
        }
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
/// Try to convert an OpExpr to a HeapExpr for non-indexed field comparisons
unsafe fn try_create_heap_expr_from_opexpr(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    opexpr: &OpExpr,  // Take a reference to avoid move
    relation_oid: pg_sys::Oid,
) -> Option<Qual> {
    debug_log!("try_create_heap_expr_from_opexpr called with opno: {}", opexpr.opno());
    
    // Get the node pointer from the OpExpr
    let opexpr_node = match opexpr {
        OpExpr::Array(expr) => *expr as *mut pg_sys::Node,
        OpExpr::Single(expr) => *expr as *mut pg_sys::Node,
    };
    
    // Check if this expression references our relation
    if !contains_relation_reference(opexpr_node, rti) {
        debug_log!("OpExpr doesn't reference our relation, skipping");
        return None;
    }
    
    // Create a description for debugging
    let expr_description = format!("OpExpr with operator OID {}", opexpr.opno());
    
    debug_log!("Creating HeapExpr with PostgreSQL expression: {}", expr_description);
    
    Some(Qual::HeapExpr {
        expr_node: opexpr_node,
        expr_description,
        search_query_input: Box::new(SearchQueryInput::All),
    })
}

/// Try to extract a HeapOperand from a PostgreSQL node
unsafe fn try_extract_heap_operand(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    relation_oid: pg_sys::Oid,
) -> Option<HeapOperand> {
    if node.is_null() {
        debug_log!("Node is null");
        return None;
    }
    
    debug_log!("Node type: {:?}", (*node).type_);
    
    match (*node).type_ {
        pg_sys::NodeTag::T_Var => {
            let var = nodecast!(Var, T_Var, node)?;
            debug_log!("Found Var node: varno={}, rti={}, attno={}", (*var).varno, rti, (*var).varattno);
            if (*var).varno as pg_sys::Index == rti {
                // This is a field reference to our relation
                let attno = (*var).varattno;
                let field_name = get_field_name_from_attno(relation_oid, attno)?;
                debug_log!("Field name resolved: {}", field_name);
                Some(HeapOperand::Field { field: field_name, attno })
            } else {
                debug_log!("Var node varno {} doesn't match rti {}", (*var).varno, rti);
                None
            }
        }
        pg_sys::NodeTag::T_Const => {
            let const_node = nodecast!(Const, T_Const, node)?;
            debug_log!("Found Const node: type={}, is_null={}", (*const_node).consttype, (*const_node).constisnull);
            let heap_value = postgres_const_to_heap_value(const_node)?;
            debug_log!("Const value converted to HeapValue");
            Some(HeapOperand::Value(heap_value))
        }
        _ => {
            debug_log!("Unsupported node type for HeapOperand");
            None
        }
    }
}

/// Convert PostgreSQL operator OID to HeapOperator
unsafe fn postgres_oid_to_heap_operator(opno: pg_sys::Oid) -> Option<HeapOperator> {
    // Check common comparison operators
    if opno == operator_oid("=(int4,int4)") || opno == operator_oid("=(int8,int8)") ||
       opno == operator_oid("=(float4,float4)") || opno == operator_oid("=(float8,float8)") ||
       opno == operator_oid("=(text,text)") || opno == operator_oid("=(bool,bool)") ||
       opno == operator_oid("=(numeric,numeric)") ||
       // Cross-type float comparisons
       opno == operator_oid("=(real,double precision)") ||
       opno == operator_oid("=(double precision,real)") {
        Some(HeapOperator::Equal)
    } else if opno == operator_oid(">(int4,int4)") || opno == operator_oid(">(int8,int8)") ||
              opno == operator_oid(">(float4,float4)") || opno == operator_oid(">(float8,float8)") ||
              opno == operator_oid(">(text,text)") || opno == operator_oid(">(numeric,numeric)") ||
              // Cross-type float comparisons
              opno == operator_oid(">(real,double precision)") ||
              opno == operator_oid(">(double precision,real)") {
        Some(HeapOperator::GreaterThan)
    } else if opno == operator_oid("<(int4,int4)") || opno == operator_oid("<(int8,int8)") ||
              opno == operator_oid("<(float4,float4)") || opno == operator_oid("<(float8,float8)") ||
              opno == operator_oid("<(text,text)") || opno == operator_oid("<(numeric,numeric)") ||
              // Cross-type float comparisons
              opno == operator_oid("<(real,double precision)") ||
              opno == operator_oid("<(double precision,real)") {
        Some(HeapOperator::LessThan)
    } else if opno == operator_oid(">=(int4,int4)") || opno == operator_oid(">=(int8,int8)") ||
              opno == operator_oid(">=(float4,float4)") || opno == operator_oid(">=(float8,float8)") ||
              opno == operator_oid(">=(text,text)") || opno == operator_oid(">=(numeric,numeric)") ||
              // Cross-type float comparisons
              opno == operator_oid(">=(real,double precision)") ||
              opno == operator_oid(">=(double precision,real)") {
        Some(HeapOperator::GreaterThanOrEqual)
    } else if opno == operator_oid("<=(int4,int4)") || opno == operator_oid("<=(int8,int8)") ||
              opno == operator_oid("<=(float4,float4)") || opno == operator_oid("<=(float8,float8)") ||
              opno == operator_oid("<=(text,text)") || opno == operator_oid("<=(numeric,numeric)") ||
              // Cross-type float comparisons
              opno == operator_oid("<=(real,double precision)") ||
              opno == operator_oid("<=(double precision,real)") {
        Some(HeapOperator::LessThanOrEqual)
    } else {
        debug_log!("Unsupported operator OID: {}", opno);
        None
    }
}

/// Convert PostgreSQL Const node to HeapValue
unsafe fn postgres_const_to_heap_value(const_node: *mut pg_sys::Const) -> Option<HeapValue> {
    if (*const_node).constisnull {
        return None;
    }
    
    let datum = (*const_node).constvalue;
    let type_oid = (*const_node).consttype;
    
    match type_oid {
        pg_sys::TEXTOID | pg_sys::VARCHAROID => {
            pgrx::FromDatum::from_datum(datum, false).map(|s: String| HeapValue::Text(s))
        }
        pg_sys::INT4OID => {
            pgrx::FromDatum::from_datum(datum, false).map(|i: i32| HeapValue::Integer(i as i64))
        }
        pg_sys::INT8OID => {
            pgrx::FromDatum::from_datum(datum, false).map(|i: i64| HeapValue::Integer(i))
        }
        pg_sys::FLOAT4OID => {
            pgrx::FromDatum::from_datum(datum, false).map(|f: f32| HeapValue::Float(f as f64))
        }
        pg_sys::FLOAT8OID => {
            pgrx::FromDatum::from_datum(datum, false).map(|f: f64| HeapValue::Float(f))
        }
        pg_sys::BOOLOID => {
            pgrx::FromDatum::from_datum(datum, false).map(|b: bool| HeapValue::Boolean(b))
        }
        pg_sys::NUMERICOID => {
            pgrx::AnyNumeric::from_datum(datum, false)
                .map(|numeric| HeapValue::Decimal(numeric.to_string()))
        }
        _ => None,
    }
}

/// Optimize qual tree by converting ExternalVar and ExternalExpr to HeapExpr where possible
/// This is the second pass optimization mentioned in the implementation plan
pub unsafe fn optimize_quals_with_heap_expr(
    qual: &mut Qual,
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    relation_oid: pg_sys::Oid,
) {
    match qual {
        Qual::And(quals) => {
            // Process each qual in the AND
            for q in quals.iter_mut() {
                optimize_quals_with_heap_expr(q, root, rti, relation_oid);
            }
            
            // Try to optimize AND branches by pushing indexed predicates into HeapExpr search_query_input
            optimize_and_branch_with_heap_expr(quals);
        }
        Qual::Or(quals) => {
            // Process each qual in the OR
            for q in quals.iter_mut() {
                optimize_quals_with_heap_expr(q, root, rti, relation_oid);
            }
        }
        Qual::Not(qual) => {
            optimize_quals_with_heap_expr(qual, root, rti, relation_oid);
        }
        Qual::ExternalVar | Qual::ExternalExpr => {
            // Try to convert to HeapExpr
            // For now, we'll keep the original behavior and handle this in a future iteration
            // The actual conversion would require access to the original OpExpr
        }
        _ => {
            // Other qual types don't need optimization
        }
    }
}

/// Optimize AND branches by pushing indexed predicates into HeapExpr search_query_input
unsafe fn optimize_and_branch_with_heap_expr(quals: &mut Vec<Qual>) {
    debug_log!("optimize_and_branch_with_heap_expr called with {} quals", quals.len());
    
    let mut heap_expr_indices = Vec::new();
    let mut indexed_qual_indices = Vec::new();
    
    // Find HeapExpr and indexed quals
    for (i, qual) in quals.iter().enumerate() {
        debug_log!("Qual {}: {:?}", i, std::mem::discriminant(qual));
        match qual {
            Qual::HeapExpr { search_query_input, .. } => {
                debug_log!("Found HeapExpr at index {}", i);
                if matches!(**search_query_input, SearchQueryInput::All) {
                    debug_log!("HeapExpr has All query, adding to heap_expr_indices");
                    heap_expr_indices.push(i);
                }
            }
            Qual::OpExpr { .. } | Qual::PushdownExpr { .. } | 
            Qual::PushdownVarEqTrue { .. } | Qual::PushdownVarEqFalse { .. } |
            Qual::PushdownVarIsTrue { .. } | Qual::PushdownVarIsFalse { .. } |
            Qual::PushdownIsNotNull { .. } => {
                debug_log!("Found indexed qual at index {}", i);
                indexed_qual_indices.push(i);
            }
            Qual::Or(_) => {
                debug_log!("Found Or qual at index {} - this should be treated as indexed!", i);
                indexed_qual_indices.push(i);
            }
            _ => {
                debug_log!("Found other qual type at index {}", i);
            }
        }
    }
    
    debug_log!("Found {} HeapExpr indices and {} indexed qual indices", 
                   heap_expr_indices.len(), indexed_qual_indices.len());
    
    // If we have HeapExpr with All query and indexed predicates, optimize
    if !heap_expr_indices.is_empty() && !indexed_qual_indices.is_empty() {
        debug_log!("Proceeding with optimization");
        // First, collect the indexed queries before mutating quals
        let indexed_queries: Vec<SearchQueryInput> = indexed_qual_indices
            .iter()
            .map(|&i| SearchQueryInput::from(&quals[i]))
            .collect();
        
        // Now update the HeapExpr search_query_input
        for &heap_idx in &heap_expr_indices {
            if let Qual::HeapExpr { search_query_input, .. } = &mut quals[heap_idx] {
                if matches!(**search_query_input, SearchQueryInput::All) {
                    if !indexed_queries.is_empty() {
                        debug_log!("Updating HeapExpr search_query_input with {} indexed queries", indexed_queries.len());
                        *search_query_input = Box::new(SearchQueryInput::Boolean {
                            must: indexed_queries.clone(),
                            should: vec![],
                            must_not: vec![],
                        });
                    }
                }
            }
        }
        
        // Remove the indexed quals that were merged into HeapExpr
        // We need to do this in reverse order to maintain indices
        for &idx in indexed_qual_indices.iter().rev() {
            debug_log!("Removing indexed qual at index {}", idx);
            quals.remove(idx);
        }
        debug_log!("Optimization complete, {} quals remaining", quals.len());
    } else {
        debug_log!("Skipping optimization: not enough quals");
    }
}

/// Try to create a HeapExpr from a NullTest for non-indexed fields
unsafe fn try_create_heap_expr_from_null_test(
    nulltest: *mut pg_sys::NullTest,
    rti: pg_sys::Index,
) -> Option<Qual> {
    debug_log!("try_create_heap_expr_from_null_test called");
    
    // Extract the field being tested
    let arg = (*nulltest).arg;
    if let Some(var) = nodecast!(Var, T_Var, arg) {
        debug_log!("Found Var node, varno: {}, rti: {}", (*var).varno, rti);
        if (*var).varno as pg_sys::Index == rti {
            // This is a field reference to our relation
            let attno = (*var).varattno;
            debug_log!("Creating HeapExpr for NULL test on attno: {}", attno);
            
            let test_type = if (*nulltest).nulltesttype == pg_sys::NullTestType::IS_NULL {
                "IS NULL"
            } else {
                "IS NOT NULL"
            };
            
            let expr_description = format!("NULL test: field_{} {}", attno, test_type);
            debug_log!("Created HeapExpr with description: {}", expr_description);
            
            Some(Qual::HeapExpr {
                expr_node: nulltest as *mut pg_sys::Node,
                expr_description,
                search_query_input: Box::new(SearchQueryInput::All),
            })
        } else {
            debug_log!("Var node varno {} doesn't match rti {}", (*var).varno, rti);
            None
        }
    } else {
        debug_log!("NullTest arg is not a Var node");
        None
    }
}

/// Get field name from attribute number
unsafe fn get_field_name_from_attno(relation_oid: pg_sys::Oid, attno: pg_sys::AttrNumber) -> Option<FieldName> {
    debug_log!("get_field_name_from_attno called with relation_oid: {}, attno: {}", relation_oid, attno);
    
    let relation = pg_sys::RelationIdGetRelation(relation_oid);
    if relation.is_null() {
        debug_log!("Failed to get relation for OID: {}", relation_oid);
        return None;
    }
    
    let tuple_desc = (*relation).rd_att;
    let natts = (*tuple_desc).natts;
    debug_log!("Relation has {} attributes", natts);
    
    if attno <= 0 || (attno as i32) > natts {
        debug_log!("Invalid attno: {} (valid range: 1-{})", attno, natts);
        pg_sys::RelationClose(relation);
        return None;
    }
    
    let attr_slice = (*tuple_desc).attrs.as_slice(natts as usize);
    let form_attr = &attr_slice[(attno - 1) as usize];
    let attr_name = std::ffi::CStr::from_ptr(form_attr.attname.data.as_ptr());
    
    let field_name_str = attr_name.to_str().ok();
    debug_log!("Attribute {} name: {:?}", attno, field_name_str);
    
    let result = field_name_str.map(|s| FieldName::from(s));
    pg_sys::RelationClose(relation);
    result
}

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

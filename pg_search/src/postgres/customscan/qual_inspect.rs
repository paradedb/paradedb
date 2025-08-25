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

use crate::gucs;
use crate::nodecast;
use crate::postgres::customscan::builders::custom_path::RestrictInfoType;
use crate::postgres::customscan::opexpr::OpExpr;
use crate::postgres::customscan::pushdown::{is_complex, try_pushdown_inner, PushdownField};
use crate::postgres::customscan::{operator_oid, score_funcoid};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::var::{find_one_var_and_fieldname, VarContext};
use crate::query::heap_field_filter::HeapFieldFilter;
use crate::query::pdb_query::pdb;
use crate::query::SearchQueryInput;
use pg_sys::BoolExprType;
use pgrx::{pg_guard, pg_sys, FromDatum, IntoDatum, PgList};
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
        expr_desc: String,
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
            Qual::HeapExpr {
                search_query_input, ..
            } => matches!(**search_query_input, SearchQueryInput::All),
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

    /// Check if a query can be satisfied by a partial index
    ///
    /// For a partial index with predicate like "WHERE category = 'Electronics'",
    /// a query like "WHERE description = 'Product 3'" cannot be satisfied because
    /// Product 3 might have category = 'Footwear' and thus wouldn't be in the index.
    ///
    /// This function implements a conservative approach: if the query contains any
    /// non-indexed predicates that could filter out rows that match the partial index
    /// predicate, we cannot use the partial index.
    pub unsafe fn is_query_compatible_with_partial_index(&self) -> bool {
        // For now, implement a simple heuristic:
        // If the query contains HeapExpr (non-indexed predicates), and this is a partial index,
        // we cannot guarantee the query can be satisfied by the partial index alone.
        //
        // TODO(@mdashti): A more sophisticated implementation would:
        // 1. Parse the partial index predicate from bm25_index.rd_indpred
        // 2. Check if the query predicates are compatible with the partial index predicate
        // 3. Use PostgreSQL's constraint exclusion logic
        //
        // For now, we use a conservative approach to fix the immediate bug.

        !self.contains_heap_expr()
    }

    /// Check if a Qual contains any HeapExpr (non-indexed predicates)
    pub fn contains_heap_expr(&self) -> bool {
        match self {
            Qual::HeapExpr { .. } => true,
            Qual::Not(inner) => inner.contains_heap_expr(),
            Qual::And(quals) | Qual::Or(quals) => quals.iter().any(|q| q.contains_heap_expr()),
            _ => false,
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
            Qual::PushdownVarEqTrue { field } => SearchQueryInput::FieldedQuery {
                field: field.attname(),
                query: pdb::Query::Term {
                    value: OwnedValue::Bool(true),
                    is_datetime: false,
                },
            },
            Qual::PushdownVarEqFalse { field } => SearchQueryInput::FieldedQuery {
                field: field.attname(),
                query: pdb::Query::Term {
                    value: OwnedValue::Bool(false),
                    is_datetime: false,
                },
            },
            Qual::PushdownVarIsTrue { field } => SearchQueryInput::FieldedQuery {
                field: field.attname(),
                query: pdb::Query::Term {
                    value: OwnedValue::Bool(true),
                    is_datetime: false,
                },
            },
            Qual::PushdownVarIsFalse { field } => SearchQueryInput::FieldedQuery {
                field: field.attname(),
                query: pdb::Query::Term {
                    value: OwnedValue::Bool(false),
                    is_datetime: false,
                },
            },
            Qual::PushdownIsNotNull { field } => SearchQueryInput::FieldedQuery {
                field: field.attname(),
                query: pdb::Query::Exists,
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
            Qual::HeapExpr {
                expr_node,
                expr_desc,
                search_query_input,
            } => {
                // Create HeapFieldFilter from the PostgreSQL expression
                let field_filters =
                    vec![unsafe { HeapFieldFilter::new(*expr_node, expr_desc.clone()) }];

                SearchQueryInput::HeapFilter {
                    indexed_query: search_query_input.clone(),
                    field_filters,
                }
            }
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

#[derive(Default)]
pub struct QualExtractState {
    pub uses_tantivy_to_query: bool,
    pub uses_our_operator: bool,
    pub uses_heap_expr: bool,
}

#[allow(clippy::too_many_arguments)]
pub unsafe fn extract_quals(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    indexrel: &PgSearchRelation,
    convert_external_to_special_qual: bool,
    state: &mut QualExtractState,
) -> Option<Qual> {
    if node.is_null() {
        return None;
    }

    let schema = indexrel.schema().ok()?;
    match (*node).type_ {
        pg_sys::NodeTag::T_List => {
            let mut quals = list(
                root,
                rti,
                node.cast(),
                pdbopoid,
                ri_type,
                indexrel,
                convert_external_to_special_qual,
                state,
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
                indexrel,
                convert_external_to_special_qual,
                state,
            )
        }

        pg_sys::NodeTag::T_OpExpr => opexpr(
            root,
            rti,
            OpExpr::from_single(node)?,
            pdbopoid,
            ri_type,
            indexrel,
            convert_external_to_special_qual,
            state,
        ),

        pg_sys::NodeTag::T_ScalarArrayOpExpr => opexpr(
            root,
            rti,
            OpExpr::from_array(node)?,
            pdbopoid,
            ri_type,
            indexrel,
            convert_external_to_special_qual,
            state,
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
                indexrel,
                convert_external_to_special_qual,
                state,
            )?;

            match (*boolexpr).boolop {
                pg_sys::BoolExprType::AND_EXPR => Some(Qual::And(quals)),
                pg_sys::BoolExprType::OR_EXPR => Some(Qual::Or(quals)),
                pg_sys::BoolExprType::NOT_EXPR => Some(Qual::Not(Box::new(quals.pop()?))),
                _ => panic!("unexpected `BoolExprType`: {}", (*boolexpr).boolop),
            }
        }

        pg_sys::NodeTag::T_Var => {
            // First, try to create a PushdownField to see if this is an indexed boolean field
            if let Some(field) = PushdownField::try_new(root, node, indexrel) {
                // Check if this is a boolean field reference to our relation
                if field.varno() != rti {
                    return None;
                }

                if let Some(search_field) = schema.search_field(field.attname()) {
                    if search_field.is_fast() {
                        // This is an indexed boolean field, create proper pushdown qual
                        // T_Var alone represents "field = true"
                        state.uses_tantivy_to_query = true;
                        return Some(Qual::PushdownVarEqTrue { field });
                    }
                }
            }

            // If we reach here, the field is not indexed or not fast, so create HeapExpr
            // T_Var nodes represent boolean field references without explicit "= true" comparison
            // PostgreSQL parser generates T_Var for "WHERE bool_field" vs T_OpExpr for "WHERE bool_field = true"
            // We need to handle both cases since they're semantically equivalent
            let var_node = nodecast!(Var, T_Var, node)?;
            try_create_heap_expr_from_var(root, var_node, rti, &mut state.uses_tantivy_to_query)
        }

        pg_sys::NodeTag::T_NullTest => {
            let nulltest = nodecast!(NullTest, T_NullTest, node)?;
            // TODO(@mdashti): we can use if-let chains here
            if let Some(field) = PushdownField::try_new(root, (*nulltest).arg.cast(), indexrel) {
                if let Some(search_field) = schema.search_field(field.attname().root()) {
                    if search_field.is_fast() {
                        if (*nulltest).nulltesttype == pg_sys::NullTestType::IS_NOT_NULL {
                            return Some(Qual::PushdownIsNotNull { field });
                        } else {
                            return Some(Qual::Not(Box::new(Qual::PushdownIsNotNull { field })));
                        }
                    } else {
                        // Field is not fast, try creating HeapExpr
                    }
                } else {
                    // Field not found in schema, try creating HeapExpr
                }
            } else {
                // Try to create a HeapExpr for non-indexed field NULL tests
            }
            try_create_heap_expr_from_null_test(
                nulltest,
                rti,
                root,
                &mut state.uses_tantivy_to_query,
            )
        }

        pg_sys::NodeTag::T_BooleanTest => booltest(
            root,
            rti,
            node,
            ri_type,
            indexrel,
            convert_external_to_special_qual,
            state,
        ),

        pg_sys::NodeTag::T_Const => {
            let const_node = nodecast!(Const, T_Const, node)?;

            // Check if this is a boolean constant
            if (*const_node).consttype == pg_sys::BOOLOID {
                let bool_value = if !(*const_node).constisnull {
                    bool::from_datum((*const_node).constvalue, false).unwrap_or(false)
                } else {
                    // Convert NULL to false
                    false
                };

                state.uses_tantivy_to_query = true;
                if bool_value {
                    return Some(Qual::All);
                } else {
                    return Some(Qual::Not(Box::new(Qual::All)));
                }
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
    indexrel: &PgSearchRelation,
    convert_external_to_special_qual: bool,
    state: &mut QualExtractState,
) -> Option<Vec<Qual>> {
    let args = PgList::<pg_sys::Node>::from_pg(list);
    let mut quals = Vec::new();
    for child in args.iter_ptr() {
        quals.push(extract_quals(
            root,
            rti,
            child,
            pdbopoid,
            ri_type,
            indexrel,
            convert_external_to_special_qual,
            state,
        )?)
    }

    Some(quals)
}

#[allow(clippy::too_many_arguments)]
unsafe fn opexpr(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    opexpr: OpExpr,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    indexrel: &PgSearchRelation,
    convert_external_to_special_qual: bool,
    state: &mut QualExtractState,
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
            indexrel,
            state,
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
                    indexrel,
                    state,
                    opexpr,
                    lhs,
                    rhs,
                    convert_external_to_special_qual,
                );
            }

            state.uses_our_operator = true;

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
            indexrel,
            state,
            opexpr,
            lhs,
            rhs,
            convert_external_to_special_qual,
        ),

        _ => {
            // it doesn't use our operator.
            // we'll try to convert it into a pushdown
            try_pushdown(
                root,
                rti,
                opexpr,
                indexrel,
                state,
                convert_external_to_special_qual,
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn node_opexpr(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    indexrel: &PgSearchRelation,
    state: &mut QualExtractState,
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
    state.uses_our_operator = state.uses_our_operator || is_our_operator;

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
                state.uses_tantivy_to_query = true;
                return Some(Qual::Expr {
                    node: rhs,
                    expr_state: std::ptr::null_mut(),
                });
            }
        } else {
            // it doesn't use our operator
            if contains_var(rhs) {
                // the rhs is (or contains) a Var. If it's part of a join condition,
                // select everything in this situation
                if convert_external_to_special_qual {
                    return Some(Qual::ExternalVar);
                } else {
                    return None;
                }
            } else {
                // it doesn't use our operator.
                // we'll try to convert it into a pushdown
                return try_pushdown(
                    root,
                    rti,
                    opexpr,
                    indexrel,
                    state,
                    convert_external_to_special_qual,
                );
            }
        }
    }

    let rhs = rhs_as_const?;
    if is_our_operator {
        // the rhs expression is a Const, so we can use it directly
        if is_node_range_table_entry(lhs, rti) {
            // the node comes from this range table entry, so we can use the full expression directly
            state.uses_tantivy_to_query = true;
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
        // we'll try to convert it into a pushdown
        try_pushdown(
            root,
            rti,
            opexpr,
            indexrel,
            state,
            convert_external_to_special_qual,
        )
    }
}

/// Critical decision point: determines whether a predicate can be pushed down to the index
/// or must be evaluated via heap access.
///
/// This function attempts to convert PostgreSQL OpExpr nodes into indexed predicates.
/// If the predicate can be satisfied using indexed fields (fast fields, search fields),
/// it returns an indexed Qual (OpExpr, PushdownExpr, etc.).
/// If the predicate references non-indexed fields, it returns a HeapExpr that will
/// evaluate the predicate against heap tuples.
///
/// The decision made here directly impacts query performance:
/// - Indexed predicates: Fast evaluation using Tantivy's index structures
/// - HeapExpr predicates: Slower evaluation requiring heap tuple access
unsafe fn try_pushdown(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    opexpr: OpExpr,
    indexrel: &PgSearchRelation,
    state: &mut QualExtractState,
    convert_external_to_special_qual: bool,
) -> Option<Qual> {
    // Save the operator OID and node pointer before the move
    let opno = opexpr.opno();

    // Save the node pointer before the move so we can recreate the OpExpr later
    let opexpr_node = match &opexpr {
        OpExpr::Array(expr) => *expr as *mut pg_sys::Node,
        OpExpr::Single(expr) => *expr as *mut pg_sys::Node,
    };

    // Try to convert this OpExpr into an indexed predicate (fast field, search field, etc.)
    let pushdown_result = try_pushdown_inner(root, rti, opexpr, indexrel);

    if pushdown_result.is_none() {
        // DECISION POINT: Predicate cannot be pushed down to index
        // Check if this expression references our relation
        if contains_relation_reference(opexpr_node, rti) {
            // Check if custom scan for non-indexed fields is enabled
            if !gucs::enable_filter_pushdown() {
                return None;
            }

            // Check if the expression contains subqueries (EXEC params)
            // Subqueries require proper executor context which we don't have in HeapFieldFilter
            if contains_exec_param(opexpr_node) {
                return None;
            }

            // We do use search (with heap filtering)
            state.uses_heap_expr = true;
            state.uses_tantivy_to_query = true;

            // Create HeapExpr: predicate will be evaluated via heap access
            // This is slower but necessary for non-indexed fields
            Some(Qual::HeapExpr {
                expr_node: opexpr_node,
                expr_desc: format!("OpExpr with operator OID {opno}"),
                search_query_input: Box::new(SearchQueryInput::All),
            })
        } else if convert_external_to_special_qual {
            Some(Qual::ExternalExpr)
        } else {
            None
        }
    } else {
        // SUCCESS: Predicate can be pushed down to index for fast evaluation
        state.uses_tantivy_to_query = true;
        pushdown_result
    }
}

unsafe fn is_node_range_table_entry(node: *mut pg_sys::Node, rti: pg_sys::Index) -> bool {
    match (*node).type_ {
        pg_sys::NodeTag::T_Var => {
            let var = node.cast::<pg_sys::Var>();
            (*var).varno as pg_sys::Index == rti
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
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        _data: *mut core::ffi::c_void,
    ) -> bool {
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
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        _data: *mut core::ffi::c_void,
    ) -> bool {
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
    ri_type: RestrictInfoType,
    indexrel: &PgSearchRelation,
    convert_external_to_special_qual: bool,
    state: &mut QualExtractState,
) -> Option<Qual> {
    let booltest = nodecast!(BooleanTest, T_BooleanTest, node)?;
    let arg = (*booltest).arg;

    // We only support boolean test for simple field references (Var nodes)
    // For complex expressions, the optimizer will evaluate the condition later
    let field = PushdownField::try_new(root, arg as *mut pg_sys::Node, indexrel)?;

    // It's a simple field reference, handle as specific cases
    let qual = match (*booltest).booltesttype {
        pg_sys::BoolTestType::IS_TRUE => Some(Qual::PushdownVarIsTrue { field }),
        pg_sys::BoolTestType::IS_NOT_FALSE => {
            Some(Qual::Not(Box::new(Qual::PushdownVarIsFalse { field })))
        }
        pg_sys::BoolTestType::IS_FALSE => Some(Qual::PushdownVarIsFalse { field }),
        pg_sys::BoolTestType::IS_NOT_TRUE => {
            Some(Qual::Not(Box::new(Qual::PushdownVarIsTrue { field })))
        }
        _ => None,
    };

    if qual.is_some() {
        state.uses_tantivy_to_query = true;
    }

    qual
}

/// Extract join-level search predicates that are relevant for snippet/score generation
/// This captures search predicates that reference specific fields but may not be
/// pushed down to the current scan due to join conditions.
/// Returns the entire simplified Boolean expression to preserve OR structures.
pub unsafe fn extract_join_predicates(
    root: *mut pg_sys::PlannerInfo,
    current_rti: pg_sys::Index,
    pdbopoid: pg_sys::Oid,
    indexrel: &PgSearchRelation,
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
            let mut qual_extract_state = QualExtractState::default();
            // Extract search predicates from the simplified expression
            if let Some(qual) = extract_quals(
                root,
                current_rti,
                simplified_node.cast(),
                pdbopoid,
                RestrictInfoType::BaseRelation,
                indexrel,
                true,
                &mut qual_extract_state,
            ) {
                if qual_extract_state.uses_our_operator {
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
        pg_sys::NodeTag::T_OpExpr => simplify_node_for_relation(node, current_rti),

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

        _ => simplify_node_for_relation(node, current_rti),
    }
}

unsafe fn simplify_node_for_relation(
    node: *mut pg_sys::Node,
    current_rti: pg_sys::Index,
) -> Option<*mut pg_sys::Node> {
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

    #[pg_guard]
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

/// Optimize qual tree by converting ExternalVar and ExternalExpr to HeapExpr where possible
/// This is the second pass optimization mentioned in the implementation plan
pub unsafe fn optimize_quals_with_heap_expr(qual: &mut Qual) {
    match qual {
        Qual::And(quals) => {
            // Process each qual in the AND
            for q in quals.iter_mut() {
                optimize_quals_with_heap_expr(q);
            }

            // Try to optimize AND branches by pushing indexed predicates into HeapExpr search_query_input
            optimize_and_branch_with_heap_expr(quals);
        }
        Qual::Or(quals) => {
            // Process each qual in the OR
            for q in quals.iter_mut() {
                optimize_quals_with_heap_expr(q);
            }
        }
        Qual::Not(qual) => {
            optimize_quals_with_heap_expr(qual);
        }
        Qual::ExternalVar | Qual::ExternalExpr => {
            // For ExternalVar and ExternalExpr, we cannot apply any optimization, as we don't know
            // what the underlying predicate is.
        }
        _ => {
            // Other qual types don't need optimization
        }
    }
}

/// Optimize AND branches by pushing indexed predicates into HeapExpr search_query_input
unsafe fn optimize_and_branch_with_heap_expr(quals: &mut Vec<Qual>) {
    let mut heap_expr_indices = Vec::new();
    let mut indexed_qual_indices = Vec::new();

    // Find HeapExpr and indexed quals
    for (i, qual) in quals.iter().enumerate() {
        match qual {
            Qual::HeapExpr {
                search_query_input, ..
            } => {
                if matches!(**search_query_input, SearchQueryInput::All) {
                    heap_expr_indices.push(i);
                }
            }
            Qual::OpExpr { .. }
            | Qual::PushdownExpr { .. }
            | Qual::PushdownVarEqTrue { .. }
            | Qual::PushdownVarEqFalse { .. }
            | Qual::PushdownVarIsTrue { .. }
            | Qual::PushdownVarIsFalse { .. }
            | Qual::PushdownIsNotNull { .. } => {
                indexed_qual_indices.push(i);
            }
            Qual::Or(_) => {
                indexed_qual_indices.push(i);
            }
            _ => {}
        }
    }

    // If we have HeapExpr with All query and indexed predicates, optimize
    if !heap_expr_indices.is_empty() && !indexed_qual_indices.is_empty() {
        // First, collect the indexed queries before mutating quals
        let indexed_queries: Vec<SearchQueryInput> = indexed_qual_indices
            .iter()
            .map(|&i| SearchQueryInput::from(&quals[i]))
            .collect();

        // Now update the HeapExpr search_query_input
        for &heap_idx in &heap_expr_indices {
            if let Qual::HeapExpr {
                search_query_input, ..
            } = &mut quals[heap_idx]
            {
                if matches!(**search_query_input, SearchQueryInput::All)
                    && !indexed_queries.is_empty()
                {
                    *search_query_input = Box::new(SearchQueryInput::Boolean {
                        must: indexed_queries.clone(),
                        should: vec![],
                        must_not: vec![],
                    });
                }
            }
        }

        // Remove the indexed quals that were merged into HeapExpr
        // We need to do this in reverse order to maintain indices
        for &idx in indexed_qual_indices.iter().rev() {
            quals.remove(idx);
        }
    }
}

/// Create a HeapExpr for a non-indexed field expression
/// This is a common pattern for expressions that reference fields in our relation
/// but cannot be pushed down to the index
unsafe fn create_heap_expr_for_field_ref(
    expr_node: *mut pg_sys::Node,
    var_node: *mut pg_sys::Var,
    rti: pg_sys::Index,
    expr_desc: String,
    uses_tantivy_to_query: &mut bool,
) -> Option<Qual> {
    if (*var_node).varno as pg_sys::Index == rti {
        // Check if custom scan for non-indexed fields is enabled
        if !gucs::enable_filter_pushdown() {
            return None;
        }
        *uses_tantivy_to_query = true;
        Some(Qual::HeapExpr {
            expr_node,
            expr_desc,
            search_query_input: Box::new(SearchQueryInput::All),
        })
    } else {
        None
    }
}

/// Try to create a HeapExpr from a Var node for non-indexed fields
unsafe fn try_create_heap_expr_from_var(
    root: *mut pg_sys::PlannerInfo,
    var_node: *mut pg_sys::Var,
    rti: pg_sys::Index,
    uses_tantivy_to_query: &mut bool,
) -> Option<Qual> {
    // Check if root and parse are valid
    if root.is_null() || (*root).parse.is_null() {
        return None;
    }

    let attno = (*var_node).varattno;
    create_heap_expr_for_field_ref(
        var_node as *mut pg_sys::Node,
        var_node,
        rti,
        format!("Boolean field_{attno} = true"),
        uses_tantivy_to_query,
    )
}

/// Try to create a HeapExpr from a NullTest for non-indexed fields
unsafe fn try_create_heap_expr_from_null_test(
    nulltest: *mut pg_sys::NullTest,
    rti: pg_sys::Index,
    root: *mut pg_sys::PlannerInfo,
    uses_tantivy_to_query: &mut bool,
) -> Option<Qual> {
    // Extract the field being tested
    let arg = (*nulltest).arg;
    if let Some((var, fieldname)) =
        find_one_var_and_fieldname(VarContext::from_planner(root), arg as *mut pg_sys::Node)
    {
        let attno = (*var).varattno;
        let test_type = if (*nulltest).nulltesttype == pg_sys::NullTestType::IS_NULL {
            "IS NULL"
        } else {
            "IS NOT NULL"
        };

        create_heap_expr_for_field_ref(
            nulltest as *mut pg_sys::Node,
            var,
            rti,
            format!("NULL test: field_{attno} {test_type}"),
            uses_tantivy_to_query,
        )
    } else {
        None
    }
}

unsafe fn contains_any_relation_reference(node: *mut pg_sys::Node) -> bool {
    if node.is_null() {
        return false;
    }

    #[pg_guard]
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
        let want = SearchQueryInput::FieldedQuery {
            field: "foo".into(),
            query: pdb::Query::Term {
                value: OwnedValue::Bool(true),
                is_datetime: false,
            },
        };
        assert_eq!(got, want);
    }

    #[pg_test]
    fn test_pushdown_var_eq_false() {
        let qual = Qual::PushdownVarEqFalse {
            field: PushdownField::new("bar"),
        };
        let got = SearchQueryInput::from(&qual);
        let want = SearchQueryInput::FieldedQuery {
            field: "bar".into(),
            query: pdb::Query::Term {
                value: OwnedValue::Bool(false),
                is_datetime: false,
            },
        };
        assert_eq!(got, want);
    }

    #[pg_test]
    fn test_pushdown_var_is_true() {
        let qual = Qual::PushdownVarIsTrue {
            field: PushdownField::new("baz"),
        };
        let got = SearchQueryInput::from(&qual);
        let want = SearchQueryInput::FieldedQuery {
            field: "baz".into(),
            query: pdb::Query::Term {
                value: OwnedValue::Bool(true),
                is_datetime: false,
            },
        };
        assert_eq!(got, want);
    }

    #[pg_test]
    fn test_pushdown_var_is_false() {
        let qual = Qual::PushdownVarIsFalse {
            field: PushdownField::new("qux"),
        };
        let got = SearchQueryInput::from(&qual);
        let want = SearchQueryInput::FieldedQuery {
            field: "qux".into(),
            query: pdb::Query::Term {
                value: OwnedValue::Bool(false),
                is_datetime: false,
            },
        };
        assert_eq!(got, want);
    }

    #[pg_test]
    fn test_pushdown_is_not_null() {
        let qual = Qual::PushdownIsNotNull {
            field: PushdownField::new("fld"),
        };
        let got = SearchQueryInput::from(&qual);
        let want = SearchQueryInput::FieldedQuery {
            field: "fld".into(),
            query: pdb::Query::Exists,
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
                SearchQueryInput::FieldedQuery {
                    field: f,
                    query: pdb::Query::Term { value, .. },
                },
            ) => field.attname() == *f && matches!(value, OwnedValue::Bool(true)),

            // Match boolean field FALSE cases
            (
                qual @ (Qual::PushdownVarEqFalse { field } | Qual::PushdownVarIsFalse { field }),
                SearchQueryInput::FieldedQuery {
                    field: f,
                    query: pdb::Query::Term { value, .. },
                },
            ) => field.attname() == *f && matches!(value, OwnedValue::Bool(false)),

            // Match IS NOT NULL
            (
                Qual::PushdownIsNotNull { field },
                SearchQueryInput::FieldedQuery {
                    field: f,
                    query: pdb::Query::Exists,
                },
            ) => field.attname() == *f,

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
                SearchQueryInput::FieldedQuery {
                    field: f,
                    query:
                        pdb::Query::Term {
                            value: OwnedValue::Bool(false),
                            ..
                        },
                },
            ) if matches!(**inner, Qual::PushdownVarEqTrue { field: ref a } if a.attname() == *f) => {
                true
            }

            // Match negation of PushdownVarEqFalse mapping to PushdownVarEqTrue
            (
                Qual::Not(inner),
                SearchQueryInput::FieldedQuery {
                    field: f,
                    query:
                        pdb::Query::Term {
                            value: OwnedValue::Bool(true),
                            ..
                        },
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

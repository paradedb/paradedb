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

use crate::api::operator::{anyelement_query_input_opoid, searchqueryinput_typoid};
use crate::gucs;
use crate::nodecast;
use crate::postgres::customscan::builders::custom_path::RestrictInfoType;
use crate::postgres::customscan::opexpr::OpExpr;
use crate::postgres::customscan::pushdown::{is_complex, try_pushdown_inner, PushdownField};
use crate::postgres::customscan::{operator_oid, score_funcoids};
use crate::postgres::deparse::deparse_expr;
use crate::postgres::rel::PgSearchRelation;
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
        /// For ScalarArrayOpExpr (e.g., `field @@@ ANY(array)`):
        /// - Some(true) = OR semantics (ANY)
        /// - Some(false) = AND semantics (ALL)
        /// - None = regular OpExpr, not a ScalarArrayOpExpr
        scalar_array_use_or: Option<bool>,
    },
    /// Represents an expression which can be evaluated after planning in BeginCustomScan.
    ///
    /// This happens when the expression involves parameters (e.g. `$1`), other columns, or
    /// volatile functions which are "uncorrelated": i.e., which do not come from an outer
    /// relation.
    ///
    /// It is converted to `SearchQueryInput::PostgresExpression` and then solved by
    /// `solve_postgres_expressions` before the search query is executed.
    Expr {
        node: *mut pg_sys::Node,
        expr_desc: String,
    },
    /// Represents an expression that can be evaluated at planning time.
    /// This typically happens when the expression is effectively constant (no vars/params).
    /// It is evaluated immediately during conversion to `SearchQueryInput` via
    /// `SearchQueryInput::from`.
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

    pub unsafe fn contains_correlated_param(&self, root: *mut pg_sys::PlannerInfo) -> bool {
        match self {
            Qual::All => false,
            Qual::ExternalVar => false,
            Qual::ExternalExpr => false,
            Qual::OpExpr { .. } => false,
            Qual::Expr { node, .. } => contains_correlated_param(root, *node),
            Qual::PushdownExpr { .. } => false,
            Qual::PushdownVarEqTrue { .. } => false,
            Qual::PushdownVarEqFalse { .. } => false,
            Qual::PushdownVarIsTrue { .. } => false,
            Qual::PushdownVarIsFalse { .. } => false,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => false,
            Qual::HeapExpr { expr_node, .. } => contains_correlated_param(root, *expr_node),
            Qual::And(quals) => quals.iter().any(|q| q.contains_correlated_param(root)),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_correlated_param(root)),
            Qual::Not(qual) => qual.contains_correlated_param(root),
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
            // Handle ScalarArrayOpExpr: PostgreSQL 18+ rewrites OR clauses like
            // `field @@@ 'a' OR field @@@ 'b'` into `field @@@ ANY(ARRAY['a','b'])`.
            // We decode the array and convert it to a Boolean query (should/must).
            Qual::OpExpr {
                val,
                scalar_array_use_or,
                ..
            } => unsafe {
                if let Some(use_or) = *scalar_array_use_or {
                    let elements: Vec<SearchQueryInput> = pgrx::FromDatum::from_polymorphic_datum(
                        (**val).constvalue,
                        (**val).constisnull,
                        searchqueryinput_typoid(),
                    )
                    .expect("ScalarArrayOpExpr should not contain NULL");

                    if elements.is_empty() {
                        if use_or {
                            SearchQueryInput::Empty
                        } else {
                            SearchQueryInput::All
                        }
                    } else {
                        let (must, should) = if use_or {
                            (Vec::new(), elements)
                        } else {
                            (elements, Vec::new())
                        };

                        SearchQueryInput::Boolean {
                            must,
                            should,
                            must_not: vec![],
                        }
                    }
                } else {
                    SearchQueryInput::from_datum((**val).constvalue, (**val).constisnull)
                        .expect("rhs of @@@ operator Qual must not be null")
                }
            },
            // Convert to SearchQueryInput::PostgresExpression, which will be solved by
            // `solve_postgres_expressions`.
            Qual::Expr { node, expr_desc } => {
                SearchQueryInput::postgres_expression(*node, expr_desc.clone())
            }
            // Solve the expression immediately to produce a concrete SearchQueryInput
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
                while let Some(SearchQueryInput::ScoreFilter { bounds, query: _ }) =
                    must_scores.pop()
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

/// Context for extracting quals - can be either full planner context or just a query
pub enum PlannerContext {
    /// Full planner context with PlannerInfo (supports join qual extraction)
    Planner(*mut pg_sys::PlannerInfo),
    /// Query-only context (no join qual extraction, used in planner hook)
    /// We don't store the Query pointer because we don't need it - we only need
    /// to know that we're in Query context (not Planner context)
    Query,
}

impl PlannerContext {
    pub fn from_planner(root: *mut pg_sys::PlannerInfo) -> Self {
        Self::Planner(root)
    }

    pub fn from_query(_parse: *mut pg_sys::Query) -> Self {
        Self::Query
    }

    /// Get the PlannerInfo pointer if available (for join qual extraction)
    pub fn planner_info(&self) -> Option<*mut pg_sys::PlannerInfo> {
        match self {
            Self::Planner(root) => Some(*root),
            Self::Query => None,
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
    context: &PlannerContext,
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    indexrel: &PgSearchRelation,
    convert_external_to_special_qual: bool,
    state: &mut QualExtractState,
    attempt_pushdown: bool,
) -> Option<Qual> {
    if node.is_null() {
        return None;
    }

    match (*node).type_ {
        pg_sys::NodeTag::T_FuncExpr => {
            // Standalone FuncExprs in a WHERE clause must return boolean (e.g. ST_DWithin).
            // This is distinct from FuncExprs used inside comparisons (e.g. pdb.score(id) > 0.5),
            // which are handled within opexpr().
            if contains_relation_reference(node, rti) {
                if !gucs::enable_filter_pushdown() {
                    return None;
                }

                state.uses_heap_expr = true;
                state.uses_tantivy_to_query = true;
                Some(Qual::HeapExpr {
                    expr_node: node,
                    expr_desc: deparse_expr(Some(context), indexrel, node),
                    search_query_input: Box::new(SearchQueryInput::All),
                })
            } else {
                None
            }
        }

        pg_sys::NodeTag::T_List => {
            let mut quals = list(
                context,
                rti,
                node.cast(),
                pdbopoid,
                ri_type,
                indexrel,
                convert_external_to_special_qual,
                state,
                attempt_pushdown,
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
                context,
                rti,
                clause.cast(),
                pdbopoid,
                ri_type,
                indexrel,
                convert_external_to_special_qual,
                state,
                attempt_pushdown,
            )
        }

        pg_sys::NodeTag::T_OpExpr => opexpr(
            context,
            rti,
            OpExpr::from_single(node)?,
            pdbopoid,
            ri_type,
            indexrel,
            convert_external_to_special_qual,
            state,
            attempt_pushdown,
        ),

        pg_sys::NodeTag::T_ScalarArrayOpExpr => opexpr(
            context,
            rti,
            OpExpr::from_array(node)?,
            pdbopoid,
            ri_type,
            indexrel,
            convert_external_to_special_qual,
            state,
            attempt_pushdown,
        ),

        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = nodecast!(BoolExpr, T_BoolExpr, node)?;
            let mut quals = list(
                context,
                rti,
                (*boolexpr).args,
                pdbopoid,
                ri_type,
                indexrel,
                convert_external_to_special_qual,
                state,
                attempt_pushdown,
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
            // Note: PushdownField::try_new requires PlannerInfo
            if let Some(root) = context.planner_info() {
                if let Some(field) = PushdownField::try_new(root, node, indexrel) {
                    // Check if this is a boolean field reference to our relation
                    if field.varno() != rti {
                        return None;
                    }

                    if let Some(search_field) =
                        indexrel.schema().ok()?.search_field(field.attname())
                    {
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
                try_create_heap_expr_from_var(
                    root,
                    var_node,
                    rti,
                    indexrel,
                    &mut state.uses_tantivy_to_query,
                )
            } else {
                // Query context: We can't do full pushdown analysis without PlannerInfo,
                // but we can still create HeapExpr if filter_pushdown is enabled
                if !gucs::enable_filter_pushdown() {
                    return None;
                }

                let var_node = nodecast!(Var, T_Var, node)?;
                // Check if this var references our relation
                if (*var_node).varno as pg_sys::Index != rti {
                    return None;
                }

                // We're creating a HeapExpr here - this is a "guess" that it will be needed,
                // but it's safe because filter_pushdown is enabled, which means PostgreSQL's
                // executor will handle the filtering if this predicate can't be pushed down.
                state.uses_heap_expr = true;
                state.uses_tantivy_to_query = true;
                Some(Qual::HeapExpr {
                    expr_node: node,
                    expr_desc: deparse_expr(Some(context), indexrel, node),
                    search_query_input: Box::new(SearchQueryInput::All),
                })
            }
        }

        pg_sys::NodeTag::T_NullTest => {
            let nulltest = nodecast!(NullTest, T_NullTest, node)?;
            // Note: PushdownField::try_new requires PlannerInfo
            if let Some(root) = context.planner_info() {
                if let Some(field) = PushdownField::try_new(root, (*nulltest).arg.cast(), indexrel)
                {
                    if let Some(search_field) =
                        indexrel.schema().ok()?.search_field(field.attname().root())
                    {
                        if search_field.is_fast() {
                            if (*nulltest).nulltesttype == pg_sys::NullTestType::IS_NOT_NULL {
                                return Some(Qual::PushdownIsNotNull { field });
                            } else {
                                return Some(Qual::Not(Box::new(Qual::PushdownIsNotNull {
                                    field,
                                })));
                            }
                        }
                    }
                }
                // If we reach here, try creating HeapExpr
                try_create_heap_expr_from_null_test(
                    nulltest,
                    rti,
                    root,
                    indexrel,
                    &mut state.uses_tantivy_to_query,
                )
            } else {
                // Query context: We can't do full pushdown analysis without PlannerInfo,
                // but we can still create HeapExpr if filter_pushdown is enabled
                if !gucs::enable_filter_pushdown() {
                    return None;
                }

                // We're creating a HeapExpr here - this is a "guess" that it will be needed,
                // but it's safe because filter_pushdown is enabled, which means PostgreSQL's
                // executor will handle the filtering if this predicate can't be pushed down.
                state.uses_heap_expr = true;
                state.uses_tantivy_to_query = true;
                Some(Qual::HeapExpr {
                    expr_node: node,
                    expr_desc: deparse_expr(Some(context), indexrel, node),
                    search_query_input: Box::new(SearchQueryInput::All),
                })
            }
        }

        pg_sys::NodeTag::T_BooleanTest => booltest(context, node, indexrel, state),

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
    context: &PlannerContext,
    rti: pg_sys::Index,
    list: *mut pg_sys::List,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    indexrel: &PgSearchRelation,
    convert_external_to_special_qual: bool,
    state: &mut QualExtractState,
    attempt_pushdown: bool,
) -> Option<Vec<Qual>> {
    let args = PgList::<pg_sys::Node>::from_pg(list);
    let mut quals = Vec::new();
    for child in args.iter_ptr() {
        quals.push(extract_quals(
            context,
            rti,
            child,
            pdbopoid,
            ri_type,
            indexrel,
            convert_external_to_special_qual,
            state,
            attempt_pushdown,
        )?)
    }

    Some(quals)
}

#[allow(clippy::too_many_arguments)]
unsafe fn opexpr(
    context: &PlannerContext,
    rti: pg_sys::Index,
    opexpr: OpExpr,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    indexrel: &PgSearchRelation,
    convert_external_to_special_qual: bool,
    state: &mut QualExtractState,
    attempt_pushdown: bool,
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
            context,
            rti,
            pdbopoid,
            ri_type,
            indexrel,
            state,
            opexpr,
            lhs,
            rhs,
            convert_external_to_special_qual,
            attempt_pushdown,
        ),

        pg_sys::NodeTag::T_FuncExpr => {
            // direct support for pdb.score() in the WHERE clause
            let funcexpr = nodecast!(FuncExpr, T_FuncExpr, lhs)?;
            if !score_funcoids().contains(&(*funcexpr).funcid) {
                return node_opexpr(
                    context,
                    rti,
                    pdbopoid,
                    ri_type,
                    indexrel,
                    state,
                    opexpr,
                    lhs,
                    rhs,
                    convert_external_to_special_qual,
                    attempt_pushdown,
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
        pg_sys::NodeTag::T_PlaceHolderVar => {
            // PlaceHolderVar may wrap a score function when the query has joins or aggregates.
            // We need to unwrap it to check if it's a score expression.
            let phv = nodecast!(PlaceHolderVar, T_PlaceHolderVar, lhs)?;
            let phexpr = (*phv).phexpr;
            if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, phexpr) {
                if score_funcoids().contains(&(*funcexpr).funcid) {
                    state.uses_our_operator = true;

                    if is_complex(rhs) {
                        return None;
                    }

                    return Some(Qual::ScoreExpr {
                        opoid: opexpr.opno(),
                        value: rhs,
                    });
                }
            }
            // Not a score function - fall through to pushdown
            if attempt_pushdown {
                try_pushdown(
                    context,
                    rti,
                    opexpr,
                    indexrel,
                    state,
                    convert_external_to_special_qual,
                )
            } else {
                None
            }
        }
        pg_sys::NodeTag::T_OpExpr => node_opexpr(
            context,
            rti,
            pdbopoid,
            ri_type,
            indexrel,
            state,
            opexpr,
            lhs,
            rhs,
            convert_external_to_special_qual,
            attempt_pushdown,
        ),

        _ if attempt_pushdown => {
            // it doesn't use our operator.
            // we'll try to convert it into a pushdown
            try_pushdown(
                context,
                rti,
                opexpr,
                indexrel,
                state,
                convert_external_to_special_qual,
            )
        }

        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn node_opexpr(
    context: &PlannerContext,
    rti: pg_sys::Index,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    indexrel: &PgSearchRelation,
    state: &mut QualExtractState,
    opexpr: OpExpr,
    lhs: *mut pg_sys::Node,
    mut rhs: *mut pg_sys::Node,
    convert_external_to_special_qual: bool,
    attempt_pushdown: bool,
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
                    expr_desc: deparse_expr(Some(context), indexrel, rhs),
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
            } else if attempt_pushdown {
                // it doesn't use our operator.
                // we'll try to convert it into a pushdown
                return try_pushdown(
                    context,
                    rti,
                    opexpr,
                    indexrel,
                    state,
                    convert_external_to_special_qual,
                );
            } else {
                return None;
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
                scalar_array_use_or: opexpr.use_or(),
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
    } else if attempt_pushdown {
        // it doesn't use our operator.
        // we'll try to convert it into a pushdown
        try_pushdown(
            context,
            rti,
            opexpr,
            indexrel,
            state,
            convert_external_to_special_qual,
        )
    } else {
        None
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
    context: &PlannerContext,
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
    // Note: try_pushdown_inner requires PlannerInfo
    let pushdown_result = if let Some(root) = context.planner_info() {
        try_pushdown_inner(root, rti, opexpr, indexrel)
    } else {
        // Query context: We can't call try_pushdown_inner, but we can check if this is our operator
        // by comparing the opno directly
        let our_opoid = anyelement_query_input_opoid();
        if opno == our_opoid {
            // This is our @@@ operator, we can handle it
            state.uses_our_operator = true;
            state.uses_tantivy_to_query = true;
            // Return as Expr to be evaluated at execution time
            return Some(Qual::Expr {
                node: opexpr_node,
                expr_desc: deparse_expr(Some(context), indexrel, opexpr_node),
            });
        }
        // Not our operator, can't pushdown in Query context
        None
    };

    if pushdown_result.is_none() {
        // DECISION POINT: Predicate cannot be pushed down to index
        // Check if this expression references our relation
        if contains_relation_reference(opexpr_node, rti) {
            // Check if custom scan for non-indexed fields is enabled
            if !gucs::enable_filter_pushdown() {
                return None;
            }

            // We do use search (with heap filtering)
            state.uses_heap_expr = true;
            state.uses_tantivy_to_query = true;

            // Create HeapExpr: predicate will be evaluated via heap access
            // This is slower but necessary for non-indexed fields
            Some(Qual::HeapExpr {
                expr_node: opexpr_node,
                expr_desc: deparse_expr(Some(context), indexrel, opexpr_node),
                search_query_input: Box::new(SearchQueryInput::All),
            })
        } else if contains_param(opexpr_node) {
            // Predicate doesn't reference our relation (e.g., $2 = 0 in prepared statements)
            // Check if it contains PARAM nodes - if so, create a HeapExpr that will be evaluated at execution
            // This prevents qual extraction from failing entirely when we have
            // expressions like: description @@@ $1 AND $2 = 0

            // Create HeapExpr for parameter expressions
            // These will be evaluated by PostgreSQL's executor at runtime
            state.uses_heap_expr = true;
            state.uses_tantivy_to_query = true;
            Some(Qual::HeapExpr {
                expr_node: opexpr_node,
                expr_desc: deparse_expr(Some(context), indexrel, opexpr_node),
                search_query_input: Box::new(SearchQueryInput::All),
            })
        } else if convert_external_to_special_qual {
            Some(Qual::ExternalExpr)
        } else {
            // Not a parameter expression and doesn't reference our relation
            // We can't handle this
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

/// Returns true if the expression contains a parameter that is correlated with an outer query.
/// Correlated parameters are `PARAM_EXEC` parameters that are not provided by an init plan.
pub unsafe fn contains_correlated_param(
    root: *mut pg_sys::PlannerInfo,
    node: *mut pg_sys::Node,
) -> bool {
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        let root = context as *mut pg_sys::PlannerInfo;
        if let Some(param) = nodecast!(Param, T_Param, node) {
            if (*param).paramkind == pg_sys::ParamKind::PARAM_EXEC {
                let param_is_from_init_plan =
                    PgList::<pg_sys::SubPlan>::from_pg((*root).init_plans)
                        .iter_ptr()
                        .any(|subplan| {
                            pg_sys::list_member_int((*subplan).setParam, (*param).paramid)
                        });

                if !param_is_from_init_plan {
                    // If this PARAM_EXEC param is not from any init plan, then we have to assume
                    // that it is correlated.
                    return true;
                }
            }
        }
        pg_sys::expression_tree_walker(node, Some(walker), context)
    }

    if node.is_null() {
        return false;
    }

    walker(node, root as *mut core::ffi::c_void)
}

/// Returns true if the expression contains any `PARAM_EXEC` parameter.
/// `PARAM_EXEC` parameters are evaluated at execution time, often for subqueries.
pub unsafe fn contains_exec_param(root: *mut pg_sys::Node) -> bool {
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

unsafe fn contains_param(root: *mut pg_sys::Node) -> bool {
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        _data: *mut core::ffi::c_void,
    ) -> bool {
        nodecast!(Param, T_Param, node).is_some()
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
    context: &PlannerContext,
    node: *mut pg_sys::Node,
    indexrel: &PgSearchRelation,
    state: &mut QualExtractState,
) -> Option<Qual> {
    let booltest = nodecast!(BooleanTest, T_BooleanTest, node)?;
    let arg = (*booltest).arg;

    // We only support boolean test for simple field references (Var nodes)
    // For complex expressions, the optimizer will evaluate the condition later
    // Note: PushdownField::try_new requires PlannerInfo
    let root = context.planner_info()?;
    let field = PushdownField::try_new(root, arg as *mut pg_sys::Node, indexrel);

    if let Some(field) = field {
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
        return qual;
    }

    if gucs::enable_filter_pushdown() {
        state.uses_heap_expr = true;
        state.uses_tantivy_to_query = true;
        return Some(Qual::HeapExpr {
            expr_node: node,
            expr_desc: deparse_expr(Some(context), indexrel, node),
            search_query_input: Box::new(SearchQueryInput::All),
        });
    }
    None
}

/// Extract join-level search predicates that are relevant for snippet/score generation
/// This captures search predicates that reference specific fields but may not be
/// pushed down to the current scan due to join conditions.
/// Returns the entire simplified Boolean expression to preserve OR structures.
pub unsafe fn extract_join_predicates(
    context: &PlannerContext,
    current_rti: pg_sys::Index,
    pdbopoid: pg_sys::Oid,
    indexrel: &PgSearchRelation,
    attempt_pushdown: bool,
) -> Option<SearchQueryInput> {
    // Only look at the current relation's join clauses
    // Note: This requires PlannerInfo for join clause extraction
    let root = context.planner_info()?;

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
                context,
                current_rti,
                simplified_node.cast(),
                pdbopoid,
                RestrictInfoType::BaseRelation,
                indexrel,
                true,
                &mut qual_extract_state,
                attempt_pushdown,
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

    match (*node).type_ {
        pg_sys::NodeTag::T_OpExpr => simplify_node_for_relation(node, current_rti),

        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = nodecast!(BoolExpr, T_BoolExpr, node)?;
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
            let mut simplified_args = Vec::new();

            // Recursively simplify each argument
            for arg in args.iter_ptr() {
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
    root: *mut pg_sys::PlannerInfo,
    expr_node: *mut pg_sys::Node,
    var_node: *mut pg_sys::Var,
    rti: pg_sys::Index,
    indexrel: &PgSearchRelation,
    uses_tantivy_to_query: &mut bool,
) -> Option<Qual> {
    if (*var_node).varno as pg_sys::Index == rti {
        // Check if custom scan for non-indexed fields is enabled
        if !gucs::enable_filter_pushdown() {
            return None;
        }
        *uses_tantivy_to_query = true;
        let context = PlannerContext::from_planner(root);
        Some(Qual::HeapExpr {
            expr_node,
            expr_desc: deparse_expr(Some(&context), indexrel, expr_node),
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
    indexrel: &PgSearchRelation,
    uses_tantivy_to_query: &mut bool,
) -> Option<Qual> {
    // Check if root and parse are valid
    if root.is_null() || (*root).parse.is_null() {
        return None;
    }

    create_heap_expr_for_field_ref(
        root,
        var_node as *mut pg_sys::Node,
        var_node,
        rti,
        indexrel,
        uses_tantivy_to_query,
    )
}

/// Try to create a HeapExpr from a NullTest for non-indexed fields
unsafe fn try_create_heap_expr_from_null_test(
    nulltest: *mut pg_sys::NullTest,
    rti: pg_sys::Index,
    root: *mut pg_sys::PlannerInfo,
    indexrel: &PgSearchRelation,
    uses_tantivy_to_query: &mut bool,
) -> Option<Qual> {
    // Extract the field being tested
    let arg = (*nulltest).arg;
    // Check if the arg is a Var referencing our relation
    if let Some(var) = nodecast!(Var, T_Var, arg) {
        create_heap_expr_for_field_ref(
            root,
            nulltest as *mut pg_sys::Node,
            var,
            rti,
            indexrel,
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
                Qual::PushdownVarEqTrue { field } | Qual::PushdownVarIsTrue { field },
                SearchQueryInput::FieldedQuery {
                    field: f,
                    query: pdb::Query::Term { value, .. },
                },
            ) => field.attname() == *f && matches!(value, OwnedValue::Bool(true)),

            // Match boolean field FALSE cases
            (
                Qual::PushdownVarEqFalse { field } | Qual::PushdownVarIsFalse { field },
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
                Qual::Not(_inner),
                SearchQueryInput::Boolean {
                    must,
                    should: _,
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

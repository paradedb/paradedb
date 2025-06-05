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

use crate::nodecast;
use crate::postgres::customscan::builders::custom_path::RestrictInfoType;
use crate::postgres::customscan::operator_oid;
use crate::postgres::customscan::pdbscan::projections::score::score_funcoid;
use crate::postgres::customscan::pdbscan::pushdown::{is_complex, try_pushdown, PushdownField};
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;
use pgrx::{pg_sys, FromDatum, PgList};
use std::ops::Bound;
use tantivy::schema::OwnedValue;

#[derive(Debug, Clone)]
pub enum Qual {
    All,
    ExternalVar,
    OpExpr {
        var: *mut pg_sys::Var,
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
    And(Vec<Qual>),
    Or(Vec<Qual>),
    Not(Box<Qual>),
}

impl Qual {
    pub fn contains_all(&self) -> bool {
        match self {
            Qual::All => true,
            Qual::ExternalVar => false,
            Qual::OpExpr { .. } => false,
            Qual::Expr { .. } => false,
            Qual::PushdownExpr { .. } => false,
            Qual::PushdownVarEqTrue { .. } => false,
            Qual::PushdownVarEqFalse { .. } => false,
            Qual::PushdownVarIsTrue { .. } => false,
            Qual::PushdownVarIsFalse { .. } => false,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => false,
            Qual::And(quals) => quals.iter().any(|q| q.contains_all()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_all()),
            Qual::Not(qual) => qual.contains_all(),
        }
    }

    pub fn contains_external_var(&self) -> bool {
        match self {
            Qual::All => false,
            Qual::ExternalVar => true,
            Qual::OpExpr { .. } => false,
            Qual::Expr { .. } => false,
            Qual::PushdownExpr { .. } => false,
            Qual::PushdownVarEqTrue { .. } => false,
            Qual::PushdownVarEqFalse { .. } => false,
            Qual::PushdownVarIsTrue { .. } => false,
            Qual::PushdownVarIsFalse { .. } => false,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => false,
            Qual::And(quals) => quals.iter().any(|q| q.contains_external_var()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_external_var()),
            Qual::Not(qual) => qual.contains_external_var(),
        }
    }

    pub unsafe fn contains_exec_param(&self) -> bool {
        match self {
            Qual::All => false,
            Qual::ExternalVar => false,
            Qual::OpExpr { .. } => false,
            Qual::Expr { node, .. } => contains_exec_param(*node),
            Qual::PushdownExpr { .. } => false,
            Qual::PushdownVarEqTrue { .. } => false,
            Qual::PushdownVarEqFalse { .. } => false,
            Qual::PushdownVarIsTrue { .. } => false,
            Qual::PushdownVarIsFalse { .. } => false,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => false,
            Qual::And(quals) => quals.iter().any(|q| q.contains_exec_param()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_exec_param()),
            Qual::Not(qual) => qual.contains_exec_param(),
        }
    }

    pub fn contains_exprs(&self) -> bool {
        match self {
            Qual::All => false,
            Qual::ExternalVar => false,
            Qual::OpExpr { .. } => false,
            Qual::Expr { .. } => true,
            Qual::PushdownExpr { .. } => false,
            Qual::PushdownVarEqTrue { .. } => true,
            Qual::PushdownVarEqFalse { .. } => true,
            Qual::PushdownVarIsTrue { .. } => true,
            Qual::PushdownVarIsFalse { .. } => true,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => false,
            Qual::And(quals) => quals.iter().any(|q| q.contains_exprs()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_exprs()),
            Qual::Not(qual) => qual.contains_exprs(),
        }
    }

    pub fn contains_score_exprs(&self) -> bool {
        match self {
            Qual::All => false,
            Qual::ExternalVar => false,
            Qual::OpExpr { .. } => false,
            Qual::Expr { .. } => false,
            Qual::PushdownExpr { .. } => false,
            Qual::PushdownVarEqTrue { .. } => false,
            Qual::PushdownVarEqFalse { .. } => false,
            Qual::PushdownVarIsTrue { .. } => false,
            Qual::PushdownVarIsFalse { .. } => false,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => true,
            Qual::And(quals) => quals.iter().any(|q| q.contains_score_exprs()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_score_exprs()),
            Qual::Not(qual) => qual.contains_score_exprs(),
        }
    }

    pub fn collect_exprs<'a>(&'a mut self, exprs: &mut Vec<&'a mut Qual>) {
        match self {
            Qual::Expr { .. } => exprs.push(self),
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

pub unsafe fn extract_quals(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
) -> Option<Qual> {
    match (*node).type_ {
        pg_sys::NodeTag::T_List => {
            let mut quals = list(root, rti, node.cast(), pdbopoid, ri_type, schema)?;
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
            extract_quals(root, rti, clause.cast(), pdbopoid, ri_type, schema)
        }

        pg_sys::NodeTag::T_OpExpr => opexpr(root, rti, node, pdbopoid, ri_type, schema),

        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = nodecast!(BoolExpr, T_BoolExpr, node)?;
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
            let mut quals = list(root, rti, (*boolexpr).args, pdbopoid, ri_type, schema)?;

            match (*boolexpr).boolop {
                pg_sys::BoolExprType::AND_EXPR => Some(Qual::And(quals)),
                pg_sys::BoolExprType::OR_EXPR => Some(Qual::Or(quals)),
                pg_sys::BoolExprType::NOT_EXPR => Some(Qual::Not(Box::new(quals.pop()?))),
                _ => panic!("unexpected `BoolExprType`: {}", (*boolexpr).boolop),
            }
        }

        pg_sys::NodeTag::T_Var if (*(node as *mut pg_sys::Var)).vartype == pg_sys::BOOLOID => {
            PushdownField::try_new(root, node.cast(), schema)
                .map(|field| Qual::PushdownVarEqTrue { field })
        }

        pg_sys::NodeTag::T_NullTest => {
            let nulltest = nodecast!(NullTest, T_NullTest, node)?;
            if let Some(field) = PushdownField::try_new(root, (*nulltest).arg.cast(), schema) {
                if schema.is_fast_field(&field.attname()) {
                    if (*nulltest).nulltesttype == pg_sys::NullTestType::IS_NOT_NULL {
                        Some(Qual::PushdownIsNotNull { field })
                    } else {
                        Some(Qual::Not(Box::new(Qual::PushdownIsNotNull { field })))
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }

        pg_sys::NodeTag::T_BooleanTest => booltest(root, rti, node, pdbopoid, ri_type, schema),

        // we don't understand this clause so we can't do anything
        _ => None,
    }
}

unsafe fn list(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    list: *mut pg_sys::List,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
) -> Option<Vec<Qual>> {
    let args = PgList::<pg_sys::Node>::from_pg(list);
    let mut quals = Vec::new();
    for child in args.iter_ptr() {
        quals.push(extract_quals(root, rti, child, pdbopoid, ri_type, schema)?)
    }
    Some(quals)
}

unsafe fn opexpr(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
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
        pg_sys::NodeTag::T_Var => {
            var_opexpr(root, rti, pdbopoid, ri_type, schema, opexpr, lhs, rhs)
        }

        pg_sys::NodeTag::T_FuncExpr => {
            // direct support for paradedb.score() in the WHERE clause
            let funcexpr = nodecast!(FuncExpr, T_FuncExpr, lhs)?;
            if (*funcexpr).funcid != score_funcoid() {
                return None;
            }

            if is_complex(rhs) {
                return None;
            }

            Some(Qual::ScoreExpr {
                opoid: (*opexpr).opno,
                value: rhs,
            })
        }

        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn var_opexpr(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
    opexpr: *mut pg_sys::OpExpr,
    lhs: *mut pg_sys::Node,
    mut rhs: *mut pg_sys::Node,
) -> Option<Qual> {
    while let Some(relabel_target) = nodecast!(RelabelType, T_RelabelType, rhs) {
        rhs = (*relabel_target).arg.cast();
    }

    let (var, const_) = (nodecast!(Var, T_Var, lhs)?, nodecast!(Const, T_Const, rhs));

    let is_our_operator = (*opexpr).opno == pdbopoid;

    if const_.is_none() {
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
                return Some(Qual::Expr {
                    node: rhs,
                    expr_state: std::ptr::null_mut(),
                });
            }
        } else {
            // it doesn't use our operator
            if contains_var(rhs) {
                // the rhs is (or contains) a Var too, which likely means its part of a join condition
                // we choose to just select everything in this situation
                return Some(Qual::ExternalVar);
            } else {
                // it doesn't use our operator.
                // we'll try to convert it into a pushdown
                return try_pushdown(root, rti, opexpr, schema);
            }
        }
    }

    let (lhs, rhs) = (var, const_?);
    if is_our_operator {
        // the rhs expression is a Const, so we can use it directly

        if (*lhs).varno as i32 == rti as i32 {
            // the var comes from this range table entry, so we can use the full expression directly
            Some(Qual::OpExpr {
                var: lhs,
                opno: (*opexpr).opno,
                val: rhs,
            })
        } else {
            // the var comes from a different range table
            if matches!(ri_type, RestrictInfoType::Join) {
                // and we're doing a join, so in this case we choose to just select everything
                Some(Qual::ExternalVar)
            } else {
                // the var comes from a different range table and we're not doing a join (how is that possible?!)
                // so we don't do anything
                None
            }
        }
    } else {
        // it doesn't use our operator.
        // we'll try to convert it into a pushdown
        try_pushdown(root, rti, opexpr, schema)
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

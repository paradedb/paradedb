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

use crate::api::operator::attname_from_var;
use crate::nodecast;
use crate::postgres::customscan::builders::custom_path::RestrictInfoType;
use crate::postgres::customscan::operator_oid;
use crate::postgres::customscan::pdbscan::privdat::deserialize::decodeString;
use crate::postgres::customscan::pdbscan::privdat::serialize::{
    makeInteger, makeString, AsValueNode,
};
use crate::postgres::customscan::pdbscan::projections::score::score_funcoid;
use crate::postgres::customscan::pdbscan::pushdown::{is_complex, try_pushdown};
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;
use pgrx::{pg_sys, FromDatum, PgList};
use std::ops::Bound;
use tantivy::schema::OwnedValue;

#[derive(Debug, Clone)]
pub enum Qual {
    All,
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
        attname: String,
    },
    /// Represents a SQL equality comparison: `bool_field = FALSE`
    /// - NULL values are excluded (NULL is not equal to FALSE)
    /// - Used for direct field reference equality comparisons
    /// - Negation transforms to PushdownVarEqTrue without including NULLs
    PushdownVarEqFalse {
        attname: String,
    },
    /// Represents a SQL IS operator: `bool_field IS TRUE`
    /// - NULL values are excluded (NULL is not TRUE)
    /// - Different from equality in negation semantics:
    ///   IS NOT TRUE includes both FALSE and NULL values
    PushdownVarIsTrue {
        attname: String,
    },
    /// Represents a SQL IS operator: `bool_field IS FALSE`
    /// - NULL values are excluded (NULL is not FALSE)
    /// - Different from equality in negation semantics:
    ///   IS NOT FALSE includes both TRUE and NULL values
    PushdownVarIsFalse {
        attname: String,
    },
    PushdownIsNotNull {
        attname: String,
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

    pub unsafe fn contains_exec_param(&self) -> bool {
        match self {
            Qual::All => false,
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
            Qual::OpExpr { .. } => false,
            Qual::Expr { .. } => false,
            Qual::PushdownExpr { .. } => false,
            Qual::PushdownVarEqTrue { .. } => false,
            Qual::PushdownVarEqFalse { .. } => false,
            Qual::PushdownVarIsTrue { .. } => false,
            Qual::PushdownVarIsFalse { .. } => false,
            Qual::PushdownIsNotNull { .. } => false,
            Qual::ScoreExpr { .. } => true,
            Qual::And(quals) => quals.iter().any(|q| q.contains_exprs()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_exprs()),
            Qual::Not(qual) => qual.contains_exprs(),
        }
    }

    pub fn collect_exprs<'a>(&'a mut self, exprs: &mut Vec<&'a mut Qual>) {
        match self {
            Qual::All => {}
            Qual::OpExpr { .. } => {}
            Qual::Expr { .. } => exprs.push(self),
            Qual::PushdownExpr { .. } => {}
            Qual::PushdownVarEqTrue { .. } => {}
            Qual::PushdownVarEqFalse { .. } => {}
            Qual::PushdownVarIsTrue { .. } => {}
            Qual::PushdownVarIsFalse { .. } => {}
            Qual::PushdownIsNotNull { .. } => {}
            Qual::ScoreExpr { .. } => {}
            Qual::And(quals) => quals.iter_mut().for_each(|q| q.collect_exprs(exprs)),
            Qual::Or(quals) => quals.iter_mut().for_each(|q| q.collect_exprs(exprs)),
            Qual::Not(qual) => qual.collect_exprs(exprs),
        }
    }
}

impl From<&Qual> for SearchQueryInput {
    #[track_caller]
    fn from(value: &Qual) -> Self {
        match value {
            Qual::All => SearchQueryInput::ConstScore {
                query: Box::new(SearchQueryInput::All),
                score: 0.0,
            },
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
            Qual::PushdownVarEqTrue { attname } => SearchQueryInput::Term {
                field: Some(attname.clone()),
                value: OwnedValue::Bool(true),
                is_datetime: false,
            },
            Qual::PushdownVarEqFalse { attname } => SearchQueryInput::Term {
                field: Some(attname.clone()),
                value: OwnedValue::Bool(false),
                is_datetime: false,
            },
            Qual::PushdownVarIsTrue { attname } => SearchQueryInput::Term {
                field: Some(attname.clone()),
                value: OwnedValue::Bool(true),
                is_datetime: false,
            },
            Qual::PushdownVarIsFalse { attname } => SearchQueryInput::Term {
                field: Some(attname.clone()),
                value: OwnedValue::Bool(false),
                is_datetime: false,
            },
            Qual::PushdownIsNotNull { attname } => SearchQueryInput::Exists {
                field: attname.clone(),
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
                let mut must = Vec::new();
                let mut should = Vec::new();

                for qual in quals {
                    match qual {
                        Qual::And(ands) => must.extend(ands.iter().map(SearchQueryInput::from)),
                        Qual::Or(ors) => should.extend(ors.iter().map(SearchQueryInput::from)),
                        other => must.push(SearchQueryInput::from(other)),
                    }
                }

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

                // rollup ScoreFilters from the `should` clauses into one
                let mut should_scores_bounds = vec![];
                while let Some(SearchQueryInput::ScoreFilter { bounds, .. }) = popscore(&mut should)
                {
                    should_scores_bounds.extend(bounds);
                }

                // make the Boolean clause we intend to return (or wrap)
                let mut boolean = SearchQueryInput::Boolean {
                    must,
                    should,
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

                if !should_scores_bounds.is_empty() {
                    SearchQueryInput::ScoreFilter {
                        bounds: should_scores_bounds,
                        query: Some(Box::new(boolean.clone())),
                    }
                } else {
                    boolean
                }
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
                    1 => should.into_iter().next().unwrap(),
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
                    Qual::PushdownVarEqTrue { attname } => Self::from(&Qual::PushdownVarEqFalse {
                        attname: attname.clone(),
                    }),
                    // Similarly, if we're negating a PushdownVarEqFalse, use PushdownVarEqTrue
                    // This follows SQL standard where NOT (field = FALSE) is equivalent to (field = TRUE)
                    // and does NOT include NULL values
                    Qual::PushdownVarEqFalse { attname } => Self::from(&Qual::PushdownVarEqTrue {
                        attname: attname.clone(),
                    }),
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

impl From<Qual> for PgList<pg_sys::Node> {
    fn from(value: Qual) -> Self {
        unsafe {
            let mut list = PgList::new();

            match value {
                Qual::All => list.push(makeString(Some("ALL"))),
                Qual::OpExpr { var, opno, val } => {
                    list.push(makeString(Some("OPEXPR")));
                    list.push(var.cast());
                    list.push(makeInteger(Some(opno)));
                    list.push(val.cast());
                }
                Qual::Expr { node, .. } => {
                    list.push(makeString(Some("EXPR")));
                    list.push(node);
                }
                Qual::PushdownExpr { funcexpr } => {
                    list.push(makeString(Some("PUSHDOWN")));
                    list.push(funcexpr.cast());
                }
                Qual::PushdownVarEqTrue { attname } => {
                    list.push(makeString(Some("PUSHDOWN_VAR_EQ_TRUE")));
                    list.push(makeString(Some(attname)));
                }
                Qual::PushdownVarEqFalse { attname } => {
                    list.push(makeString(Some("PUSHDOWN_VAR_EQ_FALSE")));
                    list.push(makeString(Some(attname)));
                }
                Qual::PushdownVarIsTrue { attname } => {
                    list.push(makeString(Some("PUSHDOWN_VAR_IS_TRUE")));
                    list.push(makeString(Some(attname)));
                }
                Qual::PushdownVarIsFalse { attname } => {
                    list.push(makeString(Some("PUSHDOWN_VAR_IS_FALSE")));
                    list.push(makeString(Some(attname)));
                }
                Qual::PushdownIsNotNull { attname } => {
                    list.push(makeString(Some("PUSHDOWN_IS_NOT_NULL")));
                    list.push(makeString(Some(attname)));
                }
                Qual::ScoreExpr { opoid, value } => {
                    list.push(makeString(Some("SCORE")));
                    list.push(makeInteger(Some(opoid)));
                    list.push(value);
                }
                Qual::And(quals) => {
                    list.push(makeString(Some("AND")));
                    list.push(makeInteger(Some(quals.len())));
                    for qual in quals {
                        let and: PgList<pg_sys::Node> = qual.into();
                        list.push(and.into_pg().cast());
                    }
                }
                Qual::Or(quals) => {
                    list.push(makeString(Some("OR")));
                    list.push(makeInteger(Some(quals.len())));
                    for qual in quals {
                        let or: PgList<pg_sys::Node> = qual.into();
                        list.push(or.into_pg().cast());
                    }
                }
                Qual::Not(not) => {
                    list.push(makeString(Some("NOT")));
                    let not: PgList<pg_sys::Node> = (*not).into();
                    list.push(not.into_pg().cast());
                }
            }

            list
        }
    }
}

impl From<PgList<pg_sys::Node>> for Qual {
    fn from(value: PgList<pg_sys::Node>) -> Self {
        fn inner(value: PgList<pg_sys::Node>) -> Option<Qual> {
            unsafe {
                let first = value.get_ptr(0)?;

                if let Some(type_) = decodeString::<String>(first) {
                    match type_.as_str() {
                        "ALL" => Some(Qual::All),
                        "OPEXPR" => {
                            let (var, opno, val) = (
                                nodecast!(Var, T_Var, value.get_ptr(1)?)?,
                                pg_sys::Oid::from_value_node(value.get_ptr(2)?)?,
                                nodecast!(Const, T_Const, value.get_ptr(3)?)?,
                            );
                            Some(Qual::OpExpr { var, opno, val })
                        }
                        "EXPR" => {
                            let node = value.get_ptr(1)?;
                            Some(Qual::Expr {
                                node,
                                expr_state: std::ptr::null_mut(),
                            })
                        }
                        "PUSHDOWN" => {
                            let funcexpr = nodecast!(FuncExpr, T_FuncExpr, value.get_ptr(1)?)?;
                            Some(Qual::PushdownExpr { funcexpr })
                        }
                        "PUSHDOWN_VAR_EQ_TRUE" => {
                            let attname = decodeString::<String>(value.get_ptr(1)?)?;
                            Some(Qual::PushdownVarEqTrue { attname })
                        }
                        "PUSHDOWN_VAR_EQ_FALSE" => {
                            let attname = decodeString::<String>(value.get_ptr(1)?)?;
                            Some(Qual::PushdownVarEqFalse { attname })
                        }
                        "PUSHDOWN_VAR_IS_TRUE" => {
                            let attname = decodeString::<String>(value.get_ptr(1)?)?;
                            Some(Qual::PushdownVarIsTrue { attname })
                        }
                        "PUSHDOWN_VAR_IS_FALSE" => {
                            let attname = decodeString::<String>(value.get_ptr(1)?)?;
                            Some(Qual::PushdownVarIsFalse { attname })
                        }
                        "PUSHDOWN_IS_NOT_NULL" => {
                            let attname = decodeString::<String>(value.get_ptr(1)?)?;
                            Some(Qual::PushdownIsNotNull { attname })
                        }
                        "SCORE" => {
                            let (opoid, value) = (
                                pg_sys::Oid::from_value_node(value.get_ptr(1)?)?,
                                value.get_ptr(2)?,
                            );
                            Some(Qual::ScoreExpr { opoid, value })
                        }
                        "AND" => {
                            let len = usize::from_value_node(value.get_ptr(1)?)?;
                            let mut quals = Vec::with_capacity(len);
                            for i in 2..value.len() {
                                let qual_list = PgList::<pg_sys::Node>::from_pg(nodecast!(
                                    List,
                                    T_List,
                                    value.get_ptr(i)?
                                )?);
                                quals.push(qual_list.into());
                            }
                            Some(Qual::And(quals))
                        }
                        "OR" => {
                            let len = usize::from_value_node(value.get_ptr(1)?)?;
                            let mut quals = Vec::with_capacity(len);
                            for i in 2..value.len() {
                                let qual_list = PgList::<pg_sys::Node>::from_pg(nodecast!(
                                    List,
                                    T_List,
                                    value.get_ptr(i)?
                                )?);
                                quals.push(qual_list.into());
                            }
                            Some(Qual::Or(quals))
                        }
                        "NOT" => {
                            let not_qual = PgList::<pg_sys::Node>::from_pg(nodecast!(
                                List,
                                T_List,
                                value.get_ptr(1)?
                            )?);
                            Some(Qual::Not(Box::new(not_qual.into())))
                        }
                        other => panic!("unexpected Qual list node: {other}"),
                    }
                } else {
                    panic!("malformed Qual list")
                }
            }
        }

        inner(value).expect("Qual list should not be empty")
    }
}

pub unsafe fn extract_quals(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
    schema: &SearchIndexSchema,
    uses_our_operator: &mut bool,
) -> Option<Qual> {
    match (*node).type_ {
        pg_sys::NodeTag::T_List => {
            let mut quals = list(
                root,
                rti,
                node.cast(),
                pdbopoid,
                ri_type,
                schema,
                uses_our_operator,
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
                uses_our_operator,
            )
        }

        pg_sys::NodeTag::T_OpExpr => opexpr(
            root,
            rti,
            node,
            pdbopoid,
            ri_type,
            schema,
            uses_our_operator,
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
                uses_our_operator,
            )?;

            match (*boolexpr).boolop {
                pg_sys::BoolExprType::AND_EXPR => Some(Qual::And(quals)),
                pg_sys::BoolExprType::OR_EXPR => Some(Qual::Or(quals)),
                pg_sys::BoolExprType::NOT_EXPR => Some(Qual::Not(Box::new(quals.pop()?))),
                _ => panic!("unexpected `BoolExprType`: {}", (*boolexpr).boolop),
            }
        }

        pg_sys::NodeTag::T_Var if (*(node as *mut pg_sys::Var)).vartype == pg_sys::BOOLOID => {
            Some(Qual::PushdownVarEqTrue {
                attname: attname_from_var(root, node.cast())
                    .1
                    .expect("var should have an attname"),
            })
        }

        pg_sys::NodeTag::T_NullTest => {
            let nulltest = nodecast!(NullTest, T_NullTest, node)?;
            let (_, attname) = attname_from_var(root, (*nulltest).arg.cast());
            let attname = attname?; // if the attribute isn't
            if (*nulltest).nulltesttype == pg_sys::NullTestType::IS_NOT_NULL {
                Some(Qual::PushdownIsNotNull { attname })
            } else {
                Some(Qual::Not(Box::new(Qual::PushdownIsNotNull { attname })))
            }
        }

        pg_sys::NodeTag::T_BooleanTest => booltest(
            root,
            rti,
            node,
            pdbopoid,
            ri_type,
            schema,
            uses_our_operator,
        ),

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
    uses_our_operator: &mut bool,
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
            schema,
            uses_our_operator,
        )?)
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
    uses_our_operator: &mut bool,
) -> Option<Qual> {
    let opexpr = nodecast!(OpExpr, T_OpExpr, node)?;
    let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);

    let lhs = args.get_ptr(0)?;
    let rhs = args.get_ptr(1)?;

    match (*lhs).type_ {
        pg_sys::NodeTag::T_Var => var_opexpr(
            root,
            rti,
            pdbopoid,
            ri_type,
            schema,
            uses_our_operator,
            opexpr,
            lhs,
            rhs,
        ),

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
    uses_our_operator: &mut bool,
    opexpr: *mut pg_sys::OpExpr,
    lhs: *mut pg_sys::Node,
    rhs: *mut pg_sys::Node,
) -> Option<Qual> {
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
                *uses_our_operator = true;
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
                return Some(Qual::All);
            } else {
                // it doesn't use our operator.
                // we'll try to convert it into a pushdown
                return try_pushdown(root, opexpr, schema);
            }
        }
    }

    let (lhs, rhs) = (var, const_?);
    if is_our_operator {
        // the rhs expression is a Const, so we can use it directly

        if (*lhs).varno as i32 == rti as i32 {
            // the var comes from this range table entry, so we can use the full expression directly
            *uses_our_operator = true;
            Some(Qual::OpExpr {
                var: lhs,
                opno: (*opexpr).opno,
                val: rhs,
            })
        } else {
            // the var comes from a different range table
            if matches!(ri_type, RestrictInfoType::Join) {
                // and we're doing a join, so in this case we choose to just select everything
                Some(Qual::All)
            } else {
                // the var comes from a different range table and we're not doing a join (how is that possible?!)
                // so we don't do anything
                None
            }
        }
    } else {
        // it doesn't use our operator.
        // we'll try to convert it into a pushdown
        try_pushdown(root, opexpr, schema)
    }
}

unsafe fn contains_exec_param(root: *mut pg_sys::Node) -> bool {
    unsafe extern "C" fn walker(node: *mut pg_sys::Node, _: *mut core::ffi::c_void) -> bool {
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
    unsafe extern "C" fn walker(node: *mut pg_sys::Node, _: *mut core::ffi::c_void) -> bool {
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
    uses_our_operator: &mut bool,
) -> Option<Qual> {
    let booltest = nodecast!(BooleanTest, T_BooleanTest, node)?;
    let arg = (*booltest).arg;

    // We only support boolean test for simple field references (Var nodes)
    // For complex expressions, the optimizer will evaluate the condition later
    if let Some(arg_var) = nodecast!(Var, T_Var, arg) {
        // Get the attribute name from the Var
        let (_, attname) = attname_from_var(root, arg_var);
        if let Some(attname) = attname {
            // It's a simple field reference, handle as specific cases
            match (*booltest).booltesttype {
                pg_sys::BoolTestType::IS_TRUE => Some(Qual::PushdownVarIsTrue { attname }),
                pg_sys::BoolTestType::IS_NOT_FALSE => {
                    Some(Qual::Not(Box::new(Qual::PushdownVarIsFalse { attname })))
                }
                pg_sys::BoolTestType::IS_FALSE => Some(Qual::PushdownVarIsFalse { attname }),
                pg_sys::BoolTestType::IS_NOT_TRUE => {
                    Some(Qual::Not(Box::new(Qual::PushdownVarIsTrue { attname })))
                }
                _ => None,
            }
        } else {
            // Var node but couldn't get attribute name
            None
        }
    } else {
        // Not a simple field reference - let the PostgreSQL executor handle it
        None
    }
}

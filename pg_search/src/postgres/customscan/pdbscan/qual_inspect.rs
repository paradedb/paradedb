// Copyright (c) 2023-2025 Retake, Inc.
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
use crate::postgres::customscan::pdbscan::privdat::deserialize::decodeString;
use crate::postgres::customscan::pdbscan::privdat::serialize::{
    makeInteger, makeString, AsValueNode,
};
use crate::query::SearchQueryInput;
use pgrx::{pg_sys, FromDatum, PgList};

#[derive(Debug, Clone)]
pub enum Qual {
    All,
    OperatorExpression {
        var: *mut pg_sys::Var,
        opno: pg_sys::Oid,
        val: *mut pg_sys::Const,
    },
    Param {
        var: *mut pg_sys::Var,
        opno: pg_sys::Oid,
        node: *mut pg_sys::Node,
    },
    And(Vec<Qual>),
    Or(Vec<Qual>),
    Not(Box<Qual>),
}

impl Qual {
    pub fn contains_all(&self) -> bool {
        match self {
            Qual::All => true,
            Qual::OperatorExpression { .. } => false,
            Qual::Param { .. } => false,
            Qual::And(quals) => quals.iter().any(|q| q.contains_all()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_all()),
            Qual::Not(qual) => qual.contains_all(),
        }
    }

    pub fn contains_param(&self) -> bool {
        match self {
            Qual::All => false,
            Qual::OperatorExpression { .. } => false,
            Qual::Param { .. } => true,
            Qual::And(quals) => quals.iter().any(|q| q.contains_param()),
            Qual::Or(quals) => quals.iter().any(|q| q.contains_param()),
            Qual::Not(qual) => qual.contains_param(),
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
            Qual::OperatorExpression { val, .. } => unsafe {
                SearchQueryInput::from_datum((**val).constvalue, (**val).constisnull)
                    .expect("rhs of @@@ operator Qual must not be null")
            },
            Qual::Param { .. } => todo!("parameterized plans are not currently supported.  Please `SET plan_cache_mode = force_custom_plan;` in this session, or set it in postgresql.conf and reload the configuration."),
            Qual::And(quals) => {
                let mut must = Vec::new();
                let mut should = Vec::new();
                let mut must_not = Vec::new();

                for qual in quals {
                    match qual {
                        Qual::And(ands) => must.extend(ands.iter().map(SearchQueryInput::from)),
                        Qual::Or(ors) => should.extend(ors.iter().map(SearchQueryInput::from)),
                        Qual::Not(not) => must_not.push(SearchQueryInput::from(not.as_ref())),
                        other => must.push(SearchQueryInput::from(other)),
                    }
                }

                SearchQueryInput::Boolean {
                    must,
                    should,
                    must_not,
                }
            }
            Qual::Or(quals) => {
                let should = quals.iter().map(SearchQueryInput::from).collect::<Vec<_>>();

                match should.len() {
                    0 => panic!("Qual::Or should have at least one item"),
                    1 => should.into_iter().next().unwrap(),
                    _ => SearchQueryInput::Boolean {
                        must: Default::default(),
                        should,
                        must_not: Default::default(),
                    },
                }
            }
            Qual::Not(qual) => {
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

impl From<Qual> for PgList<pg_sys::Node> {
    fn from(value: Qual) -> Self {
        unsafe {
            let mut list = PgList::new();

            match value {
                Qual::All => list.push(makeString(Some("ALL"))),
                Qual::OperatorExpression { var, opno, val } => {
                    list.push(makeString(Some("OPERATOR_EXPRESSION")));
                    list.push(var.cast());
                    list.push(makeInteger(Some(opno)));
                    list.push(val.cast());
                }
                Qual::Param { var, opno, node } => {
                    list.push(makeString(Some("PARAM")));
                    list.push(var.cast());
                    list.push(makeInteger(Some(opno)));
                    list.push(node);
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
                        "OPERATOR_EXPRESSION" => {
                            let (var, opno, val) = (
                                nodecast!(Var, T_Var, value.get_ptr(1)?)?,
                                pg_sys::Oid::from_value_node(value.get_ptr(2)?)?,
                                nodecast!(Const, T_Const, value.get_ptr(3)?)?,
                            );
                            Some(Qual::OperatorExpression { var, opno, val })
                        }
                        "PARAM" => {
                            let (var, opno, node) = (
                                nodecast!(Var, T_Var, value.get_ptr(1)?)?,
                                pg_sys::Oid::from_value_node(value.get_ptr(2)?)?,
                                value.get_ptr(3)?,
                            );
                            Some(Qual::Param { var, opno, node })
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
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
) -> Option<Qual> {
    match (*node).type_ {
        pg_sys::NodeTag::T_List => {
            let mut quals = list(rti, node.cast(), pdbopoid, ri_type)?;
            if quals.len() == 1 {
                quals.pop()
            } else {
                Some(Qual::And(quals))
            }
        }

        pg_sys::NodeTag::T_RestrictInfo => {
            let ri = nodecast!(RestrictInfo, T_RestrictInfo, node)?;
            // if (*ri).num_base_rels > 1 {
            //     return None;
            // }
            let clause = if !(*ri).orclause.is_null() {
                (*ri).orclause
            } else {
                (*ri).clause
            };
            extract_quals(rti, clause.cast(), pdbopoid, ri_type)
        }

        pg_sys::NodeTag::T_OpExpr => opexpr(rti, node, pdbopoid, ri_type),

        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = nodecast!(BoolExpr, T_BoolExpr, node)?;
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
            let mut quals = list(rti, (*boolexpr).args, pdbopoid, ri_type)?;

            match (*boolexpr).boolop {
                pg_sys::BoolExprType::AND_EXPR => Some(Qual::And(quals)),
                pg_sys::BoolExprType::OR_EXPR => Some(Qual::Or(quals)),
                pg_sys::BoolExprType::NOT_EXPR => Some(Qual::Not(Box::new(quals.pop()?))),
                _ => panic!("unexpected `BoolExprType`: {}", (*boolexpr).boolop),
            }
        }

        // we don't understand this clause so we can't do anything
        _ => {
            // pgrx::warning!("unsupported qual node kind: {:?}", (*node).type_);
            None
        }
    }
}

unsafe fn list(
    rti: pg_sys::Index,
    list: *mut pg_sys::List,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
) -> Option<Vec<Qual>> {
    let args = PgList::<pg_sys::Node>::from_pg(list);
    let mut quals = Vec::new();
    for child in args.iter_ptr() {
        quals.push(extract_quals(rti, child, pdbopoid, ri_type)?)
    }
    Some(quals)
}

unsafe fn opexpr(
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
    ri_type: RestrictInfoType,
) -> Option<Qual> {
    let opexpr = nodecast!(OpExpr, T_OpExpr, node)?;
    let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
    let (lhs, rhs) = (
        nodecast!(Var, T_Var, args.get_ptr(0)?),
        nodecast!(Const, T_Const, args.get_ptr(1)?),
    );

    let is_our_operator = (*opexpr).opno == pdbopoid;

    if lhs.is_none() || rhs.is_none() {
        if contains_param(args.get_ptr(1)?) {
            if matches!(ri_type, RestrictInfoType::BaseRelation) {
                return None;
            }

            // TODO:  this would be for moving towards parameterized plans
            return Some(Qual::Param {
                var: lhs?,
                opno: (*opexpr).opno,
                node: args.get_ptr(1)?,
            });
        } else if matches!(ri_type, RestrictInfoType::Join) {
            return Some(Qual::All);
        } else {
            return None;
        }
    }
    let (lhs, rhs) = (lhs?, rhs?);

    if is_our_operator {
        if (*lhs).varno as i32 != rti as i32 {
            if matches!(ri_type, RestrictInfoType::Join) {
                Some(Qual::All)
            } else {
                None
            }
        } else {
            Some(Qual::OperatorExpression {
                var: lhs,
                opno: (*opexpr).opno,
                val: rhs,
            })
        }
    } else {
        None
    }
}

unsafe fn contains_param(root: *mut pg_sys::Node) -> bool {
    unsafe extern "C" fn walker(node: *mut pg_sys::Node, _: *mut core::ffi::c_void) -> bool {
        if nodecast!(Param, T_Param, node).is_some() {
            return true;
        }
        pg_sys::expression_tree_walker(node, Some(walker), std::ptr::null_mut())
    }

    pg_sys::expression_tree_walker(root, Some(walker), std::ptr::null_mut())
}

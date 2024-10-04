// Copyright (c) 2023-2024 Retake, Inc.
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
use crate::query::SearchQueryInput;
use crate::schema::SearchConfig;
use pgrx::{node_to_string, pg_sys, FromDatum, JsonB, PgList};

#[derive(Debug, Clone)]
pub enum Qual {
    OperatorExpression {
        var: *mut pg_sys::Var,
        opno: pg_sys::Oid,
        val: *mut pg_sys::Const,
    },
    And(Vec<Qual>),
    Or(Vec<Qual>),
    Not(Box<Qual>),
}

impl From<Qual> for SearchConfig {
    fn from(value: Qual) -> Self {
        match value {
            Qual::OperatorExpression { val, .. } => unsafe {
                let config_jsonb = JsonB::from_datum((*val).constvalue, (*val).constisnull)
                    .expect("rhs of @@@ operator Qual must not be null");

                SearchConfig::from_jsonb(config_jsonb)
                    .expect("rhs of @@@ operator must be a valid SearchConfig")
            },

            Qual::And(quals) => {
                let mut first: SearchConfig = quals
                    .first()
                    .cloned()
                    .expect("Qual::Or should have at least one item")
                    .into();

                let must = quals
                    .into_iter()
                    .map(|qual| SearchConfig::from(qual).query)
                    .collect::<Vec<_>>();

                if must.len() > 1 {
                    first.query = SearchQueryInput::Boolean {
                        must,
                        should: Default::default(),
                        must_not: Default::default(),
                    };
                }
                first
            }
            Qual::Or(quals) => {
                let mut first: SearchConfig = quals
                    .first()
                    .cloned()
                    .expect("Qual::Or should have at least one item")
                    .into();

                let should = quals
                    .into_iter()
                    .map(|qual| SearchConfig::from(qual).query)
                    .collect::<Vec<_>>();

                if should.len() > 1 {
                    first.query = SearchQueryInput::Boolean {
                        must: Default::default(),
                        should,
                        must_not: Default::default(),
                    };
                }
                first
            }
            Qual::Not(qual) => {
                let mut not: SearchConfig = (*qual).into();

                not.query = SearchQueryInput::Boolean {
                    must: Default::default(),
                    should: Default::default(),
                    must_not: vec![not.query],
                };
                not
            }
        }
    }
}

pub unsafe fn can_use_quals(node: *mut pg_sys::Node, pdbopoid: pg_sys::Oid) -> Option<()> {
    match (*node).type_ {
        pg_sys::NodeTag::T_List => list(node.cast(), pdbopoid).map(|_| ()),

        pg_sys::NodeTag::T_RestrictInfo => {
            let ri = nodecast!(RestrictInfo, T_RestrictInfo, node)?;
            can_use_quals((*ri).clause.cast(), pdbopoid)
        }

        pg_sys::NodeTag::T_OpExpr => opexpr(node, pdbopoid).map(|_| ()),

        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = nodecast!(BoolExpr, T_BoolExpr, node)?;
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);

            match (*boolexpr).boolop {
                pg_sys::BoolExprType::AND_EXPR => Some(()),
                pg_sys::BoolExprType::OR_EXPR => Some(()),
                pg_sys::BoolExprType::NOT_EXPR => Some(()),
                _ => panic!("unexpected `BoolExprType`: {}", (*boolexpr).boolop),
            }
        }

        // we don't understand this clause so we can't do anything
        _ => {
            pgrx::warning!("unsupported qual node kind: {:?}", (*node).type_);
            None
        }
    }
}

pub unsafe fn extract_quals(node: *mut pg_sys::Node, pdbopoid: pg_sys::Oid) -> Option<Qual> {
    match (*node).type_ {
        pg_sys::NodeTag::T_List => {
            let mut quals = list(node.cast(), pdbopoid)?;
            if quals.len() == 1 {
                quals.pop()
            } else {
                Some(Qual::And(quals))
            }
        }

        pg_sys::NodeTag::T_RestrictInfo => {
            let ri = nodecast!(RestrictInfo, T_RestrictInfo, node)?;
            extract_quals((*ri).clause.cast(), pdbopoid)
        }

        pg_sys::NodeTag::T_OpExpr => opexpr(node, pdbopoid),

        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = nodecast!(BoolExpr, T_BoolExpr, node)?;
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
            let mut quals = list((*boolexpr).args, pdbopoid)?;

            match (*boolexpr).boolop {
                pg_sys::BoolExprType::AND_EXPR => Some(Qual::And(quals)),
                pg_sys::BoolExprType::OR_EXPR => Some(Qual::Or(quals)),
                pg_sys::BoolExprType::NOT_EXPR => Some(Qual::Not(Box::new(quals.pop()?))),
                _ => panic!("unexpected `BoolExprType`: {}", (*boolexpr).boolop),
            }
        }

        // we don't understand this clause so we can't do anything
        _ => {
            pgrx::warning!("unsupported qual node kind: {:?}", (*node).type_);
            None
        }
    }
}

unsafe fn list(list: *mut pg_sys::List, pdbopoid: pg_sys::Oid) -> Option<Vec<Qual>> {
    let args = PgList::<pg_sys::Node>::from_pg(list);
    let mut quals = Vec::new();
    for child in args.iter_ptr() {
        quals.push(extract_quals(child, pdbopoid)?)
    }
    Some(quals)
}

unsafe fn opexpr(node: *mut pg_sys::Node, pdbopoid: pg_sys::Oid) -> Option<Qual> {
    let opexpr = nodecast!(OpExpr, T_OpExpr, node)?;
    let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
    let (lhs, rhs) = (
        nodecast!(Var, T_Var, args.get_ptr(0)?),
        nodecast!(Const, T_Const, args.get_ptr(1)?),
    );

    if lhs.is_none() || rhs.is_none() {
        pgrx::debug1!(
            "unrecognized `OpExpr`: {}",
            node_to_string(opexpr.cast()).expect("node_to_string should not return null")
        );
        return None;
    }
    let (lhs, rhs) = (lhs?, rhs?);

    ((*opexpr).opno == pdbopoid).then(|| Qual::OperatorExpression {
        var: lhs,
        opno: (*opexpr).opno,
        val: rhs,
    })
}

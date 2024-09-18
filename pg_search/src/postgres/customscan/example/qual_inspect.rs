use crate::nodecast;
use crate::postgres::customscan::node;
use pgrx::pg_sys::Node;
use pgrx::{node_to_string, pg_sys, PgList};

#[derive(Debug)]
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

impl Qual {
    pub fn to_tantivy_query(&self) -> String {
        match self {
            Qual::OperatorExpression { var, opno, val } => {
                format!("field:(value)")
            }
            Qual::And(quals) => {
                let mut s = String::new();
                for q in quals {
                    if !s.is_empty() {
                        s.push_str(" AND ");
                    }
                    s.push_str(&q.to_tantivy_query());
                }
                s
            }
            Qual::Or(quals) => {
                let mut s = String::new();
                for q in quals {
                    if !s.is_empty() {
                        s.push_str(" OR ");
                    }
                    s.push_str(&q.to_tantivy_query());
                }
                s
            }
            Qual::Not(qual) => {
                format!("NOT {}", qual.to_tantivy_query())
            }
        }
    }
}

pub unsafe fn extract_quals(node: *mut pg_sys::Node) -> Option<Qual> {
    match (*node).type_ {
        pg_sys::NodeTag::T_List => {
            let mut quals = list(node.cast())?;
            if quals.len() == 1 {
                quals.pop()
            } else {
                Some(Qual::And(quals))
            }
        }

        pg_sys::NodeTag::T_RestrictInfo => {
            let ri = nodecast!(RestrictInfo, T_RestrictInfo, node)?;
            extract_quals((*ri).clause.cast())
        }

        pg_sys::NodeTag::T_OpExpr => opexpr(node),

        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = nodecast!(BoolExpr, T_BoolExpr, node)?;
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
            let mut quals = list((*boolexpr).args)?;

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

unsafe fn list(list: *mut pg_sys::List) -> Option<Vec<Qual>> {
    let args = PgList::<pg_sys::Node>::from_pg(list);
    let mut quals = Vec::new();
    for child in args.iter_ptr() {
        quals.push(extract_quals(child)?)
    }
    Some(quals)
}

unsafe fn opexpr(node: *mut pg_sys::Node) -> Option<Qual> {
    let opexpr = nodecast!(OpExpr, T_OpExpr, node)?;
    let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
    let (lhs, rhs) = (
        nodecast!(Var, T_Var, args.get_ptr(0)?),
        nodecast!(Const, T_Const, args.get_ptr(1)?),
    );

    if lhs.is_none() || rhs.is_none() {
        pgrx::warning!(
            "unrecognized `OpExpr`: {}",
            node_to_string(opexpr.cast()).expect("node_to_string should not return null")
        );
    }
    let (lhs, rhs) = (lhs?, rhs?);

    ((*opexpr).opno == pg_sys::Oid::from(pg_sys::TextEqualOperator)).then(|| {
        Qual::OperatorExpression {
            var: lhs,
            opno: (*opexpr).opno,
            val: rhs,
        }
    })
}

//! Expression tree deserialization and Var-node rewriting.
//!
//! `PreparedPgExpr` wraps `stringToNode` + INNER_VAR rewriting into a
//! safe API that guarantees rewriting only runs on fresh trees.
//! `InputVarInfo` describes a Var dependency with planning-time type metadata.
//!
//! These are scan-type-agnostic — usable by JoinScan, BaseScan, or any
//! future scan that evaluates PG expressions on Arrow data.

use crate::postgres::node::NodeExt;
use std::collections::HashMap;

use pgrx::pg_sys;
use serde::{Deserialize, Serialize};

/// Describes a single input variable dependency of an expression.
/// Type metadata is resolved at planning time from the Var node itself
/// (Var.vartype, Var.vartypmod, Var.varcollid), avoiding any catalog lookups
/// at execution time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputVarInfo {
    pub rti: pg_sys::Index,
    pub attno: pg_sys::AttrNumber,
    pub type_oid: pg_sys::Oid,
    pub typmod: i32,
    pub collation: pg_sys::Oid,
}

/// A deserialized and Var-rewritten PG expression, ready for ExecInitExpr.
///
/// `rewrite_var_nodes` mutates the expression tree in place. This struct
/// guarantees it only runs on a freshly deserialized tree (from stringToNode),
/// never on a shared parse-tree pointer.
pub struct PreparedPgExpr {
    expr_node: *mut pg_sys::Expr,
}

impl PreparedPgExpr {
    /// Deserialize a PG expression and rewrite its Var nodes for a synthetic slot.
    ///
    /// # Safety
    /// Must be called within a suitable PG memory context.
    pub unsafe fn from_serialized(pg_expr_string: &str, input_vars: &[InputVarInfo]) -> Self {
        let c_str = std::ffi::CString::new(pg_expr_string)
            .expect("pg_expr_string contains interior NUL byte");
        let expr_node = pg_sys::stringToNode(c_str.as_ptr().cast_mut()) as *mut pg_sys::Expr;
        rewrite_var_nodes(expr_node.cast(), input_vars);
        Self { expr_node }
    }

    pub fn as_ptr(&self) -> *mut pg_sys::Expr {
        self.expr_node
    }
}

/// Rewrite all Var nodes in an expression tree to reference sequential positions
/// in a synthetic tuple slot.
///
/// # Safety
/// `expr` must be a valid, mutable PG Node tree (freshly deserialized).
unsafe fn rewrite_var_nodes(expr: *mut pg_sys::Node, input_vars: &[InputVarInfo]) {
    let var_map: HashMap<_, _> = input_vars
        .iter()
        .enumerate()
        .map(|(i, v)| ((v.rti as i32, v.attno), (i + 1) as pg_sys::AttrNumber))
        .collect();
    expr.visit(|node| {
        if let Some(var) = crate::nodecast!(Var, T_Var, node)
            && let Some(&new_attno) = var_map.get(&((*var).varno, (*var).varattno))
        {
            (*var).varno = pg_sys::INNER_VAR;
            (*var).varattno = new_attno;
            (*var).varnosyn = pg_sys::INNER_VAR as pg_sys::Index;
            (*var).varattnosyn = new_attno;
        }
    });
}

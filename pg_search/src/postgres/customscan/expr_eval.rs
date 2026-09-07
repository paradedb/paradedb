//! Expression tree deserialization and Var-node rewriting.
//!
//! `PreparedPgExpr` wraps `stringToNode` + INNER_VAR rewriting into a
//! safe API that guarantees rewriting only runs on fresh trees.
//! `InputVarInfo` describes a Var dependency with planning-time type metadata.
//!
//! These are scan-type-agnostic — usable by JoinScan, BaseScan, or any
//! future scan that evaluates PG expressions on Arrow data.

use std::collections::HashMap;
use std::ptr::addr_of_mut;

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

// --- Private to this module ---

struct VarRewriteCtx {
    var_map: HashMap<(i32, pg_sys::AttrNumber), pg_sys::AttrNumber>,
}

/// Rewrite all Var nodes in an expression tree to reference sequential positions
/// in a synthetic tuple slot.
///
/// # Safety
/// `expr` must be a valid, mutable PG Node tree (freshly deserialized).
unsafe fn rewrite_var_nodes(expr: *mut pg_sys::Node, input_vars: &[InputVarInfo]) {
    use pgrx::pg_sys::expression_tree_walker;

    #[pgrx::pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        if (*node).type_ == pg_sys::NodeTag::T_Var {
            let var = node as *mut pg_sys::Var;
            let ctx = &*(context as *const VarRewriteCtx);
            let key = ((*var).varno, (*var).varattno);
            if let Some(&new_attno) = ctx.var_map.get(&key) {
                (*var).varno = pg_sys::INNER_VAR;
                (*var).varattno = new_attno;
                (*var).varnosyn = pg_sys::INNER_VAR as pg_sys::Index;
                (*var).varattnosyn = new_attno;
            }
            return false;
        }

        expression_tree_walker(node, Some(walker), context)
    }

    let mut ctx = VarRewriteCtx {
        var_map: input_vars
            .iter()
            .enumerate()
            .map(|(i, v)| ((v.rti as i32, v.attno), (i + 1) as pg_sys::AttrNumber))
            .collect(),
    };

    walker(expr, addr_of_mut!(ctx).cast());
}

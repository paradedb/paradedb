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

use crate::api::operator::SearchPredicate;
use crate::nodecast;
use crate::postgres::customscan::score_funcoids;
use pgrx::{pg_guard, pg_sys};

pub(crate) enum WalkControl {
    Continue,
    SkipChildren,
    Break,
}

/// Expression-tree traversal in PostgreSQL's child order, including the root.
/// Null pointers are empty trees. Query and planner nodes need their own traversal.
///
/// # Safety
/// Pointers must be null or valid PostgreSQL expression trees. Visitors may change node
/// fields, but must keep the tree valid and must not free nodes during traversal.
pub(crate) trait NodeExt: Sized {
    /// Returns true if a visitor stopped traversal with `WalkControl::Break`.
    unsafe fn walk<F: FnMut(*mut pg_sys::Node) -> WalkControl>(self, visitor: F) -> bool;

    unsafe fn any(self, mut predicate: impl FnMut(*mut pg_sys::Node) -> bool) -> bool {
        self.walk(|node| {
            if predicate(node) {
                WalkControl::Break
            } else {
                WalkControl::Continue
            }
        })
    }

    unsafe fn visit(self, mut visitor: impl FnMut(*mut pg_sys::Node)) {
        self.walk(|node| {
            visitor(node);
            WalkControl::Continue
        });
    }

    unsafe fn find_node<T: pg_sys::PgNode>(self) -> Option<*mut T> {
        let mut found = None;
        self.any(|node| {
            if T::CAST_TAGS.contains(&(*node).type_) {
                found = Some(node.cast());
                true
            } else {
                false
            }
        });
        found
    }

    unsafe fn collect_nodes<T: pg_sys::PgNode>(self) -> Vec<*mut T> {
        let mut nodes = Vec::new();
        self.visit(|node| {
            if T::CAST_TAGS.contains(&(*node).type_) {
                nodes.push(node.cast());
            }
        });
        nodes
    }

    unsafe fn contains_type(self, tag: pg_sys::NodeTag) -> bool {
        self.any(|node| (*node).type_ == tag)
    }

    unsafe fn contains_functions(self, funcids: &[pg_sys::Oid]) -> bool {
        self.any(|node| {
            nodecast!(FuncExpr, T_FuncExpr, node)
                .is_some_and(|expr| funcids.contains(&(*expr).funcid))
        })
    }

    unsafe fn contains_operators(self, opnos: &[pg_sys::Oid]) -> bool {
        self.any(|node| {
            nodecast!(OpExpr, T_OpExpr, node).is_some_and(|expr| opnos.contains(&(*expr).opno))
        })
    }

    unsafe fn contains_param_kind(self, kind: pg_sys::ParamKind::Type) -> bool {
        self.any(|node| {
            nodecast!(Param, T_Param, node).is_some_and(|param| (*param).paramkind == kind)
        })
    }

    unsafe fn contains_search_predicate(self) -> bool {
        self.any(|node| SearchPredicate::from_node(node).is_some())
    }

    unsafe fn contains_score(self) -> bool {
        self.any(|node| {
            nodecast!(FuncExpr, T_FuncExpr, node)
                .is_some_and(|expr| score_funcoids().contains(&(*expr).funcid))
        })
    }
}

impl<T: pg_sys::PgNode> NodeExt for *mut T {
    unsafe fn walk<F: FnMut(*mut pg_sys::Node) -> WalkControl>(self, mut visitor: F) -> bool {
        #[pg_guard]
        unsafe extern "C-unwind" fn walker<F: FnMut(*mut pg_sys::Node) -> WalkControl>(
            node: *mut pg_sys::Node,
            context: *mut core::ffi::c_void,
        ) -> bool {
            if node.is_null() {
                return false;
            }
            match (&mut *context.cast::<F>())(node) {
                WalkControl::Continue => {
                    pg_sys::expression_tree_walker(node, Some(walker::<F>), context)
                }
                WalkControl::SkipChildren => false,
                WalkControl::Break => true,
            }
        }

        walker::<F>(self.cast(), std::ptr::from_mut(&mut visitor).cast())
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::PgList;
    use pgrx::prelude::*;

    unsafe fn expression_tree() -> (*mut pg_sys::Expr, [*mut pg_sys::Var; 3]) {
        let vars = [1, 2, 3]
            .map(|attno| pg_sys::makeVar(1, attno, pg_sys::BOOLOID, -1, pg_sys::InvalidOid, 0));
        let mut inner = PgList::<pg_sys::Node>::new();
        inner.push(vars[0].cast());
        inner.push(vars[1].cast());
        let inner = pg_sys::makeBoolExpr(pg_sys::BoolExprType::OR_EXPR, inner.into_pg(), -1);
        let mut outer = PgList::<pg_sys::Node>::new();
        outer.push(inner.cast());
        outer.push(vars[2].cast());
        outer.push(vars[0].cast());
        (
            pg_sys::makeBoolExpr(pg_sys::BoolExprType::AND_EXPR, outer.into_pg(), -1),
            vars,
        )
    }

    #[pg_test]
    fn node_walk_includes_root_and_preserves_order_and_duplicates() {
        unsafe {
            let (tree, vars) = expression_tree();
            assert_eq!(tree.find_node::<pg_sys::BoolExpr>(), Some(tree.cast()));
            assert_eq!(tree.find_node::<pg_sys::Var>(), Some(vars[0]));
            assert_eq!(
                tree.collect_nodes::<pg_sys::Var>(),
                vec![vars[0], vars[1], vars[2], vars[0]]
            );
            assert_eq!(vars[0].collect_nodes::<pg_sys::Var>(), vec![vars[0]]);
            assert!(!std::ptr::null_mut::<pg_sys::Node>().any(|_| panic!("visited null")));
        }
    }

    #[pg_test]
    fn node_walk_stops_at_first_match() {
        unsafe {
            let (tree, vars) = expression_tree();
            let mut visited = Vec::new();
            assert!(tree.any(|node| {
                visited.push(node);
                node == tree.cast()
            }));
            assert_eq!(visited, vec![tree.cast()]);

            visited.clear();
            assert!(tree.any(|node| {
                if let Some(var) = nodecast!(Var, T_Var, node) {
                    visited.push(node);
                    return var == vars[1];
                }
                false
            }));
            assert_eq!(visited, vec![vars[0].cast(), vars[1].cast()]);
        }
    }

    #[pg_test]
    fn node_walk_skips_children_and_continues_with_siblings() {
        unsafe {
            let (tree, vars) = expression_tree();
            let mut visited = Vec::new();
            assert!(!tree.walk(|node| {
                if let Some(expr) = nodecast!(BoolExpr, T_BoolExpr, node)
                    && (*expr).boolop == pg_sys::BoolExprType::OR_EXPR
                {
                    return WalkControl::SkipChildren;
                }
                if let Some(var) = nodecast!(Var, T_Var, node) {
                    visited.push(var);
                }
                WalkControl::Continue
            }));
            assert_eq!(visited, vec![vars[2], vars[0]]);
        }
    }

    #[pg_test]
    fn node_walk_supports_captured_state_and_in_place_mutation() {
        unsafe {
            let (tree, vars) = expression_tree();
            let mut visits = 0;
            tree.visit(|node| {
                if let Some(var) = nodecast!(Var, T_Var, node) {
                    (*var).varno = 7;
                    visits += 1;
                }
            });
            assert_eq!(visits, 4);
            assert!(vars.iter().all(|var| (**var).varno == 7));
        }
    }

    #[pg_test(error = "node walk visitor failure")]
    fn node_walk_propagates_visitor_errors() {
        unsafe {
            let (tree, _) = expression_tree();
            tree.visit(|node| {
                if nodecast!(Var, T_Var, node).is_some() {
                    panic!("node walk visitor failure");
                }
            });
        }
    }
}

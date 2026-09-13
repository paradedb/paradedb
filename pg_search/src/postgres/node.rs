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

use crate::api::operator::is_paradedb_search_operator;
use crate::nodecast;
use crate::postgres::customscan::basescan::projections::snippet::{
    snippet_funcoids, snippet_positions_funcoids,
};
use crate::postgres::customscan::collation_semantics::{CollationOperation, collation_supports};
use crate::postgres::customscan::joinscan::build::JoinSource;
use crate::postgres::customscan::score_funcoids;
use crate::postgres::utils::is_unnest_func;
use pgrx::{PgList, pg_guard, pg_sys};
use std::collections::HashSet;

/// A Var reference with its range table index and attribute number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct VarRef {
    /// Range table index (varno)
    pub rti: pg_sys::Index,
    /// Attribute number (varattno), 1-indexed
    pub attno: pg_sys::AttrNumber,
}

pub(crate) enum WalkControl {
    Continue,
    SkipChildren,
    Break,
}

/// Expression-tree traversal in PostgreSQL's child order, including the root.
/// Null pointers are empty trees. Query and planner nodes need their own traversal.
///
/// # Safety
/// The receiver must be null or a valid PostgreSQL expression tree. Other pointers must
/// be valid for their declared types. Visitors may change fields, but must keep the tree
/// valid and must not free nodes during traversal.
pub(crate) trait NodeExt: Copy {
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

    unsafe fn find_single_node<T: pg_sys::PgNode>(self) -> Option<*mut T> {
        match self.collect_nodes::<T>().as_slice() {
            [node] => Some(*node),
            _ => None,
        }
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

    unsafe fn collect_rtis(self) -> HashSet<pg_sys::Index> {
        let mut rtis = HashSet::new();
        self.visit(|node| {
            if let Some(var) = nodecast!(Var, T_Var, node) {
                let varno = (*var).varno as pg_sys::Index;
                if varno > 0 && varno < pg_sys::INNER_VAR as pg_sys::Index {
                    rtis.insert(varno);
                }
            }
        });
        rtis
    }

    /// Collects positive attribute numbers; `include_special_vars` bypasses the varno filter.
    unsafe fn collect_var_refs(self, include_special_vars: bool) -> Vec<VarRef> {
        let mut vars = Vec::new();
        self.visit(|node| {
            if let Some(var) = nodecast!(Var, T_Var, node) {
                let varno = (*var).varno as pg_sys::Index;
                if (*var).varattno > 0
                    && (include_special_vars
                        || (varno > 0 && varno < pg_sys::INNER_VAR as pg_sys::Index))
                {
                    vars.push(VarRef {
                        rti: varno,
                        attno: (*var).varattno,
                    });
                }
            }
        });
        vars
    }

    unsafe fn collect_subplan_ids(self, ids: &mut crate::api::HashSet<i32>) {
        self.visit(|node| {
            if let Some(subplan) = nodecast!(SubPlan, T_SubPlan, node) {
                ids.insert((*subplan).plan_id);
            }
        });
    }

    unsafe fn contains_type(self, tag: pg_sys::NodeTag) -> bool {
        self.any(|node| (*node).type_ == tag)
    }

    unsafe fn contains_var(self) -> bool {
        self.contains_type(pg_sys::NodeTag::T_Var)
    }

    unsafe fn contains_param(self) -> bool {
        self.contains_type(pg_sys::NodeTag::T_Param)
    }

    unsafe fn contains_aggref(self) -> bool {
        self.contains_type(pg_sys::NodeTag::T_Aggref)
    }

    unsafe fn contains_window_func(self) -> bool {
        self.contains_type(pg_sys::NodeTag::T_WindowFunc)
    }

    unsafe fn contains_param_kind(self, kind: pg_sys::ParamKind::Type) -> bool {
        self.any(|node| {
            nodecast!(Param, T_Param, node).is_some_and(|param| (*param).paramkind == kind)
        })
    }

    unsafe fn contains_exec_param(self) -> bool {
        self.contains_param_kind(pg_sys::ParamKind::PARAM_EXEC)
    }

    /// Prepared-statement parameters are bound when a generic plan executes.
    unsafe fn contains_extern_param(self) -> bool {
        self.contains_param_kind(pg_sys::ParamKind::PARAM_EXTERN)
    }

    /// Correlated PARAM_EXEC values are not supplied by an init plan.
    unsafe fn contains_correlated_param(self, root: *mut pg_sys::PlannerInfo) -> bool {
        self.any(|node| {
            nodecast!(Param, T_Param, node).is_some_and(|param| {
                (*param).paramkind == pg_sys::ParamKind::PARAM_EXEC
                    && !PgList::<pg_sys::SubPlan>::from_pg((*root).init_plans)
                        .iter_ptr()
                        .any(|subplan| {
                            pg_sys::list_member_int((*subplan).setParam, (*param).paramid)
                        })
            })
        })
    }

    unsafe fn contains_relation_reference(self, target_rti: pg_sys::Index) -> bool {
        self.any(|node| {
            nodecast!(Var, T_Var, node)
                .is_some_and(|var| (*var).varno as pg_sys::Index == target_rti)
        })
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

    unsafe fn contains_paradedb_operator(self) -> bool {
        self.any(|node| {
            nodecast!(OpExpr, T_OpExpr, node)
                .is_some_and(|expr| is_paradedb_search_operator((*expr).opno))
        })
    }

    unsafe fn contains_unnest(self) -> bool {
        self.any(|node| {
            nodecast!(FuncExpr, T_FuncExpr, node).is_some_and(|expr| is_unnest_func((*expr).funcid))
        })
    }

    unsafe fn contains_score(self) -> bool {
        self.any(|node| {
            nodecast!(FuncExpr, T_FuncExpr, node)
                .is_some_and(|expr| score_funcoids().contains(&(*expr).funcid))
        })
    }

    unsafe fn contains_score_for_relation(
        self,
        score_funcoids: [pg_sys::Oid; 2],
        rti: pg_sys::Index,
    ) -> bool {
        self.any(|node| {
            if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node)
                && score_funcoids.contains(&(*funcexpr).funcid)
            {
                let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
                assert!(args.len() == 1, "score function must have 1 argument");
                return nodecast!(Var, T_Var, args.get_ptr(0).unwrap())
                    .is_some_and(|var| (*var).varno == rti as i32);
            }
            false
        })
    }

    unsafe fn contains_score_from(self, source: &JoinSource) -> bool {
        let funcoids = score_funcoids();
        self.any(|node| {
            if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node)
                && funcoids.contains(&(*funcexpr).funcid)
            {
                let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
                return args.len() == 1
                    && nodecast!(Var, T_Var, args.get_ptr(0).unwrap())
                        .is_some_and(|var| source.contains_rti((*var).varno as pg_sys::Index));
            }
            false
        })
    }

    unsafe fn maybe_needs_const_projections(self) -> bool {
        let score_funcoids = score_funcoids();
        let snippet_funcoids = snippet_funcoids();
        let snippet_positions_funcoids = snippet_positions_funcoids();
        self.any(|node| {
            nodecast!(FuncExpr, T_FuncExpr, node).is_some_and(|expr| {
                score_funcoids.contains(&(*expr).funcid)
                    || snippet_funcoids.contains(&(*expr).funcid)
                    || snippet_positions_funcoids.contains(&(*expr).funcid)
            })
        })
    }

    unsafe fn has_unsupported_collation(self) -> bool {
        self.any(|node| {
            let Some(op_expr) = nodecast!(OpExpr, T_OpExpr, node) else {
                return false;
            };
            let collid = (*op_expr).inputcollid;
            if collid == pg_sys::Oid::INVALID {
                return false;
            }
            let op_name_ptr = pg_sys::get_opname((*op_expr).opno);
            (!op_name_ptr.is_null())
                .then(|| std::ffi::CStr::from_ptr(op_name_ptr).to_str().ok())
                .flatten()
                .and_then(|op_str| match op_str {
                    "=" | "<>" | "!=" => Some(CollationOperation::Equality),
                    "<" | "<=" | ">" | ">=" => Some(CollationOperation::Ordering),
                    _ => None,
                })
                .is_some_and(|op_type| !collation_supports(collid, op_type))
        })
    }

    unsafe fn is_complex(self) -> bool {
        self.any(|node| {
            nodecast!(Var, T_Var, node).is_some()
                || nodecast!(Param, T_Param, node).is_some()
                || pg_sys::contain_volatile_functions(node)
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
            assert_eq!(tree.find_single_node::<pg_sys::Var>(), None);
            assert_eq!(vars[0].find_single_node::<pg_sys::Var>(), Some(vars[0]));
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

    #[pg_test]
    fn node_walk_contains_unnest_at_root_and_inside_target_entry() {
        unsafe {
            let mut func = PgBox::<pg_sys::FuncExpr>::alloc_node(pg_sys::NodeTag::T_FuncExpr);
            func.funcid = pg_sys::F_UNNEST_ANYARRAY.into();
            let func = func.into_pg();
            let target = pg_sys::makeTargetEntry(func.cast(), 1, std::ptr::null_mut(), false);
            assert!(func.contains_unnest());
            assert!(target.contains_unnest());
            (*func).funcid = pg_sys::InvalidOid;
            assert!(!target.contains_unnest());
        }
    }

    #[pg_test]
    fn node_walk_distinguishes_prepared_initplan_and_correlated_params() {
        unsafe {
            let mut param = PgBox::<pg_sys::Param>::alloc_node(pg_sys::NodeTag::T_Param);
            param.paramkind = pg_sys::ParamKind::PARAM_EXTERN;
            param.paramid = 7;
            let param = param.into_pg();
            let mut root = PgBox::<pg_sys::PlannerInfo>::alloc_node(pg_sys::NodeTag::T_PlannerInfo);
            assert!(param.contains_param());
            assert!(param.contains_extern_param());
            assert!(!param.contains_exec_param());
            assert!(!param.contains_correlated_param(root.as_ptr()));

            (*param).paramkind = pg_sys::ParamKind::PARAM_EXEC;
            assert!(param.contains_exec_param());
            assert!(!param.contains_extern_param());
            assert!(param.contains_correlated_param(root.as_ptr()));

            let mut subplan = PgBox::<pg_sys::SubPlan>::alloc_node(pg_sys::NodeTag::T_SubPlan);
            subplan.setParam = pg_sys::lappend_int(std::ptr::null_mut(), 7);
            let mut init_plans = PgList::new();
            init_plans.push(subplan.into_pg());
            root.init_plans = init_plans.into_pg();
            assert!(!param.contains_correlated_param(root.as_ptr()));
            (*param).paramid = 8;
            assert!(param.contains_correlated_param(root.as_ptr()));
        }
    }

    #[pg_test]
    fn node_walk_collects_relation_references_without_deduplicating_columns() {
        unsafe {
            let (tree, vars) = expression_tree();
            (*vars[1]).varno = pg_sys::INNER_VAR;
            (*vars[2]).varattno = pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber;
            assert!(tree.contains_relation_reference(1));
            assert!(!tree.contains_relation_reference(2));
            assert_eq!(tree.collect_rtis(), HashSet::from([1]));
            let column = VarRef { rti: 1, attno: 1 };
            assert_eq!(tree.collect_var_refs(false), vec![column, column]);
            assert_eq!(
                tree.collect_var_refs(true),
                vec![
                    column,
                    VarRef {
                        rti: pg_sys::INNER_VAR as pg_sys::Index,
                        attno: 2
                    },
                    column
                ]
            );
        }
    }
}

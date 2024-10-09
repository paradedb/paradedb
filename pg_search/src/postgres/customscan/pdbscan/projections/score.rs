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
use pgrx::pg_sys::expression_tree_walker;
use pgrx::{direct_function_call, pg_extern, pg_guard, pg_sys, AnyElement, IntoDatum, PgList};
use std::ptr::addr_of_mut;

#[pg_extern(name = "score", stable, parallel_safe, cost = 1)]
fn score_from_relation(_relation_reference: AnyElement) -> f32 {
    f32::NAN
}

pub fn score_funcoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"paradedb.score(anyelement)".into_datum()],
        )
        .expect("the `paradedb.score(anyelement) type should exist")
    }
}

pub unsafe fn uses_scores(node: *mut pg_sys::Node, mut score_funcoid: pg_sys::Oid) -> bool {
    #[pg_guard]
    unsafe extern "C" fn walker(node: *mut pg_sys::Node, data: *mut core::ffi::c_void) -> bool {
        if node.is_null() {
            return false;
        }

        if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
            let score_funcoid = data.cast::<pg_sys::Oid>();
            if (*funcexpr).funcid == *score_funcoid {
                return true;
            }
        }

        expression_tree_walker(node, Some(walker), data)
    }

    walker(node, addr_of_mut!(score_funcoid).cast())
}

pub unsafe fn inject_scores(
    node: *mut pg_sys::Node,
    score_funcoid: pg_sys::Oid,
    score: f32,
) -> *mut pg_sys::Node {
    #[derive(Debug)]
    struct Context {
        score_funcoid: pg_sys::Oid,
        score: f32,
    }

    #[pg_guard]
    unsafe extern "C" fn walker(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> *mut pg_sys::Node {
        if node.is_null() {
            return std::ptr::null_mut();
        }

        if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
            let context = data.cast::<Context>();
            if (*funcexpr).funcid == (*context).score_funcoid {
                let const_ = pg_sys::makeConst(
                    pg_sys::FLOAT4OID,
                    -1,
                    pg_sys::Oid::INVALID,
                    size_of::<f32>() as _,
                    (*context).score.into_datum().unwrap(),
                    false,
                    true,
                );

                return const_.cast();
            }
        }

        #[cfg(not(any(feature = "pg16", feature = "pg17")))]
        {
            let fnptr = walker as usize as *const ();
            let walker: unsafe extern "C" fn() -> *mut pg_sys::Node = std::mem::transmute(fnptr);
            pg_sys::expression_tree_mutator(node, Some(walker), data)
        }

        #[cfg(any(feature = "pg16", feature = "pg17"))]
        {
            pg_sys::expression_tree_mutator_impl(node, Some(walker), data)
        }
    }

    let mut context = Context {
        score_funcoid,
        score,
    };

    let data = addr_of_mut!(context);
    walker(node, data.cast())
}

pub unsafe fn is_score_func(node: *mut pg_sys::Node, rti: i32) -> bool {
    if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
        if (*funcexpr).funcid == score_funcoid() {
            let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
            assert!(args.len() == 1, "score function must have 1 argument");
            if let Some(var) = nodecast!(Var, T_Var, args.get_ptr(0).unwrap()) {
                if (*var).varno == rti {
                    return true;
                }
            }
        }
    }

    false
}

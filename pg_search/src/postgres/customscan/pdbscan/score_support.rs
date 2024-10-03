use crate::nodecast;
use pgrx::callconv::{Arg, ArgAbi};
use pgrx::pg_sys::expression_tree_walker;
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
};
use pgrx::{direct_function_call, pg_extern, pg_guard, pg_sys, IntoDatum};
use std::ptr::addr_of_mut;

pub struct OpaqueRecordArg;

unsafe impl<'fcx> ArgAbi<'fcx> for OpaqueRecordArg {
    unsafe fn unbox_arg_unchecked(arg: Arg<'_, 'fcx>) -> Self {
        OpaqueRecordArg
    }
}

unsafe impl SqlTranslatable for OpaqueRecordArg {
    fn argument_sql() -> Result<SqlMapping, ArgumentError> {
        Ok(SqlMapping::As("record".into()))
    }

    fn return_sql() -> Result<Returns, ReturnsError> {
        Ok(Returns::One(SqlMapping::As("record".into())))
    }
}

#[pg_extern(name = "score", stable, parallel_safe)]
fn score_from_relation(_relation_reference: OpaqueRecordArg) -> f32 {
    f32::NAN
}

pub(super) fn score_funcoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"paradedb.score(record)".into_datum()],
        )
        .expect("the `paradedb.score(record) type should exist")
    }
}

pub(super) unsafe fn has_var_for_rel(node: *mut pg_sys::Node, mut relid: pg_sys::Oid) -> bool {
    #[pg_guard]
    unsafe extern "C" fn walker(node: *mut pg_sys::Node, data: *mut core::ffi::c_void) -> bool {
        if node.is_null() {
            return false;
        }

        if let Some(var) = nodecast!(Var, T_Var, node) {
            let relid = *data.cast::<pg_sys::Oid>();
            if (*var).vartype == relid {
                return true;
            }
        }

        expression_tree_walker(node, Some(walker), data)
    }

    let data = addr_of_mut!(relid).cast();
    walker(node, data)
}

pub(super) unsafe fn requires_score(
    node: *mut pg_sys::Node,
    mut score_funcoid: pg_sys::Oid,
) -> bool {
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

pub(super) unsafe fn inject_scores(
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

        pg_sys::expression_tree_mutator_impl(node, Some(walker), data)
    }

    let mut context = Context {
        score_funcoid,
        score,
    };

    let data = addr_of_mut!(context);
    walker(node, data.cast())
}

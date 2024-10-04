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

pub mod score;
pub mod snippet;

use crate::nodecast;
use pgrx::callconv::{Arg, ArgAbi};
use pgrx::pg_sys::expression_tree_walker;
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
};
use pgrx::{pg_guard, pg_sys};
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

pub unsafe fn has_var_for_rel(node: *mut pg_sys::Node, mut relid: pg_sys::Oid) -> bool {
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

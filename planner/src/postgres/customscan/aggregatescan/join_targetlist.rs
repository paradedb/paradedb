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
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use pgrx::{PgList, pg_sys};

/// Unwrap an expression to a bare `Var`, allowing only `RelabelType` wrappers.
/// Returns `None` for anything more complex (COALESCE, FuncExpr, etc.)
/// so the caller can reject and fall back to native Postgres.
pub fn unwrap_to_var(mut node: *mut pg_sys::Node) -> Option<*mut pg_sys::Var> {
    while !node.is_null() {
        match unsafe { (*node).type_ } {
            pg_sys::NodeTag::T_Var => return Some(node as *mut pg_sys::Var),
            pg_sys::NodeTag::T_RelabelType => {
                node = unsafe { (*(node as *mut pg_sys::RelabelType)).arg as *mut pg_sys::Node };
            }
            _ => return None,
        }
    }
    None
}

/// Split an `int8` `COALESCE(column, constant)` into its column and default.
/// Returns `None` for any other shape, including a NULL default.
pub unsafe fn unwrap_int8_coalesce(node: *mut pg_sys::Node) -> Option<(*mut pg_sys::Var, i64)> {
    let coalesce = crate::nodecast!(CoalesceExpr, T_CoalesceExpr, node)?;
    if (*coalesce).coalescetype != pg_sys::INT8OID {
        return None;
    }
    let args = PgList::<pg_sys::Node>::from_pg((*coalesce).args);
    if args.len() != 2 {
        return None;
    }
    let var = unwrap_to_var(args.get_ptr(0)?)?;
    let default = crate::nodecast!(Const, T_Const, args.get_ptr(1)?)?;
    let value: Option<i64> =
        pgrx::FromDatum::from_datum((*default).constvalue, (*default).constisnull);
    Some((var, value?))
}

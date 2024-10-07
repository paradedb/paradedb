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
use crate::postgres::customscan::pdbscan::projections::score::score_funcoid;
use crate::postgres::customscan::pdbscan::projections::snippet::snippet_funcoid;
use pgrx::pg_sys::expression_tree_walker;
use pgrx::{pg_guard, pg_sys};
use std::ptr::addr_of_mut;

pub unsafe fn maybe_needs_const_projections(node: *mut pg_sys::Node) -> bool {
    #[pg_guard]
    unsafe extern "C" fn walker(node: *mut pg_sys::Node, data: *mut core::ffi::c_void) -> bool {
        if node.is_null() {
            return false;
        }

        if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
            let data = &*data.cast::<Data>();
            if (*funcexpr).funcid == data.score_funcoid
                || (*funcexpr).funcid == data.snipped_funcoid
            {
                return true;
            }
        }

        expression_tree_walker(node, Some(walker), data)
    }

    struct Data {
        score_funcoid: pg_sys::Oid,
        snipped_funcoid: pg_sys::Oid,
    }

    let mut data = Data {
        score_funcoid: score_funcoid(),
        snipped_funcoid: snippet_funcoid(),
    };

    let data = addr_of_mut!(data).cast();
    walker(node, data)
}

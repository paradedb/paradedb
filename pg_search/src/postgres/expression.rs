// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use pgrx::{
    pg_sys::{self, AsPgCStr},
    PgList, PgRelation,
};

pub const PG_SEARCH_PREFIX: &str = "_pg_search_";

/// Returns the attribute number of the node in the index.
pub fn find_funcexpr_attnum(
    heaprel: &PgRelation,
    indexrel: &PgRelation,
    node: *mut pg_sys::Node,
) -> Option<i32> {
    let index_info = unsafe { *pg_sys::BuildIndexInfo(indexrel.as_ptr()) };

    let expressions = unsafe { PgList::<pg_sys::Expr>::from_pg(index_info.ii_Expressions) };
    let ref_str = unsafe { get_expr_str(node, heaprel) };
    let mut expressions_iter = expressions.iter_ptr();

    for i in 0..index_info.ii_NumIndexAttrs {
        let heap_attno = index_info.ii_IndexAttrNumbers[i as usize];
        if heap_attno == 0 {
            let Some(expression) = expressions_iter.next() else {
                panic!("Expected expression for index attribute {i}.");
            };
            let node = expression.cast();

            let expr_str = unsafe { get_expr_str(node, heaprel) };
            if expr_str == ref_str {
                return Some(i);
            }
        }
    }
    None
}

unsafe fn get_expr_str(node: *mut pg_sys::Node, heaprel: &PgRelation) -> String {
    let pg_cstr = pg_sys::deparse_expression(
        node,
        pg_sys::deparse_context_for(heaprel.name().as_pg_cstr(), heaprel.oid()),
        false,
        false,
    );
    let ref_str = core::ffi::CStr::from_ptr(pg_cstr)
        .to_str()
        .expect("Invalid UTF8 in result of deparse_expression.")
        .to_owned();

    pg_sys::pfree(pg_cstr.cast());
    ref_str
}

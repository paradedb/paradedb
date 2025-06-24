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

use crate::postgres::rel::PgSearchRelation;
use pgrx::{pg_sys, PgList};

pub const PG_SEARCH_PREFIX: &str = "_pg_search_";

/// Returns the attribute number of the node in the index.
pub fn find_expr_attnum(indexrel: &PgSearchRelation, node: *mut pg_sys::Node) -> Option<i32> {
    let index_info = unsafe { *pg_sys::BuildIndexInfo(indexrel.as_ptr()) };

    let expressions = unsafe { PgList::<pg_sys::Expr>::from_pg(index_info.ii_Expressions) };
    let mut expressions_iter = expressions.iter_ptr();

    for i in 0..index_info.ii_NumIndexAttrs {
        let heap_attno = index_info.ii_IndexAttrNumbers[i as usize];
        if heap_attno == 0 {
            let Some(expression) = expressions_iter.next() else {
                panic!("Expected expression for index attribute {i}.");
            };

            if unsafe { pg_sys::equal(node.cast(), expression.cast()) } {
                return Some(i);
            }
        }
    }
    None
}

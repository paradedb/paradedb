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
use pgrx::{pg_guard, pg_sys};

pub(crate) trait NodeExt {
    /// # Safety
    /// The pointer must be null or point to a valid PostgreSQL expression tree.
    unsafe fn contains_search_predicate(self) -> bool;
}

impl<T: pg_sys::PgNode> NodeExt for *mut T {
    unsafe fn contains_search_predicate(self) -> bool {
        #[pg_guard]
        unsafe extern "C-unwind" fn walker(
            node: *mut pg_sys::Node,
            context: *mut core::ffi::c_void,
        ) -> bool {
            if node.is_null() {
                return false;
            }
            if SearchPredicate::from_node(node).is_some() {
                return true;
            }
            pg_sys::expression_tree_walker(node, Some(walker), context)
        }

        walker(self.cast(), std::ptr::null_mut())
    }
}

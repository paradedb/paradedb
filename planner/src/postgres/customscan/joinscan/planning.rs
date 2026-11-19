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
#![allow(clippy::missing_safety_doc, unsafe_op_in_unsafe_fn)]

use pgrx::pg_sys;

pub unsafe fn find_final_rel(root: *mut pg_sys::PlannerInfo) -> *mut pg_sys::RelOptInfo {
    let mut final_rel = std::ptr::null_mut();

    let join_rels = pgrx::PgList::<pg_sys::RelOptInfo>::from_pg((*root).join_rel_list);
    let all_baserels = (*root).all_baserels;
    for rel in join_rels.iter_ptr() {
        if pgrx::pg_sys::bms_is_subset(all_baserels, (*rel).relids) {
            final_rel = rel;
            break;
        }
    }

    if final_rel.is_null() && (*root).simple_rel_array_size > 1 {
        for i in 1..(*root).simple_rel_array_size {
            let rel = *(*root).simple_rel_array.add(i as usize);
            if !rel.is_null() && (*rel).reloptkind == pg_sys::RelOptKind::RELOPT_BASEREL {
                final_rel = rel;
                break;
            }
        }
    }

    final_rel
}

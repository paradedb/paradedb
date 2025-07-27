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

use crate::nodecast;
use crate::postgres::customscan::score_funcoid;
use pgrx::pg_sys::expression_tree_walker;
use pgrx::{
    extension_sql, pg_extern, pg_guard, pg_sys, AnyElement, PgList, PgLogLevel, PgSqlErrorCode,
};
use std::ptr::addr_of_mut;

#[pg_extern(name = "score", stable, parallel_safe, cost = 1)]
fn score_from_relation(_relation_reference: AnyElement) -> Option<f32> {
    None
}

#[pg_extern(name = "score")]
fn score_invalid_signature() {
    pg_sys::submodules::panic::ErrorReport::new(
        PgSqlErrorCode::ERRCODE_INVALID_COLUMN_REFERENCE,
        "paradedb.score has an invalid signature.",
        "",
    )
    .set_detail("Missing argument in paradedb.score")
    .set_hint("Use paradedb.score(<column_name/key_field>) instead")
    .report(PgLogLevel::ERROR);
}

extension_sql!(
    r#"
ALTER FUNCTION score SUPPORT placeholder_support;
"#,
    name = "score_placeholder",
    requires = [score_from_relation, placeholder_support]
);

pub unsafe fn uses_scores(
    node: *mut pg_sys::Node,
    score_funcoid: pg_sys::Oid,
    rti: pg_sys::Index,
) -> bool {
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
            let data = data.cast::<Data>();
            if (*funcexpr).funcid == (*data).score_funcoid {
                let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
                assert!(args.len() == 1, "score function must have 1 argument");
                if let Some(var) = nodecast!(Var, T_Var, args.get_ptr(0).unwrap()) {
                    if (*var).varno as i32 == (*data).rti as i32 {
                        return true;
                    }
                }
            }
        }

        expression_tree_walker(node, Some(walker), data)
    }

    struct Data {
        score_funcoid: pg_sys::Oid,
        rti: pg_sys::Index,
    }

    let mut data = Data { score_funcoid, rti };

    walker(node, addr_of_mut!(data).cast())
}

pub unsafe fn is_score_func(node: *mut pg_sys::Node, rti: pg_sys::Index) -> bool {
    if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
        if (*funcexpr).funcid == score_funcoid() {
            let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
            assert!(args.len() == 1, "score function must have 1 argument");
            if let Some(var) = nodecast!(Var, T_Var, args.get_ptr(0).unwrap()) {
                if (*var).varno as i32 == rti as i32 {
                    return true;
                }
            }
        }
    }

    false
}

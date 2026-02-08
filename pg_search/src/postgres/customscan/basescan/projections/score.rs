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

use crate::nodecast;
use crate::postgres::customscan::score_funcoids;
use pgrx::pg_sys::expression_tree_walker;
use pgrx::{extension_sql, pg_extern, pg_guard, pg_sys, AnyElement, PgList};
use std::ptr::addr_of_mut;

#[pgrx::pg_schema]
mod pdb {
    use pgrx::{extension_sql, pg_extern, AnyElement};

    #[allow(unused_variables)]
    #[pg_extern(name = "score", stable, parallel_safe, cost = 1)]
    fn score_from_relation(relation_reference: AnyElement) -> f32 {
        panic!("Unsupported query shape. Please report at https://github.com/orgs/paradedb/discussions/3678");
    }

    extension_sql!(
        r#"
    ALTER FUNCTION pdb.score SUPPORT paradedb.placeholder_support;
    "#,
        name = "score_placeholder",
        requires = [score_from_relation, placeholder_support]
    );
}

// In `0.19.0`, we renamed the schema from `paradedb` to `pdb`.
// This is a backwards compatibility shim to ensure that old queries continue to work.
#[warn(deprecated)]
#[allow(unused_variables)]
#[pg_extern(name = "score", stable, parallel_safe, cost = 1)]
fn paradedb_score_from_relation(relation_reference: AnyElement) -> Option<f32> {
    panic!("Unsupported query shape. Please report at https://github.com/orgs/paradedb/discussions/3678");
}

extension_sql!(
    r#"
    ALTER FUNCTION paradedb.score SUPPORT paradedb.placeholder_support;
    "#,
    name = "paradedb_score_placeholder",
    requires = [paradedb_score_from_relation, placeholder_support]
);

pub unsafe fn uses_scores(
    node: *mut pg_sys::Node,
    score_funcoids: [pg_sys::Oid; 2],
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
            if (*data).score_funcoids.contains(&(*funcexpr).funcid) {
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
        score_funcoids: [pg_sys::Oid; 2],
        rti: pg_sys::Index,
    }

    let mut data = Data {
        score_funcoids,
        rti,
    };

    walker(node, addr_of_mut!(data).cast())
}

pub unsafe fn is_score_func(node: *mut pg_sys::Node, rti: pg_sys::Index) -> bool {
    if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
        if score_funcoids().contains(&(*funcexpr).funcid) {
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

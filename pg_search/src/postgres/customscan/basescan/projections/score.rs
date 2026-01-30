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
use pgrx::{extension_sql, pg_extern, pg_sys, AnyElement, PgList};

#[pgrx::pg_schema]
mod pdb {
    use pgrx::{extension_sql, pg_extern, AnyElement};

    #[pg_extern(name = "score", stable, parallel_safe, cost = 1)]
    fn score_from_relation(_relation_reference: AnyElement) -> f32 {
        panic!("Unsupported query shape. Please report at https://github.com/orgs/paradedb/discussions/3678");
    }

    /// Score function with custom BM25 parameters.
    /// - `b`: Length normalization parameter (0.0 to 1.0, default 0.75)
    /// - `k1`: Term saturation parameter (default 1.2)
    #[pg_extern(name = "score", stable, parallel_safe, cost = 1)]
    fn score_from_relation_with_bm25_settings(
        _relation_reference: AnyElement,
        _b: f32,
        _k1: f32,
    ) -> f32 {
        panic!("Unsupported query shape. Please report at https://github.com/orgs/paradedb/discussions/3678");
    }

    extension_sql!(
        r#"
    ALTER FUNCTION pdb.score(anyelement) SUPPORT paradedb.placeholder_support;
    ALTER FUNCTION pdb.score(anyelement, float4, float4) SUPPORT paradedb.placeholder_support;
    "#,
        name = "score_placeholder",
        requires = [
            score_from_relation,
            score_from_relation_with_bm25_settings,
            placeholder_support
        ]
    );
}

// In `0.19.0`, we renamed the schema from `paradedb` to `pdb`.
// This is a backwards compatibility shim to ensure that old queries continue to work.
// Note: Only 1-arg variant is supported for paradedb.score (use pdb.score for custom BM25 params)
#[warn(deprecated)]
#[pg_extern(name = "score", stable, parallel_safe, cost = 1)]
fn paradedb_score_from_relation(_relation_reference: AnyElement) -> Option<f32> {
    panic!("Unsupported query shape. Please report at https://github.com/orgs/paradedb/discussions/3678");
}

extension_sql!(
    r#"
    ALTER FUNCTION paradedb.score(anyelement) SUPPORT paradedb.placeholder_support;
    "#,
    name = "paradedb_score_placeholder",
    requires = [paradedb_score_from_relation, placeholder_support]
);

pub unsafe fn is_score_func(node: *mut pg_sys::Node, rti: pg_sys::Index) -> bool {
    if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
        if score_funcoids().contains(&(*funcexpr).funcid) {
            let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
            // Score function can have 1 argument (key) or 3 arguments (key, b, k1)
            assert!(
                args.len() == 1 || args.len() == 3,
                "score function must have 1 or 3 arguments"
            );
            if let Some(var) = nodecast!(Var, T_Var, args.get_ptr(0).unwrap()) {
                if (*var).varno as i32 == rti as i32 {
                    return true;
                }
            }
        }
    }

    false
}

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
use crate::postgres::customscan::basescan::projections::snippet::{
    snippet_funcoids, snippet_positions_funcoids,
};
use crate::postgres::customscan::collation_semantics::{CollationOperation, collation_supports};
use crate::postgres::customscan::score_funcoids;
use crate::postgres::node::NodeExt;
use pgrx::{PgList, pg_sys};

pub(crate) trait CustomScanNodeExt: NodeExt {
    unsafe fn contains_score(self) -> bool {
        self.any(|node| {
            nodecast!(FuncExpr, T_FuncExpr, node)
                .is_some_and(|expr| score_funcoids().contains(&(*expr).funcid))
        })
    }

    unsafe fn contains_score_for_relation(
        self,
        score_funcoids: [pg_sys::Oid; 2],
        rti: pg_sys::Index,
    ) -> bool {
        self.any(|node| {
            if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node)
                && score_funcoids.contains(&(*funcexpr).funcid)
            {
                let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
                assert!(args.len() == 1, "score function must have 1 argument");
                return nodecast!(Var, T_Var, args.get_ptr(0).unwrap())
                    .is_some_and(|var| (*var).varno == rti as i32);
            }
            false
        })
    }

    unsafe fn maybe_needs_const_projections(self) -> bool {
        let score_funcoids = score_funcoids();
        let snippet_funcoids = snippet_funcoids();
        let snippet_positions_funcoids = snippet_positions_funcoids();
        self.any(|node| {
            nodecast!(FuncExpr, T_FuncExpr, node).is_some_and(|expr| {
                score_funcoids.contains(&(*expr).funcid)
                    || snippet_funcoids.contains(&(*expr).funcid)
                    || snippet_positions_funcoids.contains(&(*expr).funcid)
            })
        })
    }

    unsafe fn has_unsupported_collation(self) -> bool {
        self.any(|node| {
            let Some(op_expr) = nodecast!(OpExpr, T_OpExpr, node) else {
                return false;
            };
            let collid = (*op_expr).inputcollid;
            if collid == pg_sys::Oid::INVALID {
                return false;
            }
            let op_name_ptr = pg_sys::get_opname((*op_expr).opno);
            (!op_name_ptr.is_null())
                .then(|| std::ffi::CStr::from_ptr(op_name_ptr).to_str().ok())
                .flatten()
                .and_then(|op_str| match op_str {
                    "=" | "<>" | "!=" => Some(CollationOperation::Equality),
                    "<" | "<=" | ">" | ">=" => Some(CollationOperation::Ordering),
                    _ => None,
                })
                .is_some_and(|op_type| !collation_supports(collid, op_type))
        })
    }
}

impl<T: NodeExt> CustomScanNodeExt for T {}

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
use pgrx::{pg_sys, FromDatum, PgList};
use serde::{Deserialize, Serialize};

/// Parsed LIMIT and OFFSET from a query's parse tree.
///
/// Both fields are `Option<u32>` mirroring PostgreSQL's native representation.
/// Callers decide how to handle absence — JoinScan bails out when `limit`
/// is `None`, while AggregateScan proceeds normally.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct LimitOffset {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl LimitOffset {
    /// Extract LIMIT and OFFSET from the parse tree.
    ///
    /// Reads `limitCount` and `limitOffset` directly from `parse` as Const nodes.
    /// Use this for AggregateScan and other callers that don't need `limit_tuples`.
    pub unsafe fn from_parse(parse: *mut pg_sys::Query) -> Self {
        Self {
            limit: extract_const_u32((*parse).limitCount),
            offset: extract_const_u32((*parse).limitOffset),
        }
    }

    /// Extract LIMIT and OFFSET from the planner root.
    ///
    /// When `limit_tuples > -1.0`, PostgreSQL has already baked `limit + offset`
    /// into it. We subtract the offset to recover the true limit value.
    /// Use this for JoinScan where `limit_tuples` may be set by the planner.
    pub unsafe fn from_root(root: *mut pg_sys::PlannerInfo) -> Self {
        let parse = (*root).parse;
        let offset = extract_const_u32((*parse).limitOffset);

        let limit = if (*root).limit_tuples > -1.0 {
            let combined = (*root).limit_tuples as u32;
            Some(combined - offset.unwrap_or(0))
        } else {
            extract_const_u32((*parse).limitCount)
        };

        Self { limit, offset }
    }

    pub fn limit(&self) -> Option<u32> {
        self.limit
    }

    pub fn offset(&self) -> Option<u32> {
        self.offset
    }

    /// Returns limit + offset as usize, which is the total number of rows DataFusion
    /// must produce before PostgreSQL's outer Limit node applies the skip.
    pub fn fetch(&self) -> Option<usize> {
        self.limit.map(|l| (l + self.offset.unwrap_or(0)) as usize)
    }
}

/// Extract a `Const` node's value as `u32`.
/// Returns `None` if the node is null, not a Const, or the datum is null.
pub unsafe fn extract_const_u32(node: *mut pg_sys::Node) -> Option<u32> {
    let const_node = nodecast!(Const, T_Const, node)?;
    u32::from_datum((*const_node).constvalue, (*const_node).constisnull)
}

/// Extract a `Const` node's value as `i64`.
/// Returns `None` if the node is null, not a Const, or the datum is null.
pub unsafe fn extract_const_i64(node: *mut pg_sys::Node) -> Option<i64> {
    let const_node = nodecast!(Const, T_Const, node)?;
    i64::from_datum((*const_node).constvalue, (*const_node).constisnull)
}

/// If the expression tree contains an external `Param` node, return its 1-based paramid.
/// Handles cases where the Param is wrapped in a FuncExpr (e.g., int4 → int8 cast)
/// or other single-argument wrappers.
pub unsafe fn find_extern_param_id(node: *mut pg_sys::Node) -> Option<i32> {
    if node.is_null() {
        return None;
    }

    if let Some(param) = nodecast!(Param, T_Param, node) {
        if (*param).paramkind == pg_sys::ParamKind::PARAM_EXTERN {
            return Some((*param).paramid);
        }
        return None;
    }

    // LIMIT $2 where $2 is int4 gets wrapped in int48(Param) by the planner
    if let Some(func_expr) = nodecast!(FuncExpr, T_FuncExpr, node) {
        let args = PgList::<pg_sys::Node>::from_pg((*func_expr).args);
        if args.len() == 1 {
            return find_extern_param_id(args.get_ptr(0).unwrap());
        }
    }

    if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, node) {
        return find_extern_param_id((*relabel).arg.cast());
    }

    if let Some(coerce) = nodecast!(CoerceViaIO, T_CoerceViaIO, node) {
        return find_extern_param_id((*coerce).arg.cast());
    }

    None
}

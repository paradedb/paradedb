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
use crate::postgres::customscan::parameterized_value::ParameterizedValue;
use crate::DEFAULT_PARAMETERIZED_LIMIT_ESTIMATE;
use pgrx::{pg_sys, FromDatum};
use serde::{Deserialize, Serialize};

/// LIMIT and OFFSET extracted from a query, with values that may be either
/// `Const` (resolved at planning time) or extern `Param` (resolved at
/// execution time from `EState::es_param_list_info`).
///
/// The struct only exists when a LIMIT clause is present. Callers that need to
/// represent "no LIMIT" hold an `Option<LimitOffset>` instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitOffset {
    pub limit: ParameterizedValue<i64>,
    pub offset: Option<ParameterizedValue<i64>>,
}

impl LimitOffset {
    /// Extract LIMIT and OFFSET from the parse tree.
    ///
    /// Returns `None` if there is no LIMIT clause. Handles both `Const` and
    /// extern `Param` nodes (the latter for GENERIC prepared plans).
    pub unsafe fn from_parse(parse: *mut pg_sys::Query) -> Option<Self> {
        if parse.is_null() {
            return None;
        }
        let limit = ParameterizedValue::<i64>::from_node((*parse).limitCount)?;
        let offset = ParameterizedValue::<i64>::from_node((*parse).limitOffset);
        Some(Self { limit, offset })
    }

    /// Extract LIMIT and OFFSET from the planner root, prefering PG's combined
    /// `limit_tuples` when available (subtracting any static OFFSET to recover
    /// the original LIMIT) and otherwise falling back to the parse tree.
    ///
    /// This mirrors PG's planner intent loosely; callers that need to respect
    /// `limit_tuples == -1` as a "do not push" signal (e.g., BaseScan beneath
    /// a GROUP BY) should consult `(*root).limit_tuples` themselves and gate
    /// usage of the returned `LimitOffset` accordingly.
    pub unsafe fn from_root(root: *mut pg_sys::PlannerInfo) -> Option<Self> {
        if root.is_null() {
            return None;
        }
        let parse = (*root).parse;
        if parse.is_null() {
            return None;
        }

        if (*root).limit_tuples > -1.0 {
            let limit_pv = ParameterizedValue::<i64>::from_node((*parse).limitCount);
            let offset_pv = ParameterizedValue::<i64>::from_node((*parse).limitOffset);

            if let (Some(ParameterizedValue::Static(_)), maybe_offset) = (&limit_pv, &offset_pv) {
                let combined = (*root).limit_tuples as i64;
                let offset_val = match offset_pv {
                    Some(ParameterizedValue::Static(o)) => o,
                    _ => 0,
                };
                let limit_val = (combined - offset_val).max(0);
                return Some(Self {
                    limit: ParameterizedValue::Static(limit_val),
                    offset: maybe_offset.clone(),
                });
            }
        }

        Self::from_parse(parse)
    }

    /// Returns the static LIMIT value if known at planning time; `None` if
    /// the LIMIT is parameterized.
    pub fn static_limit(&self) -> Option<i64> {
        self.limit.static_value().copied()
    }

    /// Returns the static OFFSET value if known at planning time; `None` if
    /// there is no OFFSET or it is parameterized.
    pub fn static_offset(&self) -> Option<i64> {
        self.offset.as_ref().and_then(|o| o.static_value()).copied()
    }

    /// Returns true if both LIMIT and OFFSET (or just LIMIT, if no OFFSET) are
    /// known at planning time.
    pub fn has_static_limit(&self) -> bool {
        self.static_limit().is_some()
    }

    /// Returns true if either LIMIT or OFFSET is a Param (i.e., this came from
    /// a GENERIC prepared plan where PG couldn't fold the value to a Const).
    pub fn has_any_param(&self) -> bool {
        matches!(self.limit, ParameterizedValue::Param { .. })
            || self
                .offset
                .as_ref()
                .is_some_and(|o| matches!(o, ParameterizedValue::Param { .. }))
    }

    /// Resolves the LIMIT at execution time. Returns `None` if the value
    /// cannot be resolved (null param, missing param list).
    pub unsafe fn resolve_limit(&self, estate: *mut pg_sys::EState) -> Option<usize> {
        self.limit.resolve(estate).map(|v| v.max(0) as usize)
    }

    /// Resolves the OFFSET at execution time. Returns 0 when there is no
    /// OFFSET clause or the resolved value is null.
    pub unsafe fn resolve_offset(&self, estate: *mut pg_sys::EState) -> usize {
        self.offset
            .as_ref()
            .and_then(|o| o.resolve(estate))
            .map(|v| v.max(0) as usize)
            .unwrap_or(0)
    }

    /// Returns `LIMIT + OFFSET` resolved at execution time. This is the number
    /// of rows our scan must produce so PostgreSQL's outer Limit node has
    /// enough rows to apply OFFSET and return LIMIT.
    pub unsafe fn fetch(&self, estate: *mut pg_sys::EState) -> Option<usize> {
        self.resolve_limit(estate)
            .map(|l| l + self.resolve_offset(estate))
    }

    /// Returns `LIMIT + OFFSET` only when both are statically known. Used at
    /// planning time by callers that need a value usable in serializable
    /// guards (e.g., aggregate bucket-limit checks). Returns `None` when
    /// either side is parameterized — those values must be resolved at
    /// execution time via `fetch(estate)`.
    pub fn static_fetch(&self) -> Option<usize> {
        let limit = self.static_limit()?;
        // If an OFFSET clause exists, it MUST be statically known too;
        // otherwise we cannot compute the fetch count without an EState.
        let offset = match &self.offset {
            None => 0,
            Some(_) => self.static_offset()?,
        };
        Some((limit + offset).max(0) as usize)
    }

    /// Planning-time row estimate. Uses static values when available, falls
    /// back to `DEFAULT_PARAMETERIZED_LIMIT_ESTIMATE` for parameterized values.
    pub fn planning_estimate(&self) -> f64 {
        let limit = self
            .static_limit()
            .map(|v| v as f64)
            .unwrap_or(DEFAULT_PARAMETERIZED_LIMIT_ESTIMATE);
        let offset = self.static_offset().map(|v| v as f64).unwrap_or(0.0);
        limit + offset
    }
}

/// Extract a `Const` node's value as `i64`. Returns `None` if the node is null,
/// not a `Const`, or the datum is null.
///
/// Retained for callers that intentionally reject non-Const inputs (e.g.,
/// `is_minmax_implicit_limit`, which only matches PG's MIN/MAX rewrite where
/// the LIMIT 1 is always a Const).
#[allow(dead_code)]
pub unsafe fn extract_const_i64(node: *mut pg_sys::Node) -> Option<i64> {
    let const_node = nodecast!(Const, T_Const, node)?;
    i64::from_datum((*const_node).constvalue, (*const_node).constisnull)
}

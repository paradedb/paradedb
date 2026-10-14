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

use crate::DEFAULT_PARAMETERIZED_LIMIT_ESTIMATE;
use crate::postgres::customscan::parameterized_value::ParameterizedValue;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};

/// LIMIT and OFFSET extracted from a query, with values that may be either
/// `Const` (resolved at planning time) or extern `Param` (resolved at
/// execution time from `EState::es_param_list_info`).
///
/// The struct only exists when a LIMIT clause is present — callers hold
/// `Option<LimitOffset>` to represent "no LIMIT".
///
/// # Resolution patterns
///
/// - **Exec method init (TopK, Columnar, JoinScan):** call `resolve_mut(estate)`
///   once, then use `static_fetch()` / `limit.static_value()` freely.
/// - **Planning time cost math:** call `planning_estimate()` — returns a
///   real value for Static, a default (1000.0) for Param.
/// - **Planning time guards:** call `has_any_param()` and `static_fetch()`.
/// - **Immutable context (snippets):** call `resolve(estate)` — clones on
///   each call, acceptable when dominated by heavier per-row work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitClauseKind {
    Limit,
    Offset,
}

impl LimitClauseKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Limit => "LIMIT",
            Self::Offset => "OFFSET",
        }
    }
}

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
    pub fn from_parse(parse: *mut pg_sys::Query) -> Option<Self> {
        if parse.is_null() {
            return None;
        }
        let parse = unsafe { &*parse };
        if parse.limitOption == pg_sys::LimitOption::LIMIT_OPTION_WITH_TIES {
            return None;
        }

        let limit = ParameterizedValue::<i64>::from_node(parse.limitCount)?;
        let offset = ParameterizedValue::<i64>::from_node(parse.limitOffset);
        Some(Self { limit, offset })
    }

    /// Parse a static `LIMIT` or `OFFSET` bound for a scan planned at
    /// `UPPERREL_FINAL` (where no PostgreSQL `Limit` node sits above the scan).
    ///
    /// Unlike [`Self::from_parse`] / [`Self::static_limit`] (which sit below
    /// PostgreSQL's `Limit` node and can return `None` on non-constants or clamp
    /// negatives to `0` while letting `ExecLimit` raise the error), a scan at
    /// `UPPERREL_FINAL` must:
    /// - treat `NULL` (`LIMIT ALL`, `LIMIT NULL`, `OFFSET NULL`) as `Ok(None)`,
    /// - reject non-constant bounds (`Param` or volatile expressions) with `Err`,
    /// - reject negative values with `"{clause} must not be negative"`.
    pub unsafe fn parse_static_bound(
        node: *mut pg_sys::Node,
        clause: LimitClauseKind,
    ) -> Result<Option<usize>, String> {
        if node.is_null() {
            return Ok(None);
        }
        if let Some(const_node) = crate::nodecast!(Const, T_Const, node)
            && (*const_node).constisnull
        {
            return Ok(None);
        }
        let clause_name = clause.as_str();
        let val = ParameterizedValue::<i64>::from_node(node)
            .and_then(|pv| pv.static_value().copied())
            .ok_or_else(|| {
                format!("non-constant {clause_name} is not supported with pdb.combine_agg()")
            })?;
        let val =
            usize::try_from(val).map_err(|_| format!("{clause_name} must not be negative"))?;
        Ok(Some(val))
    }

    /// Extract LIMIT and OFFSET from the planner root, preferring PG's combined
    /// `limit_tuples` when available (subtracting any static OFFSET to recover
    /// the original LIMIT) and otherwise falling back to the parse tree.
    ///
    /// This mirrors PG's planner intent loosely; callers that need to respect
    /// `limit_tuples == -1` as a "do not push" signal (e.g., BaseScan beneath
    /// a GROUP BY) should consult `(*root).limit_tuples` themselves and gate
    /// usage of the returned `LimitOffset` accordingly.
    pub fn from_root(root: *mut pg_sys::PlannerInfo) -> Option<Self> {
        if root.is_null() {
            return None;
        }
        let root = unsafe { &*root };
        let parse = root.parse;
        if parse.is_null() {
            return None;
        }

        if root.limit_tuples > -1.0 {
            let limit_pv = ParameterizedValue::<i64>::from_node(unsafe { (*parse).limitCount });
            let offset_pv = ParameterizedValue::<i64>::from_node(unsafe { (*parse).limitOffset });

            if let (Some(ParameterizedValue::Static(_)), maybe_offset) = (&limit_pv, &offset_pv) {
                let combined = root.limit_tuples as i64;
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

    /// Returns true if either LIMIT or OFFSET is a Param (i.e., this came from
    /// a GENERIC prepared plan where PG couldn't fold the value to a Const).
    pub fn has_any_param(&self) -> bool {
        self.limit.is_param() || self.offset.as_ref().is_some_and(|o| o.is_param())
    }

    /// Returns `LIMIT + OFFSET` only when both are statically known. Used at
    /// planning time and as the chained call after `resolve_mut` to read the
    /// now-static sum.
    pub fn static_fetch(&self) -> Option<usize> {
        let limit = self.static_limit()?;
        let offset = self.static_offset()?;
        Some(limit.saturating_add(offset))
    }

    pub fn fetch_display(&self) -> String {
        match &self.offset {
            None | Some(ParameterizedValue::Static(0)) => self.limit.to_string(),
            Some(off) => format!("{} + {off}", self.limit),
        }
    }

    /// Returns `LIMIT` only when statically known.
    pub fn static_limit(&self) -> Option<usize> {
        let limit = *self.limit.static_value()?;
        Some(limit.max(0) as usize)
    }

    /// Returns `OFFSET` only when statically known (including `Some(0)` when no offset was specified).
    pub fn static_offset(&self) -> Option<usize> {
        match &self.offset {
            None => Some(0),
            Some(o) => o.static_value().map(|v| (*v).max(0) as usize),
        }
    }

    /// Planning-time row estimate. Uses static values when available, falls
    /// back to `DEFAULT_PARAMETERIZED_LIMIT_ESTIMATE` for parameterized values.
    pub fn planning_estimate(&self) -> f64 {
        let limit = self
            .limit
            .static_value()
            .map(|v| *v as f64)
            .unwrap_or(DEFAULT_PARAMETERIZED_LIMIT_ESTIMATE);
        let offset = self
            .offset
            .as_ref()
            .and_then(|o| o.static_value())
            .map(|v| *v as f64)
            .unwrap_or(0.0);
        limit + offset
    }

    /// Returns `LIMIT + OFFSET` resolved at execution time. This is the number
    /// of rows our scan must produce so PostgreSQL's outer Limit node has
    /// enough rows to apply OFFSET and return LIMIT.
    pub fn resolve(&self, estate: *mut pg_sys::EState) -> Option<usize> {
        let limit = self.limit.resolve(estate)?.max(0) as usize;
        let offset = self
            .offset
            .as_ref()
            .and_then(|o| o.resolve(estate))
            .map(|v| v.max(0) as usize)
            .unwrap_or(0);
        Some(limit + offset)
    }

    /// Resolve both LIMIT and OFFSET `Param`s to `Static` in place. Returns
    /// `&mut Self` on success so callers can chain `.static_fetch()` to read
    /// the now-static sum.
    pub fn resolve_mut(&mut self, estate: *mut pg_sys::EState) -> Option<&mut Self> {
        self.limit.resolve_mut(estate)?;
        if let Some(ref mut o) = self.offset {
            o.resolve_mut(estate);
        }
        Some(self)
    }

    #[cfg(test)]
    pub fn from_k(k: i64) -> Self {
        Self {
            limit: ParameterizedValue::Static(k),
            offset: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limit_offset(
        limit: ParameterizedValue<i64>,
        offset: Option<ParameterizedValue<i64>>,
    ) -> LimitOffset {
        LimitOffset { limit, offset }
    }

    #[test]
    fn fetch_display_reads_as_written() {
        use ParameterizedValue::{Param, Static};

        assert_eq!(
            limit_offset(Param { param_id: 1 }, None).fetch_display(),
            "$1"
        );
        assert_eq!(
            limit_offset(Param { param_id: 1 }, Some(Static(10))).fetch_display(),
            "$1 + 10"
        );
        assert_eq!(
            limit_offset(Static(5), Some(Param { param_id: 2 })).fetch_display(),
            "5 + $2"
        );
        assert_eq!(
            limit_offset(Param { param_id: 1 }, Some(Param { param_id: 2 })).fetch_display(),
            "$1 + $2"
        );
        // A static zero OFFSET is left out, as the EXPLAIN header leaves it out.
        assert_eq!(
            limit_offset(Param { param_id: 1 }, Some(Static(0))).fetch_display(),
            "$1"
        );
    }
}

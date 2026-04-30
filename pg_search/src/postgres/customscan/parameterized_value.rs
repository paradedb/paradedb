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

//! A value that is either known at planning time (a SQL `Const`) or deferred
//! to execution time (a SQL `Param` from `EState::es_param_list_info`).
//!
//! # When to use which method
//!
//! | Situation | Method | Why |
//! |-----------|--------|-----|
//! | You own the value (`&mut self`) and will read it again later | `resolve_mut` | Converts Param → Static in place. All subsequent reads are zero-cost borrows. |
//! | You don't own the value (HashMap key, shared reference) | `resolve` | Returns an owned clone. Costs a heap alloc for String/complex types per call. |
//! | Planning time only, need the value if it's known | `static_value` | No EState needed. Returns `None` for Params — use `planning_estimate()` on `LimitOffset` for cost math. |
//!
//! # Rules of thumb
//!
//! 1. **Prefer `resolve_mut` in exec method `init()`.** TopK, Columnar,
//!    and JoinScan all have `&mut` access to their `LimitOffset`. Call
//!    `resolve_mut` once during init — every later access (EXPLAIN,
//!    iteration, batch sizing) gets the cached Static value for free.
//!
//! 2. **Use `resolve` only when `&mut` is unavailable.** The main case
//!    is `SnippetType` used as a HashMap key — you can't mutate a key.
//!    For everything else, prefer `resolve_mut`.
//!
//! 3. **Never call `resolve` in a per-tuple loop if you can help it.**
//!    For types like `String`, each call clones. If you're in a loop,
//!    either resolve once before the loop, or restructure so `resolve_mut`
//!    is viable.
//!
//! 4. **At planning time, don't try to resolve Params.** There's no
//!    EState yet. Use `static_value()` / `static_fetch()` and handle
//!    the `None` case (parameterized) with a fallback like
//!    `planning_estimate()`.
//!
//! 5. **Watch out for `nodecast!(Const, T_Const, ...)` in new code.**
//!    If you're extracting a value from a PG expression node at planning
//!    time, ask: "will this work in GENERIC mode?" If the node could be a
//!    `Param`, use `ParameterizedValue::from_node` instead of `nodecast!`.
//!    If you genuinely need a compile-time constant (like
//!    `is_minmax_implicit_limit` checking for PG's `LIMIT 1` rewrite),
//!    `nodecast!` is correct — document why.

use crate::nodecast;
use pgrx::{pg_sys, FromDatum, PgList};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterizedValue<T>
where
    T: Clone,
{
    Static(T),
    Param { param_id: i32 },
}

impl<T> fmt::Display for ParameterizedValue<T>
where
    T: Clone + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterizedValue::Static(v) => write!(f, "{v}"),
            ParameterizedValue::Param { param_id } => write!(f, "${param_id}"),
        }
    }
}

impl<T> ParameterizedValue<T>
where
    T: Clone + FromDatum,
{
    /// Extract a `ParameterizedValue<T>` from a Postgres expression node.
    ///
    /// Returns `None` if the node is null or cannot be interpreted as either
    /// a `Const` of type `T` or an extern `Param`.
    ///
    /// Recurses through `FuncExpr` (single arg), `RelabelType`, and
    /// `CoerceViaIO` wrappers — these are commonly inserted by the parser
    /// around LIMIT/OFFSET expressions for type coercion.
    pub unsafe fn from_node(node: *mut pg_sys::Node) -> Option<Self> {
        if node.is_null() {
            return None;
        }

        if let Some(const_node) = nodecast!(Const, T_Const, node) {
            let value = T::from_datum((*const_node).constvalue, (*const_node).constisnull)?;
            return Some(ParameterizedValue::Static(value));
        }

        unwrap_to_extern_param_id(node).map(|param_id| ParameterizedValue::Param { param_id })
    }

    /// Resolve at execution time using the executor's parameter list.
    ///
    /// `Static(v)` returns `Some(v.clone())`.
    /// `Param { param_id }` looks up `estate.es_param_list_info.params[param_id - 1]`.
    /// Returns `None` if the parameter is null or out of range.
    pub unsafe fn resolve(&self, estate: *mut pg_sys::EState) -> Option<T> {
        match self {
            ParameterizedValue::Static(v) => Some(v.clone()),
            ParameterizedValue::Param { param_id } => {
                if estate.is_null() {
                    return None;
                }
                let param_list = (*estate).es_param_list_info;
                if param_list.is_null() {
                    return None;
                }
                let num_params = (*param_list).numParams as usize;
                let idx = (*param_id - 1) as usize;
                if idx >= num_params {
                    return None;
                }
                let param_data = &(*param_list).params.as_slice(num_params)[idx];
                T::from_datum(param_data.value, param_data.isnull)
            }
        }
    }

    /// Returns the static value if this is `Static`, otherwise `None`.
    pub fn static_value(&self) -> Option<&T> {
        match self {
            ParameterizedValue::Static(v) => Some(v),
            _ => None,
        }
    }

    /// Returns true if this value is a `Param` (deferred to execution time).
    pub fn is_param(&self) -> bool {
        matches!(self, ParameterizedValue::Param { .. })
    }

    /// Resolve and convert `Param` → `Static` in place. Returns `&T`.
    ///
    /// On first call for a `Param`, resolves from `EState` and replaces
    /// `self` with `Static(value)`. Subsequent calls hit the static path
    /// at zero cost. Use this when `&mut self` is available (TopK,
    /// Columnar, JoinScan). Snippet configs use `resolve()` instead
    /// because `SnippetType` is a `HashMap` key and only `&self` is
    /// available there.
    pub unsafe fn resolve_mut(&mut self, estate: *mut pg_sys::EState) -> Option<&T> {
        if self.is_param() {
            let value = self.resolve(estate)?;
            *self = ParameterizedValue::Static(value);
        }
        self.static_value()
    }
}

// Manual Hash/Eq/PartialEq impls (rather than `derive`) so each trait bound is
// only required when the concrete `T` actually needs it. Several callers store
// `ParameterizedValue<T>` inside a `HashMap` key (e.g., snippet configs in
// `SnippetType`), so this is required for the type to be usable in those
// contexts. Two `Param { param_id: N }` values compare equal regardless of T,
// because the param ID is the only identity at planning time.
impl<T: Clone + PartialEq> PartialEq for ParameterizedValue<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Static(a), Self::Static(b)) => a == b,
            (Self::Param { param_id: a }, Self::Param { param_id: b }) => a == b,
            _ => false,
        }
    }
}

impl<T: Clone + Eq> Eq for ParameterizedValue<T> {}

impl<T: Clone + Hash> Hash for ParameterizedValue<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Static(v) => {
                0u8.hash(state);
                v.hash(state);
            }
            Self::Param { param_id } => {
                1u8.hash(state);
                param_id.hash(state);
            }
        }
    }
}

/// Walk through commonly-inserted coercion wrappers to find an extern Param's
/// `paramid`. Returns `None` if the node is not (or does not wrap) a Param of
/// kind `PARAM_EXTERN`.
unsafe fn unwrap_to_extern_param_id(node: *mut pg_sys::Node) -> Option<i32> {
    if node.is_null() {
        return None;
    }

    if let Some(param) = nodecast!(Param, T_Param, node) {
        if (*param).paramkind == pg_sys::ParamKind::PARAM_EXTERN {
            return Some((*param).paramid);
        }
        return None;
    }

    if let Some(func_expr) = nodecast!(FuncExpr, T_FuncExpr, node) {
        let args = PgList::<pg_sys::Node>::from_pg((*func_expr).args);
        if args.len() == 1 {
            return unwrap_to_extern_param_id(args.get_ptr(0).unwrap());
        }
    }

    if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, node) {
        return unwrap_to_extern_param_id((*relabel).arg.cast());
    }

    if let Some(coerce) = nodecast!(CoerceViaIO, T_CoerceViaIO, node) {
        return unwrap_to_extern_param_id((*coerce).arg.cast());
    }

    None
}

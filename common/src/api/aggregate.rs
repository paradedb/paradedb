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
#![allow(unsafe_op_in_unsafe_fn)]

use pgrx::{FromDatum, JsonB};
use serde::{Deserialize, Serialize};

use crate::nodecast;
use crate::postgres::utils::lookup_pdb_function;

/// Get the OID of the pdb.agg_fn() placeholder function
/// Returns InvalidOid if the function doesn't exist yet (e.g., during extension creation)
pub fn agg_fn_oid() -> pgrx::pg_sys::Oid {
    lookup_pdb_function("agg_fn", &[pgrx::pg_sys::TEXTOID])
}

/// Get the OID of the pdb.combine_agg(jsonb) aggregate function.
/// Returns InvalidOid if the function doesn't exist yet (e.g., during extension creation).
pub fn combine_agg_funcoid() -> pgrx::pg_sys::Oid {
    lookup_pdb_function("combine_agg", &[pgrx::pg_sys::JSONBOID])
}

/// True when `funcoid` is `pdb.combine_agg(jsonb)`.
pub fn is_combine_agg_funcoid(funcoid: u32) -> bool {
    let oid = combine_agg_funcoid().to_u32();
    oid != pgrx::pg_sys::InvalidOid.to_u32() && funcoid == oid
}

/// Get the OID of the pdb.agg() aggregate function
/// Returns InvalidOid if the function doesn't exist yet (e.g., during extension creation)
pub fn agg_funcoid() -> pgrx::pg_sys::Oid {
    lookup_pdb_function("agg", &[pgrx::pg_sys::JSONBOID])
}

/// Get the OID of the deprecated pdb.agg(jsonb, bool) aggregate function
/// Returns InvalidOid if the function doesn't exist yet (e.g., during extension creation)
fn agg_with_solve_mvcc_funcoid() -> pgrx::pg_sys::Oid {
    lookup_pdb_function("agg", &[pgrx::pg_sys::JSONBOID, pgrx::pg_sys::BOOLOID])
}

/// Get the OID of the pdb.agg(jsonb, text) aggregate function with the `visibility` parameter
/// Returns InvalidOid if the function doesn't exist yet (e.g., during extension creation)
fn agg_with_visibility_funcoid() -> pgrx::pg_sys::Oid {
    lookup_pdb_function("agg", &[pgrx::pg_sys::JSONBOID, pgrx::pg_sys::TEXTOID])
}

/// The OIDs of every `pdb.agg()` overload: the one-argument form, the deprecated
/// `(jsonb, bool)` form, and the `(jsonb, text)` form.
///
/// Each entry is a catalog lookup, so hoist this out of any loop that walks an
/// expression tree rather than calling `is_agg_funcoid` per node.
pub fn agg_funcoids() -> [u32; 3] {
    [
        agg_funcoid().to_u32(),
        agg_with_solve_mvcc_funcoid().to_u32(),
        agg_with_visibility_funcoid().to_u32(),
    ]
}

/// True for any `pdb.agg()` overload. Convenient for a one-off check; use
/// `agg_funcoids()` when the check is inside a tree walk.
pub fn is_agg_funcoid(aggfnoid: u32) -> bool {
    agg_funcoids().contains(&aggfnoid)
}

/// Decode the visibility argument of a `pdb.agg()` call.
///
/// `second_arg` is the second argument's expression, or `None` for the one-argument
/// overload. A non-`Const` or NULL argument, and any overload without a visibility
/// argument, yield the default: an undecodable mode must not silently downgrade
/// accuracy.
///
/// # Safety
/// The caller must ensure `second_arg`, when present, is a valid `Node` pointer.
pub unsafe fn visibility_from_agg_arg(
    aggfnoid: u32,
    second_arg: Option<*mut pgrx::pg_sys::Node>,
) -> MvccVisibility {
    let Some(const_node) = second_arg.and_then(|node| nodecast!(Const, node)) else {
        return MvccVisibility::default();
    };

    if aggfnoid == agg_with_solve_mvcc_funcoid().to_u32() {
        if extract_solve_mvcc_from_const(const_node) {
            MvccVisibility::Transaction
        } else {
            MvccVisibility::Raw
        }
    } else if aggfnoid == agg_with_visibility_funcoid().to_u32() {
        extract_visibility_from_const(const_node)
    } else {
        MvccVisibility::default()
    }
}

/// The spec and visibility of a `pdb.agg()` call, or `None` when the spec is not
/// a non-null constant. Field validation and the plan shape both need the spec at
/// plan time, so a parameter cannot stand in for it.
///
/// # Safety
/// The caller must ensure `spec_arg` and `visibility_arg`, when present, are valid
/// `Node` pointers.
pub unsafe fn pdb_agg_spec(
    aggfnoid: u32,
    spec_arg: *mut pgrx::pg_sys::Node,
    visibility_arg: Option<*mut pgrx::pg_sys::Node>,
) -> Option<(serde_json::Value, MvccVisibility)> {
    let const_node = nodecast!(Const, spec_arg)?;
    let spec = JsonB::from_datum((*const_node).constvalue, (*const_node).constisnull)?.0;
    Some((spec, visibility_from_agg_arg(aggfnoid, visibility_arg)))
}

/// Extract solve_mvcc boolean from a Const node.
/// Returns true (MVCC enabled) if the value can't be extracted or is null.
fn extract_solve_mvcc_from_const(const_node: *mut pgrx::pg_sys::Const) -> bool {
    if const_node.is_null() || unsafe { (*const_node).constisnull } {
        return true;
    }
    let bool_datum = unsafe { (*const_node).constvalue };
    unsafe { pgrx::FromDatum::from_datum(bool_datum, false) }.unwrap_or(true)
}

/// Extract a `visibility` mode from a `text` Const node. A NULL or undecodable
/// value yields the default; an unrecognized string is an error.
fn extract_visibility_from_const(const_node: *mut pgrx::pg_sys::Const) -> MvccVisibility {
    if const_node.is_null() || unsafe { (*const_node).constisnull } {
        return MvccVisibility::default();
    }
    let datum = unsafe { (*const_node).constvalue };
    match unsafe { <String as pgrx::FromDatum>::from_datum(datum, false) } {
        Some(value) => MvccVisibility::from_sql_value(&value),
        None => MvccVisibility::default(),
    }
}

/// Controls transaction visibility filtering for aggregate computations.
///
/// Aggregates read the index rather than the heap, so an index entry for a row that
/// is dead or invisible to the current snapshot is only excluded if it is checked
/// against the heap. This enum decides whether that check runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MvccVisibility {
    /// Apply visibility checking, so aggregates count only rows visible to the
    /// current transaction. The default, and the only mode that matches vanilla
    /// Postgres semantics.
    #[default]
    Transaction,
    /// Skip visibility checking and aggregate raw index data. Faster, and
    /// approximate whenever the index holds entries for rows the transaction
    /// cannot see.
    Raw,
    /// Apply visibility checking only when the query's estimated matching row
    /// count is below `paradedb.visibility_threshold`. Resolved per execution,
    /// not per plan.
    Threshold,
}

impl MvccVisibility {
    /// Parse a SQL-level `visibility` value (case-insensitive), or `None` when the
    /// value is not one of the recognized spellings.
    ///
    /// The legacy `solve_mvcc` spellings are accepted as aliases so that queries
    /// written against the old parameter keep working.
    ///
    /// Kept free of `pgrx::error!` so it stays a pure function. Reporting the error
    /// is `from_sql_value`'s job: a unit test that reached the reporting path would
    /// pull Postgres's `ereport` symbols into the test binary, which does not link
    /// on Linux.
    pub fn try_from_sql_value(value: &str) -> Option<Self> {
        Some(match value.trim().to_lowercase().as_str() {
            // The boolean spellings are the ones Postgres itself accepts for a
            // `boolean` input, since this value used to be one.
            "transaction" | "true" | "t" | "yes" | "y" | "on" | "1" | "enabled" | "always" => {
                MvccVisibility::Transaction
            }
            "raw" | "false" | "f" | "no" | "n" | "off" | "0" | "disabled" | "never" => {
                MvccVisibility::Raw
            }
            "threshold" | "estimated" => MvccVisibility::Threshold,
            _ => return None,
        })
    }

    /// Parse a SQL-level `visibility` value, erroring on an unrecognized one rather
    /// than guessing. Guessing wrong either costs accuracy silently or costs the
    /// performance the caller asked for.
    pub fn from_sql_value(value: &str) -> Self {
        Self::try_from_sql_value(value).unwrap_or_else(|| {
            pgrx::error!(
                "unrecognized visibility mode '{value}'. \
                 Valid values: 'transaction' (default), 'raw', 'threshold'."
            )
        })
    }

    /// The single visibility a query runs under, given every `pdb.agg()` setting in
    /// it. Two different settings are an error even when one of them came from an
    /// omitted argument, since an omitted argument is indistinguishable from an
    /// explicit `'transaction'` by the time it is seen here.
    pub fn resolve_shared(settings: impl Iterator<Item = MvccVisibility>) -> MvccVisibility {
        let mut resolved: Option<MvccVisibility> = None;
        for visibility in settings {
            match resolved {
                None => resolved = Some(visibility),
                Some(previous) if previous == visibility => {}
                Some(previous) => pgrx::error!(
                    "pdb.agg() calls have contradicting visibility settings: \
                     '{}' and '{}'. All pdb.agg() calls in a query must use the same \
                     visibility value, and omitting the argument selects '{}'.",
                    previous.as_sql_value(),
                    visibility.as_sql_value(),
                    MvccVisibility::default().as_sql_value()
                ),
            }
        }
        resolved.unwrap_or_default()
    }

    /// The canonical SQL spelling, for error and deprecation messages.
    pub fn as_sql_value(&self) -> &'static str {
        match self {
            MvccVisibility::Transaction => "transaction",
            MvccVisibility::Raw => "raw",
            MvccVisibility::Threshold => "threshold",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MvccVisibility;

    #[test]
    fn parses_canonical_visibility_values() {
        assert_eq!(
            MvccVisibility::try_from_sql_value("transaction"),
            Some(MvccVisibility::Transaction)
        );
        assert_eq!(
            MvccVisibility::try_from_sql_value("raw"),
            Some(MvccVisibility::Raw)
        );
        assert_eq!(
            MvccVisibility::try_from_sql_value("threshold"),
            Some(MvccVisibility::Threshold)
        );
    }

    #[test]
    fn parses_legacy_solve_mvcc_spellings() {
        for value in ["true", "t", "yes", "y", "on", "1", "enabled", "always"] {
            assert_eq!(
                MvccVisibility::try_from_sql_value(value),
                Some(MvccVisibility::Transaction),
                "{value} should map to transaction"
            );
        }
        for value in ["false", "f", "no", "n", "off", "0", "disabled", "never"] {
            assert_eq!(
                MvccVisibility::try_from_sql_value(value),
                Some(MvccVisibility::Raw),
                "{value} should map to raw"
            );
        }
        assert_eq!(
            MvccVisibility::try_from_sql_value("estimated"),
            Some(MvccVisibility::Threshold)
        );
    }

    #[test]
    fn parsing_ignores_case_and_surrounding_whitespace() {
        assert_eq!(
            MvccVisibility::try_from_sql_value("  ThReSHoLd \t"),
            Some(MvccVisibility::Threshold)
        );
        assert_eq!(
            MvccVisibility::try_from_sql_value("RAW"),
            Some(MvccVisibility::Raw)
        );
    }

    #[test]
    fn rejects_unrecognized_values() {
        for value in ["nonsense", "", "  ", "transactional", "rawish", "10000"] {
            assert_eq!(
                MvccVisibility::try_from_sql_value(value),
                None,
                "{value:?} should not parse"
            );
        }
    }

    #[test]
    fn transaction_is_the_default() {
        assert_eq!(MvccVisibility::default(), MvccVisibility::Transaction);
    }

    #[test]
    fn sql_values_round_trip() {
        for visibility in [
            MvccVisibility::Transaction,
            MvccVisibility::Raw,
            MvccVisibility::Threshold,
        ] {
            assert_eq!(
                MvccVisibility::try_from_sql_value(visibility.as_sql_value()),
                Some(visibility)
            );
        }
    }
}

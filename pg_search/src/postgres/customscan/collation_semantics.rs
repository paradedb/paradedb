// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

//! Semantic compatibility checks for PostgreSQL collations.
//!
//! PostgreSQL delegates text equality and ordering to the expression's
//! collation. Tantivy and DataFusion compare text by its underlying value, so
//! a custom path must prove that its operation preserves the relevant
//! PostgreSQL semantics before it lowers an expression.

use crate::postgres::catalog::{
    lookup_collation_locale, lookup_database_collation_locale, CollationLocale, CollationProvider,
};
use pgrx::pg_sys;

/// The text semantic a custom execution engine must preserve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollationOperation {
    /// Equality semantics used by `=`, `<>`, `GROUP BY`, and hash/merge join
    /// keys. A deterministic collation preserves byte-based equality.
    Equality,
    /// Ordering semantics used by range comparisons and `ORDER BY`. Only
    /// byte-ordered C-like collations can be delegated to the current engines.
    Ordering,
}

/// The result of assessing a collation for one [`CollationOperation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollationSafety {
    Safe,
    NondeterministicEquality,
    NonByteOrdered,
}

impl CollationSafety {
    pub const fn is_safe(self) -> bool {
        matches!(self, Self::Safe)
    }
}

/// Assess whether a collation preserves the requested text semantics when a
/// custom execution engine compares raw values.
///
/// `InvalidOid` denotes a non-collatable type, whose equality and ordering do
/// not depend on a PostgreSQL collation.
///
/// # Safety
///
/// Must run inside an active PostgreSQL backend because it reads collation
/// catalog state.
pub unsafe fn assess_collation(
    collation: pg_sys::Oid,
    operation: CollationOperation,
) -> CollationSafety {
    match operation {
        CollationOperation::Equality => {
            if collation == pg_sys::Oid::INVALID
                || unsafe { pg_sys::get_collation_isdeterministic(collation) }
            {
                CollationSafety::Safe
            } else {
                CollationSafety::NondeterministicEquality
            }
        }
        CollationOperation::Ordering => {
            if is_byte_ordered_collation(collation) {
                CollationSafety::Safe
            } else {
                CollationSafety::NonByteOrdered
            }
        }
    }
}

/// Return whether the given collation can be delegated for `operation`.
///
/// Prefer [`assess_collation`] when the caller needs a diagnostic or must make
/// different fallback decisions for different unsafe states.
///
/// # Safety
///
/// Must run inside an active PostgreSQL backend because it reads collation
/// catalog state.
pub unsafe fn collation_supports(collation: pg_sys::Oid, operation: CollationOperation) -> bool {
    unsafe { assess_collation(collation, operation) }.is_safe()
}

/// Normalize a collation name for case- and hyphen-insensitive comparison.
/// For example, `C.utf8`, `C.UTF-8`, and `C.UTF8` normalize to `C.UTF8`.
fn normalize_collation_name(mut collation_name: String) -> String {
    collation_name.retain(|c| c != '-');
    collation_name.make_ascii_uppercase();
    collation_name
}

/// Return whether `collation` has byte ordering compatible with Tantivy and
/// DataFusion's current text ordering.
fn is_byte_ordered_collation(collation: pg_sys::Oid) -> bool {
    const NORMALIZED_SAFE_COLLATION_NAMES: &[&str] = &["C", "POSIX", "C.UTF8", "POSIX.UTF8"];

    let locale = match collation {
        pg_sys::Oid::INVALID | pg_sys::C_COLLATION_OID => return true,
        pg_sys::DEFAULT_COLLATION_OID => lookup_database_collation_locale(),
        _ => lookup_collation_locale(collation),
    };

    match locale {
        #[cfg(any(feature = "pg17", feature = "pg18"))]
        Some(CollationLocale {
            provider: CollationProvider::Builtin,
            ..
        }) => true,
        Some(CollationLocale {
            provider: CollationProvider::Libc,
            name: Some(name),
        }) => NORMALIZED_SAFE_COLLATION_NAMES.contains(&normalize_collation_name(name).as_str()),
        // ICU and anything unrecognized are not byte-ordered.
        _ => false,
    }
}

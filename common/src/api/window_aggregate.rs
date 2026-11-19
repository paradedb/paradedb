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

use pgrx::pg_sys;

use crate::postgres::utils::lookup_pdb_function;

/// Get the OID of the window_agg placeholder function
/// Returns InvalidOid if the function doesn't exist yet (e.g., during extension creation)
pub fn window_agg_oid() -> pg_sys::Oid {
    lookup_pdb_function("window_agg", &[pg_sys::TEXTOID])
}

/// Structural identity of a `WindowFunc` within a custom scan's target lists.
///
/// Encoded as a negative `consttypmod` (`-1 - id`) on the placeholder's `TEXT`
/// argument so that PostgreSQL's `_equalConst` (`COMPARE_SCALAR_FIELD(consttypmod)`)
/// distinguishes non-equal `WindowFunc` nodes even if their cosmetic EXPLAIN
/// descriptions collide, while `format_type_extended` ignores negative typmods
/// and keeps `EXPLAIN VERBOSE` output unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceholderId(pub usize);

impl PlaceholderId {
    fn into_consttypmod(self) -> i32 {
        -1 - i32::try_from(self.0).expect("window function count fits in i32")
    }
}

/// Build a `paradedb.window_agg('<text>')` placeholder `FuncExpr` with the
/// given node-level result type and collation.
///
/// Shared by the two placeholder pipelines:
/// - the basescan planner-hook replacement, where `text_arg` is the
///   serialized aggregate spec the scan parses back at plan time, and
/// - joinscan/aggregatescan's plan-time EXPLAIN placeholder, where `text_arg`
///   is a cosmetic description that is never parsed and `placeholder_id`
///   carries the deduplicated `WindowFunc` identity for `setrefs.c`.
///
/// Returns `None` when the placeholder function doesn't exist yet (during
/// extension creation) or `text_arg` contains an interior NUL.
///
/// Must be called in a memory context suitable for plan-node allocation.
pub fn make_window_agg_placeholder(
    text_arg: &str,
    placeholder_id: Option<PlaceholderId>,
    result_type: pg_sys::Oid,
    result_collation: pg_sys::Oid,
) -> Option<*mut pg_sys::FuncExpr> {
    let procid = window_agg_oid();
    if procid == pg_sys::InvalidOid {
        return None;
    }

    let typmod = placeholder_id.map_or(-1, PlaceholderId::into_consttypmod);
    let text_cstring = std::ffi::CString::new(text_arg).ok()?;
    let text_const = unsafe {
        let text_datum =
            pg_sys::Datum::from(pg_sys::cstring_to_text(text_cstring.as_ptr()) as usize);
        pg_sys::makeConst(
            pg_sys::TEXTOID,
            typmod,
            pg_sys::DEFAULT_COLLATION_OID,
            -1,
            text_datum,
            false, // not null
            false, // text is varlena, not by-value
        )
    };
    let mut args = pgrx::PgList::<pg_sys::Node>::new();
    args.push(text_const.cast());

    Some(unsafe {
        pg_sys::makeFuncExpr(
            procid,
            result_type,
            args.into_pg(),
            result_collation,
            pg_sys::DEFAULT_COLLATION_OID,
            pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
        )
    })
}

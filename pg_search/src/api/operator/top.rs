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

//! The `::pdb.top(n)` arm annotation for rank fusion.
//!
//! Casting a boolean search-predicate subtree to `pdb.top(n)` turns it into
//! a fusion *arm*: "this row is among the top `n` rows ranked by this
//! subtree's own measure" (BM25 for text predicates, proximity for a `~~~`
//! knn predicate). The typmod carries `n`, following the same pattern as
//! `'term'::pdb.boost(2)`.
//!
//! ```sql
//! WHERE (description ||| 'shoes' OR category === 'footwear')::pdb.top(100)
//!    OR (embedding ~~~ '[...]')::pdb.top(200)
//! ORDER BY pdb.rrf(id)
//! LIMIT 10;
//! ```
//!
//! The type itself is inert: every executable path errors, because the
//! annotation only has meaning inside a ParadeDB rank-fusion scan, which
//! consumes it at plan time (see `qual_inspect`) and never evaluates it.

use pgrx::{extension_sql, pg_extern, pg_sys};
use std::ffi::{CStr, CString};
use std::str::FromStr;

const TOP_NOT_EXECUTABLE: &str = "::pdb.top(n) can only annotate a search predicate in a query executed by a ParadeDB rank-fusion scan: use `WHERE (<predicate>)::pdb.top(<n>) ... ORDER BY pdb.rrf(<key>) LIMIT <k>`";

/// The `pdb.top` SQL type. Its runtime value is never legitimately
/// constructed (the annotation is consumed at plan time), so the payload is
/// a placeholder `bool`.
#[derive(Debug)]
#[repr(transparent)]
pub struct TopType(#[allow(dead_code)] bool);

// pgrx boilerplate to let `TopType` cross the SQL/Rust boundary.
mod sql_datum_support {
    use super::TopType;
    use pgrx::callconv::{Arg, ArgAbi, BoxRet, FcInfo};
    use pgrx::nullable::Nullable;
    use pgrx::pgrx_sql_entity_graph::metadata::{
        ArgumentError, ReturnsError, ReturnsRef, SqlMappingRef, SqlTranslatable, TypeOrigin,
    };
    use pgrx::{pg_sys, FromDatum, IntoDatum};

    impl IntoDatum for TopType {
        fn into_datum(self) -> Option<pg_sys::Datum> {
            self.0.into_datum()
        }

        fn type_oid() -> pg_sys::Oid {
            pg_sys::BOOLOID
        }
    }

    impl FromDatum for TopType {
        unsafe fn from_polymorphic_datum(
            datum: pg_sys::Datum,
            is_null: bool,
            typoid: pg_sys::Oid,
        ) -> Option<Self> {
            bool::from_polymorphic_datum(datum, is_null, typoid).map(TopType)
        }
    }

    unsafe impl SqlTranslatable for TopType {
        const TYPE_IDENT: &'static str = pgrx::pgrx_resolved_type!(TopType);
        const TYPE_ORIGIN: TypeOrigin = TypeOrigin::External;
        const ARGUMENT_SQL: Result<SqlMappingRef, ArgumentError> =
            Ok(SqlMappingRef::literal("pdb.top"));
        const RETURN_SQL: Result<ReturnsRef, ReturnsError> =
            Ok(ReturnsRef::One(SqlMappingRef::literal("pdb.top")));
    }

    unsafe impl BoxRet for TopType {
        unsafe fn box_into<'fcx>(self, fcinfo: &mut FcInfo<'fcx>) -> pgrx::datum::Datum<'fcx> {
            match self.into_datum() {
                Some(datum) => unsafe { fcinfo.return_raw_datum(datum) },
                None => fcinfo.return_null(),
            }
        }
    }

    unsafe impl<'fcx> ArgAbi<'fcx> for TopType {
        unsafe fn unbox_arg_unchecked(arg: Arg<'_, 'fcx>) -> Self {
            let index = arg.index();
            unsafe {
                arg.unbox_arg_using_from_datum()
                    .unwrap_or_else(|| panic!("argument {index} must not be null"))
            }
        }

        unsafe fn unbox_nullable_arg(arg: Arg<'_, 'fcx>) -> Nullable<Self> {
            unsafe { arg.unbox_arg_using_from_datum().into() }
        }
    }
}

extension_sql!(
    r#"
        CREATE SCHEMA IF NOT EXISTS pdb;
        CREATE TYPE pdb.top;
    "#,
    name = "TopType_shell",
    creates = [Type(TopType)]
);

#[pg_extern(immutable, parallel_safe)]
fn top_in(_input: &CStr, _typoid: pg_sys::Oid, _typmod: i32) -> TopType {
    panic!("{TOP_NOT_EXECUTABLE}");
}

#[pg_extern(immutable, parallel_safe)]
fn top_out(_input: TopType) -> CString {
    CString::from_str("pdb.top").unwrap()
}

/// Parse the `(n)` of `::pdb.top(n)`: a single positive candidate-window
/// size, stored directly as the typmod.
#[pg_extern(immutable, parallel_safe)]
fn top_typmod_in(typmod_parts: pgrx::Array<&CStr>) -> i32 {
    assert!(
        typmod_parts.len() == 1,
        "pdb.top takes exactly one modifier"
    );
    let n_str = typmod_parts
        .get(0)
        .unwrap()
        .expect("typmod cstring must not be NULL")
        .to_str()
        .unwrap();
    let n = i32::from_str(n_str).unwrap_or_else(|_| panic!("invalid pdb.top window: {n_str}"));
    if n < 1 {
        panic!("pdb.top window must be >= 1, got {n}");
    }
    n
}

#[pg_extern(immutable, parallel_safe)]
fn top_typmod_out(typmod: i32) -> CString {
    CString::from_str(&format!("({typmod})")).unwrap()
}

extension_sql!(
    r#"
        CREATE TYPE pdb.top (
            INPUT = top_in,
            OUTPUT = top_out,
            LIKE = bool,
            TYPMOD_IN = top_typmod_in,
            TYPMOD_OUT = top_typmod_out
        );
    "#,
    name = "TopType_final",
    requires = [
        "TopType_shell",
        top_in,
        top_out,
        top_typmod_in,
        top_typmod_out
    ]
);

/// `(bool_expr)::pdb.top(n)`: the cast that creates an arm. Consumed at plan
/// time by qual inspection; never legitimately executed.
#[pg_extern(immutable, parallel_safe)]
fn bool_to_top(_input: bool, _typmod: i32, _is_explicit: bool) -> TopType {
    panic!("{TOP_NOT_EXECUTABLE}");
}

/// Re-annotation (`x::pdb.top(3)::pdb.top(5)`); the outermost typmod wins.
#[pg_extern(immutable, parallel_safe)]
fn top_to_top(_input: TopType, _typmod: i32, _is_explicit: bool) -> TopType {
    panic!("{TOP_NOT_EXECUTABLE}");
}

/// Lets the annotated expression stand where SQL requires a boolean (the
/// WHERE clause). Also consumed at plan time.
#[pg_extern(immutable, parallel_safe)]
fn top_to_bool(_input: TopType) -> bool {
    panic!("{TOP_NOT_EXECUTABLE}");
}

extension_sql!(
    r#"
        CREATE CAST (boolean AS pdb.top) WITH FUNCTION bool_to_top(boolean, integer, boolean) AS ASSIGNMENT;
        CREATE CAST (pdb.top AS pdb.top) WITH FUNCTION top_to_top(pdb.top, integer, boolean) AS IMPLICIT;
        CREATE CAST (pdb.top AS boolean) WITH FUNCTION top_to_bool(pdb.top) AS IMPLICIT;
    "#,
    name = "cast_top",
    requires = [bool_to_top, top_to_top, top_to_bool, "TopType_final"]
);

/// Look up a function in the `paradedb` schema, returning `InvalidOid` when
/// it does not exist yet (upgrade safety).
fn lookup_paradedb_function(func_name: &str, arg_types: &[pg_sys::Oid]) -> pg_sys::Oid {
    unsafe {
        let schema = pg_sys::get_namespace_oid(c"paradedb".as_ptr(), true);
        if schema == pg_sys::InvalidOid {
            return pg_sys::InvalidOid;
        }
        let mut name_list = pgrx::PgList::<pg_sys::Node>::new();
        name_list.push(pg_sys::makeString(c"paradedb".as_ptr() as *mut std::ffi::c_char) as *mut _);
        let func_name_cstr = std::ffi::CString::new(func_name).unwrap();
        name_list
            .push(pg_sys::makeString(func_name_cstr.as_ptr() as *mut std::ffi::c_char) as *mut _);
        pg_sys::LookupFuncName(
            name_list.as_ptr(),
            arg_types.len() as i32,
            arg_types.as_ptr(),
            true,
        )
    }
}

pub fn top_typoid() -> pg_sys::Oid {
    crate::postgres::catalog::lookup_typoid(c"pdb", c"top").unwrap_or(pg_sys::InvalidOid)
}

/// Oid of `paradedb.top_to_bool(pdb.top)`, the outer layer of the cast
/// sandwich qual inspection unwraps.
pub fn top_to_bool_procoid() -> pg_sys::Oid {
    let top = top_typoid();
    if top == pg_sys::InvalidOid {
        return pg_sys::InvalidOid;
    }
    lookup_paradedb_function("top_to_bool", &[top])
}

/// Oid of `paradedb.bool_to_top(boolean, integer, boolean)`.
pub fn bool_to_top_procoid() -> pg_sys::Oid {
    lookup_paradedb_function(
        "bool_to_top",
        &[pg_sys::BOOLOID, pg_sys::INT4OID, pg_sys::BOOLOID],
    )
}

/// Oid of `paradedb.top_to_top(pdb.top, integer, boolean)`.
pub fn top_to_top_procoid() -> pg_sys::Oid {
    let top = top_typoid();
    if top == pg_sys::InvalidOid {
        return pg_sys::InvalidOid;
    }
    lookup_paradedb_function("top_to_top", &[top, pg_sys::INT4OID, pg_sys::BOOLOID])
}

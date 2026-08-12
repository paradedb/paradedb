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

//! The `::pdb.top_bm25(n)` / `::pdb.top_knn(n)` arm annotations for rank
//! fusion.
//!
//! Casting a boolean search-predicate subtree to one of these types turns it
//! into a fusion *arm*: "this row is among the top `n` rows ranked by the
//! named measure". The type name declares what ranks the arm (BM25 relevance
//! or knn proximity); every other predicate inside the arm is a filter. The
//! typmod carries `n`, following the same pattern as `'term'::pdb.boost(2)`.
//!
//! ```sql
//! WHERE (category === 'footwear' AND description ||| 'running shoes')::pdb.top_bm25(100)
//!    OR (category === 'footwear' AND embedding ~~~ '[...]')::pdb.top_knn(200)
//! ORDER BY pdb.rrf(id)
//! LIMIT 10;
//! ```
//!
//! The types themselves are inert: every executable path errors, because the
//! annotation only has meaning inside a ParadeDB rank-fusion scan, which
//! consumes it at plan time (see `qual_inspect`) and never evaluates it.

use pgrx::{extension_sql, pg_extern, pg_sys};
use std::ffi::{CStr, CString};
use std::str::FromStr;

const TOP_NOT_EXECUTABLE: &str = "::pdb.top_bm25(n)/::pdb.top_knn(n) can only annotate a search predicate in a query executed by a ParadeDB rank-fusion scan: use `WHERE (<predicate>)::pdb.top_bm25(<n>) ... ORDER BY pdb.rrf(<key>) LIMIT <k>`";

/// The `pdb.top_bm25` SQL type: the annotated arm is ranked by BM25
/// relevance. Its runtime value is never legitimately constructed (the
/// annotation is consumed at plan time), so the payload is a placeholder
/// `bool`.
#[derive(Debug)]
#[repr(transparent)]
pub struct TopBm25Type(#[allow(dead_code)] bool);

/// The `pdb.top_knn` SQL type: the annotated arm is ranked by the proximity
/// of its single `~~~` knn predicate. See [`TopBm25Type`].
#[derive(Debug)]
#[repr(transparent)]
pub struct TopKnnType(#[allow(dead_code)] bool);

// pgrx boilerplate to let the annotation types cross the SQL/Rust boundary.
mod sql_datum_support {
    use super::{TopBm25Type, TopKnnType};
    use pgrx::callconv::{Arg, ArgAbi, BoxRet, FcInfo};
    use pgrx::nullable::Nullable;
    use pgrx::pgrx_sql_entity_graph::metadata::{
        ArgumentError, ReturnsError, ReturnsRef, SqlMappingRef, SqlTranslatable, TypeOrigin,
    };
    use pgrx::{pg_sys, FromDatum, IntoDatum};

    macro_rules! top_type_boilerplate {
        ($ty:ident, $sql_name:literal) => {
            impl IntoDatum for $ty {
                fn into_datum(self) -> Option<pg_sys::Datum> {
                    self.0.into_datum()
                }

                fn type_oid() -> pg_sys::Oid {
                    pg_sys::BOOLOID
                }
            }

            impl FromDatum for $ty {
                unsafe fn from_polymorphic_datum(
                    datum: pg_sys::Datum,
                    is_null: bool,
                    typoid: pg_sys::Oid,
                ) -> Option<Self> {
                    bool::from_polymorphic_datum(datum, is_null, typoid).map($ty)
                }
            }

            unsafe impl SqlTranslatable for $ty {
                const TYPE_IDENT: &'static str = pgrx::pgrx_resolved_type!($ty);
                const TYPE_ORIGIN: TypeOrigin = TypeOrigin::External;
                const ARGUMENT_SQL: Result<SqlMappingRef, ArgumentError> =
                    Ok(SqlMappingRef::literal($sql_name));
                const RETURN_SQL: Result<ReturnsRef, ReturnsError> =
                    Ok(ReturnsRef::One(SqlMappingRef::literal($sql_name)));
            }

            unsafe impl BoxRet for $ty {
                unsafe fn box_into<'fcx>(
                    self,
                    fcinfo: &mut FcInfo<'fcx>,
                ) -> pgrx::datum::Datum<'fcx> {
                    match self.into_datum() {
                        Some(datum) => unsafe { fcinfo.return_raw_datum(datum) },
                        None => fcinfo.return_null(),
                    }
                }
            }

            unsafe impl<'fcx> ArgAbi<'fcx> for $ty {
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
        };
    }

    top_type_boilerplate!(TopBm25Type, "pdb.top_bm25");
    top_type_boilerplate!(TopKnnType, "pdb.top_knn");
}

extension_sql!(
    r#"
        CREATE SCHEMA IF NOT EXISTS pdb;
        CREATE TYPE pdb.top_bm25;
        CREATE TYPE pdb.top_knn;
    "#,
    name = "TopType_shell",
    creates = [Type(TopBm25Type), Type(TopKnnType)]
);

/// Parse the `(n)` of a `::pdb.top_bm25(n)` / `::pdb.top_knn(n)` annotation:
/// a single positive candidate-window size, stored directly as the typmod.
/// Shared by both annotation types.
#[pg_extern(immutable, parallel_safe)]
fn top_typmod_in(typmod_parts: pgrx::Array<&CStr>) -> i32 {
    assert!(
        typmod_parts.len() == 1,
        "fusion-arm annotations take exactly one modifier"
    );
    let n_str = typmod_parts
        .get(0)
        .unwrap()
        .expect("typmod cstring must not be NULL")
        .to_str()
        .unwrap();
    let n = i32::from_str(n_str).unwrap_or_else(|_| panic!("invalid arm window: {n_str}"));
    if n < 1 {
        panic!("the arm window must be >= 1, got {n}");
    }
    n
}

#[pg_extern(immutable, parallel_safe)]
fn top_typmod_out(typmod: i32) -> CString {
    CString::from_str(&format!("({typmod})")).unwrap()
}

#[pg_extern(immutable, parallel_safe)]
fn top_bm25_in(_input: &CStr, _typoid: pg_sys::Oid, _typmod: i32) -> TopBm25Type {
    panic!("{TOP_NOT_EXECUTABLE}");
}

#[pg_extern(immutable, parallel_safe)]
fn top_bm25_out(_input: TopBm25Type) -> CString {
    CString::from_str("pdb.top_bm25").unwrap()
}

#[pg_extern(immutable, parallel_safe)]
fn top_knn_in(_input: &CStr, _typoid: pg_sys::Oid, _typmod: i32) -> TopKnnType {
    panic!("{TOP_NOT_EXECUTABLE}");
}

#[pg_extern(immutable, parallel_safe)]
fn top_knn_out(_input: TopKnnType) -> CString {
    CString::from_str("pdb.top_knn").unwrap()
}

extension_sql!(
    r#"
        CREATE TYPE pdb.top_bm25 (
            INPUT = top_bm25_in,
            OUTPUT = top_bm25_out,
            LIKE = bool,
            TYPMOD_IN = top_typmod_in,
            TYPMOD_OUT = top_typmod_out
        );
        CREATE TYPE pdb.top_knn (
            INPUT = top_knn_in,
            OUTPUT = top_knn_out,
            LIKE = bool,
            TYPMOD_IN = top_typmod_in,
            TYPMOD_OUT = top_typmod_out
        );
    "#,
    name = "TopType_final",
    requires = [
        "TopType_shell",
        top_bm25_in,
        top_bm25_out,
        top_knn_in,
        top_knn_out,
        top_typmod_in,
        top_typmod_out
    ]
);

/// `(bool_expr)::pdb.top_bm25(n)`: the cast that creates a BM25-ranked arm.
/// Consumed at plan time by qual inspection; never legitimately executed.
#[pg_extern(immutable, parallel_safe)]
fn bool_to_top_bm25(_input: bool, _typmod: i32, _is_explicit: bool) -> TopBm25Type {
    panic!("{TOP_NOT_EXECUTABLE}");
}

/// Re-annotation (`x::pdb.top_bm25(3)::pdb.top_bm25(5)`); the outermost
/// typmod wins.
#[pg_extern(immutable, parallel_safe)]
fn top_bm25_to_top_bm25(_input: TopBm25Type, _typmod: i32, _is_explicit: bool) -> TopBm25Type {
    panic!("{TOP_NOT_EXECUTABLE}");
}

/// Lets the annotated expression stand where SQL requires a boolean (the
/// WHERE clause). Also consumed at plan time.
#[pg_extern(immutable, parallel_safe)]
fn top_bm25_to_bool(_input: TopBm25Type) -> bool {
    panic!("{TOP_NOT_EXECUTABLE}");
}

/// `(bool_expr)::pdb.top_knn(n)`: the cast that creates a knn-ranked arm.
#[pg_extern(immutable, parallel_safe)]
fn bool_to_top_knn(_input: bool, _typmod: i32, _is_explicit: bool) -> TopKnnType {
    panic!("{TOP_NOT_EXECUTABLE}");
}

/// Re-annotation (`x::pdb.top_knn(3)::pdb.top_knn(5)`); the outermost typmod
/// wins.
#[pg_extern(immutable, parallel_safe)]
fn top_knn_to_top_knn(_input: TopKnnType, _typmod: i32, _is_explicit: bool) -> TopKnnType {
    panic!("{TOP_NOT_EXECUTABLE}");
}

#[pg_extern(immutable, parallel_safe)]
fn top_knn_to_bool(_input: TopKnnType) -> bool {
    panic!("{TOP_NOT_EXECUTABLE}");
}

extension_sql!(
    r#"
        CREATE CAST (boolean AS pdb.top_bm25) WITH FUNCTION bool_to_top_bm25(boolean, integer, boolean) AS ASSIGNMENT;
        CREATE CAST (pdb.top_bm25 AS pdb.top_bm25) WITH FUNCTION top_bm25_to_top_bm25(pdb.top_bm25, integer, boolean) AS IMPLICIT;
        CREATE CAST (pdb.top_bm25 AS boolean) WITH FUNCTION top_bm25_to_bool(pdb.top_bm25) AS IMPLICIT;
        CREATE CAST (boolean AS pdb.top_knn) WITH FUNCTION bool_to_top_knn(boolean, integer, boolean) AS ASSIGNMENT;
        CREATE CAST (pdb.top_knn AS pdb.top_knn) WITH FUNCTION top_knn_to_top_knn(pdb.top_knn, integer, boolean) AS IMPLICIT;
        CREATE CAST (pdb.top_knn AS boolean) WITH FUNCTION top_knn_to_bool(pdb.top_knn) AS IMPLICIT;
    "#,
    name = "cast_top",
    requires = [
        bool_to_top_bm25,
        top_bm25_to_top_bm25,
        top_bm25_to_bool,
        bool_to_top_knn,
        top_knn_to_top_knn,
        top_knn_to_bool,
        "TopType_final"
    ]
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

/// The cast-function oids that make up one annotation type's cast sandwich
/// `<to_bool>(<retypmod>*(<creator>(subtree, typmod, _)))`; see
/// `qual_inspect::top_arm`.
pub struct TopCastProcs {
    pub to_bool: pg_sys::Oid,
    pub creator: pg_sys::Oid,
    pub retypmod: pg_sys::Oid,
}

fn cast_procs(type_name: &CStr, base: &str) -> Option<TopCastProcs> {
    let typoid = crate::postgres::catalog::lookup_typoid(c"pdb", type_name)?;
    Some(TopCastProcs {
        to_bool: lookup_paradedb_function(&format!("{base}_to_bool"), &[typoid]),
        creator: lookup_paradedb_function(
            &format!("bool_to_{base}"),
            &[pg_sys::BOOLOID, pg_sys::INT4OID, pg_sys::BOOLOID],
        ),
        retypmod: lookup_paradedb_function(
            &format!("{base}_to_{base}"),
            &[typoid, pg_sys::INT4OID, pg_sys::BOOLOID],
        ),
    })
}

/// Cast-sandwich oids for `pdb.top_bm25`, or `None` before the type exists
/// (upgrade safety).
pub fn top_bm25_procs() -> Option<TopCastProcs> {
    cast_procs(c"top_bm25", "top_bm25")
}

/// Cast-sandwich oids for `pdb.top_knn`.
pub fn top_knn_procs() -> Option<TopCastProcs> {
    cast_procs(c"top_knn", "top_knn")
}

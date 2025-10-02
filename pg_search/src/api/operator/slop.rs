// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::api::operator::boost::{query_to_boost, BoostType};
use crate::query::pdb_query::pdb;
use pgrx::{extension_sql, pg_cast, pg_extern};

/// [`SlopType`] is a user-facing type used in SQL queries to indicate that certain query predicates
/// can have a "slop" value applied to them.  This is limited to "phrase queries"
///
/// While there's no indication on the Rust type, [`SlopType`] wants a Postgres type modifier (typmod)
/// when constructed so that a [`pdb::Query::Slop { slop: $typemod, query }`] can be constructed.
///
/// Users would use this type like so:
///
/// ```sql
/// SELECT * FROM t WHERE body @@@ 'beer'::slop(3);
/// ```
///
/// It's up to individual operators to decide if/how they support [`SlopType`]
#[derive(Debug)]
#[repr(transparent)]
pub struct SlopType(pdb::Query);

// Contains all the boilerplate required by pgrx to make a custom type from scratch
mod sql_datum_support {
    use crate::api::operator::slop::SlopType;
    use crate::api::operator::slop_typoid;
    use crate::query::pdb_query::pdb;
    use pgrx::callconv::{Arg, ArgAbi, BoxRet, FcInfo};
    use pgrx::nullable::Nullable;
    use pgrx::pgrx_sql_entity_graph::metadata::{
        ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
    };
    use pgrx::{pg_sys, FromDatum, IntoDatum};

    impl From<SlopType> for pdb::Query {
        fn from(value: SlopType) -> Self {
            value.0
        }
    }

    impl IntoDatum for SlopType {
        fn into_datum(self) -> Option<pg_sys::Datum> {
            self.0.into_datum()
        }

        fn type_oid() -> pg_sys::Oid {
            slop_typoid()
        }
    }

    impl FromDatum for SlopType {
        unsafe fn from_polymorphic_datum(
            datum: pg_sys::Datum,
            is_null: bool,
            typoid: pg_sys::Oid,
        ) -> Option<Self> {
            pdb::Query::from_polymorphic_datum(datum, is_null, typoid).map(SlopType)
        }
    }

    unsafe impl SqlTranslatable for SlopType {
        fn argument_sql() -> Result<SqlMapping, ArgumentError> {
            Ok(SqlMapping::As("slop".into()))
        }

        fn return_sql() -> Result<Returns, ReturnsError> {
            Ok(Returns::One(SqlMapping::As("slop".into())))
        }
    }

    unsafe impl BoxRet for SlopType {
        unsafe fn box_into<'fcx>(self, fcinfo: &mut FcInfo<'fcx>) -> pgrx::datum::Datum<'fcx> {
            match self.into_datum() {
                Some(datum) => unsafe { fcinfo.return_raw_datum(datum) },
                None => fcinfo.return_null(),
            }
        }
    }

    unsafe impl<'fcx> ArgAbi<'fcx> for SlopType {
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

// [`SlopType`]'s SQL type definition and necessary functions to support creating
mod typedef {
    use crate::api::operator::slop::SlopType;
    use crate::query::pdb_query::pdb;
    use crate::query::pdb_query::pdb::{query_out, SlopData};
    use pgrx::{extension_sql, pg_extern, pg_sys, Array};
    use std::ffi::{CStr, CString};
    use std::str::FromStr;

    extension_sql!(
        r#"
            CREATE TYPE pg_catalog.slop;
        "#,
        name = "SlopType_shell",
        creates = [Type(SlopType)]
    );

    #[pg_extern(immutable, parallel_safe)]
    fn slop_in(input: &CStr, _typoid: pg_sys::Oid, typmod: i32) -> SlopType {
        let mut query =
            pdb::Query::unclassified_string(input.to_str().expect("input must not be NULL"));
        query.apply_slop_data((typmod != -1).then(|| typmod.into()));
        SlopType(query)
    }

    #[pg_extern(immutable, parallel_safe)]
    fn slop_out(input: SlopType) -> CString {
        query_out(input.0)
    }

    /// Parse the user-specified "typmod" value string and encode it into an i32 following the
    /// `impl From<SlopData> for i32` implementation over on [`crate::query::pdb_query::pdb::SlopData`]
    #[pg_extern(immutable, parallel_safe)]
    fn slop_typmod_in(typmod_parts: Array<&CStr>) -> i32 {
        let slop_str = typmod_parts
            .get(0)
            .unwrap()
            .expect("typmod cstring must not be NULL");

        let slop = u32::from_str(slop_str.to_str().unwrap())
            .unwrap_or_else(|_| panic!("invalid slop value: {}", slop_str.to_str().unwrap()));

        SlopData { slop }.into()
    }

    #[pg_extern(immutable, parallel_safe)]
    fn slop_typmod_out(typmod: i32) -> CString {
        let slop_data: SlopData = typmod.into();
        CString::from_str(&slop_data.to_string()).unwrap()
    }

    extension_sql!(
        r#"
            CREATE TYPE pg_catalog.slop (
                INPUT = slop_in,
                OUTPUT = slop_out,
                INTERNALLENGTH = VARIABLE,
                LIKE = text,
                TYPMOD_IN = slop_typmod_in,
                TYPMOD_OUT = slop_typmod_out
            );
        "#,
        name = "SlopType_final",
        requires = [
            "SlopType_shell",
            slop_in,
            slop_out,
            slop_typmod_in,
            slop_typmod_out
        ]
    );
}

#[pg_extern(immutable, parallel_safe)]
fn query_to_slop(mut input: pdb::Query, typmod: i32, _is_explicit: bool) -> SlopType {
    input.apply_slop_data((typmod != -1).then(|| typmod.into()));
    SlopType(input)
}

#[pg_cast(implicit, immutable, parallel_safe)]
fn slop_to_query(input: SlopType) -> pdb::Query {
    input.0
}

#[pg_extern(immutable, parallel_safe)]
fn slop_to_boost(input: SlopType, typmod: i32, is_explicit: bool) -> BoostType {
    query_to_boost(input.0, typmod, is_explicit)
}

/// SQL `CAST` function used by Postgres to apply the `typmod` value after the type has been constructed
///
/// One would think the type's input function, which also allows for a `typmod` argument would be
/// able to do this, but alas, Postgres always sets that to `-1` for historical reasons.
///
/// In our case, a simple expression like:
///
/// ```sql
/// SELECT 'foo'::slop(3);
/// ```
///
/// Will first go through the `slop_in` function with a `-1` typmod, and then Postgres will call
/// the `typmod_in`/`typmod_out` functions defined for [`SlopType`], then pass that output value
/// to this function so that we can apply it.  Fun!
#[pg_extern(immutable, parallel_safe)]
pub fn slop_to_slop(input: SlopType, typmod: i32, is_explicit: bool) -> SlopType {
    query_to_slop(input.0, typmod, is_explicit)
}

extension_sql!(
    r#"
        CREATE CAST (pdb.query AS pg_catalog.slop) WITH FUNCTION query_to_slop(pdb.query, integer, boolean) AS ASSIGNMENT;
        CREATE CAST (pg_catalog.slop AS pg_catalog.boost) WITH FUNCTION slop_to_boost(pg_catalog.slop, integer, boolean) AS IMPLICIT;
        CREATE CAST (pg_catalog.slop AS pg_catalog.slop) WITH FUNCTION slop_to_slop(pg_catalog.slop, integer, boolean) AS IMPLICIT;
    "#,
    name = "cast_to_slop",
    requires = [query_to_slop, slop_to_boost, slop_to_slop, "SlopType_final"]
);

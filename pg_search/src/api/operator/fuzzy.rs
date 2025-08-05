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

/// [`FuzzyType`] is a user-facing type used in SQL queries to indicate that certain query predicates
/// can have a "fuzzy" value applied to them.  Typically this is "term" and "match" queries.
///
/// While there's no indication on the Rust type, [`FuzzyType`] wants a Postgres type modifier (typmod)
/// when constructed so that a [`pdb::Query::Fuzzy { fuzzy: $typemod, query }`] can be constructed.
///
/// Users would use this type like so:
///
/// ```sql
/// SELECT * FROM t WHERE body @@@ 'beer'::fuzzy(3);
/// ```
///
/// It's up to individual operators to decide if/how they support [`FuzzyType`]
#[derive(Debug)]
#[repr(transparent)]
pub struct FuzzyType(pdb::Query);

// Contains all the boilerplate required by pgrx to make a custom type from scratch
mod sql_datum_support {
    use crate::api::operator::fuzzy::FuzzyType;
    use crate::api::operator::fuzzy_typoid;
    use crate::query::pdb_query::pdb;
    use pgrx::callconv::{Arg, ArgAbi, BoxRet, FcInfo};
    use pgrx::nullable::Nullable;
    use pgrx::pgrx_sql_entity_graph::metadata::{
        ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
    };
    use pgrx::{pg_sys, FromDatum, IntoDatum};

    impl From<FuzzyType> for pdb::Query {
        fn from(value: FuzzyType) -> Self {
            value.0
        }
    }

    impl IntoDatum for FuzzyType {
        fn into_datum(self) -> Option<pg_sys::Datum> {
            self.0.into_datum()
        }

        fn type_oid() -> pg_sys::Oid {
            fuzzy_typoid()
        }
    }

    impl FromDatum for FuzzyType {
        unsafe fn from_polymorphic_datum(
            datum: pg_sys::Datum,
            is_null: bool,
            typoid: pg_sys::Oid,
        ) -> Option<Self> {
            pdb::Query::from_polymorphic_datum(datum, is_null, typoid).map(FuzzyType)
        }
    }

    unsafe impl SqlTranslatable for FuzzyType {
        fn argument_sql() -> Result<SqlMapping, ArgumentError> {
            Ok(SqlMapping::As("fuzzy".into()))
        }

        fn return_sql() -> Result<Returns, ReturnsError> {
            Ok(Returns::One(SqlMapping::As("fuzzy".into())))
        }
    }

    unsafe impl BoxRet for FuzzyType {
        unsafe fn box_into<'fcx>(self, fcinfo: &mut FcInfo<'fcx>) -> pgrx::datum::Datum<'fcx> {
            match self.into_datum() {
                Some(datum) => unsafe { fcinfo.return_raw_datum(datum) },
                None => fcinfo.return_null(),
            }
        }
    }

    unsafe impl<'fcx> ArgAbi<'fcx> for FuzzyType {
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

// [`FuzzyType`]'s SQL type definition and necessary functions to support creating
mod typedef {
    use crate::api::operator::fuzzy::FuzzyType;
    use crate::query::pdb_query::pdb;
    use crate::query::pdb_query::pdb::{query_out, FuzzyData};
    use pgrx::{extension_sql, pg_extern, pg_sys, Array};
    use std::ffi::{CStr, CString};
    use std::str::FromStr;

    extension_sql!(
        r#"
            CREATE TYPE pg_catalog.fuzzy;
        "#,
        name = "FuzzyType_shell",
        creates = [Type(FuzzyType)]
    );

    #[pg_extern(immutable, parallel_safe)]
    fn fuzzy_in(input: &CStr, _typoid: pg_sys::Oid, typmod: i32) -> FuzzyType {
        let mut query =
            pdb::Query::unclassified_string(input.to_str().expect("input must not be NULL"));
        query.apply_fuzzy_data((typmod != -1).then(|| typmod.into()));
        FuzzyType(query)
    }

    #[pg_extern(immutable, parallel_safe)]
    fn fuzzy_out(input: FuzzyType) -> CString {
        query_out(input.0)
    }

    /// Parse the user-specified "typmod" value string and encode it into an i32 following the
    /// `impl From<FuzzyData> for i32` implementation over on [`crate::query::pdb_query::pdb::FuzzyData`]
    #[pg_extern(immutable, parallel_safe)]
    fn fuzzy_typmod_in(typmod_parts: Array<&CStr>) -> i32 {
        assert!(typmod_parts.len() <= 3);
        let fuzzy_str = typmod_parts
            .get(0)
            .unwrap()
            .expect("typmod cstring must not be NULL");
        let is_prefix = typmod_parts
            .get(1)
            .unwrap_or(Some(c"false"))
            .expect("prefix value cannot be NULL")
            .to_str()
            .unwrap()
            .starts_with("t");
        let transposition_cost_one = typmod_parts
            .get(2)
            .unwrap_or(Some(c"false"))
            .expect("transposition cost value cannot be NULL")
            .to_str()
            .unwrap()
            .starts_with("t");

        let fuzzy = i8::from_str(fuzzy_str.to_str().unwrap())
            .unwrap_or_else(|_| panic!("invalid fuzzy value: {}", fuzzy_str.to_str().unwrap()));

        if !(0..=2).contains(&fuzzy) {
            panic!("fuzzy value must be 0, 1, or 2");
        }

        FuzzyData {
            distance: fuzzy as u8,
            prefix: is_prefix,
            transposition_cost_one,
        }
        .into()
    }

    #[pg_extern(immutable, parallel_safe)]
    fn fuzzy_typmod_out(typmod: i32) -> CString {
        let fuzzy_data: FuzzyData = typmod.into();
        CString::from_str(&fuzzy_data.to_string()).unwrap()
    }

    extension_sql!(
        r#"
            CREATE TYPE pg_catalog.fuzzy (
                INPUT = fuzzy_in,
                OUTPUT = fuzzy_out,
                INTERNALLENGTH = VARIABLE,
                LIKE = text,
                TYPMOD_IN = fuzzy_typmod_in,
                TYPMOD_OUT = fuzzy_typmod_out
            );
        "#,
        name = "FuzzyType_final",
        requires = [
            "FuzzyType_shell",
            fuzzy_in,
            fuzzy_out,
            fuzzy_typmod_in,
            fuzzy_typmod_out
        ]
    );
}

#[pg_extern(immutable, parallel_safe)]
fn query_to_fuzzy(mut input: pdb::Query, typmod: i32, _is_explicit: bool) -> FuzzyType {
    input.apply_fuzzy_data((typmod != -1).then(|| typmod.into()));
    FuzzyType(input)
}

#[pg_cast(implicit, immutable, parallel_safe)]
fn fuzzy_to_query(input: FuzzyType) -> pdb::Query {
    input.0
}

#[pg_extern(immutable, parallel_safe)]
fn fuzzy_to_boost(input: FuzzyType, typmod: i32, is_explicit: bool) -> BoostType {
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
/// SELECT 'foo'::fuzzy(3);
/// ```
///
/// Will first go through the `fuzzy_in` function with a `-1` typmod, and then Postgres will call
/// the `typmod_in`/`typmod_out` functions defined for [`FuzzyType`], then pass that output value
/// to this function so that we can apply it.  Fun!
#[pg_extern(immutable, parallel_safe)]
pub fn fuzzy_to_fuzzy(input: FuzzyType, typmod: i32, is_explicit: bool) -> FuzzyType {
    query_to_fuzzy(input.0, typmod, is_explicit)
}

extension_sql!(
    r#"
        CREATE CAST (pdb.query AS pg_catalog.fuzzy) WITH FUNCTION query_to_fuzzy(pdb.query, integer, boolean) AS ASSIGNMENT;
        CREATE CAST (pg_catalog.fuzzy AS pg_catalog.boost) WITH FUNCTION fuzzy_to_boost(pg_catalog.fuzzy, integer, boolean) AS IMPLICIT;
        CREATE CAST (pg_catalog.fuzzy AS pg_catalog.fuzzy) WITH FUNCTION fuzzy_to_fuzzy(pg_catalog.fuzzy, integer, boolean) AS IMPLICIT;
    "#,
    name = "cast_to_fuzzy",
    requires = [
        query_to_fuzzy,
        fuzzy_to_boost,
        fuzzy_to_fuzzy,
        "FuzzyType_final"
    ]
);

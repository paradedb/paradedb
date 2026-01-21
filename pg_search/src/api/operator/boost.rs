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

use crate::api::operator::f16_typmod::deserialize_i32_to_f32;
use crate::query::pdb_query::pdb;
use crate::query::pdb_query::pdb::ScoreAdjustStyle;
use crate::query::proximity::ProximityClause;
use pgrx::{extension_sql, pg_cast, pg_extern};

/// [`BoostType`] is a user-facing type used in SQL queries to indicate that should boost a query
/// predicate's score.  The "boost" value is a multiplier so users should watch out for zero (`0`).
///
/// While there's no indication on the Rust type, [`BoostType`] wants a Postgres type modifier (typmod)
/// when constructed so that a [`pdb::Query::Boost { boost: $typemod, query }`] can be constructed.
///
/// Users would use this type like so:
///
/// ```sql
/// SELECT * FROM t WHERE body @@@ 'beer'::boost(3);
/// ```
///
/// It's up to individual operators to decide if/how they support [`BoostType`]
#[derive(Debug)]
#[repr(transparent)]
pub struct BoostType(pdb::Query);

// Contains all the boilerplate required by pgrx to make a custom type from scratch
mod sql_datum_support {
    use crate::api::operator::boost::BoostType;
    use crate::api::operator::boost_typoid;
    use crate::query::pdb_query::pdb;
    use pgrx::callconv::{Arg, ArgAbi, BoxRet, FcInfo};
    use pgrx::nullable::Nullable;
    use pgrx::pgrx_sql_entity_graph::metadata::{
        ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
    };
    use pgrx::{pg_sys, FromDatum, IntoDatum};

    impl From<BoostType> for pdb::Query {
        fn from(value: BoostType) -> Self {
            match value.0 {
                boost @ pdb::Query::ScoreAdjusted { .. } => boost,
                other => pdb::Query::ScoreAdjusted {
                    query: Box::new(other),
                    score: None,
                },
            }
        }
    }

    impl IntoDatum for BoostType {
        fn into_datum(self) -> Option<pg_sys::Datum> {
            self.0.into_datum()
        }

        fn type_oid() -> pg_sys::Oid {
            boost_typoid()
        }
    }

    impl FromDatum for BoostType {
        unsafe fn from_polymorphic_datum(
            datum: pg_sys::Datum,
            is_null: bool,
            typoid: pg_sys::Oid,
        ) -> Option<Self> {
            pdb::Query::from_polymorphic_datum(datum, is_null, typoid).map(BoostType)
        }
    }

    unsafe impl SqlTranslatable for BoostType {
        fn argument_sql() -> Result<SqlMapping, ArgumentError> {
            Ok(SqlMapping::As("pdb.boost".into()))
        }

        fn return_sql() -> Result<Returns, ReturnsError> {
            Ok(Returns::One(SqlMapping::As("pdb.boost".into())))
        }
    }

    unsafe impl BoxRet for BoostType {
        unsafe fn box_into<'fcx>(self, fcinfo: &mut FcInfo<'fcx>) -> pgrx::datum::Datum<'fcx> {
            match self.into_datum() {
                Some(datum) => unsafe { fcinfo.return_raw_datum(datum) },
                None => fcinfo.return_null(),
            }
        }
    }

    unsafe impl<'fcx> ArgAbi<'fcx> for BoostType {
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

// [`BoostType`]'s SQL type definition and necessary functions to support creating
mod typedef {
    use crate::api::operator::boost::BoostType;
    use crate::api::operator::f16_typmod::{
        deserialize_i32_to_f32, serialize_f32_to_i32, TYPMOD_BOUNDS,
    };
    use crate::query::pdb_query::pdb;
    use crate::query::pdb_query::pdb::{query_out, ScoreAdjustStyle};
    use pgrx::{extension_sql, pg_extern, pg_sys, Array};
    use std::ffi::{CStr, CString};
    use std::str::FromStr;

    extension_sql!(
        r#"
            CREATE TYPE pdb.boost;
        "#,
        name = "BoostType_shell",
        creates = [Type(BoostType)]
    );

    #[pg_extern(immutable, parallel_safe)]
    fn boost_in(input: &CStr, _typoid: pg_sys::Oid, typmod: i32) -> BoostType {
        let query =
            pdb::Query::unclassified_string(input.to_str().expect("input must not be NULL"));
        BoostType(pdb::Query::ScoreAdjusted {
            query: Box::new(query),
            score: (typmod != -1)
                .then(|| deserialize_i32_to_f32(typmod))
                .map(ScoreAdjustStyle::Boost),
        })
    }

    #[pg_extern(immutable, parallel_safe)]
    fn boost_out(input: BoostType) -> CString {
        query_out(input.0)
    }

    /// Parse the user-specified "typmod" value string and encode it into an i32 after round-tripping
    /// it through a [`half::f16`] so that the user's value will fit in the positive side of an i32,
    /// which is all Postgres lets us use.
    ///
    /// We clamp the user-provided value to `[-2048.0..2028.0]` to avoid confusion around precision
    /// loss of larger integers, due to the nature of 16 bit floats.
    #[pg_extern(immutable, parallel_safe)]
    fn boost_typmod_in(typmod_parts: Array<&CStr>) -> i32 {
        assert!(typmod_parts.len() == 1);
        let boost_str = typmod_parts
            .get(0)
            .unwrap()
            .expect("typmod cstring must not be NULL");
        let boost_f32 = f32::from_str(boost_str.to_str().unwrap())
            .unwrap_or_else(|_| panic!("invalid boost value: {}", boost_str.to_str().unwrap()));

        let boost = boost_f32.clamp(TYPMOD_BOUNDS.0, TYPMOD_BOUNDS.1);
        serialize_f32_to_i32(boost)
    }

    #[pg_extern(immutable, parallel_safe)]
    fn boost_typmod_out(typmod: i32) -> CString {
        let boost = deserialize_i32_to_f32(typmod);
        CString::from_str(&boost.to_string()).unwrap()
    }

    extension_sql!(
        r#"
            CREATE TYPE pdb.boost (
                INPUT = boost_in,
                OUTPUT = boost_out,
                INTERNALLENGTH = VARIABLE,
                LIKE = text,
                TYPMOD_IN = boost_typmod_in,
                TYPMOD_OUT = boost_typmod_out
            );
        "#,
        name = "BoostType_final",
        requires = [
            "BoostType_shell",
            boost_in,
            boost_out,
            boost_typmod_in,
            boost_typmod_out
        ]
    );
}

#[pg_extern(immutable, parallel_safe)]
pub fn query_to_boost(input: pdb::Query, typmod: i32, _is_explicit: bool) -> BoostType {
    let boost = deserialize_i32_to_f32(typmod);
    BoostType(pdb::Query::ScoreAdjusted {
        query: Box::new(input),
        score: Some(ScoreAdjustStyle::Boost(boost)),
    })
}

#[pg_extern(immutable, parallel_safe)]
fn text_array_to_boost(array: Vec<String>, typmod: i32, _is_explicit: bool) -> BoostType {
    let boost = deserialize_i32_to_f32(typmod);
    let query = pdb::Query::UnclassifiedArray {
        array,
        fuzzy_data: None,
        slop_data: None,
    };
    BoostType(pdb::Query::ScoreAdjusted {
        query: Box::new(query),
        score: Some(ScoreAdjustStyle::Boost(boost)),
    })
}

#[pg_extern(immutable, parallel_safe)]
fn prox_to_boost(input: ProximityClause, typmod: i32, _is_explicit: bool) -> BoostType {
    let boost = deserialize_i32_to_f32(typmod);

    let prox = if let ProximityClause::Proximity {
        left,
        right,
        distance,
    } = input
    {
        pdb::Query::Proximity {
            left: *left,
            right: *right,
            distance,
        }
    } else {
        panic!("invalid ProximityClause variant: {input:?}")
    };

    BoostType(pdb::Query::ScoreAdjusted {
        query: Box::new(prox),
        score: Some(ScoreAdjustStyle::Boost(boost)),
    })
}

#[pg_cast(implicit, immutable, parallel_safe)]
fn boost_to_query(input: BoostType) -> pdb::Query {
    input.0
}

/// SQL `CAST` function used by Postgres to apply the `typmod` value after the type has been constructed
///
/// One would think the type's input function, which also allows for a `typmod` argument would be
/// able to do this, but alas, Postgres always sets that to `-1` for historical reasons.
///
/// In our case, a simple expression like:
///
/// ```sql
/// SELECT 'foo'::boost(3);
/// ```
///
/// Will first go through the `boost_in` function with a `-1` typmod, and then Postgres will call
/// the `typmod_in`/`typmod_out` functions defined for [`BoostType`], then pass that output value
/// to this function so that we can apply it.  Fun!
#[pg_extern(immutable, parallel_safe)]
pub fn boost_to_boost(input: BoostType, typmod: i32, _is_explicit: bool) -> BoostType {
    let new_boost = deserialize_i32_to_f32(typmod);
    let mut query = input.0;
    if let pdb::Query::ScoreAdjusted { score, .. } = &mut query {
        *score = Some(ScoreAdjustStyle::Boost(new_boost));
        BoostType(query)
    } else {
        BoostType(pdb::Query::ScoreAdjusted {
            query: Box::new(query),
            score: Some(ScoreAdjustStyle::Boost(new_boost)),
        })
    }
}

extension_sql!(
    r#"
        CREATE CAST (text[] AS pdb.boost) WITH FUNCTION text_array_to_boost(text[], integer, boolean) AS ASSIGNMENT;
        CREATE CAST (pdb.query AS pdb.boost) WITH FUNCTION query_to_boost(pdb.query, integer, boolean) AS ASSIGNMENT;
        CREATE CAST (pdb.proximityclause AS pdb.boost) WITH FUNCTION prox_to_boost(pdb.proximityclause, integer, boolean) AS ASSIGNMENT;
        CREATE CAST (pdb.boost AS pdb.boost) WITH FUNCTION boost_to_boost(pdb.boost, integer, boolean) AS IMPLICIT;
    "#,
    name = "cast_to_boost",
    requires = [
        query_to_boost,
        prox_to_boost,
        boost_to_boost,
        text_array_to_boost,
        "BoostType_final"
    ]
);

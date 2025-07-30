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

use crate::api::operator::boost_typoid;
use crate::query::pdb_query::pdb;
use crate::query::pdb_query::pdb::query_out;
use crate::query::proximity::ProximityClause;
use pgrx::callconv::{Arg, ArgAbi, BoxRet, FcInfo};
use pgrx::nullable::Nullable;
use pgrx::pg_sys::Datum;
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
};
use pgrx::{extension_sql, pg_cast, pg_extern, pg_sys, Array, FromDatum, IntoDatum};
use std::ffi::{CStr, CString};
use std::str::FromStr;

#[derive(Debug)]
pub struct BoostType(pdb::Query);

impl From<BoostType> for pdb::Query {
    fn from(value: BoostType) -> Self {
        match value.0 {
            boost @ pdb::Query::Boost { .. } => boost,
            other => pdb::Query::Boost {
                query: Box::new(other),
                boost: None,
            },
        }
    }
}

impl IntoDatum for BoostType {
    fn into_datum(self) -> Option<Datum> {
        self.0.into_datum()
    }

    fn type_oid() -> pg_sys::Oid {
        boost_typoid()
    }
}

impl FromDatum for BoostType {
    unsafe fn from_polymorphic_datum(
        datum: Datum,
        is_null: bool,
        typoid: pg_sys::Oid,
    ) -> Option<Self> {
        pdb::Query::from_polymorphic_datum(datum, is_null, typoid).map(BoostType)
    }
}

unsafe impl SqlTranslatable for BoostType {
    fn argument_sql() -> Result<SqlMapping, ArgumentError> {
        Ok(SqlMapping::As("boost".into()))
    }

    fn return_sql() -> Result<Returns, ReturnsError> {
        Ok(Returns::One(SqlMapping::As("boost".into())))
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

extension_sql!(
    r#"
        CREATE TYPE pg_catalog.boost;
    "#,
    name = "BoostType_shell",
    creates = [Type(BoostType)]
);

#[pg_extern(immutable, parallel_safe)]
fn boost_in(input: &CStr, _typoid: pg_sys::Oid, typmod: i32) -> BoostType {
    let query = pdb::Query::UnclassifiedString {
        string: input.to_str().expect("input must not be NULL").to_string(),
    };
    BoostType(pdb::Query::Boost {
        query: Box::new(query),
        boost: (typmod != -1).then(|| deserialize_i32_to_f32(typmod)),
    })
}

#[pg_extern(immutable, parallel_safe)]
fn boost_out(input: BoostType) -> CString {
    query_out(input.0)
}

#[pg_extern(immutable, parallel_safe)]
fn boost_typmod_in(typmod_parts: Array<&CStr>) -> i32 {
    assert!(typmod_parts.len() == 1);
    let boost_str = typmod_parts
        .get(0)
        .unwrap()
        .expect("typmod cstring must not be NULL");
    let boost: half::f16 =
        half::f16::from_str(boost_str.to_str().unwrap()).expect("boost typmod must be a valid f16");
    serialize_f32_to_i32(boost.to_f32())
}

#[pg_extern(immutable, parallel_safe)]
fn boost_typmod_out(typmod: i32) -> CString {
    let boost = deserialize_i32_to_f32(typmod);
    CString::from_str(&boost.to_string()).unwrap()
}

#[pg_extern(immutable, parallel_safe)]
fn query_to_boost(input: pdb::Query, typmod: i32, _is_explicit: bool) -> BoostType {
    let boost = deserialize_i32_to_f32(typmod);
    BoostType(pdb::Query::Boost {
        query: Box::new(input),
        boost: Some(boost),
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

    BoostType(pdb::Query::Boost {
        query: Box::new(prox),
        boost: Some(boost),
    })
}

#[pg_cast(implicit, immutable, parallel_safe)]
fn boost_to_query(input: BoostType) -> pdb::Query {
    input.0
}

#[pg_extern(immutable, parallel_safe)]
pub fn boost_to_boost(input: BoostType, typmod: i32, _is_explicit: bool) -> BoostType {
    let new_boost = deserialize_i32_to_f32(typmod);
    let mut query = input.0;
    if let pdb::Query::Boost { boost, .. } = &mut query {
        *boost = Some(new_boost);
        BoostType(query)
    } else {
        BoostType(pdb::Query::Boost {
            query: Box::new(query),
            boost: Some(new_boost),
        })
    }
}

extension_sql!(
    r#"
        CREATE TYPE pg_catalog.boost (
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

extension_sql!(
    r#"
        CREATE CAST (pdb.query AS pg_catalog.boost) WITH FUNCTION query_to_boost(pdb.query, integer, boolean) AS ASSIGNMENT;
        CREATE CAST (pdb.proximityclause AS pg_catalog.boost) WITH FUNCTION prox_to_boost(pdb.proximityclause, integer, boolean) AS ASSIGNMENT;
        CREATE CAST (pg_catalog.boost AS pg_catalog.boost) WITH FUNCTION boost_to_boost(pg_catalog.boost, integer, boolean) AS IMPLICIT;
    "#,
    name = "cast_to_boost",
    requires = [
        query_to_boost,
        prox_to_boost,
        boost_to_boost,
        "BoostType_final"
    ]
);

const HALF_MAX: f32 = half::f16::MAX.to_f32_const(); // ≈ 65504.0

/// Serialize an f32 to a non‑negative i32 by first converting to f16.
/// Panics if val is NaN/Inf or out of f16’s representable range.
fn serialize_f32_to_i32(val: f32) -> i32 {
    assert!(
        val.is_finite() || val.abs() < HALF_MAX,
        "only 16 bit floats in the range [-{HALF_MAX}..{HALF_MAX}] are supported"
    );

    let half: half::f16 = half::f16::from_f32(val);
    let bits: u16 = half.to_bits();
    bits as i32 // in [0, 0xFFFF], always ≥ 0
}

/// Deserialize the i32 back to a f32 via f16.
/// Panics if encoded is outside [0, 65535].
fn deserialize_i32_to_f32(encoded: i32) -> f32 {
    assert!(
        (0..=u16::MAX as i32).contains(&encoded),
        "invalid typemod `{encoded}`: must be between 0 and {}",
        u16::MAX
    );

    let bits: u16 = encoded as u16;
    let half = half::f16::from_bits(bits);
    half.to_f32()
}

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

use crate::api::tokenizers::type_is_tokenizer;
use crate::nodecast;
use crate::postgres::catalog::is_citext_oid;
use crate::postgres::catalog::{facet_encoded_str_to_ltree_text, is_ltree_oid};
use crate::postgres::datetime::PostgresDateTime;
use crate::postgres::jsonb_support::jsonb_datum_to_serde_json_value;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::range::RangeToTantivyValue;
use crate::schema::AnyEnum;
use ordered_float::OrderedFloat;
use pgrx::datum::datetime_support::DateTimeConversionError;
use pgrx::pg_sys::Datum;
use pgrx::pg_sys::Oid;
use pgrx::PostgresType;
use pgrx::{pg_sys, IntoDatum};
use pgrx::{FromDatum, PgBuiltInOids, PgOid};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::net::{AddrParseError, IpAddr};
use std::num::ParseFloatError;
use std::str::FromStr;
use tantivy::schema::{Facet, IntoIpv6Addr};
use thiserror::Error;

use super::jsonb_support::JsonbConversionError;

/// A row-oriented wrapper around Tantivy's OwnedValue.
///
/// When working with large batches of TantivyValues, consider using the `types_arrow` module
/// instead.
#[derive(Clone, Debug, Eq, PartialEq, PostgresType)]
pub struct TantivyValue(pub PdbOwnedValue);

impl Default for TantivyValue {
    fn default() -> Self {
        TantivyValue(PdbOwnedValue::Null)
    }
}

impl TantivyValue {
    pub fn into_tantivy_value(
        self,
        index_created_by_version: Option<crate::api::version::Version>,
    ) -> tantivy::schema::OwnedValue {
        self.0.into_tantivy_value(index_created_by_version)
    }

    pub fn into_owned_datetime(self) -> Option<PdbOwnedValue> {
        match self.0 {
            PdbOwnedValue::Date(_) => Some(self.0),
            _ => None,
        }
    }

    pub unsafe fn try_into_datum(self, oid: PgOid) -> Result<Option<Datum>, TantivyValueError> {
        if matches!(self.0, PdbOwnedValue::Null) {
            return Ok(None);
        }

        match &oid {
            PgOid::BuiltIn(builtin) => {
                let datum = match builtin {
                    PgBuiltInOids::BOOLOID => bool::try_from(self)?.into_datum(),
                    PgBuiltInOids::INT2OID => i16::try_from(self)?.into_datum(),
                    PgBuiltInOids::INT4OID => i32::try_from(self)?.into_datum(),
                    PgBuiltInOids::INT8OID => i64::try_from(self)?.into_datum(),
                    PgBuiltInOids::OIDOID => Oid::from(u32::try_from(self)?).into_datum(),
                    PgBuiltInOids::FLOAT4OID => f32::try_from(self)?.into_datum(),
                    PgBuiltInOids::FLOAT8OID => f64::try_from(self)?.into_datum(),
                    PgBuiltInOids::NUMERICOID => pgrx::AnyNumeric::try_from(self)?.into_datum(),
                    PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                        String::try_from(self)?.into_datum()
                    }
                    PgBuiltInOids::JSONOID => pgrx::datum::JsonString::try_from(self)?.into_datum(),
                    PgBuiltInOids::JSONBOID => pgrx::JsonB::try_from(self)?.into_datum(),
                    PgBuiltInOids::DATEOID => pgrx::datum::Date::try_from(self)?.into_datum(),
                    PgBuiltInOids::TIMESTAMPOID => {
                        pgrx::datum::Timestamp::try_from(self)?.into_datum()
                    }
                    PgBuiltInOids::TIMESTAMPTZOID => {
                        pgrx::datum::TimestampWithTimeZone::try_from(self)?.into_datum()
                    }
                    PgBuiltInOids::TIMEOID => pgrx::datum::Time::try_from(self)?.into_datum(),
                    PgBuiltInOids::TIMETZOID => {
                        pgrx::datum::TimeWithTimeZone::try_from(self)?.into_datum()
                    }
                    PgBuiltInOids::UUIDOID => pgrx::datum::Uuid::try_from(self)?.into_datum(),
                    PgBuiltInOids::INETOID => pgrx::datum::Inet::try_from(self)?.into_datum(),
                    _ => return Err(TantivyValueError::UnsupportedOid(oid.value())),
                };
                Ok(datum)
            }

            PgOid::Custom(custom) => {
                if is_citext_oid(*custom) {
                    return Ok(String::try_from(self)?.into_datum());
                }
                if is_ltree_oid(*custom) {
                    // Convert Facet back to ltree dot-separated text, then use PG's input function.
                    // Two read paths are possible:
                    //   - Stored-fields path returns `OwnedValue::Facet` with the parsed structure.
                    //   - Fast-field (columnar) path returns `OwnedValue::Str` with the raw
                    //     null-byte-separated internal Tantivy representation (e.g. `\0Top\0Science`).
                    // `facet_encoded_str_to_ltree_text` handles both cases uniformly.
                    let ltree_text = match self.0 {
                        PdbOwnedValue::Facet(ref facet) => facet.to_path().join("."),
                        PdbOwnedValue::Str(ref s) => facet_encoded_str_to_ltree_text(s),
                        _ => return Err(TantivyValueError::InvalidOid),
                    };

                    let mut typinput: pg_sys::Oid = pg_sys::InvalidOid;
                    let mut typioparam: pg_sys::Oid = pg_sys::InvalidOid;
                    pg_sys::getTypeInputInfo(*custom, &mut typinput, &mut typioparam);
                    let cstring = std::ffi::CString::new(ltree_text)
                        .map_err(|_| TantivyValueError::DatumDeref)?;
                    let datum = pg_sys::OidInputFunctionCall(
                        typinput,
                        cstring.as_ptr() as *mut std::ffi::c_char,
                        typioparam,
                        -1,
                    );
                    return Ok(Some(datum));
                }
                Err(TantivyValueError::UnsupportedOid(oid.value()))
            }
            _ => Err(TantivyValueError::InvalidOid),
        }
    }

    fn auto_promote_str_to_date(value: &mut PdbOwnedValue) {
        match value {
            PdbOwnedValue::Str(s) => {
                if let Ok(pgdt) = PostgresDateTime::try_from(s.as_str()) {
                    *value = PdbOwnedValue::Date(pgdt);
                }
            }
            PdbOwnedValue::Array(vals) => {
                for v in vals.iter_mut() {
                    Self::auto_promote_str_to_date(v)
                }
            }
            PdbOwnedValue::Object(kvs) => {
                for (_, v) in kvs.iter_mut() {
                    Self::auto_promote_str_to_date(v)
                }
            }
            _ => (),
        }
    }

    fn try_single_json_value_to_tantivy_value(
        value: Value,
    ) -> Result<TantivyValue, TantivyValueError> {
        let mut pdb_val = PdbOwnedValue::try_from(value)?;
        Self::auto_promote_str_to_date(&mut pdb_val);
        Ok(TantivyValue(pdb_val))
    }

    fn try_json_value_to_tantivy_value(
        value: Value,
    ) -> Result<Vec<TantivyValue>, TantivyValueError> {
        match value {
            // A tantivy JSON value can't be a top-level array, so we have to make
            // separate values out of each entry.
            Value::Array(value_vec) => value_vec
                .into_iter()
                .map(Self::try_single_json_value_to_tantivy_value)
                .collect(),
            _ => Ok(vec![Self::try_single_json_value_to_tantivy_value(value)?]),
        }
    }

    pub unsafe fn try_from_datum_array(
        datum: Datum,
        oid: PgOid,
    ) -> Result<Vec<Self>, TantivyValueError> {
        match &oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::BOOLOID
                | PgBuiltInOids::INT2OID
                | PgBuiltInOids::INT4OID
                | PgBuiltInOids::INT8OID
                | PgBuiltInOids::OIDOID
                | PgBuiltInOids::FLOAT4OID
                | PgBuiltInOids::FLOAT8OID
                | PgBuiltInOids::NUMERICOID
                | PgBuiltInOids::TEXTOID
                | PgBuiltInOids::VARCHAROID
                | PgBuiltInOids::DATEOID
                | PgBuiltInOids::TIMESTAMPOID
                | PgBuiltInOids::TIMESTAMPTZOID
                | PgBuiltInOids::TIMEOID
                | PgBuiltInOids::TIMETZOID
                | PgBuiltInOids::UUIDOID
                | PgBuiltInOids::INETOID => {
                    let array: pgrx::Array<Datum> = pgrx::Array::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?;
                    array
                        .iter()
                        .flatten()
                        .map(|element_datum| Self::try_from_datum(element_datum, oid))
                        .collect()
                }
                _ => Err(TantivyValueError::UnsupportedArrayOid(oid.value())),
            },
            _ => Err(TantivyValueError::InvalidOid),
        }
    }

    pub unsafe fn try_from_datum_json(
        datum: Datum,
        oid: PgOid,
    ) -> Result<Vec<Self>, TantivyValueError> {
        match &oid {
            PgOid::BuiltIn(builtin) => match builtin {
                // Tantivy has a limitation that prevents JSON top-level arrays from being
                // inserted into the index. Therefore, we need to flatten the array elements
                // individually before converting them into Tantivy values.
                PgBuiltInOids::JSONBOID => {
                    let serde_json_value = jsonb_datum_to_serde_json_value(datum)
                        .ok_or(TantivyValueError::DatumDeref)?
                        .map_err(TantivyValueError::from)?;
                    Self::try_json_value_to_tantivy_value(serde_json_value)
                }
                PgBuiltInOids::JSONOID => {
                    let pgrx_value = pgrx::Json::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?;
                    Self::try_json_value_to_tantivy_value(pgrx_value.0)
                }
                _ => Err(TantivyValueError::UnsupportedJsonOid(oid.value())),
            },
            _ => Err(TantivyValueError::InvalidOid),
        }
    }

    pub unsafe fn try_from_datum(datum: Datum, oid: PgOid) -> Result<Self, TantivyValueError> {
        match &oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::BOOLOID => TantivyValue::try_from(
                    bool::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::INT2OID => TantivyValue::try_from(
                    i16::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::INT4OID => TantivyValue::try_from(
                    i32::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::INT8OID => TantivyValue::try_from(
                    i64::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::OIDOID => TantivyValue::try_from(
                    u32::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::FLOAT4OID => TantivyValue::try_from(
                    f32::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::FLOAT8OID => TantivyValue::try_from(
                    f64::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::NUMERICOID => TantivyValue::try_from(
                    pgrx::AnyNumeric::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => TantivyValue::try_from(
                    String::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::DATEOID => TantivyValue::try_from(
                    pgrx::datum::Date::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::TIMESTAMPOID => TantivyValue::try_from(
                    pgrx::datum::Timestamp::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::TIMESTAMPTZOID => TantivyValue::try_from(
                    pgrx::datum::TimestampWithTimeZone::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::TIMEOID => TantivyValue::try_from(
                    pgrx::datum::Time::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::TIMETZOID => TantivyValue::try_from(
                    pgrx::datum::TimeWithTimeZone::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::UUIDOID => TantivyValue::try_from(
                    pgrx::datum::Uuid::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::INETOID => TantivyValue::try_from(
                    pgrx::datum::Inet::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::INT4RANGEOID => TantivyValue::from_range(
                    pgrx::datum::Range::<i32>::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::INT8RANGEOID => TantivyValue::from_range(
                    pgrx::datum::Range::<i64>::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::NUMRANGEOID => TantivyValue::from_range(
                    pgrx::datum::Range::<pgrx::AnyNumeric>::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::DATERANGEOID => TantivyValue::from_range(
                    pgrx::datum::Range::<pgrx::datum::Date>::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::TSRANGEOID => TantivyValue::from_range(
                    pgrx::datum::Range::<pgrx::datum::Timestamp>::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::TSTZRANGEOID => TantivyValue::from_range(
                    pgrx::datum::Range::<pgrx::datum::TimestampWithTimeZone>::from_datum(
                        datum, false,
                    )
                    .ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::JSONBOID => TantivyValue::try_from(
                    jsonb_datum_to_serde_json_value(datum)
                        .ok_or(TantivyValueError::DatumDeref)??,
                ),
                PgBuiltInOids::JSONOID => TantivyValue::try_from(
                    pgrx::datum::JsonString::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                ),
                _ => Err(TantivyValueError::UnsupportedOid(oid.value())),
            },
            PgOid::Custom(custom) if pg_sys::type_is_enum(*custom) => {
                let (_, _, ordinal) = pgrx::enum_helper::lookup_enum_by_oid(
                    pgrx::pg_sys::Oid::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                );
                TantivyValue::try_from(ordinal)
            }

            PgOid::Custom(custom) if type_is_tokenizer(*custom) => TantivyValue::try_from(
                String::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?,
            ),

            PgOid::Custom(custom) if is_ltree_oid(*custom) => {
                // ltree is an extension type - we need to use PostgreSQL's output function
                // to convert it to its text representation, then store as a Tantivy Facet
                let mut typoutput: pg_sys::Oid = pg_sys::InvalidOid;
                let mut is_varlena: bool = false;
                pg_sys::getTypeOutputInfo(*custom, &mut typoutput, &mut is_varlena);
                let cstring_ptr = pg_sys::OidOutputFunctionCall(typoutput, datum);
                let cstr = std::ffi::CStr::from_ptr(cstring_ptr);
                // Copy the text before freeing the palloc'd CString to avoid a memory leak
                // on bulk index builds iterating over many rows.
                let text = cstr
                    .to_str()
                    .map_err(|_| TantivyValueError::DatumDeref)?
                    .to_owned();
                pg_sys::pfree(cstring_ptr.cast());
                // Convert ltree dot-separated path to Tantivy Facet
                // e.g. "Top.Science.Astronomy" -> Facet with path ["Top", "Science", "Astronomy"]
                let path_components: Vec<&str> = text.split('.').collect();
                let facet = Facet::from_path(path_components);
                Ok(TantivyValue(PdbOwnedValue::Facet(facet)))
            }

            PgOid::Custom(custom) => {
                if is_citext_oid(*custom) {
                    return TantivyValue::try_from(
                        String::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?,
                    );
                }
                Err(TantivyValueError::UnsupportedOid(oid.value()))
            }

            _ => Err(TantivyValueError::InvalidOid),
        }
    }

    pub unsafe fn try_from_anyelement(
        any_element: pgrx::AnyElement,
    ) -> Result<Self, TantivyValueError> {
        Self::try_from_datum(any_element.datum(), PgOid::from_untagged(any_element.oid()))
    }
}

impl fmt::Display for TantivyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            PdbOwnedValue::Str(string) => write!(f, "{}", string.clone()),
            PdbOwnedValue::U64(u64) => write!(f, "{u64}"),
            PdbOwnedValue::I64(i64) => write!(f, "{i64}"),
            PdbOwnedValue::F64(f64) => write!(f, "{f64}"),
            PdbOwnedValue::Bool(bool) => write!(f, "{bool}"),
            PdbOwnedValue::Bytes(bytes) => {
                write!(
                    f,
                    "{}",
                    String::from_utf8(bytes.clone()).expect("bytes should be valid utf-8")
                )
            }
            PdbOwnedValue::IpAddr(addr) => write!(f, "{addr}"),
            PdbOwnedValue::Facet(facet) => {
                write!(f, "{}", facet.to_path().join("."))
            }
            PdbOwnedValue::Object(_) => write!(f, "json object"),
            PdbOwnedValue::Null => write!(f, "<null>"),
            PdbOwnedValue::Date(date) => write!(f, "{date}"),
            _ => panic!("tantivy owned value not supported"),
        }
    }
}

impl Hash for TantivyValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self.0 {
            PdbOwnedValue::Str(string) => string.hash(state),
            PdbOwnedValue::U64(u64) => u64.hash(state),
            PdbOwnedValue::I64(i64) => i64.hash(state),
            PdbOwnedValue::F64(f64) => OrderedFloat(*f64).hash(state),
            PdbOwnedValue::Bool(bool) => bool.hash(state),
            PdbOwnedValue::Bytes(bytes) => bytes.hash(state),
            PdbOwnedValue::Facet(facet) => facet.encoded_str().hash(state),
            PdbOwnedValue::Null => 0_u8.hash(state),
            PdbOwnedValue::Date(date) => date.hash(state),
            _ => panic!("tantivy owned value not supported"),
        }
    }
}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for TantivyValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (&self.0, &other.0) {
            (PdbOwnedValue::Str(string), PdbOwnedValue::Str(other_string)) => {
                string.partial_cmp(other_string)
            }
            (PdbOwnedValue::U64(u64), PdbOwnedValue::U64(other_u64)) => u64.partial_cmp(other_u64),
            (PdbOwnedValue::I64(i64), PdbOwnedValue::I64(other_i64)) => i64.partial_cmp(other_i64),
            (PdbOwnedValue::F64(f64), PdbOwnedValue::F64(other_f64)) => f64.partial_cmp(other_f64),
            (PdbOwnedValue::Bool(bool), PdbOwnedValue::Bool(other_bool)) => {
                bool.partial_cmp(other_bool)
            }
            (PdbOwnedValue::Facet(facet), PdbOwnedValue::Facet(other_facet)) => {
                facet.partial_cmp(other_facet)
            }
            (PdbOwnedValue::Date(date), PdbOwnedValue::Date(other_date)) => {
                date.partial_cmp(other_date)
            }
            _ => None,
        }
    }
}

impl Serialize for TantivyValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser = serializer.serialize_struct("TantivyValue", 1)?;
        ser.serialize_field("val", &self.0)?;
        ser.end()
    }
}

impl<'de> Deserialize<'de> for TantivyValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner_val = PdbOwnedValue::deserialize(deserializer)?;

        Ok(TantivyValue(inner_val))
    }
}

impl TryFrom<Vec<u8>> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::Bytes(val)))
    }
}

impl TryFrom<TantivyValue> for Vec<u8> {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let PdbOwnedValue::Bytes(val) = value.0 {
            Ok(val)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "Vec<u8>".to_string(),
            ))
        }
    }
}

impl TryFrom<String> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: String) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::Str(val)))
    }
}

impl TryFrom<TantivyValue> for String {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        Ok(if let PdbOwnedValue::Str(val) = value.0 {
            val
        } else {
            // TODO(mdashti): make sure the string conversion for all values is aligned with the
            // postgres logic, especially for JSON types (i.e., string, boolean, number, object, array).
            // This is specially used for the `->>` JSON operator, as it returns a string.
            value.to_string()
        })
    }
}

impl TryFrom<i8> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: i8) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::I64(val as i64)))
    }
}

impl TryFrom<TantivyValue> for i8 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let PdbOwnedValue::I64(val) = value.0 {
            Ok(val as i8)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "i8".to_string(),
            ))
        }
    }
}

impl TryFrom<i16> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: i16) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::I64(val as i64)))
    }
}

impl TryFrom<TantivyValue> for i16 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        match value.0 {
            PdbOwnedValue::U64(val) => Ok(val as i16),
            PdbOwnedValue::I64(val) => Ok(val as i16),
            PdbOwnedValue::F64(val) => Ok(val as i16),
            _ => Err(TantivyValueError::UnsupportedIntoConversion(
                "i16".to_string(),
            )),
        }
    }
}

impl TryFrom<i32> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: i32) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::I64(val as i64)))
    }
}

impl TryFrom<TantivyValue> for i32 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        match value.0 {
            PdbOwnedValue::U64(val) => Ok(val as i32),
            PdbOwnedValue::I64(val) => Ok(val as i32),
            PdbOwnedValue::F64(val) => Ok(val as i32),
            _ => Err(TantivyValueError::UnsupportedIntoConversion(
                "i32".to_string(),
            )),
        }
    }
}

impl TryFrom<i64> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: i64) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::I64(val)))
    }
}

impl TryFrom<TantivyValue> for i64 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        match value.0 {
            PdbOwnedValue::U64(val) => Ok(val as i64),
            PdbOwnedValue::I64(val) => Ok(val),
            PdbOwnedValue::F64(val) => Ok(val as i64),
            _ => Err(TantivyValueError::UnsupportedIntoConversion(
                "i64".to_string(),
            )),
        }
    }
}

impl TryFrom<f32> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: f32) -> Result<Self, Self::Error> {
        // Casting f32 to f64 causes some precision errors when Tantivy writes the document.
        //     To avoid this, we string format the f32 and then read it as f64.
        let f32_string = format!("{val}");
        let val_as_f64 = f64::from_str(&f32_string)?;

        Ok(TantivyValue(PdbOwnedValue::F64(val_as_f64)))
    }
}

impl TryFrom<TantivyValue> for f32 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let PdbOwnedValue::F64(val) = value.0 {
            // Casting f32 to f64 causes some precision errors when Tantivy writes the document.
            //     To avoid this, we string format the stored f64 and then read it as f32.
            let f64_string = format!("{val}");
            let val_as_f32 = f32::from_str(&f64_string)?;

            Ok(val_as_f32)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "f32".to_string(),
            ))
        }
    }
}

impl TryFrom<f64> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: f64) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::F64(val)))
    }
}

impl TryFrom<TantivyValue> for f64 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        match value.0 {
            PdbOwnedValue::F64(val) => Ok(val),
            PdbOwnedValue::I64(val) => Ok(val as f64),
            PdbOwnedValue::U64(val) => Ok(val as f64),
            _ => Err(TantivyValueError::UnsupportedIntoConversion(
                "f64".to_string(),
            )),
        }
    }
}

impl TryFrom<u32> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: u32) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::U64(val as u64)))
    }
}

impl TryFrom<TantivyValue> for u32 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        match value.0 {
            PdbOwnedValue::U64(val) => Ok(val as u32),
            PdbOwnedValue::I64(val) => Ok(val as u32),
            PdbOwnedValue::F64(val) => Ok(val as u32),
            _ => Err(TantivyValueError::UnsupportedIntoConversion(
                "u32".to_string(),
            )),
        }
    }
}

impl TryFrom<u64> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: u64) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::U64(val)))
    }
}

impl TryFrom<TantivyValue> for u64 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        match value.0 {
            PdbOwnedValue::U64(val) => Ok(val),
            PdbOwnedValue::I64(val) => Ok(val as u64),
            PdbOwnedValue::F64(val) => Ok(val as u64),
            _ => Err(TantivyValueError::UnsupportedIntoConversion(
                "u64".to_string(),
            )),
        }
    }
}

/// Convert NUMERIC to string representation to preserve precision.
/// The string will be converted to the appropriate type (I64, Bytes, or F64)
/// later when schema context is available.
///
/// For document indexing with known field types, use
/// try_from_numeric_i64/try_from_numeric_bytes instead.
impl TryFrom<pgrx::AnyNumeric> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::AnyNumeric) -> Result<Self, Self::Error> {
        // Store as string to preserve full precision until we know the field type
        Ok(TantivyValue(PdbOwnedValue::Str(
            val.normalize().to_string(),
        )))
    }
}

impl TantivyValue {
    /// Convert a PostgreSQL NUMERIC datum to a TantivyValue with I64 fixed-point storage.
    /// Used for NUMERIC(p,s) where p <= 18.
    ///
    /// The value is scaled by 10^scale to convert to integer representation.
    /// For example, NUMERIC(10,2) value 123.45 with scale=2 becomes I64(12345).
    ///
    /// Delegates to the centralized `scale_i64` in the numeric module.
    pub unsafe fn try_from_numeric_i64(
        datum: Datum,
        scale: i16,
    ) -> Result<Self, TantivyValueError> {
        use crate::query::numeric::scale_i64;

        let numeric =
            pgrx::AnyNumeric::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?;

        let numeric_str = numeric.normalize().to_string();

        let scaled = scale_i64(&numeric_str, scale).map_err(|e| {
            TantivyValueError::NumericConversion(format!(
                "Failed to convert NUMERIC '{}' to I64 with scale {}: {}",
                numeric_str, scale, e
            ))
        })?;

        Ok(TantivyValue(PdbOwnedValue::I64(scaled)))
    }

    /// Convert a PostgreSQL NUMERIC datum to a TantivyValue with raw bytes storage.
    /// Used for NUMERIC with precision > 18 or unlimited precision.
    ///
    /// The byte encoding is lexicographically sortable, supporting range queries.
    pub unsafe fn try_from_numeric_bytes(datum: Datum) -> Result<Self, TantivyValueError> {
        use decimal_bytes::Decimal;
        use std::str::FromStr;

        let numeric =
            pgrx::AnyNumeric::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?;

        // Convert AnyNumeric to string, then to lexicographically sortable bytes
        let numeric_str = numeric.normalize().to_string();

        let decimal = Decimal::from_str(&numeric_str).map_err(|e| {
            TantivyValueError::NumericConversion(format!(
                "Failed to convert NUMERIC '{}' to bytes: {:?}",
                numeric_str, e
            ))
        })?;

        // Store as raw bytes for Bytes field storage
        Ok(TantivyValue(PdbOwnedValue::Bytes(decimal.into_bytes())))
    }

    /// Convert a PostgreSQL NUMERIC[] array to TantivyValues with I64 fixed-point storage.
    /// Used for NUMERIC arrays with precision <= 18.
    ///
    /// Delegates to the centralized `scale_i64` in the numeric module.
    pub unsafe fn try_from_numeric_array_i64(
        datum: Datum,
        scale: i16,
    ) -> Result<Vec<Self>, TantivyValueError> {
        use crate::query::numeric::scale_i64;

        let array: pgrx::Array<Datum> =
            pgrx::Array::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?;

        array
            .iter()
            .flatten()
            .map(|element_datum| {
                let numeric = pgrx::AnyNumeric::from_datum(element_datum, false)
                    .ok_or(TantivyValueError::DatumDeref)?;

                let numeric_str = numeric.normalize().to_string();
                let scaled = scale_i64(&numeric_str, scale).map_err(|e| {
                    TantivyValueError::NumericConversion(format!(
                        "Failed to convert NUMERIC array element '{}' with scale {}: {}",
                        numeric_str, scale, e
                    ))
                })?;

                Ok(TantivyValue(PdbOwnedValue::I64(scaled)))
            })
            .collect()
    }

    /// Convert a PostgreSQL NUMERIC[] array to TantivyValues with raw bytes storage.
    /// Used for NUMERIC arrays with precision > 18 or unlimited precision.
    pub unsafe fn try_from_numeric_array_bytes(
        datum: Datum,
    ) -> Result<Vec<Self>, TantivyValueError> {
        use decimal_bytes::Decimal;
        use std::str::FromStr;

        let array: pgrx::Array<Datum> =
            pgrx::Array::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?;

        array
            .iter()
            .flatten()
            .map(|element_datum| {
                let numeric = pgrx::AnyNumeric::from_datum(element_datum, false)
                    .ok_or(TantivyValueError::DatumDeref)?;

                let numeric_str = numeric.normalize().to_string();
                let decimal = Decimal::from_str(&numeric_str).map_err(|e| {
                    TantivyValueError::NumericConversion(format!(
                        "Failed to convert NUMERIC array element '{}' to bytes: {:?}",
                        numeric_str, e
                    ))
                })?;

                // Store as raw bytes for Bytes field storage
                Ok(TantivyValue(PdbOwnedValue::Bytes(decimal.into_bytes())))
            })
            .collect()
    }

    /// Convert a PostgreSQL NUMERIC datum to a TantivyValue with F64 storage.
    /// Used only for legacy indexes (pre-v0.22.0) whose tantivy schema stored
    /// NUMERIC fields as F64. Subject to f64 precision loss for values that
    /// don't round-trip through a double.
    pub unsafe fn try_from_numeric_f64(datum: Datum) -> Result<Self, TantivyValueError> {
        let numeric =
            pgrx::AnyNumeric::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?;
        Ok(TantivyValue(PdbOwnedValue::F64(numeric.try_into()?)))
    }

    /// Convert a PostgreSQL NUMERIC[] array to TantivyValues with F64 storage.
    /// Legacy-only counterpart to `try_from_numeric_f64`.
    pub unsafe fn try_from_numeric_array_f64(datum: Datum) -> Result<Vec<Self>, TantivyValueError> {
        let array: pgrx::Array<Datum> =
            pgrx::Array::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?;

        array
            .iter()
            .flatten()
            .map(|element_datum| {
                let numeric = pgrx::AnyNumeric::from_datum(element_datum, false)
                    .ok_or(TantivyValueError::DatumDeref)?;
                Ok(TantivyValue(PdbOwnedValue::F64(numeric.try_into()?)))
            })
            .collect()
    }
}

impl TryFrom<TantivyValue> for pgrx::AnyNumeric {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        match value.0 {
            PdbOwnedValue::U64(val) => Ok(pgrx::AnyNumeric::from(val)),
            PdbOwnedValue::I64(val) => Ok(pgrx::AnyNumeric::from(val)),
            PdbOwnedValue::F64(val) => Ok(val.try_into()?),
            _ => Err(TantivyValueError::UnsupportedIntoConversion(
                "numeric".to_string(),
            )),
        }
    }
}

impl TryFrom<bool> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: bool) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::Bool(val)))
    }
}

impl TryFrom<TantivyValue> for bool {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        match value.0 {
            PdbOwnedValue::Bool(val) => Ok(val),
            PdbOwnedValue::U64(val) => Ok(val != 0),
            PdbOwnedValue::I64(val) => Ok(val != 0),
            PdbOwnedValue::F64(val) => Ok(val != 0.0),
            _ => Err(TantivyValueError::UnsupportedIntoConversion(
                "bool".to_string(),
            )),
        }
    }
}

impl TryFrom<pgrx::datum::JsonString> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::datum::JsonString) -> Result<Self, Self::Error> {
        let json_value: Value = serde_json::from_slice(&serde_json::to_vec(&val.0)?)?;
        Ok(TantivyValue(PdbOwnedValue::try_from(json_value)?))
    }
}

/// Helper function to convert Tantivy OwnedValue to serde_json::Value
/// Handles both Object types and String types (which may be JSON strings)
fn tantivy_to_json_value(
    tv: PdbOwnedValue,
    is_jsonb: bool,
) -> Result<serde_json::Value, TantivyValueError> {
    match tv {
        PdbOwnedValue::Object(val) => Ok(serde_json::to_value(val)?),
        PdbOwnedValue::Str(s) => {
            // When grouping by JSON fields, the values come back as strings
            // Try to parse as JSON first, fall back to treating as plain string
            let json_value: serde_json::Value =
                serde_json::from_str(&s).unwrap_or_else(|_| serde_json::Value::String(s));
            Ok(json_value)
        }
        _ => Err(TantivyValueError::UnsupportedIntoConversion(
            if is_jsonb { "jsonb" } else { "json" }.to_string(),
        )),
    }
}

impl TryFrom<TantivyValue> for pgrx::datum::JsonString {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        let json_value = tantivy_to_json_value(value.0, false)?;
        Ok(pgrx::datum::JsonString(serde_json::to_string(&json_value)?))
    }
}

impl TryFrom<pgrx::JsonB> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::JsonB) -> Result<Self, Self::Error> {
        Self::try_from(val.0)
    }
}

impl TryFrom<TantivyValue> for pgrx::JsonB {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        let json_value = tantivy_to_json_value(value.0, true)?;
        Ok(pgrx::datum::JsonB(json_value))
    }
}

impl TryFrom<serde_json::Value> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: serde_json::Value) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::try_from(val)?))
    }
}

impl TryFrom<TantivyValue> for serde_json::Value {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let PdbOwnedValue::Object(val) = value.0 {
            Ok(serde_json::to_value(val)?)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "jsonb".to_string(),
            ))
        }
    }
}

impl TryFrom<pgrx::datum::Date> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::datum::Date) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::Date(PostgresDateTime::from(
            val,
        ))))
    }
}

impl TryFrom<TantivyValue> for pgrx::datum::Date {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let PdbOwnedValue::Date(date) = value.0 {
            Ok(date.into())
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "date".to_string(),
            ))
        }
    }
}

impl TryFrom<pgrx::datum::Time> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::datum::Time) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::Date(PostgresDateTime::from(
            val,
        ))))
    }
}

impl TryFrom<TantivyValue> for pgrx::datum::Time {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let PdbOwnedValue::Date(date) = value.0 {
            Ok(date.into())
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "time".to_string(),
            ))
        }
    }
}

impl TryFrom<pgrx::datum::Timestamp> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::datum::Timestamp) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::Date(PostgresDateTime::from(
            val,
        ))))
    }
}

impl TryFrom<TantivyValue> for pgrx::datum::Timestamp {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let PdbOwnedValue::Date(date) = value.0 {
            Ok(date.into())
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "timestamp".to_string(),
            ))
        }
    }
}

impl TryFrom<pgrx::datum::TimeWithTimeZone> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::datum::TimeWithTimeZone) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::Date(PostgresDateTime::from(
            val,
        ))))
    }
}

impl TryFrom<TantivyValue> for pgrx::datum::TimeWithTimeZone {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let PdbOwnedValue::Date(date) = value.0 {
            Ok(date.into())
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "timetz".to_string(),
            ))
        }
    }
}

impl TryFrom<pgrx::datum::TimestampWithTimeZone> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::datum::TimestampWithTimeZone) -> Result<Self, Self::Error> {
        Ok(TantivyValue(PdbOwnedValue::Date(PostgresDateTime::from(
            val,
        ))))
    }
}

impl TryFrom<TantivyValue> for pgrx::datum::TimestampWithTimeZone {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let PdbOwnedValue::Date(date) = value.0 {
            Ok(date.into())
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "timestamptz".to_string(),
            ))
        }
    }
}

impl TryFrom<pgrx::Uuid> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Uuid) -> Result<Self, Self::Error> {
        let uuid = uuid::Uuid::from_slice(val.as_bytes())?;
        Ok(TantivyValue(PdbOwnedValue::Str(uuid.to_string())))
    }
}

impl TryFrom<TantivyValue> for pgrx::Uuid {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let PdbOwnedValue::Str(val) = value.0 {
            let uuid = uuid::Uuid::parse_str(&val)?;
            Ok(pgrx::Uuid::from_slice(uuid.as_bytes())
                .map_err(TantivyValueError::UuidConversionError)?)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "uuid".to_string(),
            ))
        }
    }
}

impl TryFrom<AnyEnum> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: AnyEnum) -> Result<Self, Self::Error> {
        match val.ordinal() {
            Some(ordinal) => Ok(TantivyValue(PdbOwnedValue::F64(ordinal.into()))),
            None => Ok(TantivyValue(PdbOwnedValue::Null)),
        }
    }
}

impl TryFrom<pgrx::AnyArray> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(_val: pgrx::AnyArray) -> Result<Self, Self::Error> {
        Err(TantivyValueError::UnsupportedFromConversion(
            "array".to_string(),
        ))
    }
}

impl TryFrom<pgrx::pg_sys::BOX> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(_val: pgrx::pg_sys::BOX) -> Result<Self, Self::Error> {
        Err(TantivyValueError::UnsupportedFromConversion(
            "box".to_string(),
        ))
    }
}

impl TryFrom<pgrx::pg_sys::Point> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(_val: pgrx::pg_sys::Point) -> Result<Self, Self::Error> {
        Err(TantivyValueError::UnsupportedFromConversion(
            "point".to_string(),
        ))
    }
}

impl TryFrom<pgrx::pg_sys::ItemPointerData> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(_val: pgrx::pg_sys::ItemPointerData) -> Result<Self, Self::Error> {
        Err(TantivyValueError::UnsupportedFromConversion(
            "tid".to_string(),
        ))
    }
}

impl TryFrom<pgrx::Inet> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Inet) -> Result<Self, Self::Error> {
        match val.parse::<IpAddr>() {
            Ok(addr) => Ok(TantivyValue(PdbOwnedValue::IpAddr(addr.into_ipv6_addr()))),
            Err(err) => Err(TantivyValueError::InetError(err)),
        }
    }
}

impl TryFrom<TantivyValue> for pgrx::Inet {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let PdbOwnedValue::IpAddr(val) = value.0 {
            Ok(val.to_string().into())
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "inet".to_string(),
            ))
        }
    }
}

/// A wrapper around a `pg_sys::Const` node
pub struct ConstNode(*mut pg_sys::Const);

impl ConstNode {
    pub fn try_from(node: *mut pg_sys::Node) -> Option<Self> {
        let const_node = unsafe { nodecast!(Const, T_Const, node)? };
        Some(Self(const_node))
    }
}

impl TryFrom<ConstNode> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(value: ConstNode) -> Result<Self, Self::Error> {
        if unsafe { (*value.0).constisnull } {
            return Ok(TantivyValue(PdbOwnedValue::Null));
        }

        unsafe {
            TantivyValue::try_from_datum((*value.0).constvalue, PgOid::from((*value.0).consttype))
        }
    }
}

#[derive(Error, Debug)]
pub enum TantivyValueError {
    #[error("date {0} is out of range: {1}")]
    DateOutOfRange(pgrx::datum::Date, String),

    #[error(transparent)]
    PgrxNumericError(#[from] pgrx::datum::numeric_support::error::Error),

    #[error("NUMERIC conversion error: {0}")]
    NumericConversion(String),

    #[error(transparent)]
    UuidError(#[from] uuid::Error),

    #[error("Could not generate datetime datum")]
    DateTimeConversionError(#[from] DateTimeConversionError),

    #[error("Failed UUID conversion: {0}")]
    UuidConversionError(String),

    #[error(transparent)]
    InetError(#[from] AddrParseError),

    #[error("Could not dereference postgres datum")]
    DatumDeref,

    #[error("Could not deserialize json object")]
    JsonDeserializeError,

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    ParseFloatError(#[from] ParseFloatError),

    #[error("Cannot convert oid of InvalidOid to TantivyValue")]
    InvalidOid,

    #[error("Type {0:?} is not yet supported")]
    UnsupportedOid(Oid),

    #[error("Arrays of type {0:?} are not yet supported")]
    UnsupportedArrayOid(Oid),

    #[error("Cannot convert builtin json oid of {0:?} to TantivyValue")]
    UnsupportedJsonOid(Oid),

    #[error("Cannot convert type {0} to a TantivyValue")]
    UnsupportedFromConversion(String),

    #[error("Cannot convert TantivyValue to type {0}")]
    UnsupportedIntoConversion(String),

    #[error("UTF8 conversion error: {0}")]
    Utf8ConversionError(#[from] std::str::Utf8Error),
}
impl From<JsonbConversionError> for TantivyValueError {
    fn from(value: JsonbConversionError) -> Self {
        match value {
            JsonbConversionError::Utf8(v) => Self::Utf8ConversionError(v),
            JsonbConversionError::Serde(v) => Self::SerdeJsonError(v),
        }
    }
}

/// Check if the given OID is a date/time type that requires special conversion
pub fn is_datetime_type(typoid: pg_sys::Oid) -> bool {
    is_pgoid_datetime_type(PgOid::from_untagged(typoid))
}

pub fn is_pgoid_datetime_type(pgoid: PgOid) -> bool {
    match pgoid {
        PgOid::Invalid => false,
        PgOid::Custom(_) => false,
        PgOid::BuiltIn(oid) => matches!(
            oid,
            pgrx::pg_sys::BuiltinOid::DATEOID
                | pgrx::pg_sys::BuiltinOid::TIMESTAMPOID
                | pgrx::pg_sys::BuiltinOid::TIMESTAMPTZOID
                | pgrx::pg_sys::BuiltinOid::TIMEOID
                | pgrx::pg_sys::BuiltinOid::TIMETZOID
        ),
    }
}

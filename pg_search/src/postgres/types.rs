use crate::postgres::datetime::{
    pgrx_date_to_tantivy_value, pgrx_time_to_tantivy_value, pgrx_timestamp_to_tantivy_value,
    pgrx_timestamptz_to_tantivy_value, pgrx_timetz_to_tantivy_value, tantivy_value_to_pgrx_date,
    tantivy_value_to_pgrx_time, tantivy_value_to_pgrx_timestamp, tantivy_value_to_pgrx_timestamptz,
    tantivy_value_to_pgrx_timetz, DatetimeConversionError,
};
use ordered_float::OrderedFloat;
use pgrx::pg_sys::Datum;
use pgrx::pg_sys::Oid;
use pgrx::IntoDatum;
use pgrx::PostgresType;
use pgrx::{FromDatum, PgBuiltInOids, PgOid};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde::{Deserialize, Deserializer};
use serde_json::Map;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, PostgresType)]
pub struct TantivyValue(pub tantivy::schema::OwnedValue);

impl TantivyValue {
    pub fn tantivy_schema_value(&self) -> tantivy::schema::OwnedValue {
        self.0.clone()
    }

    pub unsafe fn try_into_datum(self, oid: PgOid) -> Result<Datum, TantivyValueError> {
        Ok(match &oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::BOOLOID => bool::try_from(self).unwrap().into_datum().unwrap(),
                PgBuiltInOids::INT2OID => i16::try_from(self).unwrap().into_datum().unwrap(),
                PgBuiltInOids::INT4OID => i32::try_from(self).unwrap().into_datum().unwrap(),
                PgBuiltInOids::INT8OID => i64::try_from(self).unwrap().into_datum().unwrap(),
                PgBuiltInOids::OIDOID => u32::try_from(self).unwrap().into_datum().unwrap(),
                PgBuiltInOids::FLOAT4OID => f32::try_from(self).unwrap().into_datum().unwrap(),
                PgBuiltInOids::FLOAT8OID => f64::try_from(self).unwrap().into_datum().unwrap(),
                PgBuiltInOids::NUMERICOID => pgrx::AnyNumeric::try_from(self)
                    .unwrap()
                    .into_datum()
                    .unwrap(),
                PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                    String::try_from(self).unwrap().into_datum().unwrap()
                }
                PgBuiltInOids::JSONOID => pgrx::JsonString::try_from(self)
                    .unwrap()
                    .into_datum()
                    .unwrap(),
                PgBuiltInOids::JSONBOID => {
                    pgrx::JsonB::try_from(self).unwrap().into_datum().unwrap()
                }
                PgBuiltInOids::DATEOID => pgrx::datum::Date::try_from(self)
                    .unwrap()
                    .into_datum()
                    .unwrap(),
                PgBuiltInOids::TIMESTAMPOID => pgrx::datum::Timestamp::try_from(self)
                    .unwrap()
                    .into_datum()
                    .unwrap(),
                PgBuiltInOids::TIMESTAMPTZOID => pgrx::datum::TimestampWithTimeZone::try_from(self)
                    .unwrap()
                    .into_datum()
                    .unwrap(),
                PgBuiltInOids::TIMEOID => pgrx::datum::Time::try_from(self)
                    .unwrap()
                    .into_datum()
                    .unwrap(),
                PgBuiltInOids::TIMETZOID => pgrx::datum::TimeWithTimeZone::try_from(self)
                    .unwrap()
                    .into_datum()
                    .unwrap(),
                PgBuiltInOids::UUIDOID => pgrx::datum::Uuid::try_from(self)
                    .unwrap()
                    .into_datum()
                    .unwrap(),
                _ => return Err(TantivyValueError::UnsupportedOid(oid.value())),
            },
            _ => return Err(TantivyValueError::InvalidOid),
        })
    }

    pub unsafe fn try_from_datum_array(
        datum: Datum,
        oid: PgOid,
    ) -> Result<Vec<Self>, TantivyValueError> {
        match &oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                    let array: pgrx::Array<Datum> = pgrx::Array::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?;
                    array
                        .iter()
                        .flatten()
                        .map(|element_datum| {
                            TantivyValue::try_from(
                                String::from_datum(element_datum, false)
                                    .ok_or(TantivyValueError::DatumDeref)?,
                            )
                        })
                        .collect()
                }
                _ => Err(TantivyValueError::UnsupportedArrayOid(oid.value())),
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
                PgBuiltInOids::JSONOID => TantivyValue::try_from(
                    pgrx::JsonString::from_datum(datum, false)
                        .ok_or(TantivyValueError::DatumDeref)?,
                ),
                PgBuiltInOids::JSONBOID => TantivyValue::try_from(
                    pgrx::JsonB::from_datum(datum, false).ok_or(TantivyValueError::DatumDeref)?,
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
                _ => Err(TantivyValueError::UnsupportedOid(oid.value())),
            },
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
        match self.tantivy_schema_value() {
            tantivy::schema::OwnedValue::Str(string) => write!(f, "{}", string.clone()),
            tantivy::schema::OwnedValue::U64(u64) => write!(f, "{}", u64),
            tantivy::schema::OwnedValue::I64(i64) => write!(f, "{}", i64),
            tantivy::schema::OwnedValue::F64(f64) => write!(f, "{}", f64),
            tantivy::schema::OwnedValue::Bool(bool) => write!(f, "{}", bool),
            tantivy::schema::OwnedValue::Date(datetime) => {
                write!(f, "{}", datetime.into_primitive())
            }
            tantivy::schema::OwnedValue::Bytes(bytes) => {
                write!(f, "{}", String::from_utf8(bytes.clone()).unwrap())
            }
            tantivy::schema::OwnedValue::Object(_) => write!(f, "json object"),
            _ => panic!("tantivy owned value not supported"),
        }
    }
}

impl Hash for TantivyValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.tantivy_schema_value() {
            tantivy::schema::OwnedValue::Str(string) => string.hash(state),
            tantivy::schema::OwnedValue::U64(u64) => u64.hash(state),
            tantivy::schema::OwnedValue::I64(i64) => i64.hash(state),
            tantivy::schema::OwnedValue::F64(f64) => OrderedFloat(f64).hash(state),
            tantivy::schema::OwnedValue::Bool(bool) => bool.hash(state),
            tantivy::schema::OwnedValue::Date(datetime) => datetime.hash(state),
            tantivy::schema::OwnedValue::Bytes(bytes) => bytes.hash(state),
            _ => panic!("tantivy owned value not supported"),
        }
    }
}

impl PartialOrd for TantivyValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.tantivy_schema_value() {
            tantivy::schema::OwnedValue::Str(string) => {
                if let tantivy::schema::OwnedValue::Str(other_string) = other.tantivy_schema_value() {
                    string.partial_cmp(&other_string)
                } else {
                    None
                }
            }
            tantivy::schema::OwnedValue::U64(u64) => {
                if let tantivy::schema::OwnedValue::U64(other_u64) = other.tantivy_schema_value() {
                    u64.partial_cmp(&other_u64)
                } else {
                    None
                }
            }
            tantivy::schema::OwnedValue::I64(i64) => {
                if let tantivy::schema::OwnedValue::I64(other_i64) = other.tantivy_schema_value() {
                    i64.partial_cmp(&other_i64)
                } else {
                    None
                }
            }
            tantivy::schema::OwnedValue::F64(f64) => {
                if let tantivy::schema::OwnedValue::F64(other_f64) = other.tantivy_schema_value() {
                    f64.partial_cmp(&other_f64)
                } else {
                    None
                }
            }
            tantivy::schema::OwnedValue::Bool(bool) => {
                if let tantivy::schema::OwnedValue::Bool(other_bool) = other.tantivy_schema_value() {
                    bool.partial_cmp(&other_bool)
                } else {
                    None
                }
            }
            tantivy::schema::OwnedValue::Date(datetime) => {
                if let tantivy::schema::OwnedValue::Date(other_datetime) = other.tantivy_schema_value() {
                    datetime.partial_cmp(&other_datetime)
                } else {
                    None
                }
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
        let inner_val = tantivy::schema::OwnedValue::deserialize(deserializer)?;

        Ok(TantivyValue(inner_val))
    }
}

impl TryFrom<Vec<u8>> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::Bytes(val)))
    }
}

impl TryFrom<TantivyValue> for Vec<u8> {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::Bytes(val) = value.0 {
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
        Ok(TantivyValue(tantivy::schema::OwnedValue::Str(val)))
    }
}

impl TryFrom<TantivyValue> for String {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::Str(val) = value.0 {
            Ok(val)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "String".to_string(),
            ))
        }
    }
}

impl TryFrom<i8> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: i8) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::I64(val as i64)))
    }
}

impl TryFrom<TantivyValue> for i8 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::I64(val) = value.0 {
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
        Ok(TantivyValue(tantivy::schema::OwnedValue::I64(val as i64)))
    }
}

impl TryFrom<TantivyValue> for i16 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::I64(val) = value.0 {
            Ok(val as i16)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "i16".to_string(),
            ))
        }
    }
}

impl TryFrom<i32> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: i32) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::I64(val as i64)))
    }
}

impl TryFrom<TantivyValue> for i32 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::I64(val) = value.0 {
            Ok(val as i32)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "i32".to_string(),
            ))
        }
    }
}

impl TryFrom<i64> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: i64) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::I64(val)))
    }
}

impl TryFrom<TantivyValue> for i64 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::I64(val) = value.0 {
            Ok(val)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "i64".to_string(),
            ))
        }
    }
}

impl TryFrom<f32> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: f32) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::F64(val as f64)))
    }
}

impl TryFrom<TantivyValue> for f32 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::F64(val) = value.0 {
            Ok(val as f32)
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
        Ok(TantivyValue(tantivy::schema::OwnedValue::F64(val)))
    }
}

impl TryFrom<TantivyValue> for f64 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::F64(val) = value.0 {
            Ok(val)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "f64".to_string(),
            ))
        }
    }
}

impl TryFrom<u32> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: u32) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::U64(val as u64)))
    }
}

impl TryFrom<TantivyValue> for u32 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::U64(val) = value.0 {
            Ok(val as u32)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "u32".to_string(),
            ))
        }
    }
}

impl TryFrom<u64> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: u64) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::U64(val)))
    }
}

impl TryFrom<TantivyValue> for u64 {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::U64(val) = value.0 {
            Ok(val)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "u64".to_string(),
            ))
        }
    }
}

impl TryFrom<pgrx::AnyNumeric> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::AnyNumeric) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::F64(val.try_into()?)))
    }
}

impl TryFrom<TantivyValue> for pgrx::AnyNumeric {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::F64(val) = value.0 {
            Ok(val.try_into()?)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "numeric".to_string(),
            ))
        }
    }
}

impl TryFrom<bool> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: bool) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::Bool(val)))
    }
}

impl TryFrom<TantivyValue> for bool {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::Bool(val) = value.0 {
            Ok(val)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "bool".to_string(),
            ))
        }
    }
}

impl TryFrom<pgrx::JsonString> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::JsonString) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::Object(
            serde_json::from_str::<Map<String, serde_json::Value>>(&val.0)?,
        )))
    }
}

impl TryFrom<TantivyValue> for pgrx::JsonString {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::Object(val) = value.0 {
            Ok(pgrx::JsonString(serde_json::Value::from(val).to_string()))
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "json".to_string(),
            ))
        }
    }
}

impl TryFrom<pgrx::JsonB> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::JsonB) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::Object(
            serde_json::from_slice::<Map<String, serde_json::Value>>(&serde_json::to_vec(&val.0)?)?,
        )))
    }
}

impl TryFrom<TantivyValue> for pgrx::JsonB {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::Object(val) = value.0 {
            Ok(pgrx::JsonB(serde_json::Value::from(val)))
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "jsonb".to_string(),
            ))
        }
    }
}

impl TryFrom<pgrx::Date> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Date) -> Result<Self, Self::Error> {
        Ok(TantivyValue(pgrx_date_to_tantivy_value(val)?))
    }
}

impl TryFrom<TantivyValue> for pgrx::Date {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::Date(val) = value.0 {
            Ok(tantivy_value_to_pgrx_date(val)?)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "date".to_string(),
            ))
        }
    }
}

impl TryFrom<pgrx::Time> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Time) -> Result<Self, Self::Error> {
        Ok(TantivyValue(pgrx_time_to_tantivy_value(val)?))
    }
}

impl TryFrom<TantivyValue> for pgrx::Time {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::Date(val) = value.0 {
            Ok(tantivy_value_to_pgrx_time(val)?)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "time".to_string(),
            ))
        }
    }
}

impl TryFrom<pgrx::Timestamp> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Timestamp) -> Result<Self, Self::Error> {
        Ok(TantivyValue(pgrx_timestamp_to_tantivy_value(val)?))
    }
}

impl TryFrom<TantivyValue> for pgrx::Timestamp {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::Date(val) = value.0 {
            Ok(tantivy_value_to_pgrx_timestamp(val)?)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "timestamp".to_string(),
            ))
        }
    }
}

impl TryFrom<pgrx::TimeWithTimeZone> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::TimeWithTimeZone) -> Result<Self, Self::Error> {
        Ok(TantivyValue(pgrx_timetz_to_tantivy_value(val)?))
    }
}

impl TryFrom<TantivyValue> for pgrx::TimeWithTimeZone {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::Date(val) = value.0 {
            Ok(tantivy_value_to_pgrx_timetz(val)?)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "timetz".to_string(),
            ))
        }
    }
}

impl TryFrom<pgrx::TimestampWithTimeZone> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::TimestampWithTimeZone) -> Result<Self, Self::Error> {
        Ok(TantivyValue(pgrx_timestamptz_to_tantivy_value(val)?))
    }
}

impl TryFrom<TantivyValue> for pgrx::TimestampWithTimeZone {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::Date(val) = value.0 {
            Ok(tantivy_value_to_pgrx_timestamptz(val)?)
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
        Ok(TantivyValue(tantivy::schema::OwnedValue::Str(val.to_string())))
    }
}

impl TryFrom<TantivyValue> for pgrx::Uuid {
    type Error = TantivyValueError;

    fn try_from(value: TantivyValue) -> Result<Self, Self::Error> {
        if let tantivy::schema::OwnedValue::Str(val) = value.0 {
            Ok(pgrx::Uuid::from_slice(val.as_bytes())
                .map_err(|err| TantivyValueError::UuidConversionError(err))?)
        } else {
            Err(TantivyValueError::UnsupportedIntoConversion(
                "uuid".to_string(),
            ))
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

    fn try_from(_val: pgrx::Inet) -> Result<Self, Self::Error> {
        Err(TantivyValueError::UnsupportedFromConversion(
            "inet".to_string(),
        ))
    }
}

impl TryFrom<pgrx::Range<i32>> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(_val: pgrx::Range<i32>) -> Result<Self, Self::Error> {
        Err(TantivyValueError::UnsupportedFromConversion(
            "int4 range".to_string(),
        ))
    }
}

impl TryFrom<pgrx::Range<i64>> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(_val: pgrx::Range<i64>) -> Result<Self, Self::Error> {
        Err(TantivyValueError::UnsupportedFromConversion(
            "int8 range".to_string(),
        ))
    }
}

impl TryFrom<pgrx::Range<pgrx::AnyNumeric>> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(_val: pgrx::Range<pgrx::AnyNumeric>) -> Result<Self, Self::Error> {
        Err(TantivyValueError::UnsupportedFromConversion(
            "nuemric range".to_string(),
        ))
    }
}

impl TryFrom<pgrx::Range<pgrx::Date>> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(_val: pgrx::Range<pgrx::Date>) -> Result<Self, Self::Error> {
        Err(TantivyValueError::UnsupportedFromConversion(
            "date range".to_string(),
        ))
    }
}

impl TryFrom<pgrx::Range<pgrx::Timestamp>> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(_val: pgrx::Range<pgrx::Timestamp>) -> Result<Self, Self::Error> {
        Err(TantivyValueError::UnsupportedFromConversion(
            "timestamp range".to_string(),
        ))
    }
}

impl TryFrom<pgrx::Range<pgrx::TimestampWithTimeZone>> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(_val: pgrx::Range<pgrx::TimestampWithTimeZone>) -> Result<Self, Self::Error> {
        Err(TantivyValueError::UnsupportedFromConversion(
            "timestamp with time zone range".to_string(),
        ))
    }
}

#[derive(Error, Debug)]
pub enum TantivyValueError {
    #[error(transparent)]
    PgrxNumericError(#[from] pgrx::datum::numeric_support::error::Error),

    #[error(transparent)]
    DatetimeConversionError(#[from] DatetimeConversionError),

    #[error("Failed UUID conversion: {0}")]
    UuidConversionError(String),

    #[error("Could not dereference postgres datum")]
    DatumDeref,

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Cannot convert oid of InvalidOid to TantivyValue")]
    InvalidOid,

    #[error("Cannot convert builtin oid of {0} to TantivyValue")]
    UnsupportedOid(Oid),

    #[error("Cannot convert builtin array oid of {0} to TantivyValue")]
    UnsupportedArrayOid(Oid),

    #[error("Cannot convert type {0} to a TantivyValue")]
    UnsupportedFromConversion(String),

    #[error("Cannot convert TantivyValue to type {0}")]
    UnsupportedIntoConversion(String),
}

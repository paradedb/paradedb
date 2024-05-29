// Convert pgrx types into tantivy values
use crate::postgres::datetime::{
    pgrx_date_to_tantivy_value, pgrx_time_to_tantivy_value, pgrx_timestamp_to_tantivy_value,
    pgrx_timestamptz_to_tantivy_value, pgrx_timetz_to_tantivy_value,
};
use ordered_float::OrderedFloat;
use thiserror::Error;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde::{Deserialize, Deserializer};
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Display, Eq, PartialEq)]
pub struct TantivyValue(pub tantivy::schema::OwnedValue);

impl TantivyValue {
    pub fn tantivy_schema_value(&self) -> tantivy::schema::OwnedValue {
        self.0.clone()
    }
}

#[derive(Error, Debug)]
pub enum TantivyValueError {
    #[error("{0} term not supported")]
    TermNotImplemented(String),

    #[error(transparent)]
    PgrxNumericError(#[from] pgrx::datum::numeric_support::error::Error),
}

impl ToString for TantivyValue {
    fn to_string(&self) -> String {
        match self.tantivy_schema_value() {
            tantivy::schema::OwnedValue::Str(string) => string.clone(),
            tantivy::schema::OwnedValue::U64(u64) => format!("{:?}", u64),
            tantivy::schema::OwnedValue::I64(i64) => format!("{:?}", i64),
            tantivy::schema::OwnedValue::F64(f64) => format!("{:?}", f64),
            tantivy::schema::OwnedValue::Bool(bool) => format!("{:?}", bool),
            tantivy::schema::OwnedValue::Date(datetime) => datetime.into_primitive().to_string(),
            tantivy::schema::OwnedValue::Bytes(bytes) => String::from_utf8(bytes.clone()).unwrap(),
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

impl Serialize for TantivyValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // match self.tantivy_schema_value() {
        //     tantivy::schema::OwnedValue::Str(string) => serializer.serialize_str(&string),
        //     tantivy::schema::OwnedValue::U64(u64) => serializer.serialize_u64(u64),
        //     tantivy::schema::OwnedValue::I64(i64) => serializer.serialize_i64(i64),
        //     tantivy::schema::OwnedValue::F64(f64) => serializer.serialize_f64(f64),
        //     tantivy::schema::OwnedValue::Bool(bool) => serializer.serialize_bool(bool),
        //     tantivy::schema::OwnedValue::Date(datetime) => serializer.serialize_str(&self.to_string()),
        //     tantivy::schema::OwnedValue::Bytes(bytes) => serializer.serialize_str(&self.to_string()),
        //     _ => panic!("tantivy owned value not supported"),
        // }
        let mut rgb = serializer.serialize_struct("TantivyValue", 1)?;
        rgb.serialize_field("val", &self.0)?;
        rgb.end()
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

impl TryFrom<String> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: String) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::Str(val)))
    }
}

impl TryFrom<i8> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: i8) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::I64(val as i64)))
    }
}

impl TryFrom<i16> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: i16) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::I64(val as i64)))
    }
}

impl TryFrom<i32> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: i32) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::I64(val as i64)))
    }
}

impl TryFrom<i64> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: i64) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::I64(val)))
    }
}

impl TryFrom<f32> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: f32) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::F64(val as f64)))
    }
}

impl TryFrom<f64> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: f64) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::F64(val)))
    }
}

impl TryFrom<bool> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: bool) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::Bool(val)))
    }
}

impl TryFrom<pgrx::Json> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Json) -> Result<Self, Self::Error> {
        Err(TantivyValueError::TermNotImplemented("json".to_string()))
    }
}

impl TryFrom<pgrx::JsonB> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::JsonB) -> Result<Self, Self::Error> {
        Err(TantivyValueError::TermNotImplemented("jsonb".to_string()))
    }
}

impl TryFrom<pgrx::Date> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Date) -> Result<Self, Self::Error> {
        Ok(TantivyValue(pgrx_date_to_tantivy_value(val)))
    }
}

impl TryFrom<pgrx::Time> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Time) -> Result<Self, Self::Error> {
        Ok(TantivyValue(pgrx_time_to_tantivy_value(val)))
    }
}

impl TryFrom<pgrx::Timestamp> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Timestamp) -> Result<Self, Self::Error> {
        Ok(TantivyValue(pgrx_timestamp_to_tantivy_value(val)))
    }
}

impl TryFrom<pgrx::TimeWithTimeZone> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::TimeWithTimeZone) -> Result<Self, Self::Error> {
        Ok(TantivyValue(pgrx_timetz_to_tantivy_value(val)))
    }
}

impl TryFrom<pgrx::TimestampWithTimeZone> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::TimestampWithTimeZone) -> Result<Self, Self::Error> {
        Ok(TantivyValue(pgrx_timestamptz_to_tantivy_value(val)))
    }
}

impl TryFrom<pgrx::AnyArray> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::AnyArray) -> Result<Self, Self::Error> {
        Err(TantivyValueError::TermNotImplemented("array".to_string()))
    }
}

impl TryFrom<pgrx::pg_sys::BOX> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::pg_sys::BOX) -> Result<Self, Self::Error> {
        Err(TantivyValueError::TermNotImplemented("box".to_string()))
    }
}

impl TryFrom<pgrx::pg_sys::Point> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::pg_sys::Point) -> Result<Self, Self::Error> {
        Err(TantivyValueError::TermNotImplemented("point".to_string()))
    }
}

impl TryFrom<pgrx::pg_sys::ItemPointerData> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::pg_sys::ItemPointerData) -> Result<Self, Self::Error> {
        Err(TantivyValueError::TermNotImplemented("tid".to_string()))
    }
}

impl TryFrom<pgrx::Inet> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Inet) -> Result<Self, Self::Error> {
        Err(TantivyValueError::TermNotImplemented("inet".to_string()))
    }
}

impl TryFrom<pgrx::AnyNumeric> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::AnyNumeric) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::F64(val.try_into()?)))
    }
}

impl TryFrom<pgrx::Range<i32>> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Range<i32>) -> Result<Self, Self::Error> {
        Err(TantivyValueError::TermNotImplemented("int4 range".to_string()))
    }
}

impl TryFrom<pgrx::Range<i64>> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Range<i64>) -> Result<Self, Self::Error> {
        Err(TantivyValueError::TermNotImplemented("int8 range".to_string()))
    }
}

impl TryFrom<pgrx::Range<pgrx::AnyNumeric>> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Range<pgrx::AnyNumeric>) -> Result<Self, Self::Error> {
        Err(TantivyValueError::TermNotImplemented("nuemric range".to_string()))
    }
}

impl TryFrom<pgrx::Range<pgrx::Date>> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Range<pgrx::Date>) -> Result<Self, Self::Error> {
        Err(TantivyValueError::TermNotImplemented("date range".to_string()))
    }
}

impl TryFrom<pgrx::Range<pgrx::Timestamp>> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Range<pgrx::Timestamp>) -> Result<Self, Self::Error> {
        Err(TantivyValueError::TermNotImplemented("timestamp range".to_string()))
    }
}

impl TryFrom<pgrx::Range<pgrx::TimestampWithTimeZone>> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Range<pgrx::TimestampWithTimeZone>) -> Result<Self, Self::Error> {
        Err(TantivyValueError::TermNotImplemented(
            "timestamp with time zone range".to_string(),
        ))
    }
}

impl TryFrom<pgrx::Uuid> for TantivyValue {
    type Error = TantivyValueError;

    fn try_from(val: pgrx::Uuid) -> Result<Self, Self::Error> {
        Ok(TantivyValue(tantivy::schema::OwnedValue::Str(val.to_string())))
    }
}

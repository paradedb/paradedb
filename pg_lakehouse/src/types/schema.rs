use datafusion::arrow::datatypes::{DataType, TimeUnit};
use pgrx::*;
use std::fmt;
use thiserror::Error;

use super::datetime::*;

#[derive(Clone)]
pub struct PgAttribute {
    pub oid: pg_sys::Oid,
    pub typemod: i32,
}

impl PgAttribute {
    pub fn new(oid: pg_sys::Oid, typemod: i32) -> Self {
        Self { oid, typemod }
    }
}

impl PartialEq for PgAttribute {
    fn eq(&self, other: &Self) -> bool {
        self.oid == other.oid && self.typemod == other.typemod
    }
}

impl Eq for PgAttribute {}

impl fmt::Debug for PgAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tuple = unsafe {
            pg_sys::SearchSysCache1(
                pg_sys::SysCacheIdentifier_TYPEOID as i32,
                pg_sys::Datum::from(self.oid),
            )
        };

        if tuple.is_null() {
            return write!(f, "{:?}", self.oid);
        }

        let type_form = unsafe { pg_sys::GETSTRUCT(tuple) as pg_sys::Form_pg_type };
        let type_name = unsafe { name_data_to_str(&(*type_form).typname) };

        if self.typemod >= 0 {
            write!(f, "{}({})", type_name, self.typemod)
        } else {
            write!(f, "{}", type_name)
        }
    }
}

pub static DEFAULT_TYPE_MOD: i32 = -1;

pub fn can_convert_to_attribute(
    datatype: DataType,
    attribute: PgAttribute,
) -> Result<(), SchemaError> {
    let supported_attributes = match datatype {
        DataType::Boolean => vec![PgAttribute::new(pg_sys::BOOLOID, DEFAULT_TYPE_MOD)],
        DataType::Utf8 => vec![
            PgAttribute::new(pg_sys::TEXTOID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::VARCHAROID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::BPCHAROID, DEFAULT_TYPE_MOD),
        ],
        DataType::LargeUtf8 => vec![PgAttribute::new(pg_sys::TEXTOID, DEFAULT_TYPE_MOD)],
        DataType::Int8 => vec![
            PgAttribute::new(pg_sys::INT2OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::INT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
        ],
        DataType::Int16 => vec![
            PgAttribute::new(pg_sys::INT2OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::INT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
        ],
        DataType::Int32 => vec![
            PgAttribute::new(pg_sys::INT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
        ],
        DataType::Int64 => vec![
            PgAttribute::new(pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
        ],
        DataType::UInt8 => vec![
            PgAttribute::new(pg_sys::INT2OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::INT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
        ],
        DataType::UInt16 => vec![
            PgAttribute::new(pg_sys::INT2OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::INT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
        ],
        DataType::UInt32 => vec![
            PgAttribute::new(pg_sys::INT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
        ],
        DataType::UInt64 => vec![
            PgAttribute::new(pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
        ],
        DataType::Float16 => vec![
            PgAttribute::new(pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
        ],
        DataType::Float32 => vec![
            PgAttribute::new(pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
        ],
        DataType::Float64 => vec![PgAttribute::new(pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD)],
        DataType::Date32 => vec![PgAttribute::new(pg_sys::DATEOID, DEFAULT_TYPE_MOD)],
        DataType::Date64 => vec![PgAttribute::new(pg_sys::DATEOID, DEFAULT_TYPE_MOD)],
        DataType::Timestamp(TimeUnit::Microsecond, None) => vec![
            PgAttribute::new(pg_sys::TIMESTAMPOID, DEFAULT_TYPE_MOD),
            PgAttribute::new(
                pg_sys::TIMESTAMPOID,
                PgTimestampPrecision::Microsecond.value(),
            ),
        ],
        DataType::Timestamp(TimeUnit::Millisecond, None) => vec![PgAttribute::new(
            pg_sys::TIMESTAMPOID,
            PgTimestampPrecision::Millisecond.value(),
        )],
        DataType::Timestamp(TimeUnit::Second, None) => vec![PgAttribute::new(
            pg_sys::TIMESTAMPOID,
            PgTimestampPrecision::Second.value(),
        )],
        unsupported => return Err(SchemaError::UnsupportedArrowType(unsupported)),
    };

    if !supported_attributes.contains(&attribute) {
        return Err(SchemaError::UnsupportedConversion(
            datatype,
            attribute,
            supported_attributes,
        ));
    }

    Ok(())
}

#[derive(Error, Debug)]
pub enum SchemaError {
    #[error("Unsupported Arrow type: {0:?}")]
    UnsupportedArrowType(DataType),

    #[error("Cannot convert Arrow type {0:?} to Postgres type {1:?}. Supported Postgres types are: {2:?}")]
    UnsupportedConversion(DataType, PgAttribute, Vec<PgAttribute>),
}

use datafusion::arrow::datatypes::{DataType, Field, TimeUnit};
use pgrx::*;
use std::fmt;
use thiserror::Error;

use super::datetime::*;

#[derive(Clone)]
pub struct PgAttribute {
    name: String,
    oid: pg_sys::Oid,
    typemod: i32,
}

impl PgAttribute {
    pub fn new(name: &str, oid: pg_sys::Oid, typemod: i32) -> Self {
        Self {
            name: name.to_string(),
            oid,
            typemod,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }
}

impl PartialEq for PgAttribute {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.oid == other.oid
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
        let type_name = unsafe { name_data_to_str(&(*type_form).typname).to_uppercase() };

        if self.typemod >= 0 {
            write!(f, "{}({})", type_name, self.typemod)
        } else {
            write!(f, "{}", type_name)
        }
    }
}

pub static DEFAULT_TYPE_MOD: i32 = -1;

pub fn can_convert_to_attribute(field: &Field, attribute: PgAttribute) -> Result<(), SchemaError> {
    if *field.name() != attribute.name() {
        return Err(SchemaError::ColumnNameMismatch(
            field.name().to_string(),
            attribute.name(),
        ));
    }

    let supported_attributes = match field.data_type() {
        DataType::Boolean => vec![PgAttribute::new(
            field.name(),
            pg_sys::BOOLOID,
            DEFAULT_TYPE_MOD,
        )],
        DataType::Utf8 => vec![
            PgAttribute::new(field.name(), pg_sys::TEXTOID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::VARCHAROID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::BPCHAROID, DEFAULT_TYPE_MOD),
        ],
        DataType::LargeUtf8 => vec![PgAttribute::new(
            field.name(),
            pg_sys::TEXTOID,
            DEFAULT_TYPE_MOD,
        )],
        DataType::Int8 => vec![
            PgAttribute::new(field.name(), pg_sys::INT2OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::INT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::NUMERICOID, DEFAULT_TYPE_MOD),
        ],
        DataType::Int16 => vec![
            PgAttribute::new(field.name(), pg_sys::INT2OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::INT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::NUMERICOID, DEFAULT_TYPE_MOD),
        ],
        DataType::Int32 => vec![
            PgAttribute::new(field.name(), pg_sys::INT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::NUMERICOID, DEFAULT_TYPE_MOD),
        ],
        DataType::Int64 => vec![
            PgAttribute::new(field.name(), pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::NUMERICOID, DEFAULT_TYPE_MOD),
        ],
        DataType::UInt8 => vec![
            PgAttribute::new(field.name(), pg_sys::INT2OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::INT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::NUMERICOID, DEFAULT_TYPE_MOD),
        ],
        DataType::UInt16 => vec![
            PgAttribute::new(field.name(), pg_sys::INT2OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::INT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::NUMERICOID, DEFAULT_TYPE_MOD),
        ],
        DataType::UInt32 => vec![
            PgAttribute::new(field.name(), pg_sys::INT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::NUMERICOID, DEFAULT_TYPE_MOD),
        ],
        DataType::UInt64 => vec![
            PgAttribute::new(field.name(), pg_sys::INT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::NUMERICOID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::NUMERICOID, DEFAULT_TYPE_MOD),
        ],
        DataType::Float16 => vec![
            PgAttribute::new(field.name(), pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::NUMERICOID, DEFAULT_TYPE_MOD),
        ],
        DataType::Float32 => vec![
            PgAttribute::new(field.name(), pg_sys::FLOAT4OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::NUMERICOID, DEFAULT_TYPE_MOD),
        ],
        DataType::Float64 => vec![
            PgAttribute::new(field.name(), pg_sys::FLOAT8OID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::NUMERICOID, DEFAULT_TYPE_MOD),
        ],
        DataType::Date32 => vec![PgAttribute::new(
            field.name(),
            pg_sys::DATEOID,
            DEFAULT_TYPE_MOD,
        )],
        DataType::Date64 => vec![PgAttribute::new(
            field.name(),
            pg_sys::DATEOID,
            DEFAULT_TYPE_MOD,
        )],
        DataType::Timestamp(TimeUnit::Microsecond, None) => vec![
            PgAttribute::new(field.name(), pg_sys::TIMESTAMPOID, DEFAULT_TYPE_MOD),
            PgAttribute::new(
                field.name(),
                pg_sys::TIMESTAMPOID,
                PgTimestampPrecision::Microsecond.value(),
            ),
        ],
        DataType::Timestamp(TimeUnit::Millisecond, None) => vec![PgAttribute::new(
            field.name(),
            pg_sys::TIMESTAMPOID,
            PgTimestampPrecision::Millisecond.value(),
        )],
        DataType::Timestamp(TimeUnit::Second, None) => vec![PgAttribute::new(
            field.name(),
            pg_sys::TIMESTAMPOID,
            PgTimestampPrecision::Second.value(),
        )],
        DataType::Binary => vec![
            PgAttribute::new(field.name(), pg_sys::TEXTOID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::VARCHAROID, DEFAULT_TYPE_MOD),
            PgAttribute::new(field.name(), pg_sys::BPCHAROID, DEFAULT_TYPE_MOD),
        ],
        unsupported => {
            return Err(SchemaError::UnsupportedArrowType(
                field.name().to_string(),
                unsupported.clone(),
            ))
        }
    };

    if !supported_attributes.contains(&attribute) {
        return Err(SchemaError::UnsupportedConversion(
            field.name().to_string(),
            attribute,
            field.data_type().clone(),
            supported_attributes,
        ));
    }

    // For TIMESTAMP, the type modifier must match the precision of the Arrow field
    if let DataType::Timestamp(_, None) = field.data_type() {
        if !supported_attributes
            .iter()
            .any(|attr| attr.typemod == attribute.typemod)
        {
            return Err(SchemaError::UnsupportedConversion(
                field.name().to_string(),
                attribute,
                field.data_type().clone(),
                supported_attributes,
            ));
        }
    }

    Ok(())
}

#[derive(Error, Debug)]
pub enum SchemaError {
    #[error("Column name mismatch: Expected column to be named {0} but found {1}. Note that column names are case-sensitive and must be enclosed in double quotes")]
    ColumnNameMismatch(String, String),

    #[error(
        "Unsupported Arrow type: Column {0} has Arrow type {1:?}, which is not yet supported."
    )]
    UnsupportedArrowType(String, DataType),

    #[error("Type mismatch: Column {0} was assigned type {1:?}, which is not valid for the underlying Arrow type {2:?}. Please change the column type to one of the supported types: {3:?}")]
    UnsupportedConversion(String, PgAttribute, DataType, Vec<PgAttribute>),
}

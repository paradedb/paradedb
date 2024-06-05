use datafusion::arrow::datatypes::{DataType, Field};
use pgrx::*;
use std::fmt;
use thiserror::Error;

#[derive(Clone)]
pub struct PgAttribute {
    name: String,
    oid: pg_sys::Oid,
}

impl PgAttribute {
    pub fn new(name: &str, oid: pg_sys::Oid) -> Self {
        Self {
            name: name.to_string(),
            oid,
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

        write!(f, "{}", type_name)
    }
}

pub fn can_convert_to_attribute(field: &Field, attribute: PgAttribute) -> Result<(), SchemaError> {
    if *field.name() != attribute.name() {
        return Err(SchemaError::ColumnNameMismatch(
            field.name().to_string(),
            attribute.name(),
        ));
    }

    let supported_attributes = match field.data_type() {
        DataType::Binary | DataType::LargeBinary => vec![
            PgAttribute::new(field.name(), pg_sys::TEXTOID),
            PgAttribute::new(field.name(), pg_sys::VARCHAROID),
            PgAttribute::new(field.name(), pg_sys::BPCHAROID),
            PgAttribute::new(field.name(), pg_sys::BYTEAOID),
        ],
        DataType::Boolean => vec![PgAttribute::new(field.name(), pg_sys::BOOLOID)],
        DataType::Utf8 | DataType::LargeUtf8 => vec![
            PgAttribute::new(field.name(), pg_sys::TEXTOID),
            PgAttribute::new(field.name(), pg_sys::VARCHAROID),
            PgAttribute::new(field.name(), pg_sys::BPCHAROID),
        ],
        DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64
        | DataType::Float16
        | DataType::Float32
        | DataType::Float64
        | DataType::Decimal128(_, _) => vec![
            PgAttribute::new(field.name(), pg_sys::INT2OID),
            PgAttribute::new(field.name(), pg_sys::INT4OID),
            PgAttribute::new(field.name(), pg_sys::INT8OID),
            PgAttribute::new(field.name(), pg_sys::FLOAT4OID),
            PgAttribute::new(field.name(), pg_sys::FLOAT8OID),
            PgAttribute::new(field.name(), pg_sys::NUMERICOID),
        ],
        DataType::Date32 | DataType::Date64 => {
            vec![PgAttribute::new(field.name(), pg_sys::DATEOID)]
        }
        DataType::Time32(_) | DataType::Time64(_) => {
            vec![PgAttribute::new(field.name(), pg_sys::TIMEOID)]
        }
        DataType::Timestamp(_, None) => vec![PgAttribute::new(field.name(), pg_sys::TIMESTAMPOID)],
        DataType::Timestamp(_, _tz) => vec![PgAttribute::new(field.name(), pg_sys::TIMESTAMPTZOID)],
        DataType::Null => vec![PgAttribute::new(field.name(), pg_sys::VOIDOID)],
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

    Ok(())
}

#[derive(Error, Debug)]
pub enum SchemaError {
    #[error("Column name mismatch: Expected column to be named {0} but found {1}. Note that column names are case-sensitive and must be enclosed in double quotes")]
    ColumnNameMismatch(String, String),

    #[error(
        "Unsupported Arrow type: Column {0} has Arrow type {1:?}, which is not yet supported. Please submit a request at https://github.com/paradedb/paradedb/issues if you would like to see this type supported."
    )]
    UnsupportedArrowType(String, DataType),

    #[error("Type mismatch: Column {0} was assigned type {1:?}, which is not valid for the underlying Arrow type {2:?}. Please change the column type to one of the supported types: {3:?}")]
    UnsupportedConversion(String, PgAttribute, DataType, Vec<PgAttribute>),
}

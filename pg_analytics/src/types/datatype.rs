use deltalake::arrow::error::ArrowError;
use deltalake::datafusion::arrow::datatypes::*;
use pgrx::*;
use thiserror::Error;

use super::date::DateError;
use super::datum::DatumError;
use super::numeric::{NumericError, PgNumericTypeMod, PgPrecision, PgScale};
use super::timestamp::TimestampError;

// By default, unspecified type mods in Postgres are -1
const DEFAULT_TYPE_MOD: i32 = -1;

pub struct PgTypeMod(pub i32);
pub struct PgAttribute(pub PgOid, pub PgTypeMod);
pub struct ArrowDataType(pub DataType);

impl TryFrom<PgAttribute> for ArrowDataType {
    type Error = DataTypeError;

    fn try_from(attribute: PgAttribute) -> Result<Self, Self::Error> {
        let PgAttribute(oid, typemod) = attribute;

        let datatype = match oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::BOOLOID => DataType::Boolean,
                PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID | PgBuiltInOids::BPCHAROID => {
                    DataType::Utf8
                }
                PgBuiltInOids::INT2OID => DataType::Int16,
                PgBuiltInOids::INT4OID => DataType::Int32,
                PgBuiltInOids::INT8OID => DataType::Int64,
                PgBuiltInOids::FLOAT4OID => DataType::Float32,
                PgBuiltInOids::FLOAT8OID => DataType::Float64,
                PgBuiltInOids::DATEOID => DataType::Date32,
                PgBuiltInOids::TIMESTAMPOID => {
                    DataType::Timestamp(TimeUnit::try_from(typemod)?, None)
                }
                PgBuiltInOids::NUMERICOID => {
                    let PgNumericTypeMod(PgPrecision(precision), PgScale(scale)) =
                        typemod.try_into()?;
                    DataType::Decimal128(precision, scale)
                }
                unsupported => return Err(DataTypeError::UnsupportedPostgresType(unsupported)),
            },
            PgOid::Invalid => return Err(DataTypeError::InvalidPostgresOid),
            PgOid::Custom(_) => return Err(DataTypeError::UnsupportedCustomType),
        };

        Ok(ArrowDataType(datatype))
    }
}

impl TryFrom<ArrowDataType> for PgAttribute {
    type Error = DataTypeError;

    fn try_from(datatype: ArrowDataType) -> Result<Self, Self::Error> {
        let ArrowDataType(datatype) = datatype;

        let result = match datatype {
            DataType::Boolean => (PgBuiltInOids::BOOLOID, PgTypeMod(DEFAULT_TYPE_MOD)),
            DataType::Utf8 => (PgBuiltInOids::TEXTOID, PgTypeMod(DEFAULT_TYPE_MOD)),
            DataType::Int16 => (PgBuiltInOids::INT2OID, PgTypeMod(DEFAULT_TYPE_MOD)),
            DataType::Int32 => (PgBuiltInOids::INT4OID, PgTypeMod(DEFAULT_TYPE_MOD)),
            DataType::Int64 => (PgBuiltInOids::INT8OID, PgTypeMod(DEFAULT_TYPE_MOD)),
            DataType::Float32 => (PgBuiltInOids::FLOAT4OID, PgTypeMod(DEFAULT_TYPE_MOD)),
            DataType::Float64 => (PgBuiltInOids::FLOAT8OID, PgTypeMod(DEFAULT_TYPE_MOD)),
            DataType::Date32 => (PgBuiltInOids::DATEOID, PgTypeMod(DEFAULT_TYPE_MOD)),
            DataType::Timestamp(timeunit, None) => {
                (PgBuiltInOids::TIMESTAMPOID, PgTypeMod::try_from(timeunit)?)
            }
            DataType::Decimal128(precision, scale) => (
                PgBuiltInOids::NUMERICOID,
                PgTypeMod::try_from(PgNumericTypeMod(PgPrecision(precision), PgScale(scale)))?,
            ),
            DataType::List(ref field) => match field.data_type() {
                DataType::Boolean => (PgBuiltInOids::BOOLARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                DataType::Utf8 => (PgBuiltInOids::TEXTARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                DataType::Int16 => (PgBuiltInOids::INT2ARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                DataType::Int32 => (PgBuiltInOids::INT4ARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                DataType::Int64 => (PgBuiltInOids::INT8ARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                DataType::Float32 => (PgBuiltInOids::FLOAT4ARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                DataType::Float64 => (PgBuiltInOids::FLOAT8ARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                DataType::Date32 => (PgBuiltInOids::DATEARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                unsupported => {
                    return Err(DataTypeError::UnsupportedArrowArrayType(
                        unsupported.clone(),
                    ))
                }
            },
            unsupported => return Err(DataTypeError::UnsupportedArrowType(unsupported)),
        };

        Ok(PgAttribute(PgOid::BuiltIn(result.0), result.1))
    }
}

#[derive(Error, Debug)]
pub enum DataTypeError {
    #[error(transparent)]
    Arrow(#[from] ArrowError),

    #[error(transparent)]
    Date(#[from] DateError),

    #[error(transparent)]
    Datum(#[from] DatumError),

    #[error(transparent)]
    Timestamp(#[from] TimestampError),

    #[error(transparent)]
    Numeric(#[from] NumericError),

    #[error("Invalid Postgres OID")]
    InvalidPostgresOid,

    #[error("Postgres type {0:?} is not yet supported")]
    UnsupportedPostgresType(PgBuiltInOids),

    #[error("Custom Postgres types are not supported")]
    UnsupportedCustomType,

    #[error("Could not convert arrow type {0:?} to Postgres type")]
    UnsupportedArrowType(DataType),

    #[error("Could not convert arrow array with type {0:?} to Postgres array")]
    UnsupportedArrowArrayType(DataType),
}

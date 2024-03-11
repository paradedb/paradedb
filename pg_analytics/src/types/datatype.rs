use deltalake::arrow::error::ArrowError;
use deltalake::datafusion::arrow::datatypes::DataType::*;
use deltalake::datafusion::arrow::datatypes::*;
use pgrx::pg_sys::BuiltinOid::*;
use pgrx::*;
use thiserror::Error;

use super::date::DateError;
use super::datum::DatumError;
use super::numeric::{NumericError, PgNumericTypeMod, PgPrecision, PgScale};
use super::time::{TimeError, TimePrecision};
use super::timestamp::{TimestampError, TimestampPrecision};

// By default, unspecified type mods in Postgres are -1
const DEFAULT_TYPE_MOD: i32 = -1;

#[derive(Copy, Clone, Debug)]
pub struct PgTypeMod(pub i32);

#[derive(Copy, Clone, Debug)]
pub struct PgAttribute(pub PgOid, pub PgTypeMod);

#[derive(Clone, Debug)]
pub struct ArrowDataType(pub DataType);

impl TryFrom<PgAttribute> for ArrowDataType {
    type Error = DataTypeError;

    fn try_from(attribute: PgAttribute) -> Result<Self, Self::Error> {
        let PgAttribute(oid, typemod) = attribute;

        let datatype = match oid {
            PgOid::BuiltIn(builtin) => match builtin {
                BOOLOID => Boolean,
                TEXTOID => Utf8,
                VARCHAROID => Utf8,
                BPCHAROID => Utf8,
                INT2OID => Int16,
                INT4OID => Int32,
                INT8OID => Int64,
                FLOAT4OID => Float32,
                FLOAT8OID => Float64,
                DATEOID => Date32,
                TIMEOID => Time64(TimePrecision::try_from(typemod)?.0),
                TIMESTAMPOID => Timestamp(TimestampPrecision::try_from(typemod)?.0, None),
                TIMESTAMPTZOID => {
                    Timestamp(TimestampPrecision::try_from(typemod)?.0, Some("UTC".into()))
                }
                NUMERICOID => {
                    let PgNumericTypeMod(PgPrecision(precision), PgScale(scale)) =
                        typemod.try_into()?;
                    Decimal128(precision, scale)
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
            Boolean => (BOOLOID, PgTypeMod(DEFAULT_TYPE_MOD)),
            Utf8 => (TEXTOID, PgTypeMod(DEFAULT_TYPE_MOD)),
            Int16 => (INT2OID, PgTypeMod(DEFAULT_TYPE_MOD)),
            Int32 => (INT4OID, PgTypeMod(DEFAULT_TYPE_MOD)),
            Int64 => (INT8OID, PgTypeMod(DEFAULT_TYPE_MOD)),
            Float32 => (FLOAT4OID, PgTypeMod(DEFAULT_TYPE_MOD)),
            Float64 => (FLOAT8OID, PgTypeMod(DEFAULT_TYPE_MOD)),
            Date32 => (DATEOID, PgTypeMod(DEFAULT_TYPE_MOD)),
            Time64(timeunit) => (TIMEOID, PgTypeMod::try_from(TimePrecision(timeunit))?),
            Timestamp(timeunit, None) => (
                TIMESTAMPOID,
                PgTypeMod::try_from(TimestampPrecision(timeunit))?,
            ),
            Timestamp(timeunit, Some(timezone)) if timezone.as_ref() == "UTC" => (
                TIMESTAMPTZOID,
                PgTypeMod::try_from(TimestampPrecision(timeunit))?,
            ),
            Decimal128(precision, scale) => (
                NUMERICOID,
                PgTypeMod::try_from(PgNumericTypeMod(PgPrecision(precision), PgScale(scale)))?,
            ),
            List(ref field) => match field.data_type() {
                Boolean => (BOOLARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                Utf8 => (TEXTARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                Int16 => (INT2ARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                Int32 => (INT4ARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                Int64 => (INT8ARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                Float32 => (FLOAT4ARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                Float64 => (FLOAT8ARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                Date32 => (DATEARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
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
    Time(#[from] TimeError),

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

use deltalake::datafusion::arrow::datatypes::*;
use deltalake::datafusion::common::arrow::array::{
    Array, ArrayRef, AsArray, BooleanArray, StringArray,
};
use pgrx::*;
use std::convert::TryInto;
use std::sync::Arc;

use super::array::{IntoArray, IntoPrimitiveArray};
use super::numeric::{PgNumeric, PgNumericTypeMod, PgPrecision, PgScale};
use super::timestamp::Microseconds;
use crate::errors::{NotSupported, ParadeError};

const DEFAULT_TYPE_MOD: i32 = -1;

pub struct PgTypeMod(pub i32);
pub struct PgAttribute(pub PgOid, pub PgTypeMod);
pub struct ParadeDataType(pub DataType);

impl TryInto<ParadeDataType> for PgAttribute {
    type Error = ParadeError;

    fn try_into(self) -> Result<ParadeDataType, ParadeError> {
        let PgAttribute(oid, typemod) = self;

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
                PgBuiltInOids::TIMESTAMPOID => DataType::Timestamp(typemod.try_into()?, None),
                PgBuiltInOids::NUMERICOID => {
                    let PgNumericTypeMod(PgPrecision(precision), PgScale(scale)) =
                        typemod.try_into()?;
                    DataType::Decimal128(precision, scale)
                }
                unsupported => return Err(NotSupported::BuiltinPostgresType(unsupported).into()),
            },
            PgOid::Invalid => return Err(NotSupported::InvalidPostgresType.into()),
            PgOid::Custom(_) => return Err(NotSupported::CustomPostgresType.into()),
        };

        Ok(ParadeDataType(datatype))
    }
}

impl TryInto<PgAttribute> for ParadeDataType {
    type Error = ParadeError;

    fn try_into(self) -> Result<PgAttribute, ParadeError> {
        let ParadeDataType(datatype) = self;

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
                (PgBuiltInOids::TIMESTAMPOID, timeunit.try_into()?)
            }
            DataType::Decimal128(precision, scale) => (
                PgBuiltInOids::NUMERICOID,
                PgNumericTypeMod(PgPrecision(precision), PgScale(scale)).try_into()?,
            ),
            unsupported => return Err(NotSupported::DataType(unsupported.clone()).into()),
        };

        Ok(PgAttribute(PgOid::BuiltIn(result.0), result.1))
    }
}

pub trait GetDatum {
    fn get_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, ParadeError>;
}

impl GetDatum for Arc<dyn Array> {
    fn get_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, ParadeError> {
        let result = match self.data_type() {
            DataType::Boolean => self
                .as_any()
                .downcast_ref::<BooleanArray>()
                .ok_or(ParadeError::DowncastGenericArray(DataType::Boolean))?
                .value(index)
                .into_datum(),
            DataType::Utf8 => self
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(ParadeError::DowncastGenericArray(DataType::Utf8))?
                .value(index)
                .into_datum(),
            DataType::Int32 => self.as_primitive::<Int32Type>().value(index).into_datum(),
            DataType::Int64 => self.as_primitive::<Int64Type>().value(index).into_datum(),
            DataType::Float32 => self.as_primitive::<Float32Type>().value(index).into_datum(),
            DataType::Float64 => self.as_primitive::<Float64Type>().value(index).into_datum(),
            DataType::Date32 => self.as_primitive::<Date32Type>().value(index).into_datum(),
            DataType::Timestamp(TimeUnit::Microsecond, None) => {
                Microseconds(self.as_primitive::<TimestampMicrosecondType>().value(index))
                    .try_into()?
            }
            DataType::Decimal128(precision, scale) => PgNumeric(
                AnyNumeric::from(self.as_primitive::<Decimal128Type>().value(index)),
                PgNumericTypeMod(PgPrecision(*precision), PgScale(*scale)),
            )
            .try_into()?,
            _ => return Ok(None),
        };

        Ok(result)
    }
}

pub trait IntoArrayRef {
    fn into_array_ref(self, oid: PgOid) -> Result<ArrayRef, ParadeError>;
}

impl<T> IntoArrayRef for T
where
    T: Iterator<Item = pg_sys::Datum>,
{
    fn into_array_ref(self, oid: PgOid) -> Result<ArrayRef, ParadeError> {
        Ok(match oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::BOOLOID => {
                    Arc::new(self.into_primitive_array::<bool>().into_array())
                }
                PgBuiltInOids::TEXTOID => {
                    Arc::new(self.into_primitive_array::<String>().into_array())
                }
                PgBuiltInOids::INT2OID => Arc::new(self.into_primitive_array::<i16>().into_array()),
                PgBuiltInOids::INT4OID => Arc::new(self.into_primitive_array::<i32>().into_array()),
                PgBuiltInOids::INT8OID => Arc::new(self.into_primitive_array::<i64>().into_array()),
                PgBuiltInOids::FLOAT4OID => {
                    Arc::new(self.into_primitive_array::<f32>().into_array())
                }
                PgBuiltInOids::FLOAT8OID => {
                    Arc::new(self.into_primitive_array::<f64>().into_array())
                }
                PgBuiltInOids::DATEOID => Arc::new(self.into_primitive_array::<i32>().into_array()),
                // PgBuiltInOids::TIMESTAMPOID => self.into_primitive_array().into_array(),
                // PgBuiltInOids::NUMERICOID => self.into_primitive_array().into_array(),
                unsupported => return Err(NotSupported::BuiltinPostgresType(unsupported).into()),
            },
            PgOid::Invalid => return Err(NotSupported::InvalidPostgresType.into()),
            PgOid::Custom(_) => return Err(NotSupported::CustomPostgresType.into()),
        })
    }
}

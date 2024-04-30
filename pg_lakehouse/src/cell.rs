use datafusion::arrow::array::types::{Date32Type, TimestampMicrosecondType};
use datafusion::arrow::array::{
    Array, ArrayAccessor, ArrayRef, AsArray, BooleanArray, Float32Array, Float64Array, Int16Array,
    Int32Array, Int64Array, StringArray,
};
use datafusion::arrow::datatypes::DataType;
use datafusion::common::{downcast_value, DataFusionError};
use pgrx::*;
use std::fmt::Debug;
use supabase_wrappers::interface::Cell;
use thiserror::Error;

use super::datetime::*;

pub trait GetDateValue
where
    Self: Array + AsArray,
{
    fn get_date_value(&self, index: usize) -> Result<Option<datum::Date>, DataTypeError> {
        let downcast_array = downcast_value!(self, Int32Array);
        let date_array = downcast_array.reinterpret_cast::<Date32Type>();

        match date_array.nulls().is_some() && date_array.is_null(index) {
            false => {
                let date = date_array
                    .value_as_date(index)
                    .ok_or(DataTypeError::DateConversion)?;

                Ok(Some(datum::Date::try_from(Date(date))?))
            }
            true => Ok(None),
        }
    }
}

pub trait GetPrimitiveValue
where
    Self: Array + AsArray,
{
    fn get_primitive_value<A>(
        &self,
        index: usize,
    ) -> Result<Option<<&A as ArrayAccessor>::Item>, DataTypeError>
    where
        A: Array + Debug + 'static,
        for<'a> &'a A: ArrayAccessor,
    {
        let downcast_array = downcast_value!(self, A);
        match downcast_array.nulls().is_some() && downcast_array.is_null(index) {
            false => Ok(Some(downcast_array.value(index))),
            true => Ok(None),
        }
    }
}

pub trait GetTimestampValue
where
    Self: Array + AsArray,
{
    fn get_timestamp_value(&self, index: usize) -> Result<Option<datum::Timestamp>, DataTypeError> {
        let downcast_array = downcast_value!(self, Int64Array);
        let timestamp_array = downcast_array.reinterpret_cast::<TimestampMicrosecondType>();

        match timestamp_array.nulls().is_some() && timestamp_array.is_null(index) {
            false => {
                let datetime = timestamp_array
                    .value_as_datetime(index)
                    .ok_or(DataTypeError::DateTimeConversion)?;

                Ok(Some(datum::Timestamp::try_from(DateTime(datetime))?))
            }
            true => Ok(None),
        }
    }
}

pub trait GetCell
where
    Self: Array + AsArray + GetDateValue + GetPrimitiveValue + GetTimestampValue,
{
    fn get_cell(&self, index: usize, oid: pg_sys::Oid) -> Result<Option<Cell>, DataTypeError> {
        match oid {
            pg_sys::BOOLOID => match self.get_primitive_value::<BooleanArray>(index)? {
                Some(value) => Ok(Some(Cell::Bool(value))),
                None => Ok(None),
            },
            pg_sys::INT2OID
            | pg_sys::INT4OID
            | pg_sys::INT8OID
            | pg_sys::FLOAT4OID
            | pg_sys::FLOAT8OID => match self.data_type() {
                DataType::Int16 => match self.get_primitive_value::<Int16Array>(index)? {
                    Some(value) => Ok(Some(Cell::I16(value))),
                    None => Ok(None),
                },
                DataType::Int32 => match self.get_primitive_value::<Int32Array>(index)? {
                    Some(value) => Ok(Some(Cell::I32(value))),
                    None => Ok(None),
                },
                DataType::Int64 => match self.get_primitive_value::<Int64Array>(index)? {
                    Some(value) => Ok(Some(Cell::I64(value))),
                    None => Ok(None),
                },
                DataType::Float32 => match self.get_primitive_value::<Float32Array>(index)? {
                    Some(value) => Ok(Some(Cell::F32(value))),
                    None => Ok(None),
                },
                DataType::Float64 => match self.get_primitive_value::<Float64Array>(index)? {
                    Some(value) => Ok(Some(Cell::F64(value))),
                    None => Ok(None),
                },
                unsupported => Err(DataTypeError::DataTypeMismatch(
                    unsupported.clone(),
                    PgOid::from(oid),
                )),
            },
            pg_sys::TEXTOID | pg_sys::VARCHAROID | pg_sys::BPCHAROID => {
                match self.get_primitive_value::<StringArray>(index)? {
                    Some(value) => Ok(Some(Cell::String(value.to_string()))),
                    None => Ok(None),
                }
            }
            pg_sys::DATEOID => match self.data_type() {
                DataType::Int32 => match self.get_date_value(index)? {
                    Some(value) => Ok(Some(Cell::Date(value))),
                    None => Ok(None),
                },
                unsupported => Err(DataTypeError::DataTypeMismatch(
                    unsupported.clone(),
                    PgOid::from(oid),
                )),
            },
            pg_sys::TIMESTAMPOID => match self.data_type() {
                DataType::Int64 => match self.get_timestamp_value(index)? {
                    Some(value) => Ok(Some(Cell::Timestamp(value))),
                    None => Ok(None),
                },
                unsupported => Err(DataTypeError::DataTypeMismatch(
                    unsupported.clone(),
                    PgOid::from(oid),
                )),
            },
            unsupported => Err(DataTypeError::UnsupportedPostgresType(PgOid::from(
                unsupported,
            ))),
        }
    }
}

impl GetCell for ArrayRef {}
impl GetDateValue for ArrayRef {}
impl GetPrimitiveValue for ArrayRef {}
impl GetTimestampValue for ArrayRef {}

#[derive(Error, Debug)]
pub enum DataTypeError {
    #[error(transparent)]
    DateTimeConversionError(#[from] datum::datetime_support::DateTimeConversionError),

    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error("Failed to convert date to NaiveDate")]
    DateConversion,

    #[error("Failed to convert timestamp to NaiveDateTime")]
    DateTimeConversion,

    #[error("Received unsupported data type {0:?} for {1:?}")]
    DataTypeMismatch(DataType, PgOid),

    #[error("Downcast Arrow array failed")]
    DowncastError,

    #[error("Postgres data type {0:?} is not supported")]
    UnsupportedPostgresType(PgOid),
}

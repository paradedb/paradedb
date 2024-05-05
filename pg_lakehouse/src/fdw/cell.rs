use datafusion::arrow::array::types::{
    ArrowTemporalType, Date32Type, Date64Type, TimestampMicrosecondType, TimestampMillisecondType,
    TimestampNanosecondType, TimestampSecondType, UInt16Type, UInt32Type, UInt64Type, UInt8Type,
};
use datafusion::arrow::array::{
    Array, ArrayAccessor, ArrayRef, ArrowPrimitiveType, AsArray, BooleanArray, Float16Array,
    Float32Array, Float64Array, GenericByteArray, Int16Array, Int32Array, Int64Array, Int8Array,
    StringArray,
};
use datafusion::arrow::datatypes::{DataType, GenericStringType, TimeUnit};
use datafusion::common::{downcast_value, DataFusionError};
use pgrx::*;
use std::fmt::Debug;
use supabase_wrappers::interface::Cell;
use thiserror::Error;

use crate::types::datetime::*;

type LargeStringArray = GenericByteArray<GenericStringType<i64>>;

pub trait GetDateValue
where
    Self: Array + AsArray,
{
    fn get_date_value<N, T>(&self, index: usize) -> Result<Option<datum::Date>, DataTypeError>
    where
        N: std::marker::Send + std::marker::Sync,
        i64: From<N>,
        T: ArrowPrimitiveType<Native = N> + ArrowTemporalType,
    {
        let downcast_array = self.as_primitive::<T>();

        match downcast_array.nulls().is_some() && downcast_array.is_null(index) {
            false => {
                let date = downcast_array
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
    fn get_timestamp_value<T>(
        &self,
        index: usize,
    ) -> Result<Option<datum::Timestamp>, DataTypeError>
    where
        T: ArrowPrimitiveType<Native = i64> + ArrowTemporalType,
    {
        let downcast_array = self.as_primitive::<T>();

        match downcast_array.nulls().is_some() && downcast_array.is_null(index) {
            false => {
                let datetime = downcast_array
                    .value_as_datetime(index)
                    .ok_or(DataTypeError::DateTimeConversion)?;

                Ok(Some(datum::Timestamp::try_from(DateTime(datetime))?))
            }
            true => Ok(None),
        }
    }
}

pub trait GetUIntValue
where
    Self: Array + AsArray,
{
    fn get_uint_value<A>(&self, index: usize) -> Result<Option<i64>, DataTypeError>
    where
        A: ArrowPrimitiveType,
        i64: TryFrom<A::Native>,
    {
        let downcast_array = self.as_primitive::<A>();
        match downcast_array.is_null(index) {
            false => {
                let value: A::Native = downcast_array.value(index);
                Ok(Some(
                    i64::try_from(value).map_err(|_| DataTypeError::UIntConversionError)?,
                ))
            }
            true => Ok(None),
        }
    }
}

pub trait GetCell
where
    Self: Array + AsArray + GetDateValue + GetPrimitiveValue + GetTimestampValue + GetUIntValue,
{
    fn get_cell(
        &self,
        index: usize,
        oid: pg_sys::Oid,
        _type_mod: i32,
    ) -> Result<Option<Cell>, DataTypeError> {
        match oid {
            pg_sys::BOOLOID => match self.get_primitive_value::<BooleanArray>(index)? {
                Some(value) => Ok(Some(Cell::Bool(value))),
                None => Ok(None),
            },
            pg_sys::INT2OID => match self.data_type() {
                DataType::Int8 => match self.get_primitive_value::<Int8Array>(index)? {
                    Some(value) => Ok(Some(Cell::I16(value as i16))),
                    None => Ok(None),
                },
                DataType::Int16 => match self.get_primitive_value::<Int16Array>(index)? {
                    Some(value) => Ok(Some(Cell::I16(value as i16))),
                    None => Ok(None),
                },
                DataType::UInt8 => match self.get_uint_value::<UInt8Type>(index)? {
                    Some(value) => Ok(Some(Cell::I16(value as i16))),
                    None => Ok(None),
                },
                DataType::UInt16 => match self.get_uint_value::<UInt16Type>(index)? {
                    Some(value) => Ok(Some(Cell::I16(value as i16))),
                    None => Ok(None),
                },
                unsupported => Err(DataTypeError::DataTypeMismatch(
                    unsupported.clone(),
                    PgOid::from(oid),
                )),
            },
            pg_sys::INT4OID => match self.data_type() {
                DataType::Int8 => match self.get_primitive_value::<Int8Array>(index)? {
                    Some(value) => Ok(Some(Cell::I32(value as i32))),
                    None => Ok(None),
                },
                DataType::Int16 => match self.get_primitive_value::<Int16Array>(index)? {
                    Some(value) => Ok(Some(Cell::I32(value as i32))),
                    None => Ok(None),
                },
                DataType::Int32 => match self.get_primitive_value::<Int32Array>(index)? {
                    Some(value) => Ok(Some(Cell::I32(value as i32))),
                    None => Ok(None),
                },
                DataType::UInt8 => match self.get_uint_value::<UInt8Type>(index)? {
                    Some(value) => Ok(Some(Cell::I32(value as i32))),
                    None => Ok(None),
                },
                DataType::UInt16 => match self.get_uint_value::<UInt16Type>(index)? {
                    Some(value) => Ok(Some(Cell::I32(value as i32))),
                    None => Ok(None),
                },
                DataType::UInt32 => match self.get_uint_value::<UInt32Type>(index)? {
                    Some(value) => Ok(Some(Cell::I32(value as i32))),
                    None => Ok(None),
                },
                unsupported => Err(DataTypeError::DataTypeMismatch(
                    unsupported.clone(),
                    PgOid::from(oid),
                )),
            },
            pg_sys::INT8OID => match self.data_type() {
                DataType::Int8 => match self.get_primitive_value::<Int8Array>(index)? {
                    Some(value) => Ok(Some(Cell::I64(value as i64))),
                    None => Ok(None),
                },
                DataType::Int16 => match self.get_primitive_value::<Int16Array>(index)? {
                    Some(value) => Ok(Some(Cell::I64(value as i64))),
                    None => Ok(None),
                },
                DataType::Int32 => match self.get_primitive_value::<Int32Array>(index)? {
                    Some(value) => Ok(Some(Cell::I64(value as i64))),
                    None => Ok(None),
                },
                DataType::Int64 => match self.get_primitive_value::<Int64Array>(index)? {
                    Some(value) => Ok(Some(Cell::I64(value as i64))),
                    None => Ok(None),
                },
                DataType::UInt8 => match self.get_uint_value::<UInt8Type>(index)? {
                    Some(value) => Ok(Some(Cell::I64(value as i64))),
                    None => Ok(None),
                },
                DataType::UInt16 => match self.get_uint_value::<UInt16Type>(index)? {
                    Some(value) => Ok(Some(Cell::I64(value as i64))),
                    None => Ok(None),
                },
                DataType::UInt32 => match self.get_uint_value::<UInt32Type>(index)? {
                    Some(value) => Ok(Some(Cell::I64(value as i64))),
                    None => Ok(None),
                },
                DataType::UInt64 => match self.get_uint_value::<UInt64Type>(index)? {
                    Some(value) => Ok(Some(Cell::I64(value as i64))),
                    None => Ok(None),
                },
                unsupported => Err(DataTypeError::DataTypeMismatch(
                    unsupported.clone(),
                    PgOid::from(oid),
                )),
            },
            pg_sys::FLOAT4OID => match self.data_type() {
                DataType::Int8 => match self.get_primitive_value::<Int8Array>(index)? {
                    Some(value) => Ok(Some(Cell::F32(value as f32))),
                    None => Ok(None),
                },
                DataType::Int16 => match self.get_primitive_value::<Int16Array>(index)? {
                    Some(value) => Ok(Some(Cell::F32(value as f32))),
                    None => Ok(None),
                },
                DataType::Int32 => match self.get_primitive_value::<Int32Array>(index)? {
                    Some(value) => Ok(Some(Cell::F32(value as f32))),
                    None => Ok(None),
                },
                DataType::Int64 => match self.get_primitive_value::<Int64Array>(index)? {
                    Some(value) => Ok(Some(Cell::F32(value as f32))),
                    None => Ok(None),
                },
                DataType::UInt8 => match self.get_uint_value::<UInt8Type>(index)? {
                    Some(value) => Ok(Some(Cell::F32(value as f32))),
                    None => Ok(None),
                },
                DataType::UInt16 => match self.get_uint_value::<UInt16Type>(index)? {
                    Some(value) => Ok(Some(Cell::F32(value as f32))),
                    None => Ok(None),
                },
                DataType::UInt32 => match self.get_uint_value::<UInt32Type>(index)? {
                    Some(value) => Ok(Some(Cell::F32(value as f32))),
                    None => Ok(None),
                },
                DataType::UInt64 => match self.get_uint_value::<UInt64Type>(index)? {
                    Some(value) => Ok(Some(Cell::F32(value as f32))),
                    None => Ok(None),
                },
                DataType::Float16 => match self.get_primitive_value::<Float16Array>(index)? {
                    Some(value) => Ok(Some(Cell::F32(value.to_f32()))),
                    None => Ok(None),
                },
                DataType::Float32 => match self.get_primitive_value::<Float32Array>(index)? {
                    Some(value) => Ok(Some(Cell::F32(value as f32))),
                    None => Ok(None),
                },
                unsupported => Err(DataTypeError::DataTypeMismatch(
                    unsupported.clone(),
                    PgOid::from(oid),
                )),
            },
            pg_sys::FLOAT8OID => match self.data_type() {
                DataType::Int8 => match self.get_primitive_value::<Int8Array>(index)? {
                    Some(value) => Ok(Some(Cell::F64(value as f64))),
                    None => Ok(None),
                },
                DataType::Int16 => match self.get_primitive_value::<Int16Array>(index)? {
                    Some(value) => Ok(Some(Cell::F64(value as f64))),
                    None => Ok(None),
                },
                DataType::Int32 => match self.get_primitive_value::<Int32Array>(index)? {
                    Some(value) => Ok(Some(Cell::F64(value as f64))),
                    None => Ok(None),
                },
                DataType::Int64 => match self.get_primitive_value::<Int64Array>(index)? {
                    Some(value) => Ok(Some(Cell::F64(value as f64))),
                    None => Ok(None),
                },
                DataType::UInt8 => match self.get_uint_value::<UInt8Type>(index)? {
                    Some(value) => Ok(Some(Cell::F64(value as f64))),
                    None => Ok(None),
                },
                DataType::UInt16 => match self.get_uint_value::<UInt16Type>(index)? {
                    Some(value) => Ok(Some(Cell::F64(value as f64))),
                    None => Ok(None),
                },
                DataType::UInt32 => match self.get_uint_value::<UInt32Type>(index)? {
                    Some(value) => Ok(Some(Cell::F64(value as f64))),
                    None => Ok(None),
                },
                DataType::UInt64 => match self.get_uint_value::<UInt64Type>(index)? {
                    Some(value) => Ok(Some(Cell::F64(value as f64))),
                    None => Ok(None),
                },
                DataType::Float16 => match self.get_primitive_value::<Float16Array>(index)? {
                    Some(value) => Ok(Some(Cell::F64(value.to_f64()))),
                    None => Ok(None),
                },
                DataType::Float32 => match self.get_primitive_value::<Float32Array>(index)? {
                    Some(value) => Ok(Some(Cell::F64(value as f64))),
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
            pg_sys::TEXTOID | pg_sys::VARCHAROID | pg_sys::BPCHAROID => match self.data_type() {
                DataType::Utf8 => match self.get_primitive_value::<StringArray>(index)? {
                    Some(value) => Ok(Some(Cell::String(value.to_string()))),
                    None => Ok(None),
                },
                DataType::LargeUtf8 => match self.get_primitive_value::<LargeStringArray>(index)? {
                    Some(value) => Ok(Some(Cell::String(value.to_string()))),
                    None => Ok(None),
                },
                unsupported => Err(DataTypeError::DataTypeMismatch(
                    unsupported.clone(),
                    PgOid::from(oid),
                )),
            },
            pg_sys::DATEOID => match self.data_type() {
                DataType::Date32 => match self.get_date_value::<i32, Date32Type>(index)? {
                    Some(value) => Ok(Some(Cell::Date(value))),
                    None => Ok(None),
                },
                DataType::Date64 => match self.get_date_value::<i64, Date64Type>(index)? {
                    Some(value) => Ok(Some(Cell::Date(value))),
                    None => Ok(None),
                },
                unsupported => Err(DataTypeError::DataTypeMismatch(
                    unsupported.clone(),
                    PgOid::from(oid),
                )),
            },
            pg_sys::TIMESTAMPOID => match self.data_type() {
                DataType::Timestamp(TimeUnit::Nanosecond, None) => {
                    match self.get_timestamp_value::<TimestampNanosecondType>(index)? {
                        Some(value) => Ok(Some(Cell::Timestamp(value))),
                        None => Ok(None),
                    }
                }
                DataType::Timestamp(TimeUnit::Microsecond, None) => {
                    match self.get_timestamp_value::<TimestampMicrosecondType>(index)? {
                        Some(value) => Ok(Some(Cell::Timestamp(value))),
                        None => Ok(None),
                    }
                }
                DataType::Timestamp(TimeUnit::Millisecond, None) => {
                    match self.get_timestamp_value::<TimestampMillisecondType>(index)? {
                        Some(value) => Ok(Some(Cell::Timestamp(value))),
                        None => Ok(None),
                    }
                }
                DataType::Timestamp(TimeUnit::Second, None) => {
                    match self.get_timestamp_value::<TimestampSecondType>(index)? {
                        Some(value) => Ok(Some(Cell::Timestamp(value))),
                        None => Ok(None),
                    }
                }
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
impl GetUIntValue for ArrayRef {}

#[derive(Error, Debug)]
pub enum DataTypeError {
    #[error(transparent)]
    DatetimeError(#[from] DatetimeError),

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

    #[error("Failed to convert UInt to i64")]
    UIntConversionError,

    #[error("Postgres data type {0:?} is not supported")]
    UnsupportedPostgresType(PgOid),
}

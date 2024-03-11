use deltalake::arrow::array::{
    Array, ArrayAccessor, ArrayRef, ArrowPrimitiveType, AsArray, BooleanArray, Date32Array,
    Float32Array, Float64Array, Int16Array, Int32Array, Int64Array, StringArray,
};
use deltalake::arrow::datatypes::{
    Date32Type, Decimal128Type, Float32Type, Float64Type, Int16Type, Int32Type, Int64Type,
    Time64NanosecondType, TimestampMicrosecondType, TimestampMillisecondType, TimestampSecondType,
};
use deltalake::datafusion::arrow::datatypes::DataType::*;
use deltalake::datafusion::arrow::datatypes::{DataType, TimeUnit};
use pgrx::*;
use std::fmt::Debug;
use thiserror::Error;

use super::datatype::DataTypeError;
use super::date::DayUnix;
use super::numeric::{PgNumeric, PgNumericTypeMod, PgPrecision, PgScale};
use super::time::NanosecondDay;
use super::timestamp::{MicrosecondUnix, MillisecondUnix, SecondUnix};

pub trait GetDatumGeneric
where
    Self: Array + AsArray,
{
    fn get_generic_datum<A>(&self, index: usize) -> Result<Option<pg_sys::Datum>, DatumError>
    where
        A: Array + Debug + 'static,
        for<'a> &'a A: ArrayAccessor + IntoIterator,
        for<'a> <&'a A as IntoIterator>::Item: IntoDatum,
    {
        let value = self
            .as_any()
            .downcast_ref::<A>()
            .ok_or(DatumError::DowncastGenericArray(format!("{:?}", self)))?
            .into_iter()
            .nth(index);

        Ok(value.into_datum())
    }
}

pub trait GetDatumPrimitive
where
    Self: Array + AsArray,
{
    fn get_primitive_datum<A>(&self, index: usize) -> Result<Option<pg_sys::Datum>, DatumError>
    where
        A: ArrowPrimitiveType,
        A::Native: IntoDatum,
    {
        let value = self.as_primitive::<A>().iter().nth(index);
        Ok(value.into_datum())
    }
}

pub trait GetDatumPrimitiveList
where
    Self: Array + AsArray,
{
    fn get_primitive_list_datum<A>(&self, index: usize) -> Result<Option<pg_sys::Datum>, DatumError>
    where
        A: Array + Debug + 'static,
        for<'a> &'a A: IntoIterator,
        for<'a> <&'a A as IntoIterator>::Item: IntoDatum,
    {
        match self.as_list::<i32>().iter().nth(index) {
            Some(Some(list)) => Ok(list
                .as_any()
                .downcast_ref::<A>()
                .ok_or(DatumError::DowncastGenericArray(format!("{:?}", self)))?
                .into_iter()
                .collect::<Vec<_>>()
                .into_datum()),
            _ => Ok(None),
        }
    }
}

pub trait GetDatumDate
where
    Self: Array + AsArray,
{
    fn get_date_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        match self.as_primitive::<Date32Type>().iter().nth(index) {
            Some(Some(value)) => Ok(datum::Date::try_from(DayUnix(value)).into_datum()),
            _ => Ok(None),
        }
    }
}

pub trait GetDatumTimestampMicrosecond
where
    Self: Array + AsArray,
{
    fn get_ts_micro_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        match self
            .as_primitive::<TimestampMicrosecondType>()
            .iter()
            .nth(index)
        {
            Some(Some(value)) => {
                Ok(datum::Timestamp::try_from(MicrosecondUnix(value)).into_datum())
            }
            _ => Ok(None),
        }
    }
}

pub trait GetDatumTimestampMillisecond
where
    Self: Array + AsArray,
{
    fn get_ts_milli_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        match self
            .as_primitive::<TimestampMillisecondType>()
            .iter()
            .nth(index)
        {
            Some(Some(value)) => {
                Ok(datum::Timestamp::try_from(MillisecondUnix(value)).into_datum())
            }
            _ => Ok(None),
        }
    }
}

pub trait GetDatumTimestampSecond
where
    Self: Array + AsArray,
{
    fn get_ts_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        match self.as_primitive::<TimestampSecondType>().iter().nth(index) {
            Some(Some(value)) => Ok(datum::Timestamp::try_from(SecondUnix(value)).into_datum()),
            _ => Ok(None),
        }
    }
}

pub trait GetDatumTime
where
    Self: Array + AsArray,
{
    fn get_time_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        match self
            .as_primitive::<Time64NanosecondType>()
            .iter()
            .nth(index)
        {
            Some(Some(value)) => Ok(datum::Time::try_from(NanosecondDay(value)).into_datum()),
            _ => Ok(None),
        }
    }
}

pub trait GetDatumNumeric
where
    Self: Array + AsArray,
{
    fn get_numeric_datum(
        &self,
        index: usize,
        precision: &u8,
        scale: &i8,
    ) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        match self.as_primitive::<Decimal128Type>().iter().nth(index) {
            Some(Some(value)) => Ok(AnyNumeric::try_from(PgNumeric(
                AnyNumeric::from(value),
                PgNumericTypeMod(PgPrecision(*precision), PgScale(*scale)),
            ))
            .into_datum()),
            _ => Ok(None),
        }
    }
}

pub trait GetDatum
where
    Self: Array
        + AsArray
        + GetDatumDate
        + GetDatumGeneric
        + GetDatumPrimitive
        + GetDatumPrimitiveList
        + GetDatumNumeric
        + GetDatumTimestampMicrosecond
        + GetDatumTimestampMillisecond
        + GetDatumTimestampSecond
        + GetDatumTime,
{
    fn get_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        let result = match self.data_type() {
            Boolean => self.get_generic_datum::<BooleanArray>(index)?,
            Utf8 => self.get_generic_datum::<StringArray>(index)?,
            Int16 => self.get_primitive_datum::<Int16Type>(index)?,
            Int32 => self.get_primitive_datum::<Int32Type>(index)?,
            Int64 => self.get_primitive_datum::<Int64Type>(index)?,
            Float32 => self.get_primitive_datum::<Float32Type>(index)?,
            Float64 => self.get_primitive_datum::<Float64Type>(index)?,
            Date32 => self.get_date_datum(index)?,
            Time64(TimeUnit::Nanosecond) => self.get_time_datum(index)?,
            Timestamp(TimeUnit::Microsecond, None) => self.get_ts_micro_datum(index)?,
            Timestamp(TimeUnit::Millisecond, None) => self.get_ts_milli_datum(index)?,
            Timestamp(TimeUnit::Second, None) => self.get_ts_datum(index)?,
            Timestamp(TimeUnit::Microsecond, Some(timezone)) if timezone.as_ref() == "UTC" => {
                self.get_ts_micro_datum(index)?
            }
            Decimal128(p, s) => self.get_numeric_datum(index, p, s)?,
            List(ref field) => match field.data_type() {
                Boolean => self.get_primitive_list_datum::<BooleanArray>(index)?,
                Utf8 => self.get_primitive_list_datum::<StringArray>(index)?,
                Int16 => self.get_primitive_list_datum::<Int16Array>(index)?,
                Int32 => self.get_primitive_list_datum::<Int32Array>(index)?,
                Int64 => self.get_primitive_list_datum::<Int64Array>(index)?,
                Float32 => self.get_primitive_list_datum::<Float32Array>(index)?,
                Float64 => self.get_primitive_list_datum::<Float64Array>(index)?,
                Date32 => self.get_primitive_list_datum::<Date32Array>(index)?,
                unsupported => {
                    return Err(DatumError::UnsupportedArrowArrayType(unsupported.clone()).into())
                }
            },
            unsupported => return Err(DatumError::UnsupportedArrowType(unsupported.clone()).into()),
        };

        Ok(result)
    }
}

impl GetDatum for ArrayRef {}
impl GetDatumDate for ArrayRef {}
impl GetDatumGeneric for ArrayRef {}
impl GetDatumPrimitive for ArrayRef {}
impl GetDatumPrimitiveList for ArrayRef {}
impl GetDatumNumeric for ArrayRef {}
impl GetDatumTimestampMicrosecond for ArrayRef {}
impl GetDatumTimestampMillisecond for ArrayRef {}
impl GetDatumTimestampSecond for ArrayRef {}
impl GetDatumTime for ArrayRef {}

#[derive(Error, Debug)]
pub enum DatumError {
    #[error("Could not downcast arrow array {0}")]
    DowncastGenericArray(String),

    #[error("Could not convert arrow type {0:?} to Postgres type")]
    UnsupportedArrowType(DataType),

    #[error("Could not convert arrow array with type {0:?} to Postgres array")]
    UnsupportedArrowArrayType(DataType),
}

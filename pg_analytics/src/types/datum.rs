use deltalake::arrow::array::{
    Array, ArrayAccessor, ArrayRef, ArrowPrimitiveType, AsArray, BooleanArray, Date32Array,
    Float32Array, Float64Array, Int16Array, Int32Array, Int64Array, StringArray,
};
use deltalake::arrow::datatypes::{
    Date32Type, Decimal128Type, Float32Type, Float64Type, Int16Type, Int32Type, Int64Type,
    TimestampMicrosecondType, TimestampMillisecondType, TimestampSecondType,
};
use deltalake::datafusion::arrow::datatypes::DataType::*;
use deltalake::datafusion::arrow::datatypes::{DataType, TimeUnit};
use pgrx::*;
use std::fmt::Debug;
use thiserror::Error;

use super::datatype::DataTypeError;
use super::date::DayUnix;
use super::numeric::{PgNumeric, PgNumericTypeMod, PgPrecision, PgScale};
use super::timestamp::{MicrosecondUnix, MillisecondUnix, SecondUnix};

pub trait GetDatumGeneric
where
    Self: Array + AsArray,
{
    fn get_generic_datum<A>(&self, index: usize) -> Result<Option<pg_sys::Datum>, DatumError>
    where
        A: Array + Debug + 'static,
        for<'a> &'a A: ArrayAccessor,
        for<'a> <&'a A as ArrayAccessor>::Item: IntoDatum,
    {
        Ok(self
            .as_any()
            .downcast_ref::<A>()
            .ok_or(DatumError::DowncastGenericArray(format!("{:?}", self)))?
            .value(index)
            .into_datum())
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
        Ok(self.as_primitive::<A>().value(index).into_datum())
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
        let list = self.as_list::<i32>().value(index);
        let datum = list
            .as_any()
            .downcast_ref::<A>()
            .ok_or(DatumError::DowncastGenericArray(format!("{:?}", self)))?
            .into_iter()
            .collect::<Vec<_>>()
            .into_datum();

        Ok(datum)
    }
}

pub trait GetDatumDate
where
    Self: Array + AsArray,
{
    fn get_date_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        Ok(
            datum::Date::try_from(DayUnix(self.as_primitive::<Date32Type>().value(index)))
                .into_datum(),
        )
    }
}

pub trait GetDatumTimestampMicrosecond
where
    Self: Array + AsArray,
{
    fn get_ts_micro_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        Ok(datum::Timestamp::try_from(MicrosecondUnix(
            self.as_primitive::<TimestampMicrosecondType>().value(index),
        ))
        .into_datum())
    }
}

pub trait GetDatumTimestampMillisecond
where
    Self: Array + AsArray,
{
    fn get_ts_milli_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        Ok(datum::Timestamp::try_from(MillisecondUnix(
            self.as_primitive::<TimestampMillisecondType>().value(index),
        ))
        .into_datum())
    }
}

pub trait GetDatumTimestampSecond
where
    Self: Array + AsArray,
{
    fn get_ts_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        Ok(datum::Timestamp::try_from(SecondUnix(
            self.as_primitive::<TimestampSecondType>().value(index),
        ))
        .into_datum())
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
        let numeric = AnyNumeric::try_from(PgNumeric(
            AnyNumeric::from(self.as_primitive::<Decimal128Type>().value(index)),
            PgNumericTypeMod(PgPrecision(*precision), PgScale(*scale)),
        ))
        .into_datum();

        Ok(numeric)
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
        + GetDatumTimestampSecond,
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
            Timestamp(TimeUnit::Microsecond, None) => self.get_ts_micro_datum(index)?,
            Timestamp(TimeUnit::Millisecond, None) => self.get_ts_milli_datum(index)?,
            Timestamp(TimeUnit::Second, None) => self.get_ts_datum(index)?,
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

#[derive(Error, Debug)]
pub enum DatumError {
    #[error("Could not downcast arrow array {0}")]
    DowncastGenericArray(String),

    #[error("Could not convert arrow type {0:?} to Postgres type")]
    UnsupportedArrowType(DataType),

    #[error("Could not convert arrow array with type {0:?} to Postgres array")]
    UnsupportedArrowArrayType(DataType),
}

use deltalake::arrow::{
    array::{Array, ArrayRef, ArrowPrimitiveType, AsArray, BooleanArray, StringArray},
    datatypes::{
        Date32Type, Decimal128Type, Float32Type, Float64Type, Int32Type, Int64Type,
        TimestampMicrosecondType, TimestampMillisecondType, TimestampSecondType,
    },
};
use deltalake::datafusion::arrow::datatypes::{DataType, TimeUnit};
use pgrx::*;

use super::datatype::DataTypeError;
use super::numeric::{PgNumeric, PgNumericTypeMod, PgPrecision, PgScale};
use super::timestamp::{MicrosecondUnix, MillisecondUnix, SecondUnix};

pub trait GetDatumPrimitive
where
    Self: Array + AsArray,
{
    fn get_primitive_datum<A>(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError>
    where
        A: ArrowPrimitiveType,
        A::Native: IntoDatum,
    {
        Ok(self.as_primitive::<A>().value(index).into_datum())
    }
}

pub trait GetDatumBoolean
where
    Self: Array + AsArray,
{
    fn get_boolean_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        Ok(self
            .as_any()
            .downcast_ref::<BooleanArray>()
            .ok_or(DataTypeError::DowncastGenericArray(DataType::Boolean))?
            .value(index)
            .into_datum())
    }
}

pub trait GetDatumString
where
    Self: Array + AsArray,
{
    fn get_string_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        Ok(self
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or(DataTypeError::DowncastGenericArray(DataType::Utf8))?
            .value(index)
            .into_datum())
    }
}

pub trait GetDatumTimestampMicrosecond
where
    Self: Array + AsArray,
{
    fn get_ts_micro_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        Ok(
            MicrosecondUnix(self.as_primitive::<TimestampMicrosecondType>().value(index))
                .try_into()?,
        )
    }
}

pub trait GetDatumTimestampMillisecond
where
    Self: Array + AsArray,
{
    fn get_ts_milli_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        Ok(
            MillisecondUnix(self.as_primitive::<TimestampMillisecondType>().value(index))
                .try_into()?,
        )
    }
}

pub trait GetDatumTimestampSecond
where
    Self: Array + AsArray,
{
    fn get_ts_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        Ok(SecondUnix(self.as_primitive::<TimestampSecondType>().value(index)).try_into()?)
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
        let numeric = PgNumeric(
            AnyNumeric::from(self.as_primitive::<Decimal128Type>().value(index)),
            PgNumericTypeMod(PgPrecision(*precision), PgScale(*scale)),
        )
        .try_into()?;

        Ok(numeric)
    }
}

pub trait GetDatum
where
    Self: Array
        + AsArray
        + GetDatumBoolean
        + GetDatumString
        + GetDatumPrimitive
        + GetDatumTimestampMicrosecond
        + GetDatumTimestampMillisecond
        + GetDatumTimestampSecond
        + GetDatumNumeric,
{
    fn get_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        let result = match self.data_type() {
            DataType::Boolean => self.get_boolean_datum(index)?,
            DataType::Utf8 => self.get_string_datum(index)?,
            DataType::Int32 => self.get_primitive_datum::<Int32Type>(index)?,
            DataType::Int64 => self.get_primitive_datum::<Int64Type>(index)?,
            DataType::Float32 => self.get_primitive_datum::<Float32Type>(index)?,
            DataType::Float64 => self.get_primitive_datum::<Float64Type>(index)?,
            DataType::Date32 => self.get_primitive_datum::<Date32Type>(index)?,
            DataType::Timestamp(TimeUnit::Microsecond, None) => self.get_ts_micro_datum(index)?,
            DataType::Timestamp(TimeUnit::Millisecond, None) => self.get_ts_milli_datum(index)?,
            DataType::Timestamp(TimeUnit::Second, None) => self.get_ts_datum(index)?,
            DataType::Decimal128(p, s) => self.get_numeric_datum(index, p, s)?,
            _ => return Ok(None),
        };

        Ok(result)
    }
}

impl GetDatumBoolean for ArrayRef {}
impl GetDatumString for ArrayRef {}
impl GetDatumPrimitive for ArrayRef {}
impl GetDatumTimestampMicrosecond for ArrayRef {}
impl GetDatumTimestampMillisecond for ArrayRef {}
impl GetDatumTimestampSecond for ArrayRef {}
impl GetDatumNumeric for ArrayRef {}
impl GetDatum for ArrayRef {}

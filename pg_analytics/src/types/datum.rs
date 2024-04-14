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
use pgrx::pg_sys::BuiltinOid::*;
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

pub trait GetDatumUuid
where
    Self: Array + AsArray,
{
    fn get_uuid_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DatumError> {
        let value = self
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or(DatumError::DowncastGenericArray(format!("{:?}", self)))?
            .into_iter()
            .nth(index);

        match value {
            Some(Some(value)) => {
                let uuid = uuid::Uuid::parse_str(value)?;
                Ok(datum::Uuid::from_slice(uuid.as_bytes()).into_datum())
            }
            _ => Ok(value.into_datum()),
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
        + GetDatumTime
        + GetDatumUuid,
{
    fn get_datum(&self, index: usize, oid: PgOid) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        let result = match oid {
            PgOid::BuiltIn(builtin) => match builtin {
                BOOLOID => self.get_generic_datum::<BooleanArray>(index)?,
                TEXTOID => self.get_generic_datum::<StringArray>(index)?,
                VARCHAROID => self.get_generic_datum::<StringArray>(index)?,
                BPCHAROID => self.get_generic_datum::<StringArray>(index)?,
                INT2OID => self.get_primitive_datum::<Int16Type>(index)?,
                INT4OID => self.get_primitive_datum::<Int32Type>(index)?,
                INT8OID => self.get_primitive_datum::<Int64Type>(index)?,
                FLOAT4OID => self.get_primitive_datum::<Float32Type>(index)?,
                FLOAT8OID => self.get_primitive_datum::<Float64Type>(index)?,
                DATEOID => self.get_date_datum(index)?,
                TIMEOID => self.get_time_datum(index)?,
                TIMESTAMPOID => match self.data_type() {
                    Timestamp(TimeUnit::Microsecond, None) => self.get_ts_micro_datum(index)?,
                    Timestamp(TimeUnit::Millisecond, None) => self.get_ts_milli_datum(index)?,
                    Timestamp(TimeUnit::Second, None) => self.get_ts_datum(index)?,
                    unsupported => {
                        return Err(DatumError::TimestampError(unsupported.clone()).into())
                    }
                },
                NUMERICOID => match self.data_type() {
                    Decimal128(p, s) => self.get_numeric_datum(index, p, s)?,
                    Float32 => match self.as_primitive::<Float32Type>().iter().nth(index) {
                        Some(Some(value)) => AnyNumeric::from(value as i128).into_datum(),
                        _ => None,
                    },
                    Float64 => match self.as_primitive::<Float64Type>().iter().nth(index) {
                        Some(Some(value)) => AnyNumeric::from(value as i128).into_datum(),
                        _ => None,
                    },
                    unsupported => return Err(DatumError::NumericError(unsupported.clone()).into()),
                },
                UUIDOID => self.get_uuid_datum(index)?,
                BOOLARRAYOID => self.get_primitive_list_datum::<BooleanArray>(index)?,
                TEXTARRAYOID => self.get_primitive_list_datum::<StringArray>(index)?,
                VARCHARARRAYOID => self.get_primitive_list_datum::<StringArray>(index)?,
                BPCHARARRAYOID => self.get_primitive_list_datum::<StringArray>(index)?,
                INT2ARRAYOID => self.get_primitive_list_datum::<Int16Array>(index)?,
                INT4ARRAYOID => self.get_primitive_list_datum::<Int32Array>(index)?,
                INT8ARRAYOID => self.get_primitive_list_datum::<Int64Array>(index)?,
                FLOAT4ARRAYOID => self.get_primitive_list_datum::<Float32Array>(index)?,
                FLOAT8ARRAYOID => self.get_primitive_list_datum::<Float64Array>(index)?,
                DATEARRAYOID => self.get_primitive_list_datum::<Date32Array>(index)?,
                unsupported => return Err(DataTypeError::UnsupportedPostgresType(unsupported)),
            },
            PgOid::Invalid => return Err(DataTypeError::InvalidPostgresOid),
            PgOid::Custom(_) => return Err(DataTypeError::UnsupportedCustomType),
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
impl GetDatumUuid for ArrayRef {}

#[derive(Error, Debug)]
pub enum DatumError {
    #[error(transparent)]
    UuidError(#[from] uuid::Error),

    #[error("Could not downcast arrow array {0}")]
    DowncastGenericArray(String),

    #[error("Error converting {0:?} into NUMERIC")]
    NumericError(DataType),

    #[error("Error converting {0:?} into TIMESTAMP")]
    TimestampError(DataType),
}

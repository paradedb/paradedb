use deltalake::arrow::array::{Array, *};
use deltalake::arrow::datatypes::*;
use deltalake::datafusion::arrow::datatypes::DataType::*;
use deltalake::datafusion::common::DataFusionError;
use pgrx::pg_sys::BuiltinOid::*;
use pgrx::*;
use std::fmt::Debug;
use thiserror::Error;

use super::datatype::DataTypeError;
use super::date::DayUnix;
use super::numeric::{PgNumeric, PgNumericTypeMod, PgPrecision, PgScale};
use super::time::NanosecondDay;
use super::timestamp::{MicrosecondUnix, MillisecondUnix, SecondUnix};

pub trait GetDatumPrimitive
where
    Self: Array + AsArray,
{
    fn get_primitive_datum<A>(&self, index: usize) -> Result<Option<pg_sys::Datum>, DatumError>
    where
        A: Array + Debug + 'static,
        for<'a> &'a A: ArrayAccessor + IntoIterator,
        for<'a> <&'a A as ArrayAccessor>::Item: IntoDatum,
    {
        let downcast_array = downcast_value!(self, A);
        match downcast_array.is_null(index) {
            false => Ok(downcast_array.value(index).into_datum()),
            true => Ok(None),
        }
    }
}

pub trait GetDatumUInt
where
    Self: Array + AsArray,
{
    fn get_uint_datum<A>(&self, index: usize) -> Result<Option<pg_sys::Datum>, DatumError>
    where
        A: ArrowPrimitiveType,
        i64: TryFrom<A::Native>,
    {
        let downcast_array = self.as_primitive::<A>();
        match downcast_array.is_null(index) {
            false => {
                let value: A::Native = downcast_array.value(index);
                Ok(i64::try_from(value)
                    .map_err(|_| DatumError::UIntConversionError)?
                    .into_datum())
            }
            true => Ok(None),
        }
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
        let downcast_array = self.as_list::<i32>();
        match downcast_array.is_null(index) {
            false => Ok(downcast_value!(downcast_array.value(index), A)
                .into_iter()
                .collect::<Vec<_>>()
                .into_datum()),
            true => Ok(None),
        }
    }
}

pub trait GetDatumDate
where
    Self: Array + AsArray,
{
    fn get_date_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        let downcast_array = downcast_value!(self, Date32Array);
        match downcast_array.is_null(index) {
            false => Ok(datum::Date::try_from(DayUnix(downcast_array.value(index))).into_datum()),
            true => Ok(None),
        }
    }
}

pub trait GetDatumTimestampMicrosecond
where
    Self: Array + AsArray,
{
    fn get_ts_micro_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        let downcast_array = downcast_value!(self, TimestampMicrosecondArray);
        match downcast_array.is_null(index) {
            false => Ok(
                datum::Timestamp::try_from(MicrosecondUnix(downcast_array.value(index)))
                    .into_datum(),
            ),
            true => Ok(None),
        }
    }
}

pub trait GetDatumTimestampMillisecond
where
    Self: Array + AsArray,
{
    fn get_ts_milli_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        let downcast_array = downcast_value!(self, TimestampMillisecondArray);
        match downcast_array.is_null(index) {
            false => Ok(
                datum::Timestamp::try_from(MillisecondUnix(downcast_array.value(index)))
                    .into_datum(),
            ),
            true => Ok(None),
        }
    }
}

pub trait GetDatumTimestampSecond
where
    Self: Array + AsArray,
{
    fn get_ts_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        let downcast_array = downcast_value!(self, TimestampSecondArray);
        match downcast_array.is_null(index) {
            false => Ok(
                datum::Timestamp::try_from(SecondUnix(downcast_array.value(index))).into_datum(),
            ),
            true => Ok(None),
        }
    }
}

pub trait GetDatumTime
where
    Self: Array + AsArray,
{
    fn get_time_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        let downcast_array = downcast_value!(self, Time64NanosecondArray);
        match downcast_array.is_null(index) {
            false => {
                Ok(datum::Time::try_from(NanosecondDay(downcast_array.value(index))).into_datum())
            }
            true => Ok(None),
        }
    }
}

pub trait GetDatumNumericFromDecimal
where
    Self: Array + AsArray,
{
    fn get_numeric_datum_from_decimal(
        &self,
        index: usize,
        precision: &u8,
        scale: &i8,
    ) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        let downcast_array = downcast_value!(self, Decimal128Array);
        match downcast_array.is_null(index) {
            false => {
                let value = downcast_array.value(index);
                Ok(AnyNumeric::try_from(PgNumeric(
                    AnyNumeric::from(value),
                    PgNumericTypeMod(PgPrecision(*precision), PgScale(*scale)),
                ))
                .into_datum())
            }
            true => Ok(None),
        }
    }
}

pub trait GetDatumNumeric
where
    Self: Array + AsArray,
{
    fn get_numeric_datum<A>(
        &self,
        index: usize,
        typemod: i32,
        func: unsafe fn(pg_sys::FunctionCallInfo) -> pg_sys::Datum,
    ) -> Result<Option<pg_sys::Datum>, DatumError>
    where
        A: Array + Debug + 'static,
        for<'a> &'a A: ArrayAccessor + IntoIterator,
        for<'a> <&'a A as ArrayAccessor>::Item: IntoDatum,
    {
        let downcast_array = downcast_value!(self, A);
        match downcast_array.is_null(index) {
            false => {
                let numeric: Option<AnyNumeric> = unsafe {
                    direct_function_call(
                        func,
                        &[
                            downcast_array.value(index).into_datum(),
                            typemod.into_datum(),
                        ],
                    )
                };
                Ok(numeric.into_datum())
            }
            true => Ok(None),
        }
    }
}

pub trait GetDatumUuid
where
    Self: Array + AsArray,
{
    fn get_uuid_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DatumError> {
        let downcast_array = downcast_value!(self, StringArray);
        match downcast_array.is_null(index) {
            false => {
                let value = downcast_array.value(index);
                let uuid = uuid::Uuid::parse_str(value)?;
                Ok(datum::Uuid::from_slice(uuid.as_bytes()).into_datum())
            }
            true => Ok(None),
        }
    }
}

pub trait GetDatum
where
    Self: Array
        + AsArray
        + GetDatumDate
        + GetDatumPrimitive
        + GetDatumPrimitiveList
        + GetDatumNumeric
        + GetDatumNumericFromDecimal
        + GetDatumTimestampMicrosecond
        + GetDatumTimestampMillisecond
        + GetDatumTimestampSecond
        + GetDatumTime
        + GetDatumUInt
        + GetDatumUuid,
{
    fn get_datum(
        &self,
        index: usize,
        oid: PgOid,
        typemod: i32,
    ) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        let result = match oid {
            PgOid::BuiltIn(builtin) => match builtin {
                BOOLOID => self.get_primitive_datum::<BooleanArray>(index)?,
                TEXTOID | VARCHAROID | BPCHAROID => {
                    self.get_primitive_datum::<StringArray>(index)?
                }
                INT2OID | INT4OID | INT8OID | FLOAT4OID | FLOAT8OID => match self.data_type() {
                    Float32 => self.get_primitive_datum::<Float32Type>(index)?,
                    Float64 => self.get_primitive_datum::<Float64Type>(index)?,
                    Int8 => self.get_primitive_datum::<Int8Type>(index)?,
                    Int16 => self.get_primitive_datum::<Int16Type>(index)?,
                    Int32 => self.get_primitive_datum::<Int32Type>(index)?,
                    Int64 => self.get_primitive_datum::<Int64Type>(index)?,
                    UInt8 => self.get_uint_datum::<UInt8Type>(index)?,
                    UInt16 => self.get_uint_datum::<UInt16Type>(index)?,
                    UInt32 => self.get_uint_datum::<UInt32Type>(index)?,
                    UInt64 => self.get_uint_datum::<UInt64Type>(index)?,
                    unsupported => {
                        return Err(DatumError::IntError(unsupported.clone(), oid).into())
                    }
                },
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
                    Decimal128(p, s) => self.get_numeric_datum_from_decimal(index, p, s)?,
                    Float32 => self.get_numeric_datum::<Float32Array>(
                        index,
                        typemod,
                        pg_sys::float4_numeric,
                    )?,
                    Float64 => self.get_numeric_datum::<Float64Array>(
                        index,
                        typemod,
                        pg_sys::float8_numeric,
                    )?,
                    Int16 => {
                        self.get_numeric_datum::<Int16Array>(index, typemod, pg_sys::int2_numeric)?
                    }
                    Int32 => {
                        self.get_numeric_datum::<Int32Array>(index, typemod, pg_sys::int4_numeric)?
                    }
                    Int64 => {
                        self.get_numeric_datum::<Int64Array>(index, typemod, pg_sys::int8_numeric)?
                    }
                    unsupported => return Err(DatumError::NumericError(unsupported.clone()).into()),
                },
                UUIDOID => self.get_uuid_datum(index)?,
                BOOLARRAYOID => self.get_primitive_list_datum::<BooleanArray>(index)?,
                TEXTARRAYOID | VARCHARARRAYOID | BPCHARARRAYOID => {
                    self.get_primitive_list_datum::<StringArray>(index)?
                }
                INT2ARRAYOID | INT4ARRAYOID | INT8ARRAYOID | FLOAT4ARRAYOID | FLOAT8ARRAYOID => {
                    match self.data_type() {
                        List(ref field) => match field.data_type().clone() {
                            Float32 => self.get_primitive_list_datum::<Float32Array>(index)?,
                            Float64 => self.get_primitive_list_datum::<Float64Array>(index)?,
                            Int16 => self.get_primitive_list_datum::<Int16Array>(index)?,
                            Int32 => self.get_primitive_list_datum::<Int32Array>(index)?,
                            Int64 => self.get_primitive_list_datum::<Int64Array>(index)?,
                            unsupported => {
                                return Err(
                                    DatumError::IntArrayError(unsupported.clone(), oid).into()
                                )
                            }
                        },
                        unsupported => {
                            return Err(DatumError::IntArrayError(unsupported.clone(), oid).into())
                        }
                    }
                }
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
impl GetDatumPrimitive for ArrayRef {}
impl GetDatumPrimitiveList for ArrayRef {}
impl GetDatumNumeric for ArrayRef {}
impl GetDatumNumericFromDecimal for ArrayRef {}
impl GetDatumTimestampMicrosecond for ArrayRef {}
impl GetDatumTimestampMillisecond for ArrayRef {}
impl GetDatumTimestampSecond for ArrayRef {}
impl GetDatumTime for ArrayRef {}
impl GetDatumUInt for ArrayRef {}
impl GetDatumUuid for ArrayRef {}

#[derive(Error, Debug)]
pub enum DatumError {
    #[error(transparent)]
    DataFusion(#[from] DataFusionError),

    #[error(transparent)]
    UuidError(#[from] uuid::Error),

    #[error("Error converting {0:?} into {1:?}")]
    IntError(DataType, PgOid),

    #[error("Error converting {0:?} array into {1:?}")]
    IntArrayError(DataType, PgOid),

    #[error("Error converting {0:?} into NUMERIC")]
    NumericError(DataType),

    #[error("Error converting {0:?} into TIMESTAMP")]
    TimestampError(DataType),

    #[error("Failed to convert UInt to i64")]
    UIntConversionError,
}

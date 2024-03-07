use deltalake::arrow::array::{
    Array, ArrayRef, BooleanArray, BooleanBuilder, Date32Array, Decimal128Array, Float32Array,
    Float64Array, GenericByteBuilder, Int16Array, Int32Array, Int64Array, ListBuilder,
    PrimitiveBuilder, StringArray, TimestampMicrosecondArray, TimestampMillisecondArray,
    TimestampSecondArray,
};
use deltalake::arrow::datatypes::{
    ArrowPrimitiveType, Float32Type, Float64Type, GenericStringType, Int16Type, Int32Type,
    Int64Type,
};
use pgrx::pg_sys::BuiltinOid::*;
use pgrx::*;
use std::sync::Arc;

use super::datatype::{DataTypeError, PgTypeMod};
use super::date::DayUnix;
use super::numeric::{scale_anynumeric, PgNumericTypeMod, PgPrecision, PgScale};
use super::timestamp::{MicrosecondUnix, MillisecondUnix, PgTimestampPrecision, SecondUnix};

type Column<T> = Vec<Option<T>>;

pub trait IntoPrimitiveArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized,
{
    fn into_array<T>(self) -> Result<Vec<Option<T>>, DataTypeError>
    where
        T: FromDatum,
    {
        let array = self
            .map(|(datum, is_null)| {
                (!is_null)
                    .then_some(datum)
                    .and_then(|datum| unsafe { T::from_datum(datum, false) })
            })
            .collect::<Vec<Option<T>>>();

        Ok(array)
    }
}

pub trait IntoPrimitiveArrowArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized + IntoPrimitiveArray,
{
    fn into_primitive_arrow_array<T, A>(self) -> Result<ArrayRef, DataTypeError>
    where
        T: FromDatum,
        A: Array + FromIterator<Option<T>> + 'static,
    {
        Ok(Arc::new(A::from_iter(self.into_array::<T>()?)))
    }
}

pub trait IntoNumericArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized,
{
    fn into_numeric_array(
        self,
        precision: u8,
        scale: i8,
    ) -> Result<Vec<Option<i128>>, DataTypeError> {
        let array = self
            .map(|(datum, is_null)| {
                (!is_null).then_some(datum).and_then(|datum| {
                    unsafe { AnyNumeric::from_datum(datum, false) }.map(|numeric| {
                        i128::try_from(
                            scale_anynumeric(numeric, precision, scale, true)
                                .unwrap_or_else(|err| panic!("{}", err)),
                        )
                        .unwrap_or_else(|err| panic!("{}", err))
                    })
                })
            })
            .collect::<Vec<Option<i128>>>();

        Ok(array)
    }
}

pub trait IntoNumericArrowArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized,
{
    fn into_numeric_arrow_array(self, typemod: PgTypeMod) -> Result<ArrayRef, DataTypeError> {
        let PgNumericTypeMod(PgPrecision(precision), PgScale(scale)) = typemod.try_into()?;
        let iter = self.into_numeric_array(precision, scale)?;

        Ok(Arc::new(
            Decimal128Array::from_iter(iter).with_precision_and_scale(precision, scale)?,
        ))
    }
}

pub trait IntoDateArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized,
{
    fn into_date_array(self) -> Result<Vec<Option<i32>>, DataTypeError> {
        let array = self
            .filter_map(|(datum, is_null)| {
                (!is_null).then_some(datum).map(|datum| {
                    unsafe { datum::Date::from_datum(datum, false) }
                        .and_then(|date| DayUnix::try_from(date).ok())
                        .map(|DayUnix(unix)| unix)
                })
            })
            .collect::<Vec<Option<i32>>>();

        Ok(array)
    }
}

pub trait IntoDateArrowArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized,
{
    fn into_date_arrow_array(self) -> Result<ArrayRef, DataTypeError> {
        Ok(Arc::new(Date32Array::from_iter(self.into_date_array()?)))
    }
}

pub trait IntoTimestampMicrosecondArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized,
{
    fn into_ts_micro_array(self) -> Result<Vec<Option<i64>>, DataTypeError> {
        let array = self
            .filter_map(|(datum, is_null)| {
                (!is_null).then_some(datum).map(|datum| {
                    unsafe { datum::Timestamp::from_datum(datum, false) }
                        .and_then(|timestamp| MicrosecondUnix::try_from(timestamp).ok())
                        .map(|MicrosecondUnix(unix)| unix)
                })
            })
            .collect::<Vec<Option<i64>>>();

        Ok(array)
    }
}

pub trait IntoTimestampMicrosecondArrowArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized,
{
    fn into_ts_micro_arrow_array(self) -> Result<ArrayRef, DataTypeError> {
        Ok(Arc::new(TimestampMicrosecondArray::from_iter(
            self.into_ts_micro_array()?,
        )))
    }
}

pub trait IntoTimestampMillisecondArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized,
{
    fn into_ts_milli_array(self) -> Result<Vec<Option<i64>>, DataTypeError> {
        let array = self
            .filter_map(|(datum, is_null)| {
                (!is_null).then_some(datum).map(|datum| {
                    unsafe { datum::Timestamp::from_datum(datum, false) }
                        .and_then(|timestamp| MillisecondUnix::try_from(timestamp).ok())
                        .map(|MillisecondUnix(unix)| unix)
                })
            })
            .collect::<Vec<Option<i64>>>();

        Ok(array)
    }
}

pub trait IntoTimestampMillisecondArrowArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized,
{
    fn into_ts_milli_arrow_array(self) -> Result<ArrayRef, DataTypeError> {
        Ok(Arc::new(TimestampMillisecondArray::from_iter(
            self.into_ts_milli_array()?,
        )))
    }
}

pub trait IntoTimestampSecondArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized,
{
    fn into_ts_second_array(self) -> Result<Vec<Option<i64>>, DataTypeError> {
        let array = self
            .filter_map(|(datum, is_null)| {
                (!is_null).then_some(datum).map(|datum| {
                    unsafe { datum::Timestamp::from_datum(datum, false) }
                        .and_then(|timestamp| SecondUnix::try_from(timestamp).ok())
                        .map(|SecondUnix(unix)| unix)
                })
            })
            .collect::<Vec<Option<i64>>>();

        Ok(array)
    }
}

pub trait IntoTimestampSecondArrowArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized,
{
    fn into_ts_second_arrow_array(self) -> Result<ArrayRef, DataTypeError> {
        Ok(Arc::new(TimestampSecondArray::from_iter(
            self.into_ts_second_array()?,
        )))
    }
}

pub trait IntoGenericBytesListArrowArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized + IntoPrimitiveArray,
{
    fn into_string_list_arrow_array(self) -> Result<ArrayRef, DataTypeError> {
        let iter = self.into_array::<Column<String>>()?;

        let mut builder = ListBuilder::new(GenericByteBuilder::<GenericStringType<i32>>::new());
        for opt_vec in iter {
            if let Some(vec) = opt_vec {
                for opt_t in vec {
                    builder.values().append_option(opt_t);
                }
                builder.append(true);
            } else {
                builder.append(false);
            }
        }
        Ok(Arc::new(builder.finish()))
    }
}

pub trait IntoBooleanListArrowArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized + IntoPrimitiveArray,
{
    fn into_bool_list_arrow_array(self) -> Result<ArrayRef, DataTypeError> {
        let iter = self.into_array::<Column<bool>>()?;

        let mut builder = ListBuilder::new(BooleanBuilder::new());
        for opt_vec in iter {
            if let Some(vec) = opt_vec {
                for opt_t in vec {
                    builder.values().append_option(opt_t);
                }
                builder.append(true);
            } else {
                builder.append(false);
            }
        }
        Ok(Arc::new(builder.finish()))
    }
}

pub trait IntoPrimitiveListArrowArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized + IntoPrimitiveArray,
{
    fn into_primitive_list_arrow_array<T, A>(self) -> Result<ArrayRef, DataTypeError>
    where
        T: FromDatum,
        A: ArrowPrimitiveType<Native = T>,
    {
        let iter = self.into_array::<Column<T>>()?;

        let mut builder = ListBuilder::new(PrimitiveBuilder::<A>::new());
        for opt_vec in iter {
            if let Some(vec) = opt_vec {
                for opt_t in vec {
                    builder.values().append_option(opt_t);
                }
                builder.append(true);
            } else {
                builder.append(false);
            }
        }
        Ok(Arc::new(builder.finish()))
    }
}

pub trait IntoArrowArray
where
    Self: Iterator<Item = (pg_sys::Datum, bool)> + Sized,
{
    fn into_arrow_array(self, oid: PgOid, typemod: PgTypeMod) -> Result<ArrayRef, DataTypeError> {
        match oid {
            PgOid::BuiltIn(builtin) => match builtin {
                BOOLOID => self.into_primitive_arrow_array::<bool, BooleanArray>(),
                BOOLARRAYOID => self.into_bool_list_arrow_array(),
                TEXTOID => self.into_primitive_arrow_array::<String, StringArray>(),
                VARCHAROID => self.into_primitive_arrow_array::<String, StringArray>(),
                BPCHAROID => self.into_primitive_arrow_array::<String, StringArray>(),
                TEXTARRAYOID => self.into_string_list_arrow_array(),
                VARCHARARRAYOID => self.into_string_list_arrow_array(),
                BPCHARARRAYOID => self.into_string_list_arrow_array(),
                INT2OID => self.into_primitive_arrow_array::<i16, Int16Array>(),
                INT2ARRAYOID => self.into_primitive_list_arrow_array::<i16, Int16Type>(),
                INT4OID => self.into_primitive_arrow_array::<i32, Int32Array>(),
                INT4ARRAYOID => self.into_primitive_list_arrow_array::<i32, Int32Type>(),
                INT8OID => self.into_primitive_arrow_array::<i64, Int64Array>(),
                INT8ARRAYOID => self.into_primitive_list_arrow_array::<i64, Int64Type>(),
                FLOAT4OID => self.into_primitive_arrow_array::<f32, Float32Array>(),
                FLOAT4ARRAYOID => self.into_primitive_list_arrow_array::<f32, Float32Type>(),
                FLOAT8OID => self.into_primitive_arrow_array::<f64, Float64Array>(),
                FLOAT8ARRAYOID => self.into_primitive_list_arrow_array::<f64, Float64Type>(),
                DATEOID => self.into_date_arrow_array(),
                TIMESTAMPOID => match PgTimestampPrecision::try_from(typemod)? {
                    PgTimestampPrecision::Default => self.into_ts_micro_arrow_array(),
                    PgTimestampPrecision::Second => self.into_ts_second_arrow_array(),
                    PgTimestampPrecision::Microsecond => self.into_ts_micro_arrow_array(),
                    PgTimestampPrecision::Millisecond => self.into_ts_milli_arrow_array(),
                },
                NUMERICOID => self.into_numeric_arrow_array(typemod),
                unsupported => Err(DataTypeError::UnsupportedPostgresType(unsupported)),
            },
            PgOid::Invalid => Err(DataTypeError::InvalidPostgresOid),
            PgOid::Custom(_) => Err(DataTypeError::UnsupportedCustomType),
        }
    }
}

impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoArrowArray for T {}
impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoDateArray for T {}
impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoNumericArray for T {}
impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoPrimitiveArray for T {}
impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoTimestampMicrosecondArray for T {}
impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoTimestampMillisecondArray for T {}
impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoTimestampSecondArray for T {}

impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoDateArrowArray for T {}
impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoNumericArrowArray for T {}
impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoPrimitiveArrowArray for T {}
impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoTimestampMicrosecondArrowArray for T {}
impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoTimestampMillisecondArrowArray for T {}
impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoTimestampSecondArrowArray for T {}

impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoPrimitiveListArrowArray for T {}
impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoBooleanListArrowArray for T {}
impl<T: Iterator<Item = (pg_sys::Datum, bool)>> IntoGenericBytesListArrowArray for T {}

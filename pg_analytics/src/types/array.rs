use deltalake::arrow::array::{
    Array, ArrayRef, BooleanArray, BooleanBuilder, Date32Array, Decimal128Array, Float32Array,
    Float64Array, GenericByteBuilder, Int16Array, Int32Array, Int64Array, ListBuilder,
    PrimitiveBuilder, StringArray, Time32MillisecondArray, Time64NanosecondArray,
    TimestampMicrosecondArray, TimestampMillisecondArray, TimestampSecondArray,
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
use super::time::NanosecondDay;
use super::timestamp::{MicrosecondUnix, MillisecondUnix, PgTimestampPrecision, SecondUnix};

type Column<T> = Vec<Option<T>>;

// Copied from pgrx - pulling this out is the only way we could get the varlena pointer before getting the str
// Original function at: https://github.com/paradedb/pgrx/blob/3ce0391e6a90ae8e8f78ec2fa2e2e786de9bab55/pgrx/src/datum/from.rs#L385
// Match cases at: https://github.com/paradedb/pgrx/blob/3ce0391e6a90ae8e8f78ec2fa2e2e786de9bab55/pgrx/src/lib.rs#L270
unsafe fn convert_varlena_to_str_memoized<'a>(
    varlena: *const pg_sys::varlena,
) -> Result<&'a str, DataTypeError> {
    match pg_sys::GetDatabaseEncoding() as core::ffi::c_uint {
        pg_sys::pg_enc_PG_UTF8 => Ok(varlena::text_to_rust_str_unchecked(varlena)),
        pg_sys::pg_enc_PG_SQL_ASCII => {
            varlena::text_to_rust_str(varlena).map_err(|_| DataTypeError::InvalidUTF8)
        }
        1..=41 => {
            let bytes = varlena_to_byte_slice(varlena);
            if bytes.is_ascii() {
                Ok(core::str::from_utf8_unchecked(bytes))
            } else {
                Err(DataTypeError::InvalidUTF8)
            }
        }
        _ => varlena::text_to_rust_str(varlena).map_err(|_| DataTypeError::InvalidUTF8),
    }
}

pub trait IntoPrimitiveArray
where
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_array<T>(self) -> Result<Vec<Option<T>>, DataTypeError>
    where
        T: FromDatum,
    {
        let array = self
            .map(|datum| datum.and_then(|datum| unsafe { T::from_datum(datum, false) }))
            .collect::<Vec<Option<T>>>();

        Ok(array)
    }
}

pub trait IntoStringArray
where
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_string_array(self) -> Result<Vec<Option<String>>, DataTypeError> {
        let mut palloc_ptrs = vec![];
        let array = self
            .map(|datum| {
                datum.and_then(|datum| unsafe {
                    // Use str::from_datum instead of String::from_datum so that we know the palloced address to free
                    let ret = if varlena::varatt_is_1b_e(datum.cast_mut_ptr::<pg_sys::varlena>())
                        || (*datum.cast_mut_ptr::<pg_sys::varattrib_1b>()).va_header & 0x03 == 0x02
                    {
                        let varl = pg_sys::pg_detoast_datum_packed(datum.cast_mut_ptr());
                        palloc_ptrs.push(varl);
                        Some(convert_varlena_to_str_memoized(varl).ok()?)
                    } else {
                        <&str>::from_datum(datum, false)
                    };
                    ret.map(|ret_str| ret_str.to_owned())
                })
            })
            .collect::<Vec<Option<String>>>();

        // It is safe to free here because `ret_str.to_owned()` creates a copy
        for varl_ptr in palloc_ptrs {
            unsafe { pg_sys::pfree(varl_ptr as *mut std::ffi::c_void) };
        }

        Ok(array)
    }
}

pub trait IntoPrimitiveArrowArray
where
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized + IntoPrimitiveArray,
{
    fn into_primitive_arrow_array<T, A>(self) -> Result<ArrayRef, DataTypeError>
    where
        T: FromDatum,
        A: Array + FromIterator<Option<T>> + 'static,
    {
        Ok(Arc::new(A::from_iter(self.into_array::<T>()?)))
    }
}

pub trait IntoStringArrowArray
where
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized + IntoPrimitiveArray,
{
    fn into_string_arrow_array(self) -> Result<ArrayRef, DataTypeError> {
        Ok(Arc::new(StringArray::from_iter(self.into_string_array()?)))
    }
}

pub trait IntoNumericArray
where
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_numeric_array(
        self,
        precision: u8,
        scale: i8,
    ) -> Result<Vec<Option<i128>>, DataTypeError> {
        let array = self
            .map(|datum| {
                datum.and_then(|datum| {
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
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
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
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_date_array(self) -> Result<Vec<Option<i32>>, DataTypeError> {
        let array = self
            .map(|datum| {
                datum.and_then(|datum| {
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
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_date_arrow_array(self) -> Result<ArrayRef, DataTypeError> {
        Ok(Arc::new(Date32Array::from_iter(self.into_date_array()?)))
    }
}

pub trait IntoTimestampMicrosecondArray
where
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_ts_micro_array(self) -> Result<Vec<Option<i64>>, DataTypeError> {
        let array = self
            .map(|datum| {
                datum.and_then(|datum| {
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
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_ts_micro_arrow_array(self) -> Result<ArrayRef, DataTypeError> {
        Ok(Arc::new(TimestampMicrosecondArray::from_iter(
            self.into_ts_micro_array()?,
        )))
    }
}

pub trait IntoTimestampMillisecondArray
where
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_ts_milli_array(self) -> Result<Vec<Option<i64>>, DataTypeError> {
        let array = self
            .map(|datum| {
                datum.and_then(|datum| {
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
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_ts_milli_arrow_array(self) -> Result<ArrayRef, DataTypeError> {
        Ok(Arc::new(TimestampMillisecondArray::from_iter(
            self.into_ts_milli_array()?,
        )))
    }
}

pub trait IntoTimestampSecondArray
where
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_ts_second_array(self) -> Result<Vec<Option<i64>>, DataTypeError> {
        let array = self
            .map(|datum| {
                datum.and_then(|datum| {
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
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_ts_second_arrow_array(self) -> Result<ArrayRef, DataTypeError> {
        Ok(Arc::new(TimestampSecondArray::from_iter(
            self.into_ts_second_array()?,
        )))
    }
}

pub trait IntoTimeNanosecondArray
where
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_time_nano_array(self) -> Result<Vec<Option<i64>>, DataTypeError> {
        let array = self
            .map(|datum| {
                datum.and_then(|datum| {
                    unsafe { datum::Time::from_datum(datum, false) }
                        .and_then(|time| NanosecondDay::try_from(time).ok())
                        .map(|NanosecondDay(nanos)| nanos)
                })
            })
            .collect::<Vec<Option<i64>>>();

        Ok(array)
    }
}

pub trait IntoTimeNanosecondArrowArray
where
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_time_nano_arrow_array(self) -> Result<ArrayRef, DataTypeError> {
        Ok(Arc::new(Time64NanosecondArray::from_iter(
            self.into_time_nano_array()?,
        )))
    }
}

#[allow(dead_code)]
pub trait IntoTimeMillisecondArray
where
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_time_milli_array(self) -> Result<Vec<Option<i32>>, DataTypeError> {
        let array = self
            .map(|datum| {
                datum.and_then(|datum| {
                    unsafe { datum::Time::from_datum(datum, false) }
                        .and_then(|time| NanosecondDay::try_from(time).ok())
                        .map(|NanosecondDay(nanos)| (nanos / 1_000_000) as i32)
                })
            })
            .collect::<Vec<Option<i32>>>();

        Ok(array)
    }
}

#[allow(dead_code)]
pub trait IntoTimeMillisecondArrowArray
where
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_time_milli_arrow_array(self) -> Result<ArrayRef, DataTypeError> {
        Ok(Arc::new(Time32MillisecondArray::from_iter(
            self.into_time_milli_array()?,
        )))
    }
}

pub trait IntoUuidArray
where
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_uuid_array(self) -> Result<Vec<Option<String>>, DataTypeError> {
        let array = self
            .map(|datum| {
                datum.and_then(|datum| {
                    unsafe { datum::Uuid::from_datum(datum, false) }.map(|uuid| uuid.to_string())
                })
            })
            .collect::<Vec<Option<String>>>();

        Ok(array)
    }
}

pub trait IntoUuidArrowArray
where
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_uuid_arrow_array(self) -> Result<ArrayRef, DataTypeError> {
        Ok(Arc::new(StringArray::from_iter(self.into_uuid_array()?)))
    }
}

pub trait IntoGenericBytesListArrowArray
where
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized + IntoPrimitiveArray,
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
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized + IntoPrimitiveArray,
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
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized + IntoPrimitiveArray,
{
    fn into_primitive_list_arrow_array<T, A>(self) -> Result<ArrayRef, DataTypeError>
    where
        T: FromDatum,
        A: ArrowPrimitiveType<Native = T>,
        Vec<Option<T>>: FromDatum,
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
    Self: Iterator<Item = Option<pg_sys::Datum>> + Sized,
{
    fn into_arrow_array(self, oid: PgOid, typemod: PgTypeMod) -> Result<ArrayRef, DataTypeError> {
        match oid {
            PgOid::BuiltIn(builtin) => match builtin {
                BOOLOID => self.into_primitive_arrow_array::<bool, BooleanArray>(),
                BOOLARRAYOID => self.into_bool_list_arrow_array(),
                TEXTOID => self.into_string_arrow_array(),
                VARCHAROID => self.into_string_arrow_array(),
                BPCHAROID => self.into_string_arrow_array(),
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
                TIMEOID => match PgTimestampPrecision::try_from(typemod)? {
                    PgTimestampPrecision::Default => self.into_time_nano_arrow_array(),
                    PgTimestampPrecision::Microsecond => self.into_time_nano_arrow_array(),
                    _ => todo!(),
                },
                NUMERICOID => self.into_numeric_arrow_array(typemod),
                UUIDOID => self.into_uuid_arrow_array(),
                unsupported => Err(DataTypeError::UnsupportedPostgresType(unsupported)),
            },
            PgOid::Invalid => Err(DataTypeError::InvalidPostgresOid),
            PgOid::Custom(_) => Err(DataTypeError::UnsupportedCustomType),
        }
    }
}

impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoArrowArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoDateArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoNumericArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoPrimitiveArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoStringArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoTimestampMicrosecondArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoTimestampMillisecondArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoTimestampSecondArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoTimeNanosecondArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoTimeMillisecondArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoUuidArray for T {}

impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoDateArrowArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoNumericArrowArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoPrimitiveArrowArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoStringArrowArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoTimestampMicrosecondArrowArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoTimestampMillisecondArrowArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoTimestampSecondArrowArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoTimeNanosecondArrowArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoTimeMillisecondArrowArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoUuidArrowArray for T {}

impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoPrimitiveListArrowArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoBooleanListArrowArray for T {}
impl<T: Iterator<Item = Option<pg_sys::Datum>>> IntoGenericBytesListArrowArray for T {}

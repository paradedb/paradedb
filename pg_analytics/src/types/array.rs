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
use super::numeric::{scale_anynumeric, PgNumericTypeMod, PgPrecision, PgScale};
use super::timestamp::{into_unix, TimestampError};

type Column<T> = Vec<Option<T>>;
// type ColumnNested<T> = Vec<Option<Column<T>>>;

pub trait IntoPrimitiveArray
where
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_array<T>(self) -> Result<Vec<Option<T>>, DataTypeError>
    where
        T: FromDatum,
    {
        let array = self
            .map(|datum| {
                (!datum.is_null())
                    .then_some(datum)
                    .and_then(|datum| unsafe { T::from_datum(datum, false) })
            })
            .collect::<Vec<Option<T>>>();

        Ok(array)
    }
}

pub trait IntoPrimitiveArrowArray
where
    Self: Iterator<Item = pg_sys::Datum> + Sized + IntoPrimitiveArray,
{
    fn into_primitive_arrow_array<T, A>(self) -> Result<ArrayRef, DataTypeError>
    where
        T: FromDatum,
        A: Array + FromIterator<Option<T>> + 'static,
    {
        let iter = self.into_array::<T>()?;
        Ok(Arc::new(A::from_iter(iter)))
    }
}

pub trait IntoNumericArray
where
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_numeric_array(
        self,
        precision: u8,
        scale: i8,
    ) -> Result<Vec<Option<i128>>, DataTypeError> {
        let array = self
            .map(|datum| {
                (!datum.is_null()).then_some(datum).and_then(|datum| {
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
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_numeric_arrow_array(self, typemod: PgTypeMod) -> Result<ArrayRef, DataTypeError> {
        let PgNumericTypeMod(PgPrecision(precision), PgScale(scale)) = typemod.try_into()?;
        let iter = self.into_numeric_array(precision, scale)?;

        Ok(Arc::new(
            Decimal128Array::from_iter(iter).with_precision_and_scale(precision, scale)?,
        ))
    }
}

pub trait IntoTimestampArray
where
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_timestamp_array(self, typemod: i32) -> Result<Vec<Option<i64>>, DataTypeError> {
        let array = self
            .map(|datum| {
                (!datum.is_null()).then_some(datum).and_then(|datum| {
                    let timestamp = unsafe { datum::Timestamp::from_datum(datum, false) };
                    into_unix(timestamp, typemod).unwrap_or_else(|err| panic!("{}", err))
                })
            })
            .collect::<Vec<Option<i64>>>();

        Ok(array)
    }
}

pub trait IntoTimestampArrowArray
where
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_timestamp_arrow_array(self, typemod: PgTypeMod) -> Result<ArrayRef, DataTypeError> {
        let PgTypeMod(typemod) = typemod;
        let iter = self.into_timestamp_array(typemod)?;

        let array: ArrayRef = match typemod {
            -1 | 6 => Arc::new(TimestampMicrosecondArray::from_iter(iter)),
            0 => Arc::new(TimestampSecondArray::from_iter(iter)),
            3 => Arc::new(TimestampMillisecondArray::from_iter(iter)),
            unsupported => return Err(TimestampError::UnsupportedTypeMod(unsupported).into()),
        };

        Ok(array)
    }
}

pub trait IntoGenericBytesListArrowArray
where
    Self: Iterator<Item = pg_sys::Datum> + Sized + IntoPrimitiveArray,
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
    Self: Iterator<Item = pg_sys::Datum> + Sized + IntoPrimitiveArray,
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
    Self: Iterator<Item = pg_sys::Datum> + Sized + IntoPrimitiveArray,
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
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_arrow_array(self, oid: PgOid, typemod: PgTypeMod) -> Result<ArrayRef, DataTypeError> {
        match oid {
            PgOid::BuiltIn(builtin) => match builtin {
                BOOLOID => self.into_primitive_arrow_array::<bool, BooleanArray>(),
                BOOLARRAYOID => self.into_bool_list_arrow_array(),
                TEXTOID | VARCHAROID | BPCHAROID => {
                    self.into_primitive_arrow_array::<String, StringArray>()
                }
                TEXTARRAYOID | VARCHARARRAYOID | BPCHARARRAYOID => {
                    self.into_string_list_arrow_array()
                }
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
                DATEOID => self.into_primitive_arrow_array::<i32, Date32Array>(),
                TIMESTAMPOID => self.into_timestamp_arrow_array(typemod),
                NUMERICOID => self.into_numeric_arrow_array(typemod),
                unsupported => Err(DataTypeError::UnsupportedPostgresType(unsupported)),
            },
            PgOid::Invalid => Err(DataTypeError::InvalidPostgresOid),
            PgOid::Custom(_) => Err(DataTypeError::UnsupportedCustomType),
        }
    }
}

impl<T: Iterator<Item = pg_sys::Datum>> IntoArrowArray for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoPrimitiveArray for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoNumericArray for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoTimestampArray for T {}

impl<T: Iterator<Item = pg_sys::Datum>> IntoPrimitiveArrowArray for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoNumericArrowArray for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoTimestampArrowArray for T {}

impl<T: Iterator<Item = pg_sys::Datum>> IntoPrimitiveListArrowArray for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoBooleanListArrowArray for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoGenericBytesListArrowArray for T {}

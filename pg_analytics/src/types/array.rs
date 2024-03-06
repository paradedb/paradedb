use deltalake::arrow::array::{
    Array, ArrayRef, BooleanArray, BooleanBuilder, Date32Array, Decimal128Array, Float32Array,
    Float64Array, GenericByteBuilder, Int16Array, Int32Array, Int64Array, ListBuilder,
    PrimitiveBuilder, StringArray, TimestampMicrosecondArray, TimestampMillisecondArray,
    TimestampSecondArray,
};
use deltalake::arrow::datatypes::{
    ArrowPrimitiveType, ByteArrayType, Float32Type, Float64Type, GenericStringType, Int16Type,
    Int32Type, Int64Type,
};
use pgrx::pg_sys::BuiltinOid::*;
use pgrx::*;
use std::sync::Arc;

use super::datatype::{DataTypeError, PgTypeMod};
use super::numeric::{scale_anynumeric, PgNumericTypeMod, PgPrecision, PgScale};
use super::timestamp::{into_unix, TimestampError};

type Column<T> = Vec<Option<T>>;
type ColumnNested<T> = Vec<Option<Column<T>>>;

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

pub trait IntoPrimitiveArrayRef<T>
where
    Self: IntoIterator<Item = Option<T>> + Sized,
{
    fn into_array_ref<A>(self) -> Result<ArrayRef, DataTypeError>
    where
        A: Array + FromIterator<Option<T>> + 'static,
    {
        Ok(Arc::new(A::from_iter(self)))
    }
}

pub trait IntoNumericArray
where
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_num_array(self, precision: u8, scale: i8) -> Result<Vec<Option<i128>>, DataTypeError> {
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

pub trait IntoNumericArrayRef
where
    Self: IntoIterator<Item = Option<i128>> + Sized,
{
    fn into_num_array_ref(self, precision: u8, scale: i8) -> Result<ArrayRef, DataTypeError> {
        Ok(Arc::new(
            Decimal128Array::from_iter(self).with_precision_and_scale(precision, scale)?,
        ))
    }
}

pub trait IntoTimestampArray
where
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_ts_array(self, typemod: i32) -> Result<Vec<Option<i64>>, DataTypeError> {
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

pub trait IntoTimestampArrayRef
where
    Self: IntoIterator<Item = Option<i64>> + Sized,
{
    fn into_ts_array_ref(self, typemod: i32) -> Result<ArrayRef, DataTypeError> {
        let array: ArrayRef = match typemod {
            -1 | 6 => Arc::new(TimestampMicrosecondArray::from_iter(self)),
            0 => Arc::new(TimestampSecondArray::from_iter(self)),
            3 => Arc::new(TimestampMillisecondArray::from_iter(self)),
            unsupported => return Err(TimestampError::UnsupportedTypeMod(unsupported).into()),
        };

        Ok(array)
    }
}

pub trait IntoGenericBytesListArrayRef<T, B>
where
    B: ByteArrayType,
    T: AsRef<B::Native>,
    Self: IntoIterator<Item = Option<Vec<Option<T>>>> + Sized,
{
    fn into_array_ref(self) -> Result<ArrayRef, DataTypeError> {
        let mut builder = ListBuilder::new(GenericByteBuilder::<B>::new());
        for opt_vec in self {
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

pub trait IntoBooleanListArrayRef
where
    Self: IntoIterator<Item = Option<Vec<Option<bool>>>> + Sized,
{
    fn into_array_ref(self) -> Result<ArrayRef, DataTypeError> {
        let mut builder = ListBuilder::new(BooleanBuilder::new());
        for opt_vec in self {
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

pub trait IntoPrimitiveListArrayRef<A>
where
    A: ArrowPrimitiveType,
    Self: IntoIterator<Item = Option<Vec<Option<A::Native>>>> + Sized,
{
    fn into_array_ref(self) -> Result<ArrayRef, DataTypeError> {
        let mut builder = ListBuilder::new(PrimitiveBuilder::<A>::new());
        for opt_vec in self {
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

pub trait IntoArrayRef
where
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_array_ref(self, oid: PgOid, typemod: PgTypeMod) -> Result<ArrayRef, DataTypeError> {
        match oid {
            PgOid::BuiltIn(builtin) => match builtin {
                BOOLOID => self.into_array::<bool>()?.into_array_ref::<BooleanArray>(),
                TEXTOID | VARCHAROID | BPCHAROID => {
                    self.into_array::<String>()?.into_array_ref::<StringArray>()
                }
                INT2OID => self.into_array::<i16>()?.into_array_ref::<Int16Array>(),
                INT4OID => self.into_array::<i32>()?.into_array_ref::<Int32Array>(),
                INT8OID => self.into_array::<i64>()?.into_array_ref::<Int64Array>(),
                FLOAT4OID => self.into_array::<f32>()?.into_array_ref::<Float32Array>(),
                FLOAT8OID => self.into_array::<f64>()?.into_array_ref::<Float64Array>(),
                DATEOID => self.into_array::<i32>()?.into_array_ref::<Date32Array>(),
                TIMESTAMPOID => {
                    let PgTypeMod(typemod) = typemod;
                    self.into_ts_array(typemod)?.into_ts_array_ref(typemod)
                }
                NUMERICOID => {
                    let PgNumericTypeMod(PgPrecision(p), PgScale(s)) = typemod.try_into()?;
                    self.into_num_array(p, s)?.into_num_array_ref(p, s)
                }
                unsupported => Err(DataTypeError::UnsupportedPostgresType(unsupported)),
            },
            PgOid::Invalid => Err(DataTypeError::InvalidPostgresOid),
            PgOid::Custom(_) => Err(DataTypeError::UnsupportedCustomType),
        }
    }
}

impl<T: Iterator<Item = pg_sys::Datum>> IntoPrimitiveArray for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoNumericArray for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoTimestampArray for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoArrayRef for T {}

impl IntoPrimitiveArrayRef<String> for Column<String> {}
impl IntoGenericBytesListArrayRef<String, GenericStringType<i32>> for ColumnNested<String> {}

impl IntoPrimitiveArrayRef<bool> for Column<bool> {}
impl IntoBooleanListArrayRef for ColumnNested<bool> {}

impl IntoPrimitiveArrayRef<i16> for Column<i16> {}
impl IntoPrimitiveListArrayRef<Int16Type> for ColumnNested<i16> {}

impl IntoPrimitiveArrayRef<i32> for Column<i32> {}
impl IntoPrimitiveListArrayRef<Int32Type> for ColumnNested<i32> {}

impl IntoPrimitiveArrayRef<i64> for Column<i64> {}
impl IntoPrimitiveListArrayRef<Int64Type> for ColumnNested<i64> {}

impl IntoPrimitiveArrayRef<f32> for Column<f32> {}
impl IntoPrimitiveListArrayRef<Float32Type> for ColumnNested<f32> {}

impl IntoPrimitiveArrayRef<f64> for Column<f64> {}
impl IntoPrimitiveListArrayRef<Float64Type> for ColumnNested<f64> {}

impl IntoNumericArrayRef for Column<i128> {}
impl IntoTimestampArrayRef for Column<i64> {}

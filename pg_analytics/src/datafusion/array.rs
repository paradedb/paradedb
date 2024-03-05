use deltalake::arrow::{
    array::{
        Array, ArrayRef, AsArray, BooleanArray, BooleanBuilder, Decimal128Array, Float32Array,
        Float64Array, GenericByteBuilder, Int16Array, Int32Array, Int64Array, ListArray,
        ListBuilder, PrimitiveBuilder, StringArray, TimestampMicrosecondArray,
        TimestampMillisecondArray, TimestampSecondArray,
    },
    datatypes::{
        ArrowPrimitiveType, ByteArrayType, Date32Type, Decimal128Type, Float32Type, Float64Type,
        Int32Type, Int64Type, TimestampMicrosecondType, TimestampMillisecondType,
        TimestampSecondType,
    },
};
use deltalake::datafusion::arrow::datatypes::{DataType, TimeUnit};
use pgrx::*;
use std::sync::Arc;

use super::datatype::{DataTypeError, PgTypeMod};
use super::numeric::{scale_anynumeric, PgNumeric, PgNumericTypeMod, PgPrecision, PgScale};
use super::timestamp::{into_unix, MicrosecondsUnix, MillisecondsUnix, SecondsUnix};

pub trait IntoArray
where
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_array<T, A>(self) -> ArrayRef
    where
        T: FromDatum,
        A: Array + FromIterator<Option<T>> + 'static,
    {
        Arc::new(A::from_iter(self.map(|datum| {
            (!datum.is_null())
                .then_some(datum)
                .and_then(|datum| unsafe { T::from_datum(datum, false) })
        })))
    }
}

pub trait IntoNumericArray
where
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_numeric_array(self, typemod: PgTypeMod) -> ArrayRef {
        let PgNumericTypeMod(PgPrecision(precision), PgScale(scale)) = typemod.try_into().unwrap();

        Arc::new(Decimal128Array::from_iter(self.map(|datum| {
            (!datum.is_null()).then_some(datum).and_then(|datum| {
                unsafe { AnyNumeric::from_datum(datum, false) }.map(|numeric| {
                    i128::try_from(scale_anynumeric(numeric, precision, scale, true).unwrap())
                        .unwrap()
                })
            })
        })))
    }
}

pub trait IntoTimestampArray
where
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_timestamp_array(self, typemod: PgTypeMod) -> ArrayRef {
        let PgTypeMod(typemod) = typemod;

        let iter = self.map(|datum| {
            (!datum.is_null()).then_some(datum).and_then(|datum| {
                let timestamp = unsafe { datum::Timestamp::from_datum(datum, false) };
                into_unix(timestamp, typemod).unwrap_or_else(|err| panic!("{}", err))
            })
        });

        match typemod {
            -1 | 6 => Arc::new(TimestampMicrosecondArray::from_iter(iter)),
            0 => Arc::new(TimestampSecondArray::from_iter(iter)),
            3 => Arc::new(TimestampMillisecondArray::from_iter(iter)),
            _ => todo!(),
        }
    }
}

pub trait IntoGenericBytesListArray<T, B>
where
    B: ByteArrayType,
    T: AsRef<B::Native>,
    Self: IntoIterator<Item = Option<Vec<Option<T>>>> + Sized,
{
    fn into_array(self) -> ListArray {
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
        builder.finish()
    }
}

pub trait IntoBooleanListArray
where
    Self: IntoIterator<Item = Option<Vec<Option<bool>>>> + Sized,
{
    fn into_array(self) -> ListArray {
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
        builder.finish()
    }
}

pub trait IntoPrimitiveListArray<A>
where
    A: ArrowPrimitiveType,
    Self: IntoIterator<Item = Option<Vec<Option<A::Native>>>> + Sized,
{
    fn into_array(self) -> ListArray {
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
        builder.finish()
    }
}

pub trait IntoArrayRef
where
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_array_ref(self, oid: PgOid, typemod: PgTypeMod) -> Result<ArrayRef, DataTypeError> {
        Ok(match oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::BOOLOID => self.into_array::<bool, BooleanArray>(),
                PgBuiltInOids::TEXTOID => self.into_array::<String, StringArray>(),
                PgBuiltInOids::INT2OID => self.into_array::<i16, Int16Array>(),
                PgBuiltInOids::INT4OID => self.into_array::<i32, Int32Array>(),
                PgBuiltInOids::INT8OID => self.into_array::<i64, Int64Array>(),
                PgBuiltInOids::FLOAT4OID => self.into_array::<f32, Float32Array>(),
                PgBuiltInOids::FLOAT8OID => self.into_array::<f64, Float64Array>(),
                PgBuiltInOids::DATEOID => self.into_array::<i32, Int32Array>(),
                PgBuiltInOids::TIMESTAMPOID => self.into_timestamp_array(typemod),
                PgBuiltInOids::NUMERICOID => self.into_numeric_array(typemod),
                unsupported => return Err(DataTypeError::UnsupportedPostgresType(unsupported)),
            },
            PgOid::Invalid => return Err(DataTypeError::InvalidPostgresOid),
            PgOid::Custom(_) => return Err(DataTypeError::UnsupportedCustomType),
        })
    }
}

pub trait GetDatum {
    fn get_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError>;
}

impl GetDatum for Arc<dyn Array> {
    fn get_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
        let result = match self.data_type() {
            DataType::Boolean => self
                .as_any()
                .downcast_ref::<BooleanArray>()
                .ok_or(DataTypeError::DowncastGenericArray(DataType::Boolean))?
                .value(index)
                .into_datum(),
            DataType::Utf8 => self
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(DataTypeError::DowncastGenericArray(DataType::Utf8))?
                .value(index)
                .into_datum(),
            DataType::Int32 => self.as_primitive::<Int32Type>().value(index).into_datum(),
            DataType::Int64 => self.as_primitive::<Int64Type>().value(index).into_datum(),
            DataType::Float32 => self.as_primitive::<Float32Type>().value(index).into_datum(),
            DataType::Float64 => self.as_primitive::<Float64Type>().value(index).into_datum(),
            DataType::Date32 => self.as_primitive::<Date32Type>().value(index).into_datum(),
            DataType::Timestamp(TimeUnit::Microsecond, None) => {
                MicrosecondsUnix(self.as_primitive::<TimestampMicrosecondType>().value(index))
                    .try_into()?
            }
            DataType::Timestamp(TimeUnit::Millisecond, None) => {
                MillisecondsUnix(self.as_primitive::<TimestampMillisecondType>().value(index))
                    .try_into()?
            }
            DataType::Timestamp(TimeUnit::Second, None) => {
                SecondsUnix(self.as_primitive::<TimestampSecondType>().value(index)).try_into()?
            }
            DataType::Decimal128(precision, scale) => PgNumeric(
                AnyNumeric::from(self.as_primitive::<Decimal128Type>().value(index)),
                PgNumericTypeMod(PgPrecision(*precision), PgScale(*scale)),
            )
            .try_into()?,
            _ => return Ok(None),
        };

        Ok(result)
    }
}

impl<T: Iterator<Item = pg_sys::Datum>> IntoArray for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoArrayRef for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoNumericArray for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoTimestampArray for T {}

// impl IntoBooleanListArray for ColumnNested<bool> {}

// impl IntoGenericBytesListArray<String, GenericStringType<i32>> for ColumnNested<String> {}

// impl IntoPrimitiveListArray<Int16Type> for ColumnNested<i16> {}

// impl IntoPrimitiveListArray<Int32Type> for ColumnNested<i32> {}

// impl IntoPrimitiveListArray<Int64Type> for ColumnNested<i64> {}

// impl IntoPrimitiveListArray<UInt32Type> for ColumnNested<u32> {}

// impl IntoPrimitiveListArray<Float32Type> for ColumnNested<f32> {}

// impl IntoPrimitiveListArray<Float64Type> for ColumnNested<f64> {}

// impl IntoArray<i128> for Iterator<Item = pg_sys::Datum> {}
// impl IntoPrimitiveListArray<Decimal128Type> for ColumnNested<i128> {}

// impl IntoPrimitiveListArray<TimestampMicrosecondType> for ColumnNested<i64> {}

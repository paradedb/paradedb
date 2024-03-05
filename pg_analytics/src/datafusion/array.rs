use deltalake::arrow::{
    array::{
        Array, ArrayRef, AsArray, BooleanArray, BooleanBuilder, Decimal128Array, Float32Array,
        Float64Array, GenericByteBuilder, Int16Array, Int32Array, Int64Array, ListArray,
        ListBuilder, PrimitiveBuilder, StringArray, UInt32Array,
    },
    datatypes::{
        ArrowPrimitiveType, ByteArrayType, Date32Type, Decimal128Type, Float32Type, Float64Type,
        GenericStringType, Int16Type, Int32Type, Int64Type, TimestampMicrosecondType, UInt32Type,
    },
};
use deltalake::datafusion::arrow::datatypes::{DataType, TimeUnit};
use pgrx::*;
use std::sync::Arc;

use super::datatype::{DataTypeError, PgTypeMod};
use super::numeric::{IntoNumericArray, PgNumeric, PgNumericTypeMod, PgPrecision, PgScale};
use super::timestamp::Microseconds;

type Column<T> = Vec<Option<T>>;
type ColumnNested<T> = Vec<Option<Column<T>>>;

pub trait IntoArray<T, A>
where
    A: Array + FromIterator<Option<T>>,
    Self: IntoIterator<Item = Option<T>> + Sized,
{
    fn into_array(self) -> A {
        A::from_iter(self)
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

pub trait IntoPrimitiveArray {
    fn into_primitive_array<Primitive>(self) -> Vec<Option<Primitive>>
    where
        Primitive: FromDatum;
}

impl<T> IntoPrimitiveArray for T
where
    T: Iterator<Item = pg_sys::Datum>,
{
    fn into_primitive_array<Primitive>(self) -> Vec<Option<Primitive>>
    where
        Primitive: FromDatum,
    {
        self.map(|datum| {
            (!datum.is_null())
                .then_some(datum)
                .and_then(|datum| unsafe { Primitive::from_datum(datum, false) })
        })
        .collect::<Vec<Option<Primitive>>>()
    }
}

impl IntoArray<bool, BooleanArray> for Column<bool> {}
impl IntoBooleanListArray for ColumnNested<bool> {}

impl IntoArray<String, StringArray> for Column<String> {}
impl IntoGenericBytesListArray<String, GenericStringType<i32>> for ColumnNested<String> {}

impl IntoArray<i16, Int16Array> for Column<i16> {}
impl IntoPrimitiveListArray<Int16Type> for ColumnNested<i16> {}

impl IntoArray<i32, Int32Array> for Column<i32> {}
impl IntoPrimitiveListArray<Int32Type> for ColumnNested<i32> {}

impl IntoArray<i64, Int64Array> for Column<i64> {}
impl IntoPrimitiveListArray<Int64Type> for ColumnNested<i64> {}

impl IntoArray<u32, UInt32Array> for Column<u32> {}
impl IntoPrimitiveListArray<UInt32Type> for ColumnNested<u32> {}

impl IntoArray<f32, Float32Array> for Column<f32> {}
impl IntoPrimitiveListArray<Float32Type> for ColumnNested<f32> {}

impl IntoArray<f64, Float64Array> for Column<f64> {}
impl IntoPrimitiveListArray<Float64Type> for ColumnNested<f64> {}

impl IntoArray<i128, Decimal128Array> for Column<i128> {}
impl IntoPrimitiveListArray<Decimal128Type> for ColumnNested<i128> {}

impl IntoPrimitiveListArray<TimestampMicrosecondType> for ColumnNested<i64> {}

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
                Microseconds(self.as_primitive::<TimestampMicrosecondType>().value(index))
                    .try_into()?
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

pub trait IntoArrayRef {
    fn into_array_ref(self, oid: PgOid, pg_typemod: PgTypeMod) -> Result<ArrayRef, DataTypeError>;
}

impl<T> IntoArrayRef for T
where
    T: Iterator<Item = pg_sys::Datum>,
{
    fn into_array_ref(self, oid: PgOid, pg_typemod: PgTypeMod) -> Result<ArrayRef, DataTypeError> {
        Ok(match oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::BOOLOID => {
                    Arc::new(self.into_primitive_array::<bool>().into_array())
                }
                PgBuiltInOids::TEXTOID => {
                    Arc::new(self.into_primitive_array::<String>().into_array())
                }
                PgBuiltInOids::INT2OID => Arc::new(self.into_primitive_array::<i16>().into_array()),
                PgBuiltInOids::INT4OID => Arc::new(self.into_primitive_array::<i32>().into_array()),
                PgBuiltInOids::INT8OID => Arc::new(self.into_primitive_array::<i64>().into_array()),
                PgBuiltInOids::FLOAT4OID => {
                    Arc::new(self.into_primitive_array::<f32>().into_array())
                }
                PgBuiltInOids::FLOAT8OID => {
                    Arc::new(self.into_primitive_array::<f64>().into_array())
                }
                PgBuiltInOids::DATEOID => Arc::new(self.into_primitive_array::<i32>().into_array()),
                // PgBuiltInOids::TIMESTAMPOID => self.into_primitive_array().into_array(),
                PgBuiltInOids::NUMERICOID => {
                    Arc::new(self.into_numeric_array(pg_typemod).into_array())
                }
                unsupported => return Err(DataTypeError::UnsupportedPostgresType(unsupported)),
            },
            PgOid::Invalid => return Err(DataTypeError::InvalidPostgresOid),
            PgOid::Custom(_) => return Err(DataTypeError::UnsupportedCustomType),
        })
    }
}

use deltalake::arrow::{
    array::{
        Array, ArrayRef, BooleanArray, BooleanBuilder, Date32Array, Decimal128Array, Float32Array,
        Float64Array, GenericByteBuilder, Int16Array, Int32Array, Int64Array, ListArray,
        ListBuilder, PrimitiveBuilder, StringArray, TimestampMicrosecondArray,
        TimestampMillisecondArray, TimestampSecondArray,
    },
    datatypes::{ArrowPrimitiveType, ByteArrayType},
};

use pgrx::*;
use std::sync::Arc;

use super::datatype::{DataTypeError, PgTypeMod};
use super::numeric::{scale_anynumeric, PgNumericTypeMod, PgPrecision, PgScale};
use super::timestamp::{into_unix, TimestampError};

pub trait IntoArray
where
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_array<T, A>(self) -> Result<ArrayRef, DataTypeError>
    where
        T: FromDatum,
        A: Array + FromIterator<Option<T>> + 'static,
    {
        let array = A::from_iter(self.map(|datum| {
            (!datum.is_null())
                .then_some(datum)
                .and_then(|datum| unsafe { T::from_datum(datum, false) })
        }));

        Ok(Arc::new(array))
    }
}

pub trait IntoNumericArray
where
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_numeric_array(self, typemod: PgTypeMod) -> Result<ArrayRef, DataTypeError> {
        let PgNumericTypeMod(PgPrecision(precision), PgScale(scale)) = typemod.try_into()?;

        let iter = self.map(|datum| {
            (!datum.is_null()).then_some(datum).and_then(|datum| {
                unsafe { AnyNumeric::from_datum(datum, false) }.map(|numeric| {
                    i128::try_from(
                        scale_anynumeric(numeric, precision, scale, true)
                            .unwrap_or_else(|err| panic!("{}", err)),
                    )
                    .unwrap_or_else(|err| panic!("{}", err))
                })
            })
        });

        let array = Decimal128Array::from_iter(iter).with_precision_and_scale(precision, scale)?;

        Ok(Arc::new(array))
    }
}

pub trait IntoTimestampArray
where
    Self: Iterator<Item = pg_sys::Datum> + Sized,
{
    fn into_timestamp_array(self, typemod: PgTypeMod) -> Result<ArrayRef, DataTypeError> {
        let PgTypeMod(typemod) = typemod;

        let iter = self.map(|datum| {
            (!datum.is_null()).then_some(datum).and_then(|datum| {
                let timestamp = unsafe { datum::Timestamp::from_datum(datum, false) };
                into_unix(timestamp, typemod).unwrap_or_else(|err| panic!("{}", err))
            })
        });

        let array: ArrayRef = match typemod {
            -1 | 6 => Arc::new(TimestampMicrosecondArray::from_iter(iter)),
            0 => Arc::new(TimestampSecondArray::from_iter(iter)),
            3 => Arc::new(TimestampMillisecondArray::from_iter(iter)),
            unsupported => return Err(TimestampError::UnsupportedTypeMod(unsupported).into()),
        };

        Ok(array)
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
        match oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::BOOLOID => self.into_array::<bool, BooleanArray>(),
                PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID | PgBuiltInOids::BPCHAROID => {
                    self.into_array::<String, StringArray>()
                }
                PgBuiltInOids::INT2OID => self.into_array::<i16, Int16Array>(),
                PgBuiltInOids::INT4OID => self.into_array::<i32, Int32Array>(),
                PgBuiltInOids::INT8OID => self.into_array::<i64, Int64Array>(),
                PgBuiltInOids::FLOAT4OID => self.into_array::<f32, Float32Array>(),
                PgBuiltInOids::FLOAT8OID => self.into_array::<f64, Float64Array>(),
                PgBuiltInOids::DATEOID => self.into_array::<i32, Date32Array>(),
                PgBuiltInOids::TIMESTAMPOID => self.into_timestamp_array(typemod),
                PgBuiltInOids::NUMERICOID => self.into_numeric_array(typemod),
                unsupported => Err(DataTypeError::UnsupportedPostgresType(unsupported)),
            },
            PgOid::Invalid => Err(DataTypeError::InvalidPostgresOid),
            PgOid::Custom(_) => Err(DataTypeError::UnsupportedCustomType),
        }
    }
}

impl<T: Iterator<Item = pg_sys::Datum>> IntoArray for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoArrayRef for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoNumericArray for T {}
impl<T: Iterator<Item = pg_sys::Datum>> IntoTimestampArray for T {}

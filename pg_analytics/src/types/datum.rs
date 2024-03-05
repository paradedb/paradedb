use deltalake::arrow::{
    array::{Array, AsArray, BooleanArray, StringArray},
    datatypes::{
        Date32Type, Decimal128Type, Float32Type, Float64Type, Int32Type, Int64Type,
        TimestampMicrosecondType, TimestampMillisecondType, TimestampSecondType,
    },
};
use deltalake::datafusion::arrow::datatypes::{DataType, TimeUnit};
use pgrx::*;
use std::sync::Arc;

use super::datatype::DataTypeError;
use super::numeric::{PgNumeric, PgNumericTypeMod, PgPrecision, PgScale};
use super::timestamp::{MicrosecondsUnix, MillisecondsUnix, SecondsUnix};

pub trait GetDatum
where
    Self: Array + AsArray,
{
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

impl GetDatum for Arc<dyn Array> {}

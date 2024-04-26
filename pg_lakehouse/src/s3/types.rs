use datafusion::arrow::array::{
    Array, ArrayAccessor, ArrayRef, ArrowPrimitiveType, AsArray, BooleanArray, Date32Array,
    Float32Array, Float64Array, Int16Array, Int32Array, Int64Array, StringArray,
};
use datafusion::arrow::datatypes::DataType;
use datafusion::common::{DataFusionError, downcast_value};
use pgrx::*;
use supabase_wrappers::interface::Cell;
use std::fmt::Debug;
use thiserror::Error;

pub trait GetPrimitiveValue
where
    Self: Array + Debug + 'static,
{
    fn get_primitive_value<T>(&self, index: usize) -> Result<Option<T>, DataTypeError>
    where 
        T: Debug,
    {
        let (buffer, nulls) = self.into_parts();

        let is_null = match nulls {
            Some(nulls) => nulls.is_null(index),
            None => false,
        };

        match is_null
        {
            false => Some(buffer.value(index)),
            true => None,
        }
    }
}

pub trait GetCell
where
    Self: Array
        + AsArray
{
    fn get_cell(&self, index: usize, oid: pg_sys::Oid) -> Result<Option<Cell>, DataTypeError> {
        match oid {
            pg_sys::BOOLOID => {
                match downcast_value!(self, BooleanArray).get_primitive_value::<bool>(index)? {
                    Some(value) => Ok(Some(Cell::Bool(value))),
                    None => Ok(None),
                }
            },
            pg_sys::INT2OID | pg_sys::INT4OID | pg_sys::INT8OID | pg_sys::FLOAT4OID | pg_sys::FLOAT8OID => {
                match self.data_type() {
                    DataType::Int16 => {
                        match downcast_value!(self, Int16Array).get_primitive_value::<i16>(index)? {
                            Some(value) => Ok(Some(Cell::I16(value))),
                            None => Ok(None),
                        }
                    },
                    DataType::Int32 => {
                        match downcast_value!(self, Int32Array).get_primitive_value::<i32>(index)? {
                            Some(value) => Ok(Some(Cell::I32(value))),
                            None => Ok(None),
                        }
                    },
                    DataType::Int64 => {
                        match downcast_value!(self, Int64Array).get_primitive_value::<i64>(index)? {
                            Some(value) => Ok(Some(Cell::I64(value))),
                            None => Ok(None),
                        }
                    },
                    DataType::Float32 => {
                        match downcast_value!(self, Float32Array).get_primitive_value::<f32>(index)? {
                            Some(value) => Ok(Some(Cell::F32(value))),
                            None => Ok(None),
                        }
                    },
                    DataType::Float64 => {
                        match downcast_value!(self, Float64Array).get_primitive_value::<f64>(index)? {
                            Some(value) => Ok(Some(Cell::F64(value))),
                            None => Ok(None),
                        }
                    },
                    unsupported => Err(DataTypeError::DataTypeMismatch(unsupported, oid)),
                }
            },
            pg_sys::TEXTOID | pg_sys::VARCHAROID | pg_sys::BPCHAROID => {
                match downcast_value!(self, StringArray).get_primitive_value::<&str>(index)? {
                    Some(value) => Ok(Some(Cell::String(value.to_string()))),
                    None => Ok(None),
                }
            },
            unsupported => Err(DataTypeError::UnsupportedPostgresType(unsupported)),
        }
    }

}

impl GetCell for ArrayRef {}
impl GetPrimitiveValue for BooleanArray {}
impl GetPrimitiveValue for Int16Array {}
impl GetPrimitiveValue for Int32Array {}
impl GetPrimitiveValue for Int64Array {}
impl GetPrimitiveValue for Float32Array {}
impl GetPrimitiveValue for Float64Array {}
impl GetPrimitiveValue for StringArray {}

#[derive(Error, Debug)]
pub enum DataTypeError {
    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error("Received unsupported data type {0:?} for {1:?}")]
    DataTypeMismatch(DataType, pg_sys::Oid),

    #[error("Downcast Arrow array failed")]
    DowncastError(),

    #[error("Postgres data type {0:?} is not supported")]
    UnsupportedPostgresType(pg_sys::Oid),
}
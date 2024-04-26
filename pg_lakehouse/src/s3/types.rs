use datafusion::arrow::array::{
    Array, ArrayAccessor, ArrayRef, ArrowPrimitiveType, AsArray, BooleanArray, Date32Array,
    Float32Array, Float64Array, Int16Array, Int32Array, Int64Array, StringArray,
};
use datafusion::arrow::datatypes::DataType;
use datafusion::common::{downcast_value, DataFusionError};
use pgrx::*;
use std::fmt::Debug;
use supabase_wrappers::interface::Cell;
use thiserror::Error;

pub trait GetPrimitiveValue
where
    Self: Array + AsArray,
{
    fn get_primitive_value<A>(
        &self,
        index: usize,
    ) -> Result<Option<<&A as ArrayAccessor>::Item>, DataTypeError>
    where
        A: Array + Debug + 'static,
        for<'a> &'a A: ArrayAccessor,
    {
        let downcast_array = downcast_value!(self, A);
        match downcast_array.is_null(index) {
            false => Ok(Some(downcast_array.value(index))),
            true => Ok(None),
        }
    }
}

pub trait GetCell
where
    Self: Array + AsArray + GetPrimitiveValue,
{
    fn get_cell(&self, index: usize, oid: pg_sys::Oid) -> Result<Option<Cell>, DataTypeError> {
        match oid {
            pg_sys::BOOLOID => match self.get_primitive_value::<BooleanArray>(index)? {
                Some(value) => Ok(Some(Cell::Bool(value))),
                None => Ok(None),
            },
            pg_sys::INT2OID
            | pg_sys::INT4OID
            | pg_sys::INT8OID
            | pg_sys::FLOAT4OID
            | pg_sys::FLOAT8OID => match self.data_type() {
                DataType::Int16 => match self.get_primitive_value::<Int16Array>(index)? {
                    Some(value) => Ok(Some(Cell::I16(value))),
                    None => Ok(None),
                },
                DataType::Int32 => match self.get_primitive_value::<Int32Array>(index)? {
                    Some(value) => Ok(Some(Cell::I32(value))),
                    None => Ok(None),
                },
                DataType::Int64 => match self.get_primitive_value::<Int64Array>(index)? {
                    Some(value) => Ok(Some(Cell::I64(value))),
                    None => Ok(None),
                },
                DataType::Float32 => match self.get_primitive_value::<Float32Array>(index)? {
                    Some(value) => Ok(Some(Cell::F32(value))),
                    None => Ok(None),
                },
                DataType::Float64 => match self.get_primitive_value::<Float64Array>(index)? {
                    Some(value) => Ok(Some(Cell::F64(value))),
                    None => Ok(None),
                },
                unsupported => Err(DataTypeError::DataTypeMismatch(unsupported.clone(), oid)),
            },
            pg_sys::TEXTOID | pg_sys::VARCHAROID | pg_sys::BPCHAROID => {
                match self.get_primitive_value::<StringArray>(index)? {
                    Some(value) => Ok(Some(Cell::String(value.to_string()))),
                    None => Ok(None),
                }
            }
            unsupported => Err(DataTypeError::UnsupportedPostgresType(unsupported.clone())),
        }
    }
}

impl GetCell for ArrayRef {}
impl GetPrimitiveValue for ArrayRef {}

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

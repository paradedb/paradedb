use deltalake::arrow::error::ArrowError;
use deltalake::datafusion::common::DataFusionError;
use deltalake::errors::DeltaTableError;
use pgrx::*;
use shared::postgres::transaction::TransactionError;
use std::ffi::{IntoStringError, NulError, OsString};
use std::num::ParseIntError;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use thiserror::Error;

use crate::storage::tid::TIDError;
use crate::types::datatype::DataTypeError;

#[derive(Error, Debug)]
pub enum ParadeError {
    #[error(transparent)]
    Arrow(#[from] ArrowError),

    #[error(transparent)]
    DataFusion(#[from] DataFusionError),

    #[error(transparent)]
    Delta(#[from] DeltaTableError),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    TIDError(#[from] TIDError),

    #[error(transparent)]
    TransactionError(#[from] TransactionError),

    #[error(transparent)]
    DataType(#[from] DataTypeError),

    #[error(transparent)]
    NotSupported(#[from] NotSupported),

    #[error("{0}")]
    Generic(String),
}

#[derive(Error, Debug)]
pub enum NotSupported {
    #[error("UPDATE is not supported because Parquet tables are append only.")]
    Update,

    #[error("DELETE is not supported because Parquet tables are append only.")]
    Delete,
}

impl From<&str> for ParadeError {
    fn from(err: &str) -> Self {
        ParadeError::Generic(err.to_string())
    }
}

impl From<ParseIntError> for ParadeError {
    fn from(err: ParseIntError) -> Self {
        ParadeError::Generic(err.to_string())
    }
}

impl From<Utf8Error> for ParadeError {
    fn from(err: Utf8Error) -> Self {
        ParadeError::Generic(err.to_string())
    }
}

impl From<FromUtf8Error> for ParadeError {
    fn from(err: FromUtf8Error) -> Self {
        ParadeError::Generic(err.to_string())
    }
}

impl From<NulError> for ParadeError {
    fn from(err: NulError) -> Self {
        ParadeError::Generic(err.to_string())
    }
}

impl From<numeric::Error> for ParadeError {
    fn from(err: numeric::Error) -> Self {
        ParadeError::Generic(err.to_string())
    }
}

impl From<OsString> for ParadeError {
    fn from(err: OsString) -> Self {
        ParadeError::Generic(err.to_string_lossy().to_string())
    }
}

impl From<spi::SpiError> for ParadeError {
    fn from(err: spi::SpiError) -> Self {
        ParadeError::Generic(err.to_string())
    }
}

impl From<regex::Error> for ParadeError {
    fn from(err: regex::Error) -> Self {
        ParadeError::Generic(err.to_string())
    }
}

impl From<IntoStringError> for ParadeError {
    fn from(err: IntoStringError) -> Self {
        ParadeError::Generic(err.to_string())
    }
}

// ParadeError into other types

impl From<ParadeError> for DataFusionError {
    fn from(err: ParadeError) -> Self {
        DataFusionError::Internal(err.to_string())
    }
}

impl From<DataTypeError> for DataFusionError {
    fn from(err: DataTypeError) -> Self {
        DataFusionError::Internal(err.to_string())
    }
}

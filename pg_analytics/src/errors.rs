use deltalake::arrow::error::ArrowError;
use deltalake::datafusion::common::DataFusionError;
use deltalake::errors::DeltaTableError;
use pgrx::*;
use std::ffi::{NulError, OsString};
use std::num::ParseIntError;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use thiserror::Error;

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

    #[error("No catalog registered with name {0}")]
    CatalogNotFound(String),

    #[error("No schema registered with name {0}")]
    SchemaNotFound(String),

    #[error("No table registered with name {0}")]
    TableNotFound(String),

    #[error("Failed to convert to datum {0}")]
    DatumNotFound(String),

    #[error("Data type {0} not supported")]
    DataTypeNotSupported(String),

    #[error("Invalid deltalake handler oid")]
    InvalidHandlerOid,

    #[error("Expected value of type {0} but found None")]
    NoneError(String),

    #[error("{0}")]
    NotSupported(String),

    #[error("{0}")]
    Generic(String),
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

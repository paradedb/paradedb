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

use crate::datafusion::table::RESERVED_TID_FIELD;
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
    NotFound(#[from] NotFound),

    #[error(transparent)]
    TIDError(#[from] TIDError),

    #[error(transparent)]
    TransactionError(#[from] TransactionError),

    #[error(transparent)]
    DataType(#[from] DataTypeError),

    #[error(transparent)]
    NotSupported(#[from] NotSupported),

    #[error(
        "pg_analytics not found in shared_preload_libraries. Check your postgresql.conf file."
    )]
    SharedPreload,

    #[error("Column name {} is reserved by pg_analytics", RESERVED_TID_FIELD)]
    ReservedFieldName,

    #[error("{0}")]
    Generic(String),
}

#[derive(Error, Debug)]
pub enum NotFound {
    #[error("Database {0} not found")]
    Database(String),

    #[error("No catalog registered with name {0}")]
    Catalog(String),

    #[error("No schema registered with name {0}")]
    Schema(String),

    #[error("No table registered with name {0}")]
    Table(String),

    #[error("Expected value of type {0} but found None")]
    Value(String),

    #[error("No function exists with name {0}")]
    Function(String),
}

#[derive(Error, Debug)]
pub enum NotSupported {
    #[error("TEMP tables are not yet supported")]
    TempTable,

    #[error("ADD COLUMN is not yet supported. Please recreate the table instead.")]
    AddColumn,

    #[error("DROP COLUMN is not yet supported. Please recreate the table instead.")]
    DropColumn,

    #[error("ALTER COLUMN is not yet supported. Please recreate the table instead.")]
    AlterColumn,

    #[error("RENAME COLUMN is not yet supported. Please recreate the table instead.")]
    RenameColumn,

    #[error("UPDATE is not supported because Parquet tables are append only.")]
    Update,

    #[error("DELETE is not supported because Parquet tables are append only.")]
    Delete,

    #[error("Inserts with ON CONFLICT are not yet supported")]
    SpeculativeInsert,

    #[error("JOIN with operation {0} not yet supported")]
    Join(pg_sys::CmdType),
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

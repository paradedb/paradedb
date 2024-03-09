use deltalake::arrow::error::ArrowError;
use deltalake::datafusion::common::DataFusionError;
use deltalake::errors::DeltaTableError;
use pgrx::*;
use shared::postgres::transaction::TransactionError;
use std::ffi::{NulError, OsString};
use std::num::ParseIntError;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use thiserror::Error;

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
    TransactionError(#[from] TransactionError),

    #[error(transparent)]
    DataType(#[from] DataTypeError),

    #[error(transparent)]
    NotSupported(#[from] NotSupported),

    #[error(
        "pg_analytics not found in shared_preload_libraries. Check your postgresql.conf file."
    )]
    SharedPreload,

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

    #[error("Heap and parquet tables in the same query is not yet supported")]
    MixedTables,
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

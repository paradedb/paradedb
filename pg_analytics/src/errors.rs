use deltalake::arrow::error::ArrowError;
use deltalake::datafusion::arrow::datatypes::DataType;
use deltalake::datafusion::common::DataFusionError;
use deltalake::datafusion::sql::sqlparser::ast::DataType as SQLDataType;
use deltalake::errors::DeltaTableError;
use pgrx::*;
use shared::postgres::transaction::TransactionError;
use std::ffi::{NulError, OsString};
use std::num::ParseIntError;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use thiserror::Error;

use crate::datafusion::timestamp::TimestampError;

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
    Timestamp(#[from] TimestampError),

    #[error(transparent)]
    NotSupported(#[from] NotSupported),

    #[error("Could not downcast generic arrow array: {0}")]
    DowncastGenericArray(DataType),

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

    #[error("Failed to convert to datum {0}")]
    Datum(String),

    #[error("Expected value of type {0} but found None")]
    Value(String),
}

#[derive(Error, Debug)]
pub enum NotSupported {
    #[error("DataType {0} not supported")]
    DataType(DataType),

    #[error("SQLDataType {0} not supported")]
    SQLDataType(SQLDataType),

    #[error("Postgres type {0:?} not supported")]
    BuiltinPostgresType(pg_sys::BuiltinOid),

    #[error("TEMP tables are not yet supported")]
    TempTable,

    #[error("Invalid Postgres type not supported")]
    InvalidPostgresType,

    #[error("Custom Postgres types are not supported")]
    CustomPostgresType,

    #[error("DROP COLUMN is not yet supported. Please recreate the table instead.")]
    DropColumn,

    #[error("ALTER COLUMN is not yet supported. Please recreate the table instead.")]
    AlterColumn,

    #[error("RENAME COLUMN is not yet supported. Please recreate the table instead.")]
    RenameColumn,

    #[error("UPDATE is not yet supported for parquet tables")]
    Update,

    #[error("Heap and parquet tables in the same query is not yet supported")]
    MixedTables,

    #[error("Nested DELETE queries are not yet supported for parquet tables")]
    NestedDelete,

    #[error("Run TRUNCATE <table_name> to delete all rows from a table")]
    ScanDelete,

    #[error("Time unit not supported")]
    TimeUnit,

    #[error("Precision {0} is not supported.")]
    NumericPrecision(i32),

    #[error("Scale {0} is not supported.")]
    NumericScale(i32),
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

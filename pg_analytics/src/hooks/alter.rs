use pgrx::*;

use crate::errors::ParadeError;

pub fn alter(_alter_stmt: *mut pg_sys::AlterTableStmt) -> Result<(), ParadeError> {
    Err(ParadeError::Generic(
        "ALTER TABLE is not yet supported. Please DROP and CREATE the table instead.".to_string(),
    ))
}

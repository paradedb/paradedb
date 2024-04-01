use deltalake::datafusion::sql::parser;
use deltalake::datafusion::sql::sqlparser::ast::{AlterTableOperation::*, Statement};
use pgrx::*;
use thiserror::Error;

use crate::hooks::handler::{HandlerError, IsColumn};

pub async unsafe fn alter(
    alter_stmt: *mut pg_sys::AlterTableStmt,
    statement: &parser::Statement,
) -> Result<(), AlterHookError> {
    let rangevar = (*alter_stmt).relation;
    let rangevar_oid = pg_sys::RangeVarGetRelidExtended(
        rangevar,
        pg_sys::ShareUpdateExclusiveLock as i32,
        0,
        None,
        std::ptr::null_mut(),
    );
    let relation = pg_sys::RelationIdGetRelation(rangevar_oid);

    if relation.is_null() {
        return Ok(());
    }

    if !relation.is_column()? {
        pg_sys::RelationClose(relation);
        return Ok(());
    }

    pg_sys::RelationClose(relation);

    if let parser::Statement::Statement(inner_statement) = statement {
        if let Statement::AlterTable { operations, .. } = inner_statement.as_ref() {
            for operation in operations {
                match operation {
                    AddColumn { .. } => {
                        return Err(AlterHookError::AddColumnNotSupported);
                    }
                    DropColumn { .. } => {
                        return Err(AlterHookError::DropColumnNotSupported);
                    }
                    AlterColumn { .. } | ChangeColumn { .. } => {
                        return Err(AlterHookError::AlterColumnNotSupported);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

#[derive(Error, Debug)]
pub enum AlterHookError {
    #[error(transparent)]
    HandlerError(#[from] HandlerError),

    #[error("ADD COLUMN is not yet supported. Please recreate the table instead.")]
    AddColumnNotSupported,

    #[error("DROP COLUMN is not yet supported. Please recreate the table instead.")]
    DropColumnNotSupported,

    #[error("ALTER COLUMN is not yet supported. Please recreate the table instead.")]
    AlterColumnNotSupported,
}

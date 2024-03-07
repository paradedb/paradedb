use deltalake::datafusion::sql::parser;
use deltalake::datafusion::sql::sqlparser::ast::{AlterTableOperation::*, Statement};
use pgrx::*;

use crate::errors::{NotSupported, ParadeError};
use crate::hooks::handler::IsColumn;

pub async unsafe fn alter(
    alter_stmt: *mut pg_sys::AlterTableStmt,
    statement: &parser::Statement,
) -> Result<(), ParadeError> {
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
                        return Err(NotSupported::AddColumn.into());
                    }
                    DropColumn { .. } => {
                        return Err(NotSupported::DropColumn.into());
                    }
                    AlterColumn { .. } | ChangeColumn { .. } => {
                        return Err(NotSupported::AlterColumn.into());
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

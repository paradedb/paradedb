use async_std::task;
use deltalake::datafusion::arrow::datatypes::{DataType, Field, Schema as ArrowSchema};
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::sql::parser;
use deltalake::datafusion::sql::sqlparser::ast::{AlterTableOperation::*, ColumnOption, Statement};
use pgrx::*;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::datatype::DatafusionTypeTranslator;
use crate::errors::{NotSupported, ParadeError};
use crate::hooks::handler::IsColumn;

pub unsafe fn alter(
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

    let pg_relation = unsafe { PgRelation::from_pg_owned(relation) };
    let _table_name = pg_relation.name();
    let schema_name = pg_relation.namespace();
    let mut fields_to_add = vec![];

    if let parser::Statement::Statement(inner_statement) = statement {
        if let Statement::AlterTable { operations, .. } = inner_statement.as_ref() {
            for operation in operations {
                match operation {
                    AddColumn { column_def, .. } => {
                        let options = &column_def.options;
                        let nullability = options
                            .iter()
                            .any(|opt| matches!(opt.option, ColumnOption::Null));
                        fields_to_add.push(Field::new(
                            column_def.name.value.clone(),
                            DataType::from_sql_data_type(column_def.data_type.clone())?,
                            !nullability,
                        ));
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

    if !fields_to_add.is_empty() {
        let schema = Arc::new(ArrowSchema::new(fields_to_add));
        let batch = RecordBatch::new_empty(schema);

        DatafusionContext::with_schema_provider(schema_name, |provider| {
            // task::block_on(provider.merge_schema(&pg_relation, batch))
            Ok(())
        })?;
    }

    Ok(())
}

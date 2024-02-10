use async_std::task;
use deltalake::datafusion::arrow::datatypes::{DataType, Field, Schema as ArrowSchema};
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::error::DataFusionError;
use deltalake::datafusion::sql::parser::{self, DFParser};
use deltalake::datafusion::sql::sqlparser::ast::{AlterTableOperation::*, ColumnOption, Statement};
use deltalake::datafusion::sql::sqlparser::dialect::PostgreSqlDialect;
use pgrx::*;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::datatype::DatafusionTypeTranslator;
use crate::errors::{NotSupported, ParadeError};
use crate::hooks::handler::DeltaHandler;

pub unsafe fn alter(
    alter_stmt: *mut pg_sys::AlterTableStmt,
    query_string: &str,
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

    if !DeltaHandler::relation_is_delta(relation)? {
        pg_sys::RelationClose(relation);
        return Ok(());
    }

    let pg_relation = unsafe { PgRelation::from_pg_owned(relation) };
    let table_name = pg_relation.name();
    let schema_name = pg_relation.namespace();

    let dialect = PostgreSqlDialect {};
    let ast = DFParser::parse_sql_with_dialect(query_string, &dialect)
        .map_err(|err| ParadeError::DataFusion(DataFusionError::SQL(err, None)))?;

    if let parser::Statement::Statement(statement) = &ast[0] {
        if let Statement::AlterTable { operations, .. } = statement.as_ref() {
            for operation in operations {
                match operation {
                    AddColumn { column_def, .. } => {
                        let options = &column_def.options;
                        let nullability = options
                            .iter()
                            .any(|opt| matches!(opt.option, ColumnOption::Null));
                        let schema = Arc::new(ArrowSchema::new(vec![Field::new(
                            column_def.name.value.clone(),
                            DataType::from_sql_data_type(column_def.data_type.clone())?,
                            !nullability,
                        )]));
                        let batch = RecordBatch::new_empty(schema);

                        DatafusionContext::with_schema_provider(schema_name, |provider| {
                            task::block_on(provider.merge_schema(table_name, batch))
                        })?;
                    }
                    DropColumn { .. } => {
                        return Err(NotSupported::DropColumn.into());
                    }
                    AlterColumn { .. } | ChangeColumn { .. } => {
                        return Err(NotSupported::AlterColumn.into());
                    }
                    RenameColumn { .. } => {
                        return Err(NotSupported::RenameColumn.into());
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

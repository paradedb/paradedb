use deltalake::datafusion::sql::parser;
use deltalake::datafusion::sql::sqlparser::ast::{AlterTableOperation::*, ColumnOption, Statement};
use pgrx::*;

use crate::datafusion::table::DatafusionTable;
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

    let pg_relation = PgRelation::from_pg_owned(relation);
    let _schema_name = pg_relation.namespace();
    let _table_path = pg_relation.table_path()?;
    // let mut fields_to_add = vec![];

    if let parser::Statement::Statement(inner_statement) = statement {
        if let Statement::AlterTable { operations, .. } = inner_statement.as_ref() {
            for operation in operations {
                match operation {
                    AddColumn { column_def, .. } => {
                        let options = &column_def.options;
                        let _nullability = options
                            .iter()
                            .any(|opt| matches!(opt.option, ColumnOption::Null));
                        // fields_to_add.push(Field::new(
                        //     column_def.name.value.clone(),
                        //     DataType::from_sql_data_type(column_def.data_type.clone())?,
                        //     !nullability,
                        // ));
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

    // if !fields_to_add.is_empty() {
    //     let schema = Arc::new(ArrowSchema::new(fields_to_add));
    //     let batch = RecordBatch::new_empty(schema);

    //     Session::with_tables(schema_name, |mut tables| {
    //         Box::pin(async move {
    //             let mut delta_table = tables.alter_schema(&table_path, batch).await?;

    //             delta_table.update().await?;
    //             tables.register(&table_path, delta_table)
    //         })
    //     })?;
    // }

    Ok(())
}

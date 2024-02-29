use deltalake::datafusion::logical_expr::LogicalPlan;
use pgrx::*;

use crate::datafusion::session::Session;
use crate::datafusion::table::DatafusionTable;
use crate::errors::{NotSupported, ParadeError};
use crate::hooks::handler::IsColumn;

pub async fn delete(
    rtable: *mut pg_sys::List,
    query_desc: PgBox<pg_sys::QueryDesc>,
    logical_plan: LogicalPlan,
) -> Result<(), ParadeError> {
    let rte: *mut pg_sys::RangeTblEntry;

    #[cfg(feature = "pg12")]
    {
        let current_cell = unsafe { (*rtable).head };
        rte = unsafe { (*current_cell).data.ptr_value as *mut pg_sys::RangeTblEntry };
    }
    #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
    {
        let elements = unsafe { (*rtable).elements };
        rte = unsafe { (*elements.offset(0)).ptr_value as *mut pg_sys::RangeTblEntry };
    }

    let relation = unsafe { pg_sys::RelationIdGetRelation((*rte).relid) };

    if relation.is_null() {
        return Ok(());
    }

    if unsafe { !relation.is_column()? } {
        unsafe { pg_sys::RelationClose(relation) };
        return Ok(());
    }

    let pg_relation = unsafe { PgRelation::from_pg_owned(relation) };
    let schema_name = pg_relation.namespace();
    let table_path = pg_relation.table_path()?;

    let optimized_plan =
        Session::with_session_context(|context| Ok(context.state().optimize(&logical_plan)?))?;

    let metrics = if let LogicalPlan::Dml(dml_statement) = optimized_plan {
        Session::with_tables(schema_name, |tables| async move {
            match dml_statement.input.as_ref() {
                LogicalPlan::Filter(filter) => {
                    let mut lock = tables.lock().await;
                    let (delta_table, metrics) = lock
                        .delete(&table_path, Some(filter.predicate.clone()))
                        .await?;

                    lock.register(&table_path, delta_table)?;

                    Ok(metrics)
                }
                LogicalPlan::TableScan(_) => Err(NotSupported::ScanDelete.into()),
                _ => Err(NotSupported::NestedDelete.into()),
            }
        })?
    } else {
        unreachable!()
    };

    if let Some(num_deleted) = metrics.num_deleted_rows {
        unsafe {
            (*(*query_desc.clone().into_pg()).estate).es_processed = num_deleted as u64;
        }
    }

    Ok(())
}

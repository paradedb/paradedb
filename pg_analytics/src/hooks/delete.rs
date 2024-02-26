use async_std::task;
use deltalake::datafusion::logical_expr::LogicalPlan;
use pgrx::*;

use crate::datafusion::context::DatafusionContext;
use crate::errors::{NotSupported, ParadeError};

pub fn delete(
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
    let pg_relation = unsafe { PgRelation::from_pg_owned(relation) };
    let table_name = pg_relation.name();
    let schema_name = pg_relation.namespace();

    let optimized_plan = DatafusionContext::with_session_context(|context| {
        Ok(context.state().optimize(&logical_plan)?)
    })?;

    let delete_metrics = if let LogicalPlan::Dml(dml_statement) = optimized_plan {
        DatafusionContext::with_schema_provider(schema_name, |provider| {
            match dml_statement.input.as_ref() {
                LogicalPlan::Filter(filter) => {
                    // task::block_on(provider.delete(table_name, Some(filter.predicate.clone())))
                    Ok(())
                }
                LogicalPlan::TableScan(_) => Err(NotSupported::ScanDelete.into()),
                _ => Err(NotSupported::NestedDelete.into()),
            }
        })?
    } else {
        unreachable!()
    };

    // if let Some(num_deleted) = delete_metrics.num_deleted_rows {
    //     unsafe {
    //         (*(*query_desc.clone().into_pg()).estate).es_processed = num_deleted as u64;
    //     }
    // }

    Ok(())
}

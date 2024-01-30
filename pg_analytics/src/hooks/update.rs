use async_std::task;
use deltalake::datafusion::logical_expr::LogicalPlan;
use deltalake::operations::delete::DeleteMetrics;
use pgrx::*;

use crate::datafusion::context::DatafusionContext;
use crate::errors::ParadeError;

pub fn delete(
    rtable: *mut pg_sys::List,
    logical_plan: LogicalPlan,
) -> Result<DeleteMetrics, ParadeError> {
    let elements = unsafe { (*rtable).elements };
    let rte = unsafe { (*elements.offset(0)).ptr_value as *mut pg_sys::RangeTblEntry };
    let relation = unsafe { pg_sys::RelationIdGetRelation((*rte).relid) };
    let pg_relation = unsafe { PgRelation::from_pg_owned(relation) };
    let table_name = pg_relation.name();
    let schema_name = pg_relation.namespace();

    let optimized_plan = DatafusionContext::with_session_context(|context| {
        Ok(context.state().optimize(&logical_plan)?)
    })?;

    if let LogicalPlan::Dml(dml_statement) = optimized_plan {
        return DatafusionContext::with_schema_provider(schema_name, |provider| {
            if let LogicalPlan::Filter(filter) = dml_statement.input.as_ref() {
                task::block_on(provider.delete(table_name, Some(filter.predicate.clone())))
            } else {
                task::block_on(provider.delete(table_name, None))
            }
        });
    } else {
        unreachable!()
    }
}

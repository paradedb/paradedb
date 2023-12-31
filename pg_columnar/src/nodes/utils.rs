use async_std::task;
use datafusion::arrow::datatypes::Schema;

use datafusion::common::{DFSchema, DataFusionError};
use datafusion::datasource::provider_as_source;
use datafusion::logical_expr::{Expr, LogicalPlan, TableSource};
use datafusion::prelude::ParquetReadOptions;
use datafusion::sql::TableReference;
use pgrx::*;

use std::sync::Arc;

use crate::tableam::utils::{get_datafusion_fields_from_pg, get_parquet_directory, CONTEXT};

pub trait DatafusionPlanTranslator {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String>;
}

pub trait DatafusionExprTranslator {
    unsafe fn datafusion_expr(
        node: *mut pg_sys::Node,
        rtable: Option<*mut pg_sys::List>,
    ) -> Result<Expr, String>;
}

pub fn datafusion_err_to_string(msg: &'static str) -> impl Fn(DataFusionError) -> String {
    move |dfe: DataFusionError| -> String { format!("{}: {}", msg, dfe) }
}

pub fn get_datafusion_schema(
    table_name: &str,
    table_source: Arc<dyn TableSource>,
) -> Result<DFSchema, String> {
    let datafusion_schema =
        DFSchema::try_from_qualified_schema(table_name, table_source.schema().as_ref())
            .map_err(datafusion_err_to_string("Result DFSchema failed"))?;

    Ok(datafusion_schema)
}

pub unsafe fn get_datafusion_table(
    table_name: &str,
    pg_relation: &PgRelation,
) -> Result<Arc<dyn TableSource>, String> {
    let table_reference = TableReference::from(table_name);
    let fields = get_datafusion_fields_from_pg(pg_relation)?;
    let schema = Schema::new(fields);

    if !CONTEXT.table_exist(table_reference.clone()).unwrap() {
        let binding = schema.clone();
        let read_options = ParquetReadOptions::default().schema(&binding);

        task::block_on(CONTEXT.register_parquet(
            table_name,
            get_parquet_directory(table_name).as_str(),
            read_options,
        ))
        .expect("Could not register Parquet");
    }

    let table_provider =
        task::block_on(CONTEXT.table_provider(table_reference)).expect("Could not get table");

    Ok(provider_as_source(table_provider))
}

pub fn get_datafusion_table_name(pg_relation: &PgRelation) -> Result<String, String> {
    Ok(format!("{}", pg_relation.oid()).replace("oid=#", ""))
}

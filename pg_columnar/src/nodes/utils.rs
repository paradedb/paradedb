use async_std::task;
use datafusion::arrow::datatypes::Schema;

use datafusion::common::{DFSchema, DataFusionError};
use datafusion::datasource::file_format::parquet::ParquetFormat;
use datafusion::datasource::listing::{
    ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl,
};
use datafusion::datasource::provider::TableProvider;
use datafusion::datasource::provider_as_source;
use datafusion::logical_expr::{Expr, LogicalPlan, TableSource};

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

    let provider = if !CONTEXT.table_exist(table_reference.clone()).unwrap() {
        register_listing_table(table_name, &schema)?
    } else {
        task::block_on(CONTEXT.table_provider(table_reference)).expect("Could not get table")
    };

    Ok(provider_as_source(provider))
}

pub fn register_listing_table(
    table_name: &str,
    schema: &Schema,
) -> Result<Arc<dyn TableProvider>, String> {
    let table_path = ListingTableUrl::parse(unsafe { get_parquet_directory(table_name)? })
        .map_err(datafusion_err_to_string("Could not parse table path"))?;
    let file_format = ParquetFormat::new().with_enable_pruning(Some(true));
    let listing_options =
        ListingOptions::new(Arc::new(file_format)).with_file_extension(".parquet");
    let config = ListingTableConfig::new(table_path)
        .with_listing_options(listing_options)
        .with_schema((*schema).clone().into());
    let provider = Arc::new(
        ListingTable::try_new(config)
            .map_err(datafusion_err_to_string("Could not create listing table"))?,
    );
    let _ = CONTEXT
        .register_table(table_name.clone(), provider.clone())
        .map_err(datafusion_err_to_string("Could not register table"))?;

    Ok(provider)
}

pub fn get_datafusion_table_name(pg_relation: &PgRelation) -> Result<String, String> {
    Ok(format!("{}", pg_relation.oid()).replace("oid=#", ""))
}

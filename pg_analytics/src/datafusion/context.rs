use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::catalog::CatalogProvider;
use deltalake::datafusion::common::arrow::datatypes::DataType;
use deltalake::datafusion::common::config::ConfigOptions;
use deltalake::datafusion::common::DataFusionError;
use deltalake::datafusion::dataframe::DataFrame;
use deltalake::datafusion::datasource::file_format::parquet::ParquetFormat;
use deltalake::datafusion::datasource::listing::{
    ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl,
};
use deltalake::datafusion::datasource::{provider_as_source, TableProvider};
use deltalake::datafusion::execution::FunctionRegistry;
use deltalake::datafusion::logical_expr::{
    col, lit, AggregateUDF, LogicalPlanBuilder, ScalarUDF, TableSource, WindowUDF,
};
use deltalake::datafusion::sql::planner::ContextProvider;
use deltalake::datafusion::sql::TableReference;
use pgrx::*;
use std::ffi::{c_char, CStr};
use std::path::PathBuf;
use std::sync::Arc;

use super::directory::ParadeDirectory;
use super::session::Session;
use super::table::{PgTableProvider, RESERVED_XMIN_FIELD};
use crate::errors::{NotFound, ParadeError};

pub struct QueryContext {
    options: ConfigOptions,
}

impl QueryContext {
    pub fn new() -> Result<Self, ParadeError> {
        Ok(Self {
            options: ConfigOptions::new(),
        })
    }

    fn get_table_source_impl(
        reference: TableReference,
    ) -> Result<Arc<dyn TableSource>, ParadeError> {
        let schema_name = reference.schema();

        if let Some(schema_name) = schema_name {
            // If a schema was provided in the query, i.e. SELECT * FROM <schema>.<table>
            let table_name = reference.table().to_string();
            get_source(schema_name.to_string(), table_name)
        } else {
            // If no schema was provided in the query, i.e. SELECT * FROM <table>
            // Read all schemas from the Postgres search path and cascade through them
            // until a table is found
            let current_schemas = unsafe {
                direct_function_call::<Array<pg_sys::Datum>>(
                    pg_sys::current_schemas,
                    &[Some(pg_sys::Datum::from(true))],
                )
            };

            if let Some(current_schemas) = current_schemas {
                for datum in current_schemas.iter().flatten() {
                    let schema_name =
                        unsafe { CStr::from_ptr(datum.cast_mut_ptr::<c_char>()).to_str()? };
                    let schema_registered =
                        Session::with_catalog(|catalog| Ok(catalog.schema(schema_name).is_some()))?;

                    if !schema_registered {
                        continue;
                    }

                    let table_name = reference.table().to_string();
                    let table_registered =
                        Session::with_schema_provider(schema_name, |provider| {
                            Box::pin(async move { Ok(provider.table_exist(&table_name)) })
                        })?;

                    if !table_registered {
                        continue;
                    }

                    let table_name = reference.table().to_string();
                    return get_source(schema_name.to_string(), table_name);
                }
            }

            Err(NotFound::Table(reference.table().to_string()).into())
        }
    }
}

impl ContextProvider for QueryContext {
    fn get_table_source(
        &self,
        reference: TableReference,
    ) -> Result<Arc<dyn TableSource>, DataFusionError> {
        Self::get_table_source_impl(reference)
            .map_err(|err| DataFusionError::Execution(err.to_string()))
    }

    fn get_function_meta(&self, name: &str) -> Option<Arc<ScalarUDF>> {
        Session::with_session_context(|context| {
            let context_res = context.udf(name);
            Box::pin(async move { Ok(context_res?) })
        })
        .ok()
    }

    fn get_aggregate_meta(&self, _name: &str) -> Option<Arc<AggregateUDF>> {
        None
    }

    fn get_variable_type(&self, _variable_names: &[String]) -> Option<DataType> {
        None
    }

    fn get_window_meta(&self, _name: &str) -> Option<Arc<WindowUDF>> {
        None
    }

    fn options(&self) -> &ConfigOptions {
        &self.options
    }
}

#[inline]
fn get_source(
    schema_name: String,
    table_name: String,
) -> Result<Arc<dyn TableSource>, ParadeError> {
    Session::with_tables(&schema_name.clone(), |mut tables| {
        Box::pin(async move {
            let table_path = table_path(&schema_name, &table_name)?.unwrap();

            let provider = tables.get_ref(&table_path).await?;
            let delta_table = provider.table();

            let listing_table_url = ListingTableUrl::parse(table_path.to_str().unwrap())?;
            let file_format = ParquetFormat::new();
            let listing_options =
                ListingOptions::new(Arc::new(file_format)).with_file_extension(".parquet");

            let state = Session::with_session_context(|context| {
                Box::pin(async move { Ok(context.state()) })
            })?;

            let resolved_schema = listing_options
                .infer_schema(&state, &listing_table_url)
                .await?;

            let config = ListingTableConfig::new(listing_table_url)
                .with_listing_options(listing_options)
                .with_schema(resolved_schema);

            let listing_provider = Arc::new(ListingTable::try_new(config)?);
            let dataframe = Session::with_session_context(|context| {
                Box::pin(async move { Ok(context.read_table(listing_provider.clone())?) })
            })?;

            let current_transaction_rows = dataframe.filter(
                col(RESERVED_XMIN_FIELD).eq(lit(unsafe { pg_sys::GetCurrentTransactionId() })),
            )?;

            let reference = TableReference::full(Session::catalog_name()?, schema_name, table_name);

            let table_scan = LogicalPlanBuilder::scan(
                reference,
                provider_as_source(Arc::new(delta_table.clone()) as Arc<dyn TableProvider>),
                None,
            )?
            .build()?;

            let committed_rows = DataFrame::new(state, table_scan);
            let full_dataframe = current_transaction_rows.union(committed_rows)?;
            let provider = PgTableProvider::new(delta_table.clone())
                .with_logical_plan(full_dataframe.logical_plan().clone());

            Ok(provider_as_source(Arc::new(provider)))
        })
    })
}

#[inline]
fn table_path(schema_name: &str, table_name: &str) -> Result<Option<PathBuf>, ParadeError> {
    let pg_relation =
        match unsafe { PgRelation::open_with_name(&format!("{}.{}", schema_name, table_name)) } {
            Ok(relation) => relation,
            Err(_) => {
                return Ok(None);
            }
        };

    Ok(Some(ParadeDirectory::table_path(
        Session::catalog_oid()?,
        pg_relation.namespace_oid(),
        pg_relation.oid(),
    )?))
}

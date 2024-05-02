use async_std::task;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::catalog::CatalogProvider;
use deltalake::datafusion::common::arrow::datatypes::{DataType, Field, SchemaBuilder};
use deltalake::datafusion::common::config::ConfigOptions;
use deltalake::datafusion::common::DataFusionError;
use deltalake::datafusion::datasource::listing::{
    ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl,
};
use deltalake::datafusion::datasource::{provider_as_source, view::ViewTable, TableProvider};
use deltalake::datafusion::execution::context::SessionState;
use deltalake::datafusion::execution::FunctionRegistry;
use deltalake::datafusion::logical_expr::{AggregateUDF, ScalarUDF, TableSource, WindowUDF};
use deltalake::datafusion::sql::planner::ContextProvider;
use deltalake::datafusion::sql::TableReference;
use fdw::format::*;
use fdw::handler::*;
use fdw::options::*;
use object_store::aws::AmazonS3;
use object_store::local::LocalFileSystem;
use pgrx::*;
use std::collections::HashMap;
use std::ffi::{c_char, CStr};
use std::sync::Arc;
use thiserror::Error;
use url::Url;

use super::catalog::CatalogError;
use super::directory::ParadeDirectory;
use super::plan::LogicalPlanDetails;
use super::query::QueryString;
use super::schema::ParadeSchemaProvider;
use super::session::Session;
use super::table::PgTableProvider;

use crate::hooks::handler::get_fdw_handler;

pub struct QueryContext {
    options: ConfigOptions,
}

impl QueryContext {
    pub fn new() -> Result<Self, ContextError> {
        Ok(Self {
            options: ConfigOptions::new(),
        })
    }

    fn get_table_source_impl(
        reference: TableReference,
    ) -> Result<Arc<dyn TableSource>, ContextError> {
        let schema_name = reference.schema();

        if let Some(schema_name) = schema_name {
            // If a schema was provided in the query, i.e. SELECT * FROM <schema>.<table>
            get_source(schema_name, reference.table())
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

                    Session::with_catalog(|catalog| {
                        Box::pin(async move {
                            if catalog.schema(schema_name).is_none() {
                                let new_schema_provider =
                                    Arc::new(ParadeSchemaProvider::try_new(schema_name).await?);
                                catalog.register_schema(schema_name, new_schema_provider)?;
                            }

                            Ok(())
                        })
                    })?;

                    let table_name = reference.table().to_string();
                    let table_exists = Session::with_schema_provider(schema_name, |provider| {
                        Box::pin(async move { Ok(provider.table_exist(&table_name)) })
                    })?;

                    if !table_exists {
                        continue;
                    }

                    return get_source(schema_name, reference.table());
                }
            }

            // If no table was found, try to register it as a view
            let pg_relation = (match schema_name {
                None => unsafe { PgRelation::open_with_name(reference.table()) },
                Some(schema_name) => unsafe {
                    PgRelation::open_with_name(
                        format!("{}.{}", schema_name, reference.table()).as_str(),
                    )
                },
            })
            .map_err(|_| ContextError::TableNotFound(reference.table().to_string()))?;

            if pg_relation.is_view() {
                let view_definition = unsafe {
                    direct_function_call::<String>(
                        pg_sys::pg_get_viewdef,
                        &[Some(pg_sys::Datum::from(pg_relation.oid()))],
                    )
                    .ok_or(ContextError::ViewNotFound(reference.table().to_string()))?
                };

                let plan = LogicalPlanDetails::try_from(QueryString(&view_definition))
                    .map_err(|_| ContextError::ViewParseError)?;
                let view_table = ViewTable::try_new(plan.logical_plan(), None)?;
                return Ok(provider_as_source(Arc::new(view_table)));
            }

            if pg_relation.is_foreign_table() {
                let foreign_table = unsafe { pg_sys::GetForeignTable(pg_relation.oid()) };
                let foreign_server = unsafe { pg_sys::GetForeignServer((*foreign_table).serverid) };
                let fdw_handler = unsafe { get_fdw_handler((*foreign_server).fdwid) };

                if fdw_handler == FdwHandler::S3 || fdw_handler == FdwHandler::LocalFile {
                    let table_options = unsafe { options_to_hashmap((*foreign_table).options)? };
                    let server_options = unsafe { options_to_hashmap((*foreign_server).options)? };

                    let format =
                        require_option_or(TableOption::Format.as_str(), &table_options, "");

                    // Register object store
                    match fdw_handler {
                        FdwHandler::S3 => {
                            Session::with_session_context(|context| {
                                Box::pin(async move {
                                    let object_store =
                                        AmazonS3::try_from(ServerOptions(server_options.clone()))?;
                                    let url = require_option(
                                        AmazonServerOption::Url.as_str(),
                                        &server_options,
                                    )?;

                                    context.runtime_env().register_object_store(
                                        &Url::parse(url)?,
                                        Arc::new(object_store),
                                    );
                                    Ok(())
                                })
                            })?;
                        }
                        FdwHandler::LocalFile => {
                            let object_store = LocalFileSystem::new();
                            Session::with_session_context(|context| {
                                Box::pin(async move {
                                    context.runtime_env().register_object_store(
                                        &Url::parse("file://")?,
                                        Arc::new(object_store),
                                    );
                                    Ok(())
                                })
                            })?;
                        }
                        _ => {}
                    };

                    let provider = match TableFormat::from(format) {
                        TableFormat::None => Session::with_session_context(|context| {
                            Box::pin(async move {
                                create_listing_provider(
                                    table_options,
                                    pg_relation,
                                    &context.state(),
                                )
                            })
                        })?,
                        TableFormat::Delta => task::block_on(create_delta_provider(
                            table_options.clone(),
                            pg_relation,
                        ))?,
                    };

                    return Ok(provider_as_source(provider));
                }
            }

            Err(ContextError::TableNotFound(reference.table().to_string()))
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

    fn udfs_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn udafs_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn udwfs_names(&self) -> Vec<String> {
        Vec::new()
    }
}

#[inline]
fn get_source(schema_name: &str, table_name: &str) -> Result<Arc<dyn TableSource>, ContextError> {
    let schema_name = schema_name.to_string();
    let table_name = table_name.to_string();

    Ok(Session::with_tables(&schema_name.clone(), |mut tables| {
        Box::pin(async move {
            let table_path =
                ParadeDirectory::table_path_from_name(&schema_name.clone(), &table_name)?;
            let delta_table = tables.get_ref(&table_path).await?;
            let provider =
                PgTableProvider::new(delta_table.clone(), &schema_name, &table_name).await?;

            Ok(provider_as_source(Arc::new(provider)))
        })
    })?)
}

#[inline]
fn create_listing_provider(
    table_options: HashMap<String, String>,
    pg_relation: PgRelation,
    state: &SessionState,
) -> Result<Arc<dyn TableProvider>, CatalogError> {
    // TODO: Move to shared
    let path = require_option(TableOption::Path.as_str(), &table_options)?;
    let extension = require_option(TableOption::Extension.as_str(), &table_options)?;

    let listing_url = ListingTableUrl::parse(path)?;
    let listing_options = ListingOptions::try_from(FileExtension(extension.to_string()))?;
    let inferred_schema = task::block_on(listing_options.infer_schema(state, &listing_url))?;
    let mut schema_builder = SchemaBuilder::new();

    for (index, attribute) in pg_relation.tuple_desc().iter().enumerate() {
        if attribute.name() != inferred_schema.field(index).name() {
            return Err(CatalogError::ColumnNameMismatch(
                inferred_schema.field(index).name().to_string(),
                index + 1,
                attribute.name().to_string(),
            ));
        }

        let data_type = match attribute.type_oid() {
            PgOid::BuiltIn(oid) => match oid {
                pg_sys::BuiltinOid::BOOLOID => DataType::Boolean,
                pg_sys::BuiltinOid::DATEOID => DataType::Int32,
                pg_sys::BuiltinOid::TIMESTAMPOID => DataType::Int64,
                pg_sys::BuiltinOid::VARCHAROID => DataType::Utf8,
                pg_sys::BuiltinOid::BPCHAROID => DataType::Utf8,
                pg_sys::BuiltinOid::TEXTOID => DataType::Utf8,
                pg_sys::BuiltinOid::INT2OID => DataType::Int16,
                pg_sys::BuiltinOid::INT4OID => DataType::Int32,
                pg_sys::BuiltinOid::INT8OID => DataType::Int64,
                pg_sys::BuiltinOid::FLOAT4OID => DataType::Float32,
                pg_sys::BuiltinOid::FLOAT8OID => DataType::Float64,
                unsupported => {
                    return Err(CatalogError::ForeignTypeNotSupported(PgOid::from(
                        unsupported,
                    )))
                }
            },
            unsupported => return Err(CatalogError::ForeignTypeNotSupported(unsupported)),
        };
        schema_builder.push(Field::new(
            inferred_schema.field(index).name(),
            data_type,
            true,
        ));
    }

    let schema = Arc::new(schema_builder.finish());

    let listing_config = ListingTableConfig::new(listing_url)
        .with_listing_options(listing_options)
        .with_schema(schema);

    let listing_table = ListingTable::try_new(listing_config)?;

    Ok(Arc::new(listing_table) as Arc<dyn TableProvider>)
}

#[inline]
async fn create_delta_provider(
    table_options: HashMap<String, String>,
    pg_relation: PgRelation,
) -> Result<Arc<dyn TableProvider>, CatalogError> {
    let path = require_option(TableOption::Path.as_str(), &table_options)?;
    let provider = Arc::new(deltalake::open_table(path).await?) as Arc<dyn TableProvider>;
    let schema = (provider.clone()).schema();

    for (index, attribute) in pg_relation.tuple_desc().iter().enumerate() {
        if attribute.name() != schema.field(index).name() {
            return Err(CatalogError::ColumnNameMismatch(
                schema.field(index).name().to_string(),
                index + 1,
                attribute.name().to_string(),
            ));
        }
    }

    Ok(provider)
}

#[inline]
pub unsafe fn options_to_hashmap(
    options: *mut pg_sys::List,
) -> Result<HashMap<String, String>, ContextError> {
    let mut ret = HashMap::new();
    let options: PgList<pg_sys::DefElem> = PgList::from_pg(options);
    for option in options.iter_ptr() {
        let name = CStr::from_ptr((*option).defname);
        let value = CStr::from_ptr(pg_sys::defGetString(option));
        let name = name.to_str()?;
        let value = value.to_str()?;
        ret.insert(name.to_string(), value.to_string());
    }
    Ok(ret)
}

#[derive(Error, Debug)]
pub enum ContextError {
    #[error(transparent)]
    CatalogError(#[from] CatalogError),

    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error(transparent)]
    DeltaTableError(#[from] deltalake::DeltaTableError),

    #[error(transparent)]
    LakeError(#[from] fdw::lake::LakeError),

    #[error(transparent)]
    OptionsError(#[from] OptionsError),

    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("No table registered with name {0}")]
    TableNotFound(String),

    #[error("Could not get definition for view {0}")]
    ViewNotFound(String),

    #[error("Could not parse view definition")]
    ViewParseError,
}

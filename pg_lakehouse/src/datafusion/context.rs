use async_std::task;
use datafusion::common::arrow::datatypes::DataType;
use datafusion::common::config::ConfigOptions;
use datafusion::common::DataFusionError;
use datafusion::datasource::{provider_as_source, view::ViewTable};
use datafusion::execution::FunctionRegistry;
use datafusion::logical_expr::{AggregateUDF, LogicalPlan, ScalarUDF, TableSource, WindowUDF};
use datafusion::sql::planner::ContextProvider;
use datafusion::sql::TableReference;
use fdw::format::*;
use fdw::handler::*;
use fdw::options::*;
use object_store::aws::AmazonS3;
use object_store::local::LocalFileSystem;
use pgrx::*;
use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::Arc;
use thiserror::Error;
use url::Url;

use crate::types::schema::*;

use super::provider::*;
use super::query::*;
use super::session::Session;

pub struct QueryContext {
    options: ConfigOptions,
}

impl QueryContext {
    pub fn new() -> Self {
        Self {
            options: ConfigOptions::new(),
        }
    }
}

impl ContextProvider for QueryContext {
    fn get_table_source(
        &self,
        reference: TableReference,
    ) -> Result<Arc<dyn TableSource>, DataFusionError> {
        task::block_on(get_table_source(reference))
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
async fn get_table_source(
    reference: TableReference<'_>,
) -> Result<Arc<dyn TableSource>, ContextError> {
    let schema_name = reference.schema();

    // If no table was found, try to register it as a view
    let pg_relation = (match schema_name {
        None => unsafe { PgRelation::open_with_name(reference.table()) },
        Some(schema_name) => unsafe {
            PgRelation::open_with_name(format!("{}.{}", schema_name, reference.table()).as_str())
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

        let logical_plan = LogicalPlan::try_from(QueryString(&view_definition))?;
        let view_table = ViewTable::try_new(logical_plan, None)?;
        return Ok(provider_as_source(Arc::new(view_table)));
    }

    if pg_relation.is_foreign_table() {
        let foreign_table = unsafe { pg_sys::GetForeignTable(pg_relation.oid()) };
        let foreign_server = unsafe { pg_sys::GetForeignServer((*foreign_table).serverid) };
        let fdw_handler = unsafe { get_fdw_handler((*foreign_server).fdwid) };

        if fdw_handler == FdwHandler::S3 || fdw_handler == FdwHandler::LocalFile {
            let table_options = unsafe { options_to_hashmap((*foreign_table).options)? };
            let server_options = unsafe { options_to_hashmap((*foreign_server).options)? };
            let user_id = unsafe { pg_sys::GetUserId() };

            let user_mapping_exists = unsafe {
                !pg_sys::SearchSysCache2(
                    pg_sys::SysCacheIdentifier_USERMAPPINGUSERSERVER as i32,
                    pg_sys::Datum::from(user_id),
                    pg_sys::Datum::from((*foreign_server).serverid),
                )
                .is_null()
            };
            let public_mapping_exists = unsafe {
                !pg_sys::SearchSysCache2(
                    pg_sys::SysCacheIdentifier_USERMAPPINGUSERSERVER as i32,
                    pg_sys::Datum::from(pg_sys::InvalidOid),
                    pg_sys::Datum::from((*foreign_server).serverid),
                )
                .is_null()
            };

            let user_mapping_options = unsafe {
                match user_mapping_exists || public_mapping_exists {
                    true => {
                        let user_mapping =
                            pg_sys::GetUserMapping(user_id, (*foreign_server).serverid);
                        options_to_hashmap((*user_mapping).options)?
                    }
                    false => HashMap::new(),
                }
            };

            let format = require_option_or(TableOption::Format.as_str(), &table_options, "");

            // Register object store
            match fdw_handler {
                FdwHandler::S3 => {
                    Session::with_session_context(|context| {
                        Box::pin(async move {
                            let object_store = AmazonS3::try_from(ServerOptions::new(
                                server_options.clone(),
                                user_mapping_options.clone(),
                            ))?;
                            let url =
                                require_option(AmazonServerOption::Url.as_str(), &server_options)?;

                            context
                                .runtime_env()
                                .register_object_store(&Url::parse(url)?, Arc::new(object_store));
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

            let attribute_map: HashMap<usize, PgAttribute> = pg_relation
                .tuple_desc()
                .iter()
                .enumerate()
                .map(|(index, attribute)| {
                    (
                        index,
                        PgAttribute::new(
                            attribute.name(),
                            attribute.atttypid,
                            attribute.type_mod(),
                        ),
                    )
                })
                .collect();

            let provider = match TableFormat::from(format) {
                TableFormat::None => Session::with_session_context(|context| {
                    Box::pin(async move {
                        Ok(
                            create_listing_provider(table_options, attribute_map, &context.state())
                                .await?,
                        )
                    })
                })?,
                TableFormat::Delta => {
                    create_delta_provider(table_options.clone(), attribute_map).await?
                }
            };

            return Ok(provider_as_source(provider));
        }
    }

    Err(ContextError::TableNotFound(reference.table().to_string()))
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

pub unsafe fn get_fdw_handler(oid: pg_sys::Oid) -> FdwHandler {
    let fdw = pg_sys::GetForeignDataWrapper(oid);
    let handler_oid = (*fdw).fdwhandler;
    let proc_tuple = pg_sys::SearchSysCache1(
        pg_sys::SysCacheIdentifier_PROCOID as i32,
        handler_oid.into_datum().unwrap(),
    );
    let pg_proc = pg_sys::GETSTRUCT(proc_tuple) as pg_sys::Form_pg_proc;
    let handler_name = name_data_to_str(&(*pg_proc).proname);
    pg_sys::ReleaseSysCache(proc_tuple);

    FdwHandler::from(handler_name)
}

#[derive(Error, Debug)]
pub enum ContextError {
    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error(transparent)]
    DeltaTableError(#[from] deltalake::DeltaTableError),

    #[error(transparent)]
    FormatError(#[from] FormatError),

    #[error(transparent)]
    LakeError(#[from] fdw::lake::LakeError),

    #[error(transparent)]
    OptionsError(#[from] OptionsError),

    #[error(transparent)]
    QueryParserError(#[from] QueryParserError),

    #[error(transparent)]
    SchemaError(#[from] SchemaError),

    #[error(transparent)]
    TableProviderError(#[from] TableProviderError),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("No table registered with name {0}")]
    TableNotFound(String),

    #[error("Could not get definition for view {0}")]
    ViewNotFound(String),
}

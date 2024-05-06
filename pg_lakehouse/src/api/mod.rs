use async_std::task;
use chrono::Datelike;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use supabase_wrappers::prelude::*;
use thiserror::Error;

use crate::datafusion::context::ContextError;
use crate::datafusion::provider::*;
use crate::datafusion::session::Session;
use crate::fdw::format::*;
use crate::fdw::handler::*;
use crate::fdw::local::register_local_file_server;
use crate::fdw::s3::register_s3_server;

#[pg_extern]
pub fn arrow_schema(
    server: String,
    path: String,
    extension: String,
    format: default!(Option<String>, "NULL"),
) -> iter::TableIterator<'static, (name!(field, String), name!(datatype, String))> {
    task::block_on(arrow_schema_impl(server, path, extension, format)).unwrap_or_else(|err| {
        panic!("{:?}", err);
    })
}

#[pg_extern]
pub fn to_date(days: i64) -> datum::Date {
    let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    let date = epoch + chrono::Duration::days(days);

    datum::Date::new(date.year(), date.month() as u8, date.day() as u8).unwrap()
}

#[inline]
async fn arrow_schema_impl(
    server: String,
    path: String,
    extension: String,
    format: Option<String>,
) -> Result<iter::TableIterator<'static, (name!(field, String), name!(datatype, String))>, ApiError>
{
    let foreign_server =
        unsafe { pg_sys::GetForeignServerByName(server.clone().as_pg_cstr(), true) };

    if foreign_server.is_null() {
        return Err(ApiError::ForeignServerNotFound(server.clone()));
    }

    let server_options = unsafe { options_to_hashmap((*foreign_server).options) }?;
    let user_mapping_options = unsafe { user_mapping_options(foreign_server) };
    let fdw_handler = unsafe { FdwHandler::from((*foreign_server).fdwid) };

    match fdw_handler {
        FdwHandler::S3 => {
            register_s3_server(server_options, user_mapping_options)?;
        }
        FdwHandler::LocalFile => {
            register_local_file_server()?;
        }
        _ => {
            return Err(ApiError::InvalidServerName(server.clone()));
        }
    }

    let provider = Session::with_session_context(|context| {
        Box::pin(async move {
            Ok(match TableFormat::from(&format.unwrap_or("".to_string())) {
                TableFormat::None => {
                    create_listing_provider(&path, &extension, &context.state()).await?
                }
                TableFormat::Delta => (create_delta_provider(&path, &extension)).await?,
            })
        })
    })?;

    Ok(iter::TableIterator::new(
        provider
            .schema()
            .fields()
            .iter()
            .map(|field| (field.name().to_string(), field.data_type().to_string()))
            .collect::<Vec<(String, String)>>(),
    ))
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error(transparent)]
    ContextError(#[from] ContextError),

    #[error(transparent)]
    OptionsError(#[from] supabase_wrappers::prelude::OptionsError),

    #[error("No foreign server with name {0} was found")]
    ForeignServerNotFound(String),

    #[error("Server {0} was not created by this extension")]
    InvalidServerName(String),
}

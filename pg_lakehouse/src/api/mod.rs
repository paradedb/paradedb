use async_std::task;
use chrono::Datelike;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use supabase_wrappers::prelude::*;
use thiserror::Error;
use url::Url;

use crate::datafusion::context::ContextError;
use crate::datafusion::format::*;
use crate::datafusion::provider::*;
use crate::fdw::handler::*;

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
        return Err(ApiError::ForeignServerNotFound(server.to_string()));
    }

    let server_options = unsafe { options_to_hashmap((*foreign_server).options) }?;
    let user_mapping_options = unsafe { user_mapping_options(foreign_server) };
    let fdw_handler = FdwHandler::from(foreign_server);
    let format = format.unwrap_or("".to_string());

    register_object_store(
        fdw_handler,
        &Url::parse(&path)?,
        TableFormat::from(&format),
        server_options,
        user_mapping_options,
    )?;

    let provider = match TableFormat::from(&format) {
        TableFormat::None => create_listing_provider(&path, &extension).await?,
        TableFormat::Delta => create_delta_provider(&path, &extension).await?,
    };

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
    Context(#[from] ContextError),

    #[error(transparent)]
    TableProviderError(#[from] TableProviderError),

    #[error(transparent)]
    Option(#[from] OptionsError),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error("No foreign server with name {0} was found")]
    ForeignServerNotFound(String),
}

use async_std::task;
use chrono::Datelike;
use pgrx::*;
use thiserror::Error;

use crate::datafusion::context::ContextError;
use crate::datafusion::format::*;
use crate::datafusion::provider::*;
use crate::datafusion::session::Session;
use crate::stores::utils::*;

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
    register_object_store(server.as_str())?;

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
    Context(#[from] ContextError),

    #[error(transparent)]
    StoreUtils(#[from] StoreUtilsError),
}

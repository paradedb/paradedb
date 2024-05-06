use async_std::task;
use chrono::Datelike;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use supabase_wrappers::prelude::*;

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
    let foreign_server = unsafe { pg_sys::GetForeignServerByName(server.as_pg_cstr(), true) };

    if foreign_server.is_null() {
        panic!("Foreign server not found");
    }

    let server_options = unsafe { options_to_hashmap((*foreign_server).options) }.unwrap();
    let user_mapping_options = unsafe { user_mapping_options(foreign_server) };
    let fdw_handler = unsafe { FdwHandler::from((*foreign_server).fdwid) };

    match fdw_handler {
        FdwHandler::S3 => {
            register_s3_server(server_options, user_mapping_options).unwrap();
        }
        FdwHandler::LocalFile => {
            register_local_file_server().unwrap();
        }
        _ => {
            panic!("Server does not belong to a foreign data wrapper created by this extension");
        }
    }

    let provider = Session::with_session_context(|context| {
        Box::pin(async move {
            Ok(match TableFormat::from(&format.unwrap_or("".to_string())) {
                TableFormat::None => {
                    task::block_on(create_listing_provider(&path, &extension, &context.state()))
                        .unwrap()
                }
                TableFormat::Delta => {
                    task::block_on(create_delta_provider(&path, &extension)).unwrap()
                }
            })
        })
    })
    .unwrap();

    let schema = provider.schema();

    iter::TableIterator::new(
        schema
            .fields()
            .iter()
            .map(|field| (field.name().to_string(), field.data_type().to_string()))
            .collect::<Vec<(String, String)>>(),
    )
}

#[pg_extern]
pub fn to_date(days: i64) -> datum::Date {
    let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    let date = epoch + chrono::Duration::days(days);

    datum::Date::new(date.year(), date.month() as u8, date.day() as u8).unwrap()
}

// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use anyhow::{anyhow, bail, Result};
use async_std::task;
use chrono::Datelike;
use datafusion::common::DataFusionError;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use supabase_wrappers::prelude::*;
use thiserror::Error;
use url::Url;

use crate::datafusion::context::{get_table_source, ContextError};
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
        panic!("{}", err);
    })
}

extension_sql!(
    r#"
    CREATE OR REPLACE PROCEDURE connect_table(table_name VARCHAR) 
    LANGUAGE C AS 'MODULE_PATHNAME', 'connect_table';
    "#,
    name = "connect_table"
);
#[pg_guard]
#[no_mangle]
pub extern "C" fn connect_table(fcinfo: pg_sys::FunctionCallInfo) {
    task::block_on(connect_table_impl(fcinfo)).unwrap_or_else(|err| {
        panic!("{}", err);
    });
}

#[pg_guard]
#[no_mangle]
extern "C" fn pg_finfo_connect_table() -> &'static pg_sys::Pg_finfo_record {
    const V1_API: pg_sys::Pg_finfo_record = pg_sys::Pg_finfo_record { api_version: 1 };
    &V1_API
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
) -> Result<iter::TableIterator<'static, (name!(field, String), name!(datatype, String))>> {
    let foreign_server =
        unsafe { pg_sys::GetForeignServerByName(server.clone().as_pg_cstr(), true) };

    if foreign_server.is_null() {
        bail!("foreign server {server} not found");
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

#[inline]
async fn connect_table_impl(fcinfo: pg_sys::FunctionCallInfo) -> Result<()> {
    let table_name: String =
        unsafe { fcinfo::pg_getarg(fcinfo, 0).ok_or_else(|| anyhow!("table_name not provided"))? };
    let _ = get_table_source(table_name.into()).await?;

    Ok(())
}

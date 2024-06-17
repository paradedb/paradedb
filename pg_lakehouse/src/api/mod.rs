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

use anyhow::{anyhow, Result};
use async_std::task;
use pgrx::*;
use supabase_wrappers::prelude::*;

use crate::duckdb::connection;
use crate::fdw::handler::*;
use crate::fdw::parquet::ParquetOption;

// #[pg_extern]
// pub fn arrow_schema(
//     server: String,
//     path: String,
//     extension: String,
//     format: default!(Option<String>, "NULL"),
// ) -> iter::TableIterator<'static, (name!(field, String), name!(datatype, String))> {
//     task::block_on(arrow_schema_impl(server, path, extension, format)).unwrap_or_else(|err| {
//         panic!("{}", err);
//     })
// }

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

// #[pg_extern]
// pub fn to_date(days: i64) -> datum::Date {
//     let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
//     let date = epoch + chrono::Duration::days(days);

//     datum::Date::new(date.year(), date.month() as u8, date.day() as u8).unwrap()
// }

// #[inline]
// async fn arrow_schema_impl(
//     server: String,
//     path: String,
//     extension: String,
//     format: Option<String>,
// ) -> Result<iter::TableIterator<'static, (name!(field, String), name!(datatype, String))>> {
//     let foreign_server =
//         unsafe { pg_sys::GetForeignServerByName(server.clone().as_pg_cstr(), true) };

//     if foreign_server.is_null() {
//         bail!("foreign server {server} not found");
//     }

//     let server_options = unsafe { options_to_hashmap((*foreign_server).options) }?;
//     let user_mapping_options = unsafe { user_mapping_options(foreign_server) };
//     let fdw_handler = FdwHandler::from(foreign_server);
//     let format = format.unwrap_or("".to_string());

//     register_object_store(
//         fdw_handler,
//         &Url::parse(&path)?,
//         TableFormat::from(&format),
//         server_options,
//         user_mapping_options,
//     )?;

//     let provider = match TableFormat::from(&format) {
//         TableFormat::None => create_listing_provider(&path, &extension).await?,
//         TableFormat::Delta => create_delta_provider(&path, &extension).await?,
//     };

//     Ok(iter::TableIterator::new(
//         provider
//             .schema()
//             .fields()
//             .iter()
//             .map(|field| (field.name().to_string(), field.data_type().to_string()))
//             .collect::<Vec<(String, String)>>(),
//     ))
// }

#[inline]
async fn connect_table_impl(fcinfo: pg_sys::FunctionCallInfo) -> Result<()> {
    let table: String =
        unsafe { fcinfo::pg_getarg(fcinfo, 0).ok_or_else(|| anyhow!("table_name not provided"))? };
    let pg_relation =
        unsafe { PgRelation::open_with_name(&table) }.map_err(|err| anyhow!(err.to_string()))?;

    if !pg_relation.is_foreign_table() {
        return Err(anyhow!("table {table} is not a foreign table"));
    }

    let foreign_table = unsafe { pg_sys::GetForeignTable(pg_relation.oid()) };
    let foreign_server = unsafe { pg_sys::GetForeignServer((*foreign_table).serverid) };
    let table_options = unsafe { options_to_hashmap((*foreign_table).options)? };
    let server_options = unsafe { options_to_hashmap((*foreign_server).options)? };
    let schema_name = pg_relation.namespace();
    let table_name = pg_relation.name();

    match FdwHandler::from(foreign_server) {
        FdwHandler::Parquet => {
            let files = require_option(ParquetOption::Files.as_str(), &table_options)?;
            let files_split = files.split(',').collect::<Vec<&str>>();
            let files_string = match files_split.len() {
                1 => format!("'{}'", files),
                _ => format!(
                    "[{}]",
                    files_split
                        .iter()
                        .map(|&chunk| format!("'{}'", chunk))
                        .collect::<Vec<String>>()
                        .join(", ")
                ),
            };

            let binary_as_string = require_option_or(
                ParquetOption::BinaryAsString.as_str(),
                &table_options,
                "false",
            );
            let file_name =
                require_option_or(ParquetOption::FileName.as_str(), &table_options, "false");
            let file_row_number = require_option_or(
                ParquetOption::FileRowNumber.as_str(),
                &table_options,
                "false",
            );
            let hive_partitioning = require_option_or(
                ParquetOption::HivePartitioning.as_str(),
                &table_options,
                "false",
            );
            let union_by_name =
                require_option_or(ParquetOption::UnionByName.as_str(), &table_options, "false");

            connection::execute(
                format!(
                    r#"
                        CREATE VIEW IF NOT EXISTS {schema_name}.{table_name} 
                        AS SELECT * FROM read_parquet(
                            {files_string},
                            binary_as_string = {binary_as_string},
                            filename = {file_name},
                            file_row_number = {file_row_number},
                            hive_partitioning = {hive_partitioning},
                            union_by_name = {union_by_name}
                        )
                    "#,
                )
                .as_str(),
                [],
            )?;
        }
        _ => {
            todo!()
        }
    }

    Ok(())
}

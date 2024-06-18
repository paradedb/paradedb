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
use crate::duckdb::parquet::{create_parquet_view, ParquetOption};
use crate::fdw::handler::*;

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
            create_parquet_view(table_name, schema_name, table_options)?;
        }
        _ => {
            todo!()
        }
    }

    Ok(())
}

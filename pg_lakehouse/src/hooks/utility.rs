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

#![allow(clippy::too_many_arguments)]

use std::ptr::null_mut;

use anyhow::Result;
use pg_sys::{CreateQueryDesc, NodeTag, QueryDesc};
use pgrx::*;

use super::query::*;
use crate::duckdb::connection;

type ProcessUtilityHook = fn(
    pstmt: PgBox<prelude::pg_sys::PlannedStmt>,
    query_string: &core::ffi::CStr,
    read_only_tree: Option<bool>,
    context: prelude::pg_sys::ProcessUtilityContext,
    params: PgBox<prelude::pg_sys::ParamListInfoData>,
    query_env: PgBox<prelude::pg_sys::QueryEnvironment>,
    dest: PgBox<prelude::pg_sys::DestReceiver>,
    completion_tag: *mut prelude::pg_sys::QueryCompletion,
) -> HookResult<()>;

pub async fn process_utility(
    pstmt: PgBox<prelude::pg_sys::PlannedStmt>,
    query_string: &core::ffi::CStr,
    read_only_tree: Option<bool>,
    context: prelude::pg_sys::ProcessUtilityContext,
    params: PgBox<prelude::pg_sys::ParamListInfoData>,
    query_env: PgBox<prelude::pg_sys::QueryEnvironment>,
    dest: PgBox<prelude::pg_sys::DestReceiver>,
    completion_tag: *mut prelude::pg_sys::QueryCompletion,
    prev_hook: ProcessUtilityHook,
) -> Result<()> {
    let stmt_type = unsafe { pstmt.utilityStmt.as_ref().unwrap().type_ };

    let is_prepare_related_stmt = is_prepare_related_stmt(stmt_type);
    // It can't check directly. need to
    // let is_duckdb_query = is_duckdb_query(&query_relations);

    if !is_prepare_related_stmt {
        prev_hook(
            pstmt,
            query_string,
            read_only_tree,
            context,
            params,
            query_env,
            dest,
            completion_tag,
        );

        return Ok(());
    }

    let query_desc = unsafe {
        PgBox::<QueryDesc, AllocatedByRust>::from_rust(CreateQueryDesc(
            pstmt.as_ptr(),
            query_string.as_ptr(),
            null_mut(),
            null_mut(),
            dest.as_ptr(),
            null_mut(),
            query_env.as_ptr(),
            0,
        ))
    };

    let need_exec_prev_hook = match stmt_type {
        pg_sys::NodeTag::T_PrepareStmt => prepare_query(query_string)?,
        pg_sys::NodeTag::T_ExecuteStmt => execute_query(query_string, query_desc)?,
        pg_sys::NodeTag::T_DeallocateStmt => true,
        _ => unreachable!(),
    };

    if need_exec_prev_hook {
        prev_hook(
            pstmt,
            query_string,
            read_only_tree,
            context,
            params,
            query_env,
            dest,
            completion_tag,
        );
    }

    Ok(())
}

fn is_prepare_related_stmt(stmt_type: NodeTag) -> bool {
    stmt_type == pg_sys::NodeTag::T_PrepareStmt
        || stmt_type == pg_sys::NodeTag::T_DeallocateStmt
        || stmt_type == pg_sys::NodeTag::T_ExecuteStmt
}

fn prepare_query(query: &core::ffi::CStr) -> Result<bool> {
    // need to parse query and check if OK to pushdown to duckDB
    // check CMD_TYPE, tables ...etc
    if let Err(e) = connection::execute(query.to_str()?, []) {
        fallback_warning!(e.to_string());
        return Ok(true);
    }

    // It's always necessary to execute the previous hook to store a prepared statement in PostgreSQL
    Ok(true)
}

fn execute_query<T: WhoAllocated>(
    query: &core::ffi::CStr,
    query_desc: PgBox<QueryDesc, T>,
) -> Result<bool> {
    // 1. need to make sure the duckdb views exist
    // 2. need to set schema correctly
    // 3. Identifying whether this query should be pushed down. (Or just use fallback pattern)
    match connection::create_arrow(query.to_str()?) {
        Err(_) => {
            connection::clear_arrow();
            return Ok(true);
        }
        Ok(false) => {
            connection::clear_arrow();
            return Ok(false);
        }
        _ => {}
    }

    match connection::get_batches() {
        Ok(batches) => write_batches_to_slots(query_desc, batches)?,
        Err(err) => {
            connection::clear_arrow();
            // Should this warning be here ? If we use fallback pattern, we can't warning here.
            // If it involves a regular PostgreSQL table, this will also trigger a warning. So I prefer to get prepare statement info to distinguish.
            fallback_warning!(err.to_string());
            return Ok(true);
        }
    }

    connection::clear_arrow();
    Ok(false)
}

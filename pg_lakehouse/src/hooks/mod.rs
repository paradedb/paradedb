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

mod executor;
mod query;
mod utility;

use async_std::task::block_on;
use pgrx::*;

pub struct LakehouseHook;

impl hooks::PgHooks for LakehouseHook {
    fn executor_run(
        &mut self,
        query_desc: PgBox<pg_sys::QueryDesc>,
        direction: pg_sys::ScanDirection,
        count: u64,
        execute_once: bool,
        prev_hook: fn(
            query_desc: PgBox<pg_sys::QueryDesc>,
            direction: pg_sys::ScanDirection,
            count: u64,
            execute_once: bool,
        ) -> HookResult<()>,
    ) -> HookResult<()> {
        block_on(executor::executor_run(
            query_desc,
            direction,
            count,
            execute_once,
            prev_hook,
        ))
        .unwrap_or_else(|err| {
            panic!("{}", err);
        });

        HookResult::new(())
    }

    fn process_utility_hook(
        &mut self,
        pstmt: PgBox<prelude::pg_sys::PlannedStmt>,
        query_string: &core::ffi::CStr,
        read_only_tree: Option<bool>,
        context: prelude::pg_sys::ProcessUtilityContext,
        params: PgBox<prelude::pg_sys::ParamListInfoData>,
        query_env: PgBox<prelude::pg_sys::QueryEnvironment>,
        dest: PgBox<prelude::pg_sys::DestReceiver>,
        completion_tag: *mut prelude::pg_sys::QueryCompletion,
        prev_hook: fn(
            pstmt: PgBox<prelude::pg_sys::PlannedStmt>,
            query_string: &core::ffi::CStr,
            read_only_tree: Option<bool>,
            context: prelude::pg_sys::ProcessUtilityContext,
            params: PgBox<prelude::pg_sys::ParamListInfoData>,
            query_env: PgBox<prelude::pg_sys::QueryEnvironment>,
            dest: PgBox<prelude::pg_sys::DestReceiver>,
            completion_tag: *mut prelude::pg_sys::QueryCompletion,
        ) -> HookResult<()>,
    ) -> HookResult<()> {
        block_on(utility::process_utility(
            pstmt,
            query_string,
            read_only_tree,
            context,
            params,
            query_env,
            dest,
            completion_tag,
            prev_hook,
        ))
        .unwrap_or_else(|err| panic!("{}", err));
        HookResult::new(())
    }
}

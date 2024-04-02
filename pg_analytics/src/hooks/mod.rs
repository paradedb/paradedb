mod alter;
mod drop;
mod executor;
mod handler;
mod process;
mod query;
mod rename;
mod select;
mod truncate;
mod udf;
mod vacuum;

use async_std::task;
use pgrx::hooks::PgHooks;
use pgrx::*;
use std::ffi::CStr;

use crate::datafusion::writer::Writer;

pub struct ParadeHook;

impl PgHooks for ParadeHook {
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
        task::block_on(Writer::flush()).unwrap_or_else(|err| {
            panic!("{}", err);
        });

        executor::executor_run(query_desc, direction, count, execute_once, prev_hook)
            .unwrap_or_else(|err| {
                panic!("{}", err);
            });

        HookResult::new(())
    }

    fn process_utility_hook(
        &mut self,
        pstmt: PgBox<pg_sys::PlannedStmt>,
        query_string: &CStr,
        read_only_tree: Option<bool>,
        context: pg_sys::ProcessUtilityContext,
        params: PgBox<pg_sys::ParamListInfoData>,
        query_env: PgBox<pg_sys::QueryEnvironment>,
        dest: PgBox<pg_sys::DestReceiver>,
        completion_tag: *mut pg_sys::QueryCompletion,
        prev_hook: fn(
            pstmt: PgBox<pg_sys::PlannedStmt>,
            query_string: &CStr,
            read_only_tree: Option<bool>,
            context: pg_sys::ProcessUtilityContext,
            params: PgBox<pg_sys::ParamListInfoData>,
            query_env: PgBox<pg_sys::QueryEnvironment>,
            dest: PgBox<pg_sys::DestReceiver>,
            completion_tag: *mut pg_sys::QueryCompletion,
        ) -> HookResult<()>,
    ) -> HookResult<()> {
        task::block_on(Writer::flush()).unwrap_or_else(|err| {
            panic!("{}", err);
        });

        process::process_utility(
            pstmt,
            query_string,
            read_only_tree,
            context,
            params,
            query_env,
            dest,
            completion_tag,
            prev_hook,
        )
        .unwrap_or_else(|err| {
            panic!("{}", err);
        });

        HookResult::new(())
    }

    fn abort(&mut self) {
        task::block_on(Writer::clear_all()).unwrap_or_else(|err| {
            panic!("{}", err);
        });
    }

    fn commit(&mut self) {
        task::block_on(Writer::flush()).unwrap_or_else(|err| {
            panic!("{}", err);
        });

        task::block_on(Writer::commit()).unwrap_or_else(|err| {
            panic!("{}", err);
        });
    }
}

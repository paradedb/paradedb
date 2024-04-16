mod alter;
mod drop;
mod executor;
mod handler;
mod process;
mod query;
mod rename;
mod select;
mod truncate;

use async_std::task;
use deltalake::datafusion::logical_expr::{col, lit};
use deltalake::operations::delete::DeleteBuilder;
use pgrx::*;
use std::ffi::CStr;

use crate::datafusion::session::Session;
use crate::datafusion::stream::STREAM_CACHE;
use crate::datafusion::table::{
    DELETE_ON_PRECOMMIT_CACHE, DROP_ON_ABORT_CACHE, DROP_ON_PRECOMMIT_CACHE, RESERVED_XMAX_FIELD,
};
use crate::datafusion::writer::Writer;

pub struct ParadeHook;

impl hooks::PgHooks for ParadeHook {
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
        // Flush all pending inserts to disk but don't commit
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
        // Flush all pending inserts to disk but don't commit
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
        // Clear all pending scans
        let mut scan_cache = task::block_on(STREAM_CACHE.lock());
        scan_cache.clear();

        // Clear all pending inserts
        task::block_on(Writer::clear_all()).unwrap_or_else(|err| {
            warning!("{}", err);
        });

        // Clear all pending deletes
        let mut delete_cache = task::block_on(DELETE_ON_PRECOMMIT_CACHE.lock());
        delete_cache.clear();

        // Clear all pending drops
        let mut drop_on_precommit_cache = task::block_on(DROP_ON_PRECOMMIT_CACHE.lock());
        drop_on_precommit_cache.clear();

        // Physically delete all tables created and rolled back in this transaction
        let mut drop_on_abort_cache = task::block_on(DROP_ON_ABORT_CACHE.lock());

        for (table_path, _) in drop_on_abort_cache.drain() {
            std::fs::remove_dir_all(table_path).unwrap_or_else(|err| {
                warning!("{}", err);
            });
        }
    }

    fn commit(&mut self) {
        // Commit all pending inserts
        task::block_on(Writer::flush()).unwrap_or_else(|err| {
            panic!("{}", err);
        });

        task::block_on(Writer::commit()).unwrap_or_else(|err| {
            panic!("{}", err);
        });

        // Commit all pending deletes
        let mut delete_cache = task::block_on(DELETE_ON_PRECOMMIT_CACHE.lock());

        for (table_path, schema_name) in delete_cache.drain() {
            Session::with_tables(&schema_name, |mut tables| {
                Box::pin(async move {
                    let delta_table = tables.get_owned(&table_path).await?;

                    DeleteBuilder::new(
                        delta_table.log_store(),
                        delta_table
                            .state
                            .expect("DeleteBuilder could not find delta table state"),
                    )
                    .with_predicate(
                        col(RESERVED_XMAX_FIELD)
                            .eq(lit(unsafe { pg_sys::GetCurrentTransactionId() } as i64)),
                    )
                    .await?;

                    Ok(())
                })
            })
            .unwrap_or_else(|err| {
                warning!("{}", err);
            });
        }

        // Physically delete all dropped tables
        let mut drop_on_precommit_cache = task::block_on(DROP_ON_PRECOMMIT_CACHE.lock());

        for (table_path, _) in drop_on_precommit_cache.drain() {
            std::fs::remove_dir_all(table_path).unwrap_or_else(|err| {
                warning!("{}", err);
            });
        }

        // Clear all pending drops
        let mut drop_on_abort_cache = task::block_on(DROP_ON_ABORT_CACHE.lock());
        drop_on_abort_cache.clear();
    }
}

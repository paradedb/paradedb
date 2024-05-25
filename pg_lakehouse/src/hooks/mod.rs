mod executor;
mod query;

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
        executor::executor_run(query_desc, direction, count, execute_once, prev_hook)
            .unwrap_or_else(|err| {
                panic!("{}", err);
            });

        HookResult::new(())
    }
}

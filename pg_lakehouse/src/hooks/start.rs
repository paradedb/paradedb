use pgrx::*;

use super::query::get_query_relations;

pub fn executor_start(
    &mut self,
    query_desc: PgBox<pg_sys::QueryDesc>,
    eflags: i32,
    prev_hook: fn(query_desc: PgBox<pg_sys::QueryDesc>, eflags: i32) -> HookResult<()>,
) -> HookResult<()> {
    let query_relations = crate::hooks::query::get_query_relations(query_desc.plannedstmt);
    info!("executor start {:?}", query_relations.len());
    prev_hook(query_desc, eflags)
}
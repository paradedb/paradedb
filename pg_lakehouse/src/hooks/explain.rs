use pgrx::*;
use std::ffi::CStr;

use crate::PREV_EXPLAIN_ONE_QUERY_HOOK;

use super::query::*;

#[pg_guard]
pub unsafe extern "C" fn explain_forign_query(
    query: *mut pg_sys::Query,
    cursor_options: ::std::os::raw::c_int,
    into: *mut pg_sys::IntoClause,
    es: *mut pg_sys::ExplainState,
    query_string: *const ::std::os::raw::c_char,
    params: pg_sys::ParamListInfo,
    query_env: *mut pg_sys::QueryEnvironment,
) {
    let rtable = unsafe { (*query).rtable };
    let query_start_index = unsafe { (*query).stmt_location };
    let query_len = unsafe { (*query).stmt_len };
    let query_str = unsafe { CStr::from_ptr(query_string) };
    let curr_query = get_current_query(query_start_index, query_len, query_str)
        .expect("should be a valid UTF8 query string");
    let query_relations = get_query_relations(rtable);

    // fall back to original hook
    if rtable.is_null()
        || (*query).commandType != pg_sys::CmdType_CMD_SELECT
        || !is_duckdb_query(&query_relations)
        || curr_query.to_lowercase().starts_with("copy")
        || curr_query.to_lowercase().starts_with("create")
    {
        if let Some(prev_hook) = PREV_EXPLAIN_ONE_QUERY_HOOK {
            prev_hook(
                query,
                cursor_options,
                into,
                es,
                query_string,
                params,
                query_env,
            );
        } else {
            // TODO: call standard hook, it will be available from PG17
            //standard_ExplainOneQuery(query, cursor_options, into, es, query_string, params, query_env);
        }
        return;
    }

    let ctx = PgMemoryContexts::CurrentMemoryContext;
    let label = ctx.pstrdup("DuckDB Scan");
    let value = ctx.pstrdup(&curr_query);
    pg_sys::ExplainPropertyText(label, value, es);
}

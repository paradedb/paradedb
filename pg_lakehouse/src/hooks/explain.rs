use anyhow::Result;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use std::ffi::CStr;
use thiserror::Error;

use super::query::*;

pub async unsafe fn explain(
    plan: *mut pg_sys::PlannedStmt,
    query: &CStr,
    dest: &PgBox<pg_sys::DestReceiver>,
) -> Result<bool> {
    Ok(false)
    // let query = get_current_query(plan, query)?;

    // match LogicalPlan::try_from(QueryString(&query)) {
    //     Ok(logical_plan) => {
    //         let explain_state = pg_sys::NewExplainState();

    //         match logical_plan {
    //             LogicalPlan::Explain(explain) => {
    //                 pg_sys::appendStringInfoString(
    //                     (*explain_state).str_,
    //                     format!("{} \n {:?}", "DataFusionScan: LogicalPlan", explain.plan)
    //                         .as_pg_cstr(),
    //                 );
    //             }
    //             LogicalPlan::Analyze(_) => {
    //                 let context = Session::session_context()?;
    //                 let dataframe = context.execute_logical_plan(logical_plan).await?;
    //                 let batches = dataframe.collect().await?;

    //                 if let Some(array) = batches[0].column_by_name("plan") {
    //                     let string_array = downcast_value!(array, StringArray);
    //                     let plan = string_array.value(0);
    //                     pg_sys::appendStringInfoString((*explain_state).str_, plan.as_pg_cstr());
    //                 } else {
    //                     return Ok(false);
    //                 }
    //             }
    //             _ => return Ok(false),
    //         };

    //         let tupdesc = pg_sys::CreateTemplateTupleDesc(1);
    //         pg_sys::TupleDescInitEntry(
    //             tupdesc,
    //             1,
    //             "QUERY PLAN".as_pg_cstr(),
    //             pg_sys::TEXTOID,
    //             -1,
    //             0,
    //         );
    //         let tstate =
    //             pg_sys::begin_tup_output_tupdesc(dest.as_ptr(), tupdesc, &pg_sys::TTSOpsVirtual);
    //         pg_sys::do_text_output_multiline(tstate, (*(*explain_state).str_).data);
    //         pg_sys::end_tup_output(tstate);
    //         pg_sys::pfree((*(*explain_state).str_).data as *mut std::ffi::c_void);

    //         Ok(true)
    //     }
    //     Err(_) => Ok(false),
    // }
}

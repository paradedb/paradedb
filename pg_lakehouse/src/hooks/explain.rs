use anyhow::Result;
use pgrx::*;
use std::ffi::CStr;
use supabase_wrappers::prelude::options_to_hashmap;

use super::query::{get_current_query, get_query_relations};
use crate::duckdb::connection;
use crate::duckdb::parquet::create_parquet_view;
use crate::fdw::handler::FdwHandler;

pub async unsafe fn explain(
    plan: *mut pg_sys::PlannedStmt,
    query: &CStr,
    dest: &PgBox<pg_sys::DestReceiver>,
) -> Result<bool> {
    info!("explain");
    // let query = get_current_query(plan, query)?;
    // let utility = (*plan).utilityStmt;

    // if (*utility).type_ == pg_sys::NodeTag::T_ExplainStmt {
    //     let query_relations = get_query_relations(plan);
    //     for pg_relation in query_relations {
    //         info!("here");
    //         if pg_relation.is_foreign_table() {
    //             let foreign_table = pg_sys::GetForeignTable(pg_relation.oid());
    //             let table_options = unsafe { options_to_hashmap((*foreign_table).options)? };
    //             let table_name = pg_relation.name();
    //             let schema_name = pg_relation.namespace();
    //             match FdwHandler::from(foreign_table) {
    //                 FdwHandler::Parquet => {
    //                     create_parquet_view(table_name, schema_name, table_options)?;
    //                 }
    //                 _ => {
    //                     todo!()
    //                 }
    //             }
    //         }
    //     }

    //     let connection = connection::inner_connection();
    //     info!("preparing");
    //     let mut statement = connection.prepare(&query).expect("failed to prepare");
    //     info!("prepared");
    //     let mut rows = statement.query([])?;

    //     let mut names = Vec::new();
    //     info!("here");
    //     while let Some(row) = rows.next()? {
    //         info!("got rows");
    //         names.push(row.get::<_, String>(1)?);
    //     }

    //     info!("got rows {:?}", names);
    // }

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

    Ok(false)
}

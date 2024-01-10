use async_std::task;
use deltalake::datafusion::arrow::array::AsArray;

use deltalake::datafusion::common::arrow::array::types::UInt64Type;
use deltalake::datafusion::common::arrow::array::RecordBatch;
use deltalake::datafusion::sql::parser::DFParser;
use deltalake::datafusion::sql::planner::SqlToRel;
use deltalake::datafusion::sql::sqlparser::dialect::PostgreSqlDialect;
use pgrx::*;
use std::ffi::CStr;
use std::num::TryFromIntError;

use crate::datafusion::context::{DatafusionContext, ParadeContextProvider};
use crate::datafusion::substrait::{DatafusionMap, DatafusionMapProducer, SubstraitTranslator};
use crate::hooks::columnar::ColumnarStmt;

pub fn executor_run(
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
    unsafe {
        let ps = query_desc.plannedstmt;
        let rtable = (*ps).rtable;

        // Only use this hook for columnar tables
        if rtable.is_null() || !ColumnarStmt::rtable_is_columnar(rtable).unwrap_or(false) {
            prev_hook(query_desc, direction, count, execute_once);
            return HookResult::new(());
        }

        // Only use this hook for SELECT queries
        // INSERT/UPDATE/DELETE are handled by the table access method
        if query_desc.operation != pg_sys::CmdType_CMD_SELECT {
            prev_hook(query_desc, direction, count, execute_once);
            return HookResult::new(());
        }

        // Parse the query into an AST
        let dialect = PostgreSqlDialect {};
        let query = CStr::from_ptr(query_desc.sourceText)
            .to_str()
            .expect("Failed to parse query string");
        let ast = DFParser::parse_sql_with_dialect(query, &dialect).expect("Failed to parse AST");
        let statement = &ast[0];

        // Convert the AST into a logical plan
        let context_provider = ParadeContextProvider::new();
        let sql_to_rel = SqlToRel::new(&context_provider);
        let logical_plan = sql_to_rel
            .statement_to_plan(statement.clone())
            .expect("Failed to create plan");

        // Execute the logical plan
        let recordbatchvec: Vec<RecordBatch> = DatafusionContext::with_read(|context| {
            let dataframe = task::block_on(context.execute_logical_plan(logical_plan)).unwrap();
            task::block_on(dataframe.collect()).unwrap()
        });

        // This is for any node types that need to do additional processing on estate
        let plan: *mut pg_sys::Plan = (*ps).planTree;
        let node = plan as *mut pg_sys::Node;
        if (*node).type_ == pg_sys::NodeTag::T_ModifyTable {
            let num_updated = recordbatchvec[0]
                .column(0)
                .as_primitive::<UInt64Type>()
                .value(0);
            (*(*query_desc.clone().into_pg()).estate).es_processed = num_updated;
        }

        // Return result tuples
        let _ = send_tuples_if_necessary(query_desc.into_pg(), recordbatchvec);

        HookResult::new(())
    }
}

#[inline]
unsafe fn send_tuples_if_necessary(
    query_desc: *mut pg_sys::QueryDesc,
    recordbatchvec: Vec<RecordBatch>,
) -> Result<(), String> {
    let sendTuples = (*query_desc).operation == pg_sys::CmdType_CMD_SELECT
        || (*(*query_desc).plannedstmt).hasReturning;

    if !sendTuples {
        return Ok(());
    }

    let dest = (*query_desc).dest;
    let rStartup = (*dest).rStartup;
    match rStartup {
        Some(f) => f(
            dest,
            (*query_desc)
                .operation
                .try_into()
                .map_err(|e: TryFromIntError| e.to_string())?,
            (*query_desc).tupDesc,
        ),
        None => return Err("No rStartup found".to_string()),
    };

    let tuple_desc = PgTupleDesc::from_pg_unchecked((*query_desc).tupDesc);
    let receiveSlot = (*dest).receiveSlot;
    let mut row_number = 0;

    match receiveSlot {
        Some(f) => {
            for recordbatch in recordbatchvec.iter() {
                // Convert the tuple_desc target types to the ones corresponding to the Datafusion column types
                let tuple_attrs = (*(*query_desc).tupDesc).attrs.as_mut_ptr();
                for (col_index, _attr) in tuple_desc.iter().enumerate() {
                    let dt = recordbatch.column(col_index).data_type();
                    (*tuple_attrs.offset(
                        col_index
                            .try_into()
                            .map_err(|e: TryFromIntError| e.to_string())?,
                    ))
                    .atttypid = PgOid::from_substrait(dt.to_substrait()?)?.value();
                }

                for row_index in 0..recordbatch.num_rows() {
                    let tuple_table_slot =
                        pg_sys::MakeTupleTableSlot((*query_desc).tupDesc, &pg_sys::TTSOpsVirtual);

                    pg_sys::ExecStoreVirtualTuple(tuple_table_slot);

                    // Assign TID to the tuple table slot
                    let mut tid = pg_sys::ItemPointerData::default();
                    u64_to_item_pointer(row_number as u64, &mut tid);
                    (*tuple_table_slot).tts_tid = tid;
                    row_number += 1;

                    for (col_index, _attr) in tuple_desc.iter().enumerate() {
                        let column = recordbatch.column(col_index);
                        let dt = column.data_type();
                        let tts_value = (*tuple_table_slot).tts_values.offset(
                            col_index
                                .try_into()
                                .map_err(|e: TryFromIntError| e.to_string())?,
                        );
                        *tts_value = DatafusionMapProducer::map(
                            dt.to_substrait()?,
                            |df_map: DatafusionMap| (df_map.index_datum)(column, row_index),
                        )??;
                    }
                    f(tuple_table_slot, dest);
                    pg_sys::ExecDropSingleTupleTableSlot(tuple_table_slot);
                }
            }
        }
        None => return Err("No receiveslot".to_string()),
    }

    let rShutdown = (*dest).rShutdown;
    match rShutdown {
        Some(f) => f(dest),
        None => return Err("No rshutdown".to_string()),
    }

    Ok(())
}

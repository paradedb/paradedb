use async_std::task;
use datafusion::arrow::array::AsArray;

use datafusion::common::arrow::array::types::UInt64Type;
use datafusion::common::arrow::array::RecordBatch;
use pgrx::*;
use std::num::TryFromIntError;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::substrait::{DatafusionMap, DatafusionMapProducer, SubstraitTranslator};
use crate::hooks::columnar::ColumnarStmt;
use crate::nodes::producer::DatafusionPlanProducer;
use crate::nodes::root::RootPlanNode;

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

        if !ColumnarStmt::planned_is_columnar(ps).unwrap_or(false) {
            prev_hook(query_desc, direction, count, execute_once);
            return HookResult::new(());
        }

        let plan: *mut pg_sys::Plan = (*ps).planTree;
        let node = plan as *mut pg_sys::Node;
        let node_tag = (*node).type_;
        let rtable = (*ps).rtable;

        let logical_plan = RootPlanNode::datafusion_plan(plan, rtable, None).unwrap();

        let recordbatchvec: Vec<RecordBatch> = DatafusionContext::with_read(|context| {
            let dataframe = task::block_on(context.execute_logical_plan(logical_plan)).unwrap();
            task::block_on(dataframe.collect()).unwrap()
        });

        // This is for any node types that need to do additional processing on estate
        if node_tag == pg_sys::NodeTag::T_ModifyTable {
            let num_updated = recordbatchvec[0]
                .column(0)
                .as_primitive::<UInt64Type>()
                .value(0);
            (*(*query_desc.clone().into_pg()).estate).es_processed = num_updated;
        }

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

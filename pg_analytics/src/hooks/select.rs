use deltalake::datafusion::logical_expr::LogicalPlan;
use pgrx::*;

use crate::datafusion::session::Session;
use crate::errors::{NotFound, ParadeError};
use crate::types::datatype::{ArrowDataType, PgAttribute, PgTypeMod};
use crate::types::datum::GetDatum;

pub fn select(
    mut query_desc: PgBox<pg_sys::QueryDesc>,
    logical_plan: LogicalPlan,
) -> Result<(), ParadeError> {
    // Execute the logical plan and collect the resulting batches
    let batches = Session::with_session_context(|context| {
        Box::pin(async move {
            let dataframe = context.execute_logical_plan(logical_plan).await?;
            Ok(dataframe.collect().await?)
        })
    })?;

    // Convert the DataFusion batches to Postgres tuples and send them to the destination
    unsafe {
        let dest = query_desc.dest;
        let startup = (*dest)
            .rStartup
            .ok_or(NotFound::Value("rStartup".to_string()))?;

        startup(dest, query_desc.operation as i32, query_desc.tupDesc);

        let tuple_desc = PgTupleDesc::from_pg_unchecked(query_desc.tupDesc);
        let receive = (*dest)
            .receiveSlot
            .ok_or(NotFound::Value("receive".to_string()))?;

        for (row_number, recordbatch) in batches.iter().enumerate() {
            // Convert the tuple_desc target types to the ones corresponding to the DataFusion column types
            let tuple_attrs = (*query_desc.tupDesc).attrs.as_mut_ptr();
            for (col_index, _attr) in tuple_desc.iter().enumerate() {
                let PgAttribute(typid, PgTypeMod(typmod)) =
                    ArrowDataType(recordbatch.column(col_index).data_type().clone()).try_into()?;

                let tuple_attr = tuple_attrs.add(col_index);
                (*tuple_attr).atttypid = typid.value();
                (*tuple_attr).atttypmod = typmod;
            }

            for row_index in 0..recordbatch.num_rows() {
                let tuple_table_slot =
                    pg_sys::MakeTupleTableSlot(query_desc.tupDesc, &pg_sys::TTSOpsVirtual);

                pg_sys::ExecStoreVirtualTuple(tuple_table_slot);

                // Assign TID to the tuple table slot
                let mut tid = pg_sys::ItemPointerData::default();
                u64_to_item_pointer(row_number as u64, &mut tid);
                (*tuple_table_slot).tts_tid = tid;

                for (col_index, _) in tuple_desc.iter().enumerate() {
                    let column = recordbatch.column(col_index);
                    let tts_value = (*tuple_table_slot).tts_values.add(col_index);
                    let tts_isnull = (*tuple_table_slot).tts_isnull.add(col_index);

                    match column.get_datum(row_index)? {
                        Some(datum) => {
                            *tts_value = datum;
                        }
                        None => {
                            *tts_isnull = true;
                        }
                    };
                }

                receive(tuple_table_slot, dest);
                pg_sys::ExecDropSingleTupleTableSlot(tuple_table_slot);
            }
        }

        let shutdown = (*dest)
            .rShutdown
            .ok_or(NotFound::Value("rShutdown".to_string()))?;
        shutdown(dest);
    }

    Ok(())
}

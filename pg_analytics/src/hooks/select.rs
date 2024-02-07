use async_std::task;
use deltalake::datafusion::logical_expr::LogicalPlan;
use pgrx::*;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::datatype::{
    DatafusionMapProducer, DatafusionTypeTranslator, PostgresTypeTranslator,
};
use crate::errors::{NotFound, ParadeError};

pub fn select(
    mut query_desc: PgBox<pg_sys::QueryDesc>,
    logical_plan: LogicalPlan,
) -> Result<(), ParadeError> {
    // Execute the logical plan and collect the resulting batches
    let batches = DatafusionContext::with_session_context(|context| {
        let dataframe = task::block_on(context.execute_logical_plan(logical_plan))?;
        Ok(task::block_on(dataframe.collect())?)
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
                let dt = recordbatch.column(col_index).data_type();
                let (typid, typmod) = PgOid::from_sql_data_type(dt.to_sql_data_type()?)?;
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

                for (col_index, _attr) in tuple_desc.iter().enumerate() {
                    let column = recordbatch.column(col_index);
                    let dt = column.data_type();
                    let tts_value = (*tuple_table_slot).tts_values.add(col_index);
                    let tts_isnull = (*tuple_table_slot).tts_isnull.add(col_index);

                    match DatafusionMapProducer::index_datum(
                        dt.to_sql_data_type()?,
                        column,
                        row_index,
                    )? {
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

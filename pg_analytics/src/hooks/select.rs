use deltalake::datafusion::logical_expr::LogicalPlan;
use pgrx::*;
use thiserror::Error;

use crate::datafusion::batch::RecordBatchError;
use crate::datafusion::session::Session;
use crate::errors::ParadeError;
use crate::types::datatype::{ArrowDataType, DataTypeError, PgAttribute, PgTypeMod};
use crate::types::datum::GetDatum;

pub fn select(
    mut query_desc: PgBox<pg_sys::QueryDesc>,
    logical_plan: LogicalPlan,
) -> Result<(), SelectHookError> {
    // Execute the logical plan and collect the resulting batches
    let mut batches = Session::with_session_context(|context| {
        Box::pin(async move {
            let dataframe = context.execute_logical_plan(logical_plan).await?;
            Ok(dataframe.collect().await?)
        })
    })?;

    // Convert the DataFusion batches to Postgres tuples and send them to the destination
    unsafe {
        let dest = query_desc.dest;
        let startup = (*dest).rStartup.ok_or(SelectHookError::RStartupNotFound)?;

        startup(dest, query_desc.operation as i32, query_desc.tupDesc);

        let tuple_desc = PgTupleDesc::from_pg_unchecked(query_desc.tupDesc);
        let receive = (*dest)
            .receiveSlot
            .ok_or(SelectHookError::ReceiveSlotNotFound)?;

        for batch in batches.iter_mut() {
            // Convert the tuple_desc target types to the ones corresponding to the DataFusion column types
            let tuple_attrs = (*query_desc.tupDesc).attrs.as_mut_ptr();
            for (col_index, _) in tuple_desc.iter().enumerate() {
                let PgAttribute(typid, PgTypeMod(typmod)) =
                    ArrowDataType(batch.column(col_index).data_type().clone()).try_into()?;

                let tuple_attr = tuple_attrs.add(col_index);
                (*tuple_attr).atttypid = typid.value();
                (*tuple_attr).atttypmod = typmod;
            }

            for row_index in 0..batch.num_rows() {
                let tuple_table_slot =
                    pg_sys::MakeTupleTableSlot(query_desc.tupDesc, &pg_sys::TTSOpsVirtual);

                pg_sys::ExecStoreVirtualTuple(tuple_table_slot);

                for (col_index, _) in tuple_desc.iter().enumerate() {
                    let column = batch.column(col_index);
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
            .ok_or(SelectHookError::RShutdownNotFound)?;
        shutdown(dest);
    }

    Ok(())
}

#[derive(Error, Debug)]
pub enum SelectHookError {
    #[error(transparent)]
    DataTypeError(#[from] DataTypeError),

    #[error(transparent)]
    ParadeError(#[from] ParadeError),

    #[error(transparent)]
    RecordBatchError(#[from] RecordBatchError),

    #[error("Unexpected error: rShutdown not found")]
    RShutdownNotFound,

    #[error("Unexpected error: receiveSlot not found")]
    ReceiveSlotNotFound,

    #[error("Unexpected error: rStartup not found")]
    RStartupNotFound,
}

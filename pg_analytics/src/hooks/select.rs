use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::common::arrow::error::ArrowError;
use deltalake::datafusion::logical_expr::{col, max, JoinType, LogicalPlan};
use deltalake::datafusion::prelude::SessionContext;
use pgrx::pg_sys::{CommandId, SnapshotType_SNAPSHOT_MVCC};
use pgrx::*;
use thiserror::Error;

use crate::datafusion::catalog::CatalogError;
use crate::datafusion::session::Session;
use crate::datafusion::table::{filter_reserved_expr, RESERVED_CMIN_FIELD, RESERVED_TID_FIELD};
use crate::types::datatype::DataTypeError;
use crate::types::datum::GetDatum;

pub fn write_batches_to_slots(
    query_desc: PgBox<pg_sys::QueryDesc>,
    mut batches: Vec<RecordBatch>,
) -> Result<(), SelectHookError> {
    // Convert the DataFusion batches to Postgres tuples and send them to the destination
    unsafe {
        let tuple_desc = PgTupleDesc::from_pg(query_desc.tupDesc);
        let estate = query_desc.estate;
        (*estate).es_processed = 0;

        let dest = query_desc.dest;
        let startup = (*dest).rStartup.ok_or(SelectHookError::RStartupNotFound)?;
        startup(dest, query_desc.operation as i32, query_desc.tupDesc);

        let receive = (*dest)
            .receiveSlot
            .ok_or(SelectHookError::ReceiveSlotNotFound)?;

        for batch in batches.iter_mut() {
            for row_index in 0..batch.num_rows() {
                let tuple_table_slot =
                    pg_sys::MakeTupleTableSlot(query_desc.tupDesc, &pg_sys::TTSOpsVirtual);

                pg_sys::ExecStoreVirtualTuple(tuple_table_slot);

                for (col_index, _) in tuple_desc.iter().enumerate() {
                    let attribute = tuple_desc
                        .get(col_index)
                        .ok_or(SelectHookError::AttributeNotFound(col_index))?;
                    let column = batch.column(col_index);
                    let tts_value = (*tuple_table_slot).tts_values.add(col_index);
                    let tts_isnull = (*tuple_table_slot).tts_isnull.add(col_index);

                    match column.get_datum(row_index, attribute.type_oid(), attribute.type_mod())? {
                        Some(datum) => {
                            *tts_value = datum;
                        }
                        None => {
                            *tts_isnull = true;
                        }
                    };
                }

                receive(tuple_table_slot, dest);
                (*estate).es_processed += 1;
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

pub fn get_datafusion_batches(
    logical_plan: LogicalPlan,
    single_thread: bool,
    transaction_id: i64,
    command_id: CommandId,
) -> Result<Vec<RecordBatch>, SelectHookError> {
    // Execute the logical plan and collect the resulting batches
    Ok(Session::with_session_context(|context| {
        Box::pin(async move {
            let full_dataframe = if single_thread {
                let config = context.copied_config();
                SessionContext::new_with_config(config.with_target_partitions(1))
                    .execute_logical_plan(logical_plan)
                    .await?
            } else {
                context.execute_logical_plan(logical_plan).await?
            };
            let filtered_dataframe = full_dataframe.filter(filter_reserved_expr(
                SnapshotType_SNAPSHOT_MVCC, // Filter out deleted/non-visible rows
                transaction_id,
                command_id,
            ))?;

            // First, we aggregate to find the maximum cmin for each TID
            let max_cmin_dataframe = filtered_dataframe.clone().aggregate(
                vec![col(RESERVED_TID_FIELD)],                         // Group by TID
                vec![max(col(RESERVED_CMIN_FIELD)).alias("max_cmin")], // Find the maximum cmin for each group
            )?;

            // Then, join this aggregated DataFrame back to the original DataFrame
            // to get the rows that match the maximum cmin for each TID
            let joined_df = filtered_dataframe.join(
                max_cmin_dataframe,
                JoinType::Inner,       // Use an inner join to filter rows
                &[RESERVED_TID_FIELD], // Left columns for the join condition
                &[RESERVED_TID_FIELD], // Right columns for the join condition
                Some(col(RESERVED_CMIN_FIELD).eq(col("max_cmin"))), // Additional filter to match max cmin
            )?;
            Ok(joined_df.collect().await?)
        })
    })?)
}

#[derive(Error, Debug)]
pub enum SelectHookError {
    #[error(transparent)]
    ArrowError(#[from] ArrowError),

    #[error(transparent)]
    CatalogError(#[from] CatalogError),

    #[error(transparent)]
    DataTypeError(#[from] DataTypeError),

    #[error("Could not find attribute {0} in tuple descriptor")]
    AttributeNotFound(usize),

    #[error("Unexpected error: rShutdown not found")]
    RShutdownNotFound,

    #[error("Unexpected error: receiveSlot not found")]
    ReceiveSlotNotFound,

    #[error("Unexpected error: rStartup not found")]
    RStartupNotFound,
}

use datafusion::arrow::array::AsArray;
use datafusion::arrow::datatypes::{DataType, TimeUnit};
use datafusion::common::arrow::array::types::{
    Date32Type, Float32Type, Int16Type, Int32Type, Int64Type, Int8Type, Time32SecondType,
    TimestampSecondType, UInt32Type,
};
use datafusion::common::arrow::array::RecordBatch;
use pgrx::*;
use std::num::TryFromIntError;

pub unsafe fn send_tuples_if_necessary(
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

                        match dt {
                            DataType::Boolean => {
                                *tts_value = column
                                    .as_primitive::<Int8Type>()
                                    .value(row_index)
                                    .into_datum()
                                    .ok_or("Could not convert Boolean into datum")?
                            }
                            DataType::Int16 => {
                                *tts_value = column
                                    .as_primitive::<Int16Type>()
                                    .value(row_index)
                                    .into_datum()
                                    .ok_or("Could not convert Int16 into datum")?
                            }
                            DataType::Int32 => {
                                *tts_value = column
                                    .as_primitive::<Int32Type>()
                                    .value(row_index)
                                    .into_datum()
                                    .ok_or("Could not convert Int32 into datum")?
                            }
                            DataType::Int64 => {
                                *tts_value = column
                                    .as_primitive::<Int64Type>()
                                    .value(row_index)
                                    .into_datum()
                                    .ok_or("Could not convert Int64 into datum")?
                            }
                            DataType::UInt32 => {
                                *tts_value = column
                                    .as_primitive::<UInt32Type>()
                                    .value(row_index)
                                    .into_datum()
                                    .ok_or("Could not convert UInt32 into datum")?
                            }
                            DataType::Float32 => {
                                *tts_value = column
                                    .as_primitive::<Float32Type>()
                                    .value(row_index)
                                    .into_datum()
                                    .ok_or("Could not convert Float32 into datum")?
                            }
                            // DataType::Utf8 => *tts_value = column.as_primitive::<GenericStringType>().value(row_index).into_datum().unwrap(),
                            DataType::Time32(TimeUnit::Second) => {
                                *tts_value = column
                                    .as_primitive::<Time32SecondType>()
                                    .value(row_index)
                                    .into_datum()
                                    .ok_or("Could not convert Time32 into datum")?
                            }
                            DataType::Timestamp(TimeUnit::Second, None) => {
                                *tts_value = column
                                    .as_primitive::<TimestampSecondType>()
                                    .value(row_index)
                                    .into_datum()
                                    .ok_or("Could not convert Timestamp into datum")?
                            }
                            DataType::Date32 => {
                                *tts_value = column
                                    .as_primitive::<Date32Type>()
                                    .value(row_index)
                                    .into_datum()
                                    .ok_or("Could not convert Date32 into datum")?
                            }
                            _ => {
                                return Err(format!(
                                    "send_tuples_if_necessary: Unsupported PostgreSQL type: {:?}",
                                    dt
                                ))
                            }
                        };
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

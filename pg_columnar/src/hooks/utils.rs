use datafusion::common::arrow::array::RecordBatch;
use pgrx::*;
use std::ffi::{c_char, CStr, CString};
use std::num::TryFromIntError;

use crate::datafusion::substrait::{ DatafusionMap, DatafusionMapProducer, SubstraitTranslator };

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
                // Convert the tuple_desc target types to the ones corresponding to the Datafusion column types
                let tuple_attrs = (*(*query_desc).tupDesc).attrs.as_mut_ptr();
                for (col_index, _attr) in tuple_desc.iter().enumerate() {
                    let dt = recordbatch.column(col_index).data_type();
                    (*tuple_attrs.offset(col_index
                        .try_into()
                        .map_err(|e: TryFromIntError| e.to_string())?
                    )).atttypid = PgOid::from_substrait(dt.to_substrait()?)?.value();
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
                        *tts_value = DatafusionMapProducer::map(dt.to_substrait()?, |df_map: DatafusionMap| {
                            (df_map.index_datum)(column, row_index as usize)
                        })??;
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

#[pg_guard]
pub unsafe fn planned_stmt_is_columnar(ps: *mut pg_sys::PlannedStmt) -> bool {
    let rtable = (*ps).rtable;
    if rtable.is_null() {
        return false;
    }

    // Get mem table AM handler OID
    let handlername_cstr = CString::new("mem").unwrap();
    let handlername_ptr = handlername_cstr.as_ptr() as *const c_char;
    let memam_oid = pg_sys::get_am_oid(handlername_ptr, true);
    if memam_oid == pg_sys::InvalidOid {
        return false;
    }

    let amTup = pg_sys::SearchSysCache1(
        pg_sys::SysCacheIdentifier_AMOID.try_into().unwrap(),
        pg_sys::Datum::from(memam_oid),
    );
    let amForm = pg_sys::heap_tuple_get_struct::<pg_sys::FormData_pg_am>(amTup);
    let memhandler_oid = (*amForm).amhandler;
    pg_sys::ReleaseSysCache(amTup);

    let elements = (*rtable).elements;
    let mut using_noncol: bool = false;
    let mut using_col: bool = false;

    for i in 0..(*rtable).length {
        let rte = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::RangeTblEntry;
        if (*rte).rtekind != pgrx::pg_sys::RTEKind_RTE_RELATION {
            continue;
        }
        let relation = pg_sys::RelationIdGetRelation((*rte).relid);
        let pg_relation = PgRelation::from_pg_owned(relation);
        if !pg_relation.is_table() {
            continue;
        }

        let am_handler = (*relation).rd_amhandler;

        // If any table uses the Table AM handler, then return true.
        // TODO: if we support more operations, this will be more complex.
        //       for example, if to support joins, some of the nodes will use
        //       table AM for the nodes while others won't. In this case,
        //       we'll have to process in postgres plan for part of it and
        //       datafusion for the other part. For now, we'll simply
        //       fail if we encounter an unsupported node, so this won't happen.
        if am_handler == memhandler_oid {
            using_col = true;
        } else {
            using_noncol = true;
        }
    }

    if using_col && using_noncol {
        panic!("Mixing table types in a single query is not supported yet");
    }

    using_col
}

#[pg_guard]
pub unsafe fn copy_stmt_is_columnar(copy_stmt: *mut pg_sys::CopyStmt) -> bool {
    let handlername_cstr = CString::new("mem").unwrap();
    let handlername_ptr = handlername_cstr.as_ptr() as *const c_char;
    let memam_oid = pg_sys::get_am_oid(handlername_ptr, true);
    if memam_oid == pg_sys::InvalidOid {
        return false;
    }

    let amTup = pg_sys::SearchSysCache1(
        pg_sys::SysCacheIdentifier_AMOID.try_into().unwrap(),
        pg_sys::Datum::from(memam_oid),
    );
    let amForm = pg_sys::heap_tuple_get_struct::<pg_sys::FormData_pg_am>(amTup);
    let memhandler_oid = (*amForm).amhandler;
    pg_sys::ReleaseSysCache(amTup);

    let relation = (*copy_stmt).relation;
    let relation_name = CStr::from_ptr((*relation).relname)
        .to_str()
        .expect("Could not get relation name");
    let pg_relation = PgRelation::open_with_name(relation_name).expect("Could not open relation");
    let relation_data = pg_relation.as_ptr();

    let am_handler = (*relation_data).rd_amhandler;

    am_handler == memhandler_oid
}

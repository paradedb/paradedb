use pgrx::*;
use std::collections::HashMap;
use std::ffi::{c_char, CString};

use crate::federation::{COLUMN_FEDERATION_KEY, ROW_FEDERATION_KEY};
use thiserror::Error;

static COLUMN_HANDLER: &str = "parquet";

pub trait TableClassifier {
    #[allow(clippy::wrong_self_convention)]
    unsafe fn table_lists(self) -> Result<HashMap<&'static str, Vec<PgRelation>>, HandlerError>;
}

impl TableClassifier for *mut pg_sys::List {
    unsafe fn table_lists(self) -> Result<HashMap<&'static str, Vec<PgRelation>>, HandlerError> {
        let col_oid = column_oid()?;

        #[cfg(feature = "pg12")]
        let mut current_cell = (*self).head;
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        let elements = (*self).elements;

        let mut row_tables = vec![];
        let mut col_tables = vec![];

        for i in 0..(*self).length {
            let rte: *mut pg_sys::RangeTblEntry;
            #[cfg(feature = "pg12")]
            {
                rte = (*current_cell).data.ptr_value as *mut pg_sys::RangeTblEntry;
                current_cell = (*current_cell).next;
            }
            #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
            {
                rte = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::RangeTblEntry;
            }

            if (*rte).rtekind != pg_sys::RTEKind_RTE_RELATION {
                continue;
            }
            let relation = pg_sys::RelationIdGetRelation((*rte).relid);
            let pg_relation = PgRelation::from_pg_owned(relation);

            if pg_relation.is_table() {
                let relation_handler_oid = (*relation).rd_amhandler;

                if col_oid != pg_sys::InvalidOid && relation_handler_oid == col_oid {
                    col_tables.push(pg_relation)
                } else {
                    row_tables.push(pg_relation)
                }
            }

            // if pg_relation.is_foreign_table() {
            //     let foreign_table = pg_sys::GetForeignTable(pg_relation.oid());
            //     let foreign_server = pg_sys::GetForeignServer((*foreign_table).serverid);
            //     let fdw_handler = get_fdw_handler((*foreign_server).fdwid);
            // }
        }

        let mut classified_tables = HashMap::new();
        classified_tables.insert(ROW_FEDERATION_KEY, row_tables);
        classified_tables.insert(COLUMN_FEDERATION_KEY, col_tables);

        Ok(classified_tables)
    }
}

pub trait IsColumn {
    #[allow(clippy::wrong_self_convention)]
    unsafe fn is_column(self) -> Result<bool, HandlerError>;
}

impl IsColumn for *mut pg_sys::RelationData {
    unsafe fn is_column(self) -> Result<bool, HandlerError> {
        if self.is_null() {
            return Ok(false);
        }

        let oid = column_oid()?;

        let relation_handler_oid = (*self).rd_amhandler;

        Ok(relation_handler_oid == oid)
    }
}

unsafe fn column_oid() -> Result<pg_sys::Oid, HandlerError> {
    let parquet_handler_str = CString::new(COLUMN_HANDLER)?;
    let parquet_handler_ptr = parquet_handler_str.as_ptr() as *const c_char;

    let parquet_oid = pg_sys::get_am_oid(parquet_handler_ptr, true);

    if parquet_oid == pg_sys::InvalidOid {
        return Ok(parquet_oid);
    }

    let heap_tuple_data = pg_sys::SearchSysCache1(
        pg_sys::SysCacheIdentifier_AMOID as i32,
        pg_sys::Datum::from(parquet_oid),
    );
    let catalog = pg_sys::heap_tuple_get_struct::<pg_sys::FormData_pg_am>(heap_tuple_data);
    pg_sys::ReleaseSysCache(heap_tuple_data);

    Ok((*catalog).amhandler)
}

#[derive(Error, Debug)]
pub enum HandlerError {
    #[error(transparent)]
    NulError(#[from] std::ffi::NulError),
}

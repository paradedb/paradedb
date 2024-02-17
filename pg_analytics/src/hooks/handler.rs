use pgrx::*;
use std::ffi::{c_char, CString};

use crate::errors::{NotSupported, ParadeError};

static COLUMN_HANDLER: &str = "parquet";

pub trait IsColumn {
    #[allow(clippy::wrong_self_convention)]
    unsafe fn is_column(self) -> Result<bool, ParadeError>;
}

impl IsColumn for *mut pg_sys::List {
    unsafe fn is_column(self) -> Result<bool, ParadeError> {
        let oid = column_oid()?;

        if oid == pg_sys::InvalidOid {
            return Ok(false);
        }

        #[cfg(feature = "pg12")]
        let mut current_cell = (*self).head;
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        let elements = (*self).elements;

        let mut using_noncol: bool = false;
        let mut using_col: bool = false;

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
            if !pg_relation.is_table() {
                continue;
            }

            let relation_handler_oid = (*relation).rd_amhandler;

            // If any table uses the Table AM handler, then return true.
            // TODO: If we support more operations, this will be more complex.
            //       for example, if to support joins, some of the nodes will use
            //       table AM for the nodes while others won't. In this case,
            //       we'll have to process in postgres plan for part of it and
            //       datafusion for the other part. For now, we'll simply
            //       fail if we encounter an unsupported node, so this won't happen.
            if relation_handler_oid == oid {
                using_col = true;
            } else {
                using_noncol = true;
            }
        }

        if using_col && using_noncol {
            return Err(NotSupported::MixedTables.into());
        }

        Ok(using_col)
    }
}

impl IsColumn for *mut pg_sys::RelationData {
    unsafe fn is_column(self) -> Result<bool, ParadeError> {
        if self.is_null() {
            return Ok(false);
        }

        let oid = column_oid()?;

        let relation_handler_oid = (*self).rd_amhandler;

        Ok(relation_handler_oid == oid)
    }
}

unsafe fn column_oid() -> Result<pg_sys::Oid, ParadeError> {
    let deltalake_handler_str = CString::new(COLUMN_HANDLER)?;
    let deltalake_handler_ptr = deltalake_handler_str.as_ptr() as *const c_char;

    let deltalake_oid = pg_sys::get_am_oid(deltalake_handler_ptr, true);

    if deltalake_oid == pg_sys::InvalidOid {
        return Ok(deltalake_oid);
    }

    let heap_tuple_data = pg_sys::SearchSysCache1(
        pg_sys::SysCacheIdentifier_AMOID as i32,
        pg_sys::Datum::from(deltalake_oid),
    );
    let catalog = pg_sys::heap_tuple_get_struct::<pg_sys::FormData_pg_am>(heap_tuple_data);
    pg_sys::ReleaseSysCache(heap_tuple_data);

    Ok((*catalog).amhandler)
}

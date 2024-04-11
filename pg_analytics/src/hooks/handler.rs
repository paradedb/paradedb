use pgrx::*;
use std::collections::HashMap;
use std::ffi::{c_char, CString};

use crate::federation::{COLUMN_FEDERATION_KEY, ROW_FEDERATION_KEY};
use crate::tableam::{deltalake_tableam_relation_oid, TableAMError};
use thiserror::Error;

pub trait TableClassifier {
    #[allow(clippy::wrong_self_convention)]
    unsafe fn table_lists(self) -> Result<HashMap<&'static str, Vec<PgRelation>>, HandlerError>;
}

impl TableClassifier for *mut pg_sys::List {
    unsafe fn table_lists(self) -> Result<HashMap<&'static str, Vec<PgRelation>>, HandlerError> {
        let col_oid = deltalake_tableam_relation_oid()?;

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
            if !pg_relation.is_table() {
                continue;
            }

            let relation_handler_oid = (*relation).rd_amhandler;

            if col_oid != pg_sys::InvalidOid && relation_handler_oid == col_oid {
                col_tables.push(pg_relation)
            } else {
                row_tables.push(pg_relation)
            }
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

        let oid = deltalake_tableam_relation_oid()?;

        let relation_handler_oid = (*self).rd_amhandler;

        Ok(relation_handler_oid == oid)
    }
}

#[derive(Error, Debug)]
pub enum HandlerError {
    #[error(transparent)]
    TableAM(#[from] TableAMError),
}

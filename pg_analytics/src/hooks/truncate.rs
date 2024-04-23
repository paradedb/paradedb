use deltalake::datafusion::arrow::record_batch::RecordBatch;
use pgrx::*;
use std::ffi::c_char;
use std::mem::size_of;
use std::sync::Arc;
use thiserror::Error;

use crate::datafusion::catalog::CatalogError;
use crate::datafusion::session::Session;
use crate::datafusion::table::{DataFusionTableError, DatafusionTable};
use crate::datafusion::writer::Writer;
use crate::hooks::handler::{HandlerError, IsColumn};
use crate::rmgr::xlog::{XLogTruncateRecord, XLOG_TRUNCATE};
use crate::rmgr::CUSTOM_RMGR_ID;
use crate::storage::metadata::{MetadataError, PgMetadata};

pub async unsafe fn truncate(
    truncate_stmt: *mut pg_sys::TruncateStmt,
) -> Result<(), TruncateHookError> {
    let rels = (*truncate_stmt).relations;
    let num_rels = (*rels).length;

    #[cfg(feature = "pg12")]
    let mut current_cell = (*rels).head;
    #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
    let elements = (*rels).elements;

    // TRUNCATE can be called on multiple relations at once, so we need to iterate over all of them
    for i in 0..num_rels {
        let rangevar: *mut pg_sys::RangeVar;

        #[cfg(feature = "pg12")]
        {
            rangevar = (*current_cell).data.ptr_value as *mut pg_sys::RangeVar;
            current_cell = (*current_cell).next;
        }
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            rangevar = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::RangeVar;
        }

        let rangevar_oid = pg_sys::RangeVarGetRelidExtended(
            rangevar,
            pg_sys::ShareUpdateExclusiveLock as i32,
            0,
            None,
            std::ptr::null_mut(),
        );
        let relation = pg_sys::RelationIdGetRelation(rangevar_oid);

        if relation.is_null() {
            continue;
        }

        if !relation.is_column()? {
            pg_sys::RelationClose(relation);
            continue;
        }

        let pg_relation = PgRelation::from_pg(relation);
        let schema_name = pg_relation.namespace();
        let table_path = pg_relation.table_path()?;

        // Removes all blocks from the relation
        pg_sys::RelationTruncate(relation, 0);

        // Reset the relation's metadata
        relation.init_metadata((*relation).rd_smgr)?;
        pg_sys::RelationClose(relation);

        // Clear all pending write commits for this table since it's being truncated
        Writer::clear_actions(&table_path).await?;

        Session::with_tables(schema_name, |mut tables| {
            Box::pin(async move {
                let pg_relation = PgRelation::from_pg(relation);
                let _ = tables.logical_delete(&table_path, None).await?;

                let arrow_schema = Arc::new(pg_relation.arrow_schema_with_reserved_fields()?);
                let batch = RecordBatch::new_empty(arrow_schema);

                tables.alter_schema(&table_path, batch).await?;

                Ok(())
            })
        })?;

        // Log truncate to pg_analytics WAL manager
        #[cfg(any(feature = "pg15", feature = "pg16"))]
        {
            pg_sys::XLogBeginInsert();
            let mut record = XLogTruncateRecord::new((*relation).rd_id);
            #[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
            {
                pg_sys::XLogRegisterData(
                    &mut record as *mut XLogTruncateRecord as *mut c_char,
                    size_of::<XLogTruncateRecord>() as i32,
                );
            }
            #[cfg(feature = "pg16")]
            {
                pg_sys::XLogRegisterData(
                    &mut record as *mut XLogTruncateRecord as *mut c_char,
                    size_of::<XLogTruncateRecord>() as u32,
                );
            }
            pg_sys::XLogInsert(CUSTOM_RMGR_ID, XLOG_TRUNCATE);
        }
    }

    Ok(())
}

#[derive(Error, Debug)]
pub enum TruncateHookError {
    #[error(transparent)]
    Catalog(#[from] CatalogError),

    #[error(transparent)]
    DataFusionTable(#[from] DataFusionTableError),

    #[error(transparent)]
    Handler(#[from] HandlerError),

    #[error(transparent)]
    Metadata(#[from] MetadataError),
}

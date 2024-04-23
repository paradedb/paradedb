use pgrx::*;
use thiserror::Error;

use crate::datafusion::catalog::CatalogError;
use crate::datafusion::session::Session;
use crate::datafusion::table::{DataFusionTableError, DatafusionTable};
use crate::storage::metadata::{MetadataError, PgMetadata};

#[inline]
fn relation_vacuum(rel: pg_sys::Relation, optimize: bool) -> Result<(), VacuumError> {
    info!("relation_vacuum");
    let pg_relation = unsafe { PgRelation::from_pg(rel) };
    let schema_name = pg_relation.namespace();
    let table_path = pg_relation.table_path()?;

    Session::with_tables(schema_name, |mut tables| {
        Box::pin(async move { Ok(tables.vacuum(&table_path, optimize).await?) })
    })?;

    Ok(())
}

/*
 * Called by non-FULL VACUUMs.
 */
#[pg_guard]
pub extern "C" fn deltalake_relation_vacuum(
    rel: pg_sys::Relation,
    _params: *mut pg_sys::VacuumParams,
    _bstrategy: pg_sys::BufferAccessStrategy,
) {
    info!("deltalake_relation_vacuum");
    unsafe {
        pg_sys::pgstat_progress_start_command(
            pg_sys::ProgressCommandType_PROGRESS_COMMAND_VACUUM,
            PgRelation::from_pg(rel).oid(),
        );
    }

    relation_vacuum(rel, false).unwrap_or_else(|err| {
        warning!("{}", err);
    });

    unsafe {
        pg_sys::pgstat_progress_end_command();
    }
}

/*
 * Called by FULL VACUUMs.
 */
#[pg_guard]
pub extern "C" fn deltalake_relation_copy_for_cluster(
    rel: pg_sys::Relation,
    _OldTable: pg_sys::Relation,
    _OldIndex: pg_sys::Relation,
    _use_sort: bool,
    _OldestXmin: pg_sys::TransactionId,
    _xid_cutoff: *mut pg_sys::TransactionId,
    _multi_cutoff: *mut pg_sys::MultiXactId,
    num_tuples: *mut f64,
    tups_vacuumed: *mut f64,
    _tups_recently_dead: *mut f64,
) {
    info!("deltalake_relation_copy_for_cluster");
    relation_vacuum(rel, true).unwrap_or_else(|err| {
        warning!("{}", err);
    });

    // Tables are append-only, so the highest row number = number of tuples
    unsafe {
        *num_tuples = (rel.read_next_row_number().unwrap_or(1) - 1) as f64;
        *tups_vacuumed = 0.0;
    }
}

#[pg_guard]
#[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
pub extern "C" fn deltalake_relation_copy_data(
    _rel: pg_sys::Relation,
    _newrnode: *const pg_sys::RelFileNode,
) {
    info!("deltalake_relation_copy_data");
    panic!("{}", VacuumError::CopyDataNotImplemented.to_string());
}

#[pg_guard]
#[cfg(feature = "pg16")]
pub extern "C" fn deltalake_relation_copy_data(
    _rel: pg_sys::Relation,
    _newrnode: *const pg_sys::RelFileLocator,
) {
    info!("deltalake_relation_copy_data");
    panic!("{}", VacuumError::CopyDataNotImplemented.to_string());
}

#[derive(Error, Debug)]
pub enum VacuumError {
    #[error(transparent)]
    CatalogError(#[from] CatalogError),

    #[error(transparent)]
    DataFusionTableError(#[from] DataFusionTableError),

    #[error(transparent)]
    MetadataError(#[from] MetadataError),

    #[error("relation_copy_data is not yet implemented")]
    CopyDataNotImplemented,
}

use pgrx::*;

use crate::datafusion::catalog::CatalogError;
use crate::datafusion::session::Session;
use crate::datafusion::table::DatafusionTable;

#[inline]
fn relation_vacuum(rel: pg_sys::Relation) -> Result<(), CatalogError> {
    let pg_relation = unsafe { PgRelation::from_pg(rel) };

    unsafe {
        pg_sys::pgstat_progress_start_command(
            pg_sys::ProgressCommandType_PROGRESS_COMMAND_VACUUM,
            pg_relation.oid(),
        );
    }

    let schema_name = pg_relation.namespace();
    let table_path = pg_relation.table_path()?;

    Session::with_tables(schema_name, |mut tables| {
        Box::pin(async move { Ok(tables.vacuum(&table_path, false).await?) })
    })?;

    unsafe {
        pg_sys::pgstat_progress_end_command();
    }

    Ok(())
}

#[pg_guard]
pub extern "C" fn deltalake_relation_vacuum(
    rel: pg_sys::Relation,
    _params: *mut pg_sys::VacuumParams,
    _bstrategy: pg_sys::BufferAccessStrategy,
) {
    relation_vacuum(rel).unwrap_or_else(|err| {
        panic!("{}", err);
    });
}

#[pg_guard]
#[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
pub extern "C" fn deltalake_relation_copy_data(
    _rel: pg_sys::Relation,
    _newrnode: *const pg_sys::RelFileNode,
) {
}

#[pg_guard]
#[cfg(feature = "pg16")]
pub extern "C" fn deltalake_relation_copy_data(
    _rel: pg_sys::Relation,
    _newrnode: *const pg_sys::RelFileLocator,
) {
}

#[pg_guard]
pub extern "C" fn deltalake_relation_copy_for_cluster(
    _NewTable: pg_sys::Relation,
    _OldTable: pg_sys::Relation,
    _OldIndex: pg_sys::Relation,
    _use_sort: bool,
    _OldestXmin: pg_sys::TransactionId,
    _xid_cutoff: *mut pg_sys::TransactionId,
    _multi_cutoff: *mut pg_sys::MultiXactId,
    _num_tuples: *mut f64,
    _tups_vacuumed: *mut f64,
    _tups_recently_dead: *mut f64,
) {
}

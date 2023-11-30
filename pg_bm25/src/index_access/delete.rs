use pgrx::*;

use crate::index_access::utils::get_parade_index;

const SETUP_SQL: &str = include_str!("../../sql/index_setup.sql");

#[pg_guard]
pub extern "C" fn ambulkdelete(
    info: *mut pg_sys::IndexVacuumInfo,
    stats: *mut pg_sys::IndexBulkDeleteResult,
    callback: pg_sys::IndexBulkDeleteCallback,
    callback_state: *mut ::std::os::raw::c_void,
) -> *mut pg_sys::IndexBulkDeleteResult {
    let info = unsafe { PgBox::from_pg(info) };
    let mut stats = unsafe { PgBox::from_pg(stats) };
    let index_rel: pg_sys::Relation = info.index;
    let index_relation = unsafe { PgRelation::from_pg(index_rel) };
    let index_name = &index_relation.name().to_string();
    let parade_index = get_parade_index(index_name.into());

    if stats.is_null() {
        stats = unsafe {
            PgBox::from_pg(
                pg_sys::palloc0(std::mem::size_of::<pg_sys::IndexBulkDeleteResult>()).cast(),
            )
        };
    }

    stats = parade_index.bulk_delete(stats, callback, callback_state);
    stats.into_pg()
}

#[cfg(feature = "pg_test")]
#[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15",))]
#[pgrx::pg_schema]
mod tests {
    use super::ambulkdelete;
    use pgrx::*;

    use crate::operator::get_index_oid;

    #[pg_test]
    fn test_ambulkdelete() {
        Spi::run(SETUP_SQL).expect("failed to create index and table");
        let oid = get_index_oid("idx_one_republic", "bm25")
            .expect("could not find oid for one_republic")
            .unwrap();

        let index = unsafe { pg_sys::index_open(oid, pg_sys::AccessShareLock as pg_sys::LOCKMODE) };
        let info = {
            let mut vacuum_info = pg_sys::IndexVacuumInfo {
                index,
                analyze_only: false,
                report_progress: true,
                estimated_count: true,
                message_level: 0,
                num_heap_tuples: 1.0,
                strategy: unsafe { pg_sys::GetAccessStrategy(pg_sys::ReadBufferMode_RBM_NORMAL) },
            };
            &mut vacuum_info as *mut pg_sys::IndexVacuumInfo
        };

        let stats = {
            let mut stat = pg_sys::IndexBulkDeleteResult {
                num_pages: 7,
                estimated_count: true,
                num_index_tuples: 1.0,
                tuples_removed: 0.0,
                #[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
                pages_newly_deleted: 2,
                #[cfg(any(feature = "pg12", feature = "pg13",))]
                pages_removed: 2,

                pages_deleted: 1,
                pages_free: 0,
            };
            &mut stat as *mut pg_sys::IndexBulkDeleteResult
        };
        let state = {
            let mut data: i32 = 42;
            &mut data as *mut _ as *mut ::std::os::raw::c_void
        };

        let callback = {
            pub extern "C" fn callback(
                _itemptr: pg_sys::ItemPointer,
                _state: *mut ::std::os::raw::c_void,
            ) -> bool {
                true
            }
            callback
        };

        let res = ambulkdelete(info, stats, Some(callback), state);
        let stats_res = unsafe { PgBox::from_pg(res) };

        assert_eq!(stats_res.num_pages, 7);
        assert_eq!(stats_res.pages_free, 0);
    }
}

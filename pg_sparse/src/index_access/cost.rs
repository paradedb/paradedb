use pgrx::*;

use crate::sparse_index::index::get_rdopts;

#[allow(clippy::too_many_arguments)]
#[pg_guard(immutable, parallel_safe)]
pub unsafe extern "C" fn amcostestimate(
    root: *mut pg_sys::PlannerInfo,
    path: *mut pg_sys::IndexPath,
    loop_count: f64,
    index_startup_cost: *mut pg_sys::Cost,
    index_total_cost: *mut pg_sys::Cost,
    index_selectivity: *mut pg_sys::Selectivity,
    index_correlation: *mut f64,
    index_pages: *mut f64,
) {
    let pathref = path.as_ref().expect("path argument is NULL");

    if pathref.indexorderbys.is_null() {
        *index_startup_cost = f64::MAX;
        *index_total_cost = f64::MAX;
        *index_selectivity = 0.0;
        *index_correlation = 0.0;
        *index_pages = 0.0;
    } else {
        let indexinfo = pathref
            .indexinfo
            .as_ref()
            .expect("indexinfo in path is NULL");
        let index_relation = unsafe {
            PgRelation::with_lock(
                indexinfo.indexoid,
                pg_sys::AccessShareLock as pg_sys::LOCKMODE,
            )
        };
        let rdopts = get_rdopts(index_relation);
        let ef_search = rdopts.ef_search as f64;

        let mut generic_costs = pg_sys::GenericCosts::default();
        pg_sys::genericcostestimate(root, path, loop_count, &mut generic_costs);

        *index_startup_cost = ef_search * pg_sys::random_page_cost;
        *index_total_cost = *index_startup_cost;
        *index_selectivity = if (*indexinfo.rel).rows != 0.0 {
            ef_search / (*indexinfo.rel).rows
        } else {
            generic_costs.indexSelectivity
        };
        *index_correlation = generic_costs.indexCorrelation;
        *index_pages = ef_search;
    }
}

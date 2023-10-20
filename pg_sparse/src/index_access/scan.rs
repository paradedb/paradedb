use pgrx::*;

use crate::sparse_index::index::SparseIndex;
use crate::sparse_index::sparse::Sparse;

// TODO: Enable this to be configured
const DEFAULT_EF_SEARCH: usize = 10;

#[derive(Debug)]
struct ScanState {
    pub index: SparseIndex,
    pub results: Vec<u64>,
    pub no_more_results: bool,
    pub current: usize,
    pub n_results: usize,
    pub k: usize,
}

#[pg_guard]
pub extern "C" fn ambeginscan(
    indexrel: pg_sys::Relation,
    nkeys: ::std::os::raw::c_int,
    norderbys: ::std::os::raw::c_int,
) -> pg_sys::IndexScanDesc {
    let mut scandesc: PgBox<pg_sys::IndexScanDescData> =
        unsafe { PgBox::from_pg(pg_sys::RelationGetIndexScan(indexrel, nkeys, norderbys)) };
    let index_relation = unsafe { PgRelation::from_pg(indexrel) };
    let index_name = index_relation.name().to_string();

    // Create the index and scan
    let sparse_index = SparseIndex::from_index_name(index_name);
    let scan_state = ScanState {
        index: sparse_index,
        results: vec![],
        current: 0,
        n_results: 0,
        no_more_results: false,
        k: DEFAULT_EF_SEARCH,
    };

    scandesc.opaque =
        PgMemoryContexts::CurrentMemoryContext.leak_and_drop_on_delete(scan_state) as void_mut_ptr;
    scandesc.into_pg()
}

// An annotation to guard the function for PostgreSQL's threading model.
#[pg_guard]
pub extern "C" fn amrescan(
    scan: pg_sys::IndexScanDesc,
    keys: pg_sys::ScanKey,
    nkeys: ::std::os::raw::c_int,
    orderbys: pg_sys::ScanKey,
    _norderbys: ::std::os::raw::c_int,
) {
    // Convert the raw pointer to a safe wrapper. This action takes ownership of the object
    // pointed to by the raw pointer in a safe way.
    let scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(scan) };

    // Extract the scan state from the opaque field of the scan descriptor.
    let state = unsafe { (scan.opaque as *mut ScanState).as_mut() }.expect("No scandesc state");

    state.results = vec![];
    state.current = 0;

    if !orderbys.is_null() && (*scan).numberOfOrderBys > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(
                orderbys,
                (*scan).orderByData,
                ((*scan).numberOfOrderBys * std::mem::size_of::<pg_sys::ScanKeyData>() as i32)
                    as usize,
            )
        };
    }
}

#[pg_guard]
pub extern "C" fn amendscan(scan: pg_sys::IndexScanDesc) {}

#[pg_guard]
pub extern "C" fn amgettuple(
    scan: pg_sys::IndexScanDesc,
    _direction: pg_sys::ScanDirection,
) -> bool {
    // Extract the scan state from the opaque field of the scan descriptor.
    let mut scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(scan) };
    let mut state = unsafe { (scan.opaque as *mut ScanState).as_mut() }.expect("No scandesc state");
    let order_by_data = unsafe { (scan.orderByData).as_mut() }.expect("No orderByData state");

    // Obtain the query vector
    let sk_argument: Option<Sparse> =
        unsafe { FromDatum::from_datum(order_by_data.sk_argument, false) };
    let sparse_vector = sk_argument.expect("Could not parse query vector");

    // First scan
    if state.current == 0 {
        state.results = state.index.search(&sparse_vector, state.k);
        state.n_results = state.results.len();
        state.no_more_results = state.n_results < state.k;
    }

    // Subsequent scans with larger k if necessary
    if state.current >= state.n_results {
        if state.no_more_results {
            return false;
        }

        state.k *= 2;

        state.results = state.index.search(&sparse_vector, state.k);
        state.n_results = state.results.len();
        state.no_more_results = state.n_results < state.k;
    }

    // Iterate through results
    if state.current < state.n_results {
        #[cfg(any(feature = "pg10", feature = "pg11"))]
        let tid = &mut scan.xs_ctup.t_self;
        #[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
        let tid = &mut scan.xs_heaptid;

        u64_to_item_pointer(state.results[state.current], tid);
        state.current += 1;

        scan.xs_recheckorderby = false;
        return true;
    }

    false
}

use hnswlib::Index;
use pgrx::*;

use crate::sparse_index::index::from_index_name;
use crate::sparse_index::sparse::Sparse;

// TODO: Enable this to be configured
const DEFAULT_EF_SEARCH: usize = 10;

struct ScanState {
    pub index: Index,
    pub results: Vec<usize>,
    pub no_more_results: bool,
    pub current: usize,
    pub n_results: usize,
    pub k: usize,
    pub query_vector: Sparse,
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
    let index = from_index_name(&index_name);

    // Create the index and scan
    let scan_state = ScanState {
        index,
        results: vec![],
        current: 0,
        n_results: 0,
        no_more_results: false,
        k: DEFAULT_EF_SEARCH,
        query_vector: Sparse {
            entries: vec![],
            n: 0
        }
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
    norderbys: ::std::os::raw::c_int,
) {
    // Convert the raw pointer to a safe wrapper. This action takes ownership of the object
    // pointed to by the raw pointer in a safe way.
    let scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(scan) };

    // Extract the scan state from the opaque field of the scan descriptor.
    let state = unsafe { (scan.opaque as *mut ScanState).as_mut() }.expect("No scandesc state");

    state.results.clear();
    state.current = 0;

    if !orderbys.is_null() && norderbys > 0 {
        let orderbys_slice = unsafe {
            std::slice::from_raw_parts(orderbys, norderbys as usize)
        };

        let sk_argument: Option<Sparse> =
            unsafe { FromDatum::from_datum(orderbys_slice[0].sk_argument, false) };
        state.query_vector = sk_argument.expect("Could not parse query vector");
    }
}

#[pg_guard]
pub extern "C" fn amendscan(scan: pg_sys::IndexScanDesc) {}

#[pg_guard]
pub extern "C" fn amgettuple(
    scan: pg_sys::IndexScanDesc,
    direction: pg_sys::ScanDirection,
) -> bool {
    info!("Begin search");
    assert!(direction == pg_sys::ScanDirection_ForwardScanDirection);

    // Extract the scan state from the opaque field of the scan descriptor.
    let mut scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(scan) };
    let mut state = unsafe { (scan.opaque as *mut ScanState).as_mut() }.expect("No scandesc state");

    // First scan
    if state.current == 0 {
        state.results =
            state
                .index
                .search_knn(state.query_vector.entries.clone(), state.k, DEFAULT_EF_SEARCH);

        info!("results {:?}", state.results);
        state.n_results = state.results.len();
        state.no_more_results = state.n_results < state.k;
    }

    // Subsequent scans with larger k if necessary
    if state.current >= state.n_results {
        if state.no_more_results {
            return false;
        }

        state.k *= 2;

        state.results =
            state
                .index
                .search_knn(state.query_vector.entries.clone(), state.k, DEFAULT_EF_SEARCH);
        state.n_results = state.results.len();
        state.no_more_results = state.n_results < state.k;
    }

    info!("iterating thorugh results");
    // Iterate through results
    #[cfg(any(feature = "pg10", feature = "pg11"))]
    let tid = &mut scan.xs_ctup.t_self;
    #[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
    let tid = &mut scan.xs_heaptid;

    info!("Setting tid {:?} to {:?}", tid, state.results[state.current] as u64);
    u64_to_item_pointer(state.results[state.current] as u64, tid);
    info!("Set tid {:?} to {:?}", tid, state.results[state.current] as u64);
    state.current += 1;
    scan.xs_recheckorderby = false;

    info!("returning true");
    true 
}

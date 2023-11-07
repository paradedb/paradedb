use hnswlib::Index;
use pgrx::*;

use crate::sparse_index::index::{from_index_name, get_rdopts};
use crate::sparse_index::sparse::Sparse;

struct ScanState {
    pub index: Index,
    pub results: Vec<usize>,
    pub no_more_results: bool,
    pub current: usize,
    pub n_results: usize,
    pub k: usize,
    pub ef_search: usize,
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
    let rdopts = get_rdopts(index_relation);

    // Create the index and scan
    let scan_state = ScanState {
        index,
        results: vec![],
        current: 0,
        n_results: 0,
        no_more_results: false,
        k: rdopts.ef_search as usize,
        ef_search: rdopts.ef_search as usize,
        query_vector: Sparse {
            entries: vec![],
            n: 0,
        },
    };

    scandesc.opaque =
        PgMemoryContexts::CurrentMemoryContext.leak_and_drop_on_delete(scan_state) as void_mut_ptr;
    scandesc.into_pg()
}

#[pg_guard]
pub extern "C" fn amrescan(
    scan: pg_sys::IndexScanDesc,
    _keys: pg_sys::ScanKey,
    _nkeys: ::std::os::raw::c_int,
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

    // Retrieve the query vector
    if !orderbys.is_null() && norderbys > 0 {
        let orderbys_slice = unsafe { std::slice::from_raw_parts(orderbys, norderbys as usize) };
        let sk_argument: Option<Sparse> =
            unsafe { FromDatum::from_datum(orderbys_slice[0].sk_argument, false) };
        state.query_vector = sk_argument.expect("Could not parse query vector");
    }
}

#[pg_guard]
pub extern "C" fn amendscan(_scan: pg_sys::IndexScanDesc) {}

// Under the hood, Postgres repeatedly calls amgettuple until it returns false,
// or until it has retrieved enough tuples to satisfy the query.
// The purpose of amgettuple is to point the heaptid inside the scan state
// to the heaptid of the next tuple (i.e. row) to be returned

// In the context of HNSW, amgettuple calls search_knn() on the first invocation
// to retrieve k nearest neighbors. If the user requests more than k tuples,
// k is doubled and search_knn() is called again. This repeats until enough tuples
// are returned or there are no more results to return.
#[pg_guard]
pub extern "C" fn amgettuple(
    scan: pg_sys::IndexScanDesc,
    direction: pg_sys::ScanDirection,
) -> bool {
    assert!(direction == pg_sys::ScanDirection_ForwardScanDirection);

    // Extract the scan state from the opaque field of the scan descriptor.
    let mut scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(scan) };
    let state = unsafe { (scan.opaque as *mut ScanState).as_mut() }.expect("No scandesc state");

    // First scan
    if state.current == 0 {
        let results =
            state
                .index
                .search_knn(state.query_vector.entries.clone(), state.k, state.ef_search);

        state.results = results.clone();
        state.n_results = results.len();
        state.no_more_results = state.n_results < state.k;
    }

    // Subsequent scans with larger k if necessary
    if state.current >= state.n_results {
        if state.no_more_results {
            return false;
        }

        state.k *= 2;

        let results =
            state
                .index
                .search_knn(state.query_vector.entries.clone(), state.k, state.ef_search);

        state.results = results.clone();
        state.n_results = state.results.len();
        state.no_more_results = state.n_results < state.k;
    }

    // Iterate through results
    #[cfg(any(
        feature = "pg12",
        feature = "pg13",
        feature = "pg14",
        feature = "pg15",
        feature = "pg16"
    ))]
    let tid = &mut scan.xs_heaptid;

    u64_to_item_pointer(state.results[state.current] as u64, tid);
    state.current += 1;
    scan.xs_recheckorderby = false;
    true
}

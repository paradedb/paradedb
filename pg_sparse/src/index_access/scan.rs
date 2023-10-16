use pgrx::*;

use crate::sparse_index::{Sparse, SparseIndex};

struct ScanState {
    pub index: SparseIndex,
    pub iterator: *mut std::vec::IntoIter<pg_sys::ItemPointerData>,
}

#[pg_guard]
pub extern "C" fn ambeginscan(
    indexrel: pg_sys::Relation,
    nkeys: ::std::os::raw::c_int,
    norderbys: ::std::os::raw::c_int,
) -> pg_sys::IndexScanDesc {
    info!("Beginning scan");
    let mut scandesc: PgBox<pg_sys::IndexScanDescData> =
        unsafe { PgBox::from_pg(pg_sys::RelationGetIndexScan(indexrel, nkeys, norderbys)) };
    let index_relation = unsafe { PgRelation::from_pg(indexrel) };
    let index_name = index_relation.name().to_string();

    // Create the index and scan
    let sparse_index = SparseIndex::from_index_name(index_name);
    let scan_state = ScanState {
        index: sparse_index,
        iterator: std::ptr::null_mut(),
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
    _orderbys: pg_sys::ScanKey,
    _norderbys: ::std::os::raw::c_int,
) {
    // Ensure there's at least one key provided for the search.
    if nkeys == 0 {
        panic!("no ScanKeys provided");
    }

    info!("Rescanning");

    // Convert the raw pointer to a safe wrapper. This action takes ownership of the object
    // pointed to by the raw pointer in a safe way.
    let scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(scan) };

    // Extract the scan state from the opaque field of the scan descriptor.
    let state =
        unsafe { (scan.opaque as *mut ScanState).as_mut() }.expect("no scandesc state");

    // Convert the raw keys into a slice for easier access.
    let nkeys = nkeys as usize;
    let keys = unsafe { std::slice::from_raw_parts(keys as *const pg_sys::ScanKeyData, nkeys) };

    // Convert keys[0].sk_argument into a Sparse object
    let sparse_vector: Option<Sparse> = unsafe {
        FromDatum::from_datum(keys[0].sk_argument, false)
    };

    info!("Got sparse vector in rescan {:?}", sparse_vector);
}

#[pg_guard]
pub extern "C" fn amendscan(_scan: pg_sys::IndexScanDesc) {}

#[pg_guard]
pub extern "C" fn amgettuple(
    scan: pg_sys::IndexScanDesc,
    _direction: pg_sys::ScanDirection,
) -> bool {
    info!("Get tuple");
    true
}


use crate::env::needs_commit;
use crate::index::state::SearchStateManager;
use crate::schema::SearchConfig;
use crate::{globals::WriterGlobal, postgres::utils::get_search_index};
use pgrx::*;
use tantivy::{DocAddress, Score};

#[pg_guard]
pub extern "C" fn ambeginscan(
    indexrel: pg_sys::Relation,
    nkeys: ::std::os::raw::c_int,
    norderbys: ::std::os::raw::c_int,
) -> pg_sys::IndexScanDesc {
    let scandesc: PgBox<pg_sys::IndexScanDescData> =
        unsafe { PgBox::from_pg(pg_sys::RelationGetIndexScan(indexrel, nkeys, norderbys)) };

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

    // Convert the raw pointer to a safe wrapper. This action takes ownership of the object
    // pointed to by the raw pointer in a safe way.
    let mut scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(scan) };

    // Convert the raw keys into a slice for easier access.
    let nkeys = nkeys as usize;
    let keys = unsafe { std::slice::from_raw_parts(keys as *const pg_sys::ScanKeyData, nkeys) };

    // Convert the first scan key argument into a byte array. This is assumed to be the `::jsonb` search config.
    let config_jsonb = unsafe {
        JsonB::from_datum(keys[0].sk_argument, false)
            .expect("failed to convert query to tuple of strings")
    };

    let search_config =
        SearchConfig::from_jsonb(config_jsonb).expect("could not parse search config");
    let index_name = &search_config.index_name;

    // Create the index and scan state
    let search_index = get_search_index(index_name);
    let writer_client = WriterGlobal::client();
    let state = search_index
        .search_state(&writer_client, &search_config, needs_commit())
        .unwrap();

    let top_docs = state.search(search_index.executor);

    SearchStateManager::set_state(state.clone()).expect("could not store search state in manager");

    // Save the iterator onto the current memory context.
    scan.opaque = PgMemoryContexts::CurrentMemoryContext
        .leak_and_drop_on_delete(top_docs.into_iter()) as void_mut_ptr;

    // Return scan state back management to Postgres.
    scan.into_pg();
}

#[pg_guard]
pub extern "C" fn amendscan(_scan: pg_sys::IndexScanDesc) {}

#[pg_guard]
pub extern "C" fn amgettuple(
    scan: pg_sys::IndexScanDesc,
    _direction: pg_sys::ScanDirection,
) -> bool {
    let mut scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(scan) };
    let iter =
        unsafe { (scan.opaque as *mut std::vec::IntoIter<(Score, DocAddress, i64, u64)>).as_mut() }
            .expect("no scandesc state");

    scan.xs_recheck = false;

    match iter.next() {
        Some((_, _, _, ctid)) => {
            #[cfg(any(
                feature = "pg12",
                feature = "pg13",
                feature = "pg14",
                feature = "pg15",
                feature = "pg16"
            ))]
            let tid = &mut scan.xs_heaptid;
            u64_to_item_pointer(ctid, tid);

            true
        }
        None => false,
    }
}

pub extern "C" fn amgetbitmap(
    scan: pg_sys::IndexScanDesc,
    tbm: *mut pg_sys::TIDBitmap,
) -> ::std::os::raw::c_long {
    let scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(scan) };
    let iter =
        unsafe { (scan.opaque as *mut std::vec::IntoIter<(Score, DocAddress, i64, u64)>).as_mut() }
            .expect("no scandesc state");

    let mut n_tids = 0;

    for (_, _, _, ctid) in iter {
        let mut tid = pg_sys::ItemPointerData::default();
        u64_to_item_pointer(ctid, &mut tid);

        unsafe {
            pg_sys::tbm_add_tuples(
                tbm,
                &tid as *const pg_sys::ItemPointerData as *mut pg_sys::ItemPointerData,
                1,
                false,
            );
        }

        n_tids += 1;
    }

    n_tids
}

#[pg_guard]
pub extern "C" fn amgetbitmap2(
    scan: pg_sys::IndexScanDesc,
    tbm: *mut pg_sys::TIDBitmap,
) -> ::std::os::raw::c_long {
    let scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(scan) };

    // Ensure there's at least one key provided for the search.
    if scan.numberOfKeys == 0 {
        panic!("no ScanKeys provided");
    }

    // Convert the raw keys into a slice for easier access.
    let nkeys = scan.numberOfKeys as usize;
    let keys =
        unsafe { std::slice::from_raw_parts(scan.keyData as *const pg_sys::ScanKeyData, nkeys) };

    // Convert the first scan key argument into a byte array. This is assumed to be the `::jsonb` search config.
    let config_jsonb = unsafe {
        JsonB::from_datum(keys[0].sk_argument, false)
            .expect("failed to convert query to tuple of strings")
    };

    let search_config =
        SearchConfig::from_jsonb(config_jsonb).expect("could not parse search config");
    let index_name = &search_config.index_name;

    // Create the index and scan state
    let search_index = get_search_index(index_name);
    let writer_client = WriterGlobal::client();
    let state = search_index
        .search_state(&writer_client, &search_config, needs_commit())
        .unwrap();

    let top_docs = state.search(search_index.executor);

    SearchStateManager::set_state(state.clone()).expect("could not store search state in manager");

    // Save the iterator onto the current memory context.
    let mut iter = top_docs.into_iter();

    // Initialize tuple counter
    let mut n_tids = 0;

    // Iterate over the results and add them to the bitmap
    while let Some((_, _, _, ctid)) = iter.next() {
        let mut tid = pg_sys::ItemPointerData::default();
        u64_to_item_pointer(ctid, &mut tid);

        unsafe {
            pg_sys::tbm_add_tuples(
                tbm,
                &tid as *const pg_sys::ItemPointerData as *mut pg_sys::ItemPointerData,
                1,
                false,
            );
        }

        n_tids += 1;
    }

    n_tids as ::std::os::raw::c_long
}

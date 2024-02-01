use crate::env::needs_commit;
use crate::schema::SearchConfig;
use crate::{globals::WriterGlobal, index::state::SearchState, postgres::utils::get_search_index};
use pgrx::*;

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

    let query_config =
        SearchConfig::from_jsonb(config_jsonb).expect("could not parse search config");
    let index_name = &query_config.index_name;

    // Create the index and scan state
    let search_index = get_search_index(index_name);
    let writer_client = WriterGlobal::client();
    let mut state = search_index
        .search_state(&writer_client, &query_config, needs_commit())
        .unwrap();

    let top_docs = state.search();

    // Store the search results in the scan state, ensuring they get freed when the current memory context is deleted.
    state.iterator =
        PgMemoryContexts::CurrentMemoryContext.leak_and_drop_on_delete(top_docs.into_iter());

    // Save the scan state onto the current memory context.
    scan.opaque =
        PgMemoryContexts::CurrentMemoryContext.leak_and_drop_on_delete(state) as void_mut_ptr;

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
    let state = unsafe { (scan.opaque as *mut SearchState).as_mut() }.expect("no scandesc state");

    scan.xs_recheck = false;

    let iter = unsafe { state.iterator.as_mut() }.expect("no iterator in state");

    match iter.next() {
        Some((_score, doc_address)) => {
            #[cfg(any(
                feature = "pg12",
                feature = "pg13",
                feature = "pg14",
                feature = "pg15",
                feature = "pg16"
            ))]
            let tid = &mut scan.xs_heaptid;

            let searcher = &state.searcher;
            let schema = &state.schema;
            let retrieved_doc = searcher.doc(doc_address).expect("could not find doc");

            let ctid_name = "ctid";
            let ctid_field = schema.schema.get_field(ctid_name).unwrap_or_else(|err| {
                panic!("error retrieving {ctid_name} field from schema: {err:?}")
            });
            let ctid_field_value = retrieved_doc
                .get_first(ctid_field)
                .unwrap_or_else(|| panic!("cannot find {ctid_name} field on retrieved document"));

            match ctid_field_value {
                tantivy::schema::Value::U64(val) => {
                    u64_to_item_pointer(*val, tid);
                    if unsafe { !item_pointer_is_valid(tid) } {
                        panic!("invalid item pointer: {:?}", item_pointer_get_both(*tid));
                    }
                }
                _ => panic!("incorrect type in {ctid_name} field: {ctid_field_value:?}"),
            };

            true
        }
        None => false,
    }
}

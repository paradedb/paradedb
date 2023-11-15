use std::str::FromStr;

use pgrx::*;
use tantivy::DocAddress;

use crate::{
    index_access::utils::{get_parade_index, SearchConfig},
    manager::{get_current_executor_manager, get_fresh_executor_manager},
    parade_index::state::TantivyScanState,
};

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

    // Convert the first scan key argument into a string. This is assumed to be the query string.
    let config_json: String = unsafe {
        String::from_datum(keys[0].sk_argument, false)
            .expect("failed to convert query to tuple of strings")
    };

    let query_config = SearchConfig::from_str(&config_json).expect("could not parse search config");
    let index_name = &query_config.index_name;

    // Create the index and scan state
    let parade_index = get_parade_index(index_name);
    let mut state = parade_index.scan_state(&query_config);

    let top_docs = state.search();

    // Cache min/max scorequery_config: SearchQuery
    let scores: Vec<f32> = top_docs.iter().map(|(score, _)| *score).collect();
    let max_score = scores.iter().fold(0.0f32, |a, b| a.max(*b));
    let min_score = scores.iter().fold(0.0f32, |a, b| a.min(*b));

    // We get a fresh executor manager here to clear out memory from previous queries.
    let manager = get_fresh_executor_manager();
    manager.set_max_score(max_score);
    manager.set_min_score(min_score);

    // Add snippet generators.
    let snippet_generators = state.snippet_generators(&query_config);
    manager.set_snippet_generators(snippet_generators);

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
    let state =
        unsafe { (scan.opaque as *mut TantivyScanState).as_mut() }.expect("no scandesc state");

    scan.xs_recheck = false;

    let iter = unsafe { state.iterator.as_mut() }.expect("no iterator in state");

    match iter.next() {
        Some((score, doc_address)) => {
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
            let _v: Vec<_> = schema.fields().collect();

            let ctid_name = "ctid";
            let ctid_field = schema.get_field(ctid_name).unwrap_or_else(|err| {
                panic!("error retrieving {ctid_name} field from schema: {err:?}")
            });
            let ctid_field_value = retrieved_doc
                .get_first(ctid_field)
                .unwrap_or_else(|| panic!("cannot find {ctid_name} field on retrieved document"));

            let key_field_name = &state.key_field_name;
            let key_field = schema
                .get_field(key_field_name)
                .unwrap_or_else(|_| panic!("field '{key_field_name}' not found in schema"));
            let key_field_value = retrieved_doc.get_first(key_field).unwrap_or_else(|| {
                panic!("cannot find id field '{key_field_name}' on retrieved document")
            });

            match ctid_field_value {
                tantivy::schema::Value::U64(val) => {
                    u64_to_item_pointer(*val, tid);
                    if unsafe { !item_pointer_is_valid(tid) } {
                        panic!("invalid item pointer: {:?}", item_pointer_get_both(*tid));
                    }
                }
                _ => panic!("incorrect type in {ctid_name} field: {ctid_field_value:?}"),
            };

            if let tantivy::schema::Value::I64(val) = key_field_value {
                write_to_manager(*val, score, doc_address);
            } else {
                panic!("incorrect type in {key_field_name} field: {key_field_value:?}")
            }

            true
        }
        None => false,
    }
}

#[inline]
fn write_to_manager(bm25_id: i64, score: f32, doc_address: DocAddress) {
    let manager = get_current_executor_manager();
    // Add score
    manager.add_score(bm25_id, score);

    // Add doc address
    manager.add_doc_address(bm25_id, doc_address);
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::ambeginscan;
    use pgrx::*;
    use shared::testing::SETUP_SQL;

    use crate::operator::get_index_oid;

    #[pg_test]
    fn test_ambeginscan() {
        Spi::run(SETUP_SQL).expect("failed to create index and table");
        let oid = get_index_oid("idx_one_republic", "bm25")
            .expect("could not find oid for one_republic")
            .unwrap();

        let index = unsafe { pg_sys::index_open(oid, pg_sys::AccessShareLock as pg_sys::LOCKMODE) };
        let index_scan = ambeginscan(index, 3 as std::os::raw::c_int, 1 as std::os::raw::c_int);
        let scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(index_scan) };

        assert_eq!(scan.numberOfKeys, 3 as std::os::raw::c_int);
        assert!(!scan.is_null());
    }
}

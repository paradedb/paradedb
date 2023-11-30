use pgrx::*;
use tantivy::{
    collector::TopDocs,
    query::{BooleanQuery, Query, RegexQuery},
    query_grammar::Occur,
    DocAddress,
};

use crate::{
    index_access::utils::{get_parade_index, SearchQuery},
    manager::{get_current_executor_manager, get_fresh_executor_manager},
    parade_index::index::TantivyScanState,
};

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
    let parade_index = get_parade_index(index_name);
    let state = parade_index.scan();

    scandesc.opaque =
        PgMemoryContexts::CurrentMemoryContext.leak_and_drop_on_delete(state) as void_mut_ptr;
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
    let scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(scan) };

    // Extract the scan state from the opaque field of the scan descriptor.
    let state =
        unsafe { (scan.opaque as *mut TantivyScanState).as_mut() }.expect("no scandesc state");

    // Convert the raw keys into a slice for easier access.
    let nkeys = nkeys as usize;
    let keys = unsafe { std::slice::from_raw_parts(keys as *const pg_sys::ScanKeyData, nkeys) };

    // Convert the first scan key argument into a string. This is assumed to be the query string.
    let raw_query = unsafe {
        String::from_datum(keys[0].sk_argument, false).expect("failed to convert query to string")
    };

    // Parse the raw query into a SearchQuery object, which has additional configuration.
    let query_config: SearchQuery = raw_query.parse().unwrap_or_else(|err| {
        panic!("Failed to parse query: {}", err);
    });

    let fuzzy_fields = query_config.config.fuzzy_fields;
    let regex_fields = query_config.config.regex_fields;

    // Determine if we're using regex fields based on the presence or absence of prefix and fuzzy fields.
    // It panics if both are provided as that's considered an invalid input.
    let using_regex_fields = match (!regex_fields.is_empty(), !fuzzy_fields.is_empty()) {
        (true, true) => panic!("cannot search with both regex_fields and fuzzy_fields"),
        (true, false) => true,
        _ => false,
    };

    // We get a fresh executor manager here to clear out memory from previous queries.
    let manager = get_fresh_executor_manager();
    // Fetching references to state components for building the query.
    let query_parser = &mut state.query_parser;
    let searcher = &state.searcher;
    let schema = &state.schema;

    // Extract limit and offset from the query config or set defaults.
    let limit = query_config.config.limit.unwrap_or({
        let num_docs = searcher.num_docs() as usize;
        if num_docs > 0 {
            num_docs // The collector will panic if it's passed a limit of 0.
        } else {
            1 // Since there's no docs to return anyways, just use 1.
        }
    });
    let offset = query_config.config.offset.unwrap_or(0);

    // Construct the actual Tantivy search query based on the mode determined above.
    let tantivy_query: Box<dyn Query> = if using_regex_fields {
        let regex_pattern = format!("{}.*", &query_config.query);
        let mut queries: Vec<Box<dyn Query>> = Vec::new();

        // Build a regex query for each specified regex field.
        for field_name in &regex_fields {
            if let Ok(field) = schema.get_field(field_name) {
                let regex_query =
                    Box::new(RegexQuery::from_pattern(&regex_pattern, field).unwrap());
                queries.push(regex_query);
            }
        }

        // If there's only one query, use it directly; otherwise, combine the queries.
        if queries.len() == 1 {
            queries.remove(0)
        } else {
            let boolean_query =
                BooleanQuery::new(queries.into_iter().map(|q| (Occur::Should, q)).collect());
            Box::new(boolean_query)
        }
    } else {
        // Set fuzzy search configuration for each specified fuzzy field.
        let fuzzy_fields: Vec<String> = fuzzy_fields;

        let require_prefix = query_config.config.prefix.unwrap_or(true);
        let transpose_cost_one = query_config.config.transpose_cost_one.unwrap_or(true);
        let max_distance = query_config.config.distance.unwrap_or(2);

        for field_name in &fuzzy_fields {
            if let Ok(field) = schema.get_field(field_name) {
                query_parser.set_field_fuzzy(
                    field,
                    require_prefix,
                    max_distance,
                    transpose_cost_one,
                );
            }
        }

        // Construct the query using the lenient parser to tolerate minor errors in the input.
        query_parser
            .parse_query(&query_config.query)
            .expect("error parsing query")
    };

    // Execute the constructed search query on Tantivy.
    let top_docs = searcher
        .search(
            &tantivy_query,
            &TopDocs::with_limit(limit).and_offset(offset),
        )
        .expect("failed to search");

    // Cache min/max score
    let scores: Vec<f32> = top_docs.iter().map(|(score, _)| *score).collect();
    let max_score = scores.iter().fold(0.0f32, |a, b| a.max(*b));
    let min_score = scores.iter().fold(0.0f32, |a, b| a.min(*b));
    manager.set_max_score(max_score);
    manager.set_min_score(min_score);

    // Extract highlight_max_num_chars from the query config and add snippet generators.
    manager.add_snippet_generators(
        searcher,
        schema,
        &tantivy_query,
        query_config.config.max_num_chars,
    );

    // Cache the constructed query in the scan state for potential subsequent use.
    state.query = tantivy_query;

    // Store the search results in the scan state, ensuring they get freed when the current memory context is deleted.
    state.iterator =
        PgMemoryContexts::CurrentMemoryContext.leak_and_drop_on_delete(top_docs.into_iter());
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

            let heap_tid_field = schema
                .get_field("heap_tid")
                .expect("field 'heap_tid' not found in schema");

            if let tantivy::schema::Value::U64(heap_tid_value) = retrieved_doc
                .get_first(heap_tid_field)
                .expect("heap_tid field not found in doc")
            {
                u64_to_item_pointer(*heap_tid_value, tid);
            }

            if unsafe { !item_pointer_is_valid(tid) } {
                panic!("invalid item pointer: {:?}", item_pointer_get_both(*tid));
            }

            write_to_manager(*tid, score, doc_address);
            true
        }
        None => false,
    }
}

#[pg_guard]
pub extern "C" fn ambitmapscan(scan: pg_sys::IndexScanDesc, tbm: *mut pg_sys::TIDBitmap) -> i64 {
    // We get a fresh executor manager here to clear out memory from previous queries.
    let manager = get_fresh_executor_manager();
    let scan: PgBox<pg_sys::IndexScanDescData> = unsafe { PgBox::from_pg(scan) };
    let state =
        unsafe { (scan.opaque as *mut TantivyScanState).as_mut() }.expect("no scandesc state");
    let searcher = &state.searcher;
    let schema = &state.schema;
    let query = &state.query;

    // Add snippet generators
    manager.add_snippet_generators(searcher, schema, query, None);

    let mut cnt = 0i64;
    let iterator = unsafe { state.iterator.as_mut() }.expect("no iterator in state");

    for (score, doc_address) in iterator {
        let retrieved_doc = searcher.doc(doc_address).expect("could not find doc");
        let heap_tid_field = schema
            .get_field("heap_tid")
            .expect("field 'heap_tid' not found in schema");

        if let tantivy::schema::Value::U64(heap_tid_value) = retrieved_doc
            .get_first(heap_tid_field)
            .expect("heap_tid field not found in doc")
        {
            let mut tid = pg_sys::ItemPointerData::default();
            u64_to_item_pointer(*heap_tid_value, &mut tid);

            unsafe {
                pg_sys::tbm_add_tuples(tbm, &mut tid, 1, false);
            }

            write_to_manager(tid, score, doc_address);
            cnt += 1;
        }
    }

    cnt
}

#[inline]
fn write_to_manager(ctid: pg_sys::ItemPointerData, score: f32, doc_address: DocAddress) {
    let manager = get_current_executor_manager();
    // Add score
    manager.add_score(item_pointer_get_both(ctid), score);

    // Add doc address
    manager.add_doc_address(item_pointer_get_both(ctid), doc_address);
}

// #[cfg(feature = "pg_test")]
mod tests {
    use super::ambeginscan;
    use pgrx::*;

    const SETUP_SQL: &str = include_str!("../../sql/index_setup.sql");

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

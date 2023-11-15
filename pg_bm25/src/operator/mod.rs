use std::str::FromStr;

use pgrx::*;
use rustc_hash::FxHashSet;

use crate::index_access::utils::{get_parade_index, SearchConfig};

#[pg_extern]
fn search_tantivy(
    element: AnyElement,
    config_json: &str,
    fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    let default_hash_set = || {
        let search_config =
            SearchConfig::from_str(config_json).expect("could not parse search config");

        let parade_index = get_parade_index(&search_config.index_name);
        let mut scan_state = parade_index.scan_state(&search_config);
        let top_docs = scan_state.search();
        let mut hs = FxHashSet::default();

        for (_, doc_address) in top_docs {
            let heap_tid_value = scan_state.heap_tid(doc_address);
            hs.insert(heap_tid_value);
        }

        hs
    };

    let hash_set = unsafe { pg_func_extra(fcinfo, default_hash_set) };

    let tid = if element.oid() == pg_sys::TIDOID {
        item_pointer_to_u64(
            unsafe { pg_sys::ItemPointerData::from_datum(element.datum(), false) }
                .expect("could not create item pointer from tuple"),
        )
    } else {
        let search_config: SearchConfig =
            SearchConfig::from_str(config_json).expect("could not parse search config");
        let index_name = search_config.index_name;

        panic!("the index {index_name} doesn't exist. call create_bm25 first.");
    };

    hash_set.contains(&tid)
}

#[inline]
pub fn scan_index(query: &str, index_oid: pg_sys::Oid) -> FxHashSet<u64> {
    unsafe {
        let index = pg_sys::index_open(index_oid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
        let heap = pg_sys::relation_open(
            index.as_ref().unwrap().rd_index.as_ref().unwrap().indrelid,
            pg_sys::AccessShareLock as pg_sys::LOCKMODE,
        );

        let mut keys = PgBox::<pg_sys::ScanKeyData>::alloc0();
        keys.sk_argument = query.into_datum().unwrap();

        let scan = pg_sys::index_beginscan(heap, index, pg_sys::GetTransactionSnapshot(), 1, 0);
        pg_sys::index_rescan(scan, keys.into_pg(), 1, std::ptr::null_mut(), 0);

        let mut lookup = FxHashSet::default();
        loop {
            check_for_interrupts!();

            #[cfg(any(
                feature = "pg12",
                feature = "pg13",
                feature = "pg14",
                feature = "pg15",
                feature = "pg16"
            ))]
            let tid = {
                let slot = pg_sys::MakeSingleTupleTableSlot(
                    heap.as_ref().unwrap().rd_att,
                    &pg_sys::TTSOpsBufferHeapTuple,
                );

                if !pg_sys::index_getnext_slot(
                    scan,
                    pg_sys::ScanDirection_ForwardScanDirection,
                    slot,
                ) {
                    pg_sys::ExecDropSingleTupleTableSlot(slot);
                    break;
                }

                let tid = item_pointer_to_u64(slot.as_ref().unwrap().tts_tid);
                pg_sys::ExecDropSingleTupleTableSlot(slot);
                tid
            };
            lookup.insert(tid);
        }
        pg_sys::index_endscan(scan);
        pg_sys::index_close(index, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
        pg_sys::relation_close(heap, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
        lookup
    }
}

#[cfg(any(test, feature = "pg_test"))]
pub fn get_index_oid(
    table_name: &str,
    index_method: &str,
) -> Result<Option<pg_sys::Oid>, spi::Error> {
    let query = format!(
        "SELECT indexrelid
         FROM pg_index
         INNER JOIN pg_class ON pg_class.oid = pg_index.indexrelid
         INNER JOIN pg_am ON pg_am.oid = pg_class.relam
         WHERE pg_class.relname = '{}'
         AND pg_am.amname = '{}'
         LIMIT 1;",
        table_name, index_method
    );

    Spi::connect(|client| {
        let mut tup_table = client.select(&query, None, None)?;

        if let Some(row) = tup_table.next() {
            let oid = row["indexrelid"]
                .value::<pg_sys::Oid>()
                .expect("failed to get oid")
                .unwrap();

            return Ok(Some(oid));
        }
        Ok(None)
    })
}

extension_sql!(
    r#"
CREATE OPERATOR pg_catalog.@@@ (
    PROCEDURE = search_tantivy,
    LEFTARG = anyelement,
    RIGHTARG = text
);

CREATE OPERATOR CLASS anyelement_bm25_ops DEFAULT FOR TYPE anyelement USING bm25 AS
    OPERATOR 1 pg_catalog.@@@(anyelement, text),
    STORAGE anyelement;

"#,
    name = "bm25_ops_anyelement_operator"
);

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::*;

    use super::{get_index_oid, scan_index};
    use shared::testing::{QUERY_SQL, SETUP_SQL};

    #[pg_test]
    fn test_get_index_oid() -> Result<(), spi::Error> {
        Spi::run(SETUP_SQL)?;
        let oid = get_index_oid("idx_one_republic", "bm25")?;
        assert!(oid.is_some());
        Ok(())
    }

    #[pg_test]
    fn test_scan_index() {
        Spi::run(SETUP_SQL).expect("failed to create table and index");
        let oid = get_index_oid("idx_one_republic", "bm25").expect("oid not found");
        assert!(oid.is_some());

        let oid = oid.unwrap();
        let result_set = scan_index("lyrics:im", oid);
        assert_eq!(result_set.len(), 2);
    }

    #[pg_test]
    #[should_panic]
    fn fail_to_scan_index() {
        // Fail since there is no index created yet
        let res = Spi::run(QUERY_SQL);
        assert!(res.is_err());

        Spi::run(SETUP_SQL).expect("failed to create table and index");
        // Fail due to wrong query
        let res = Spi::run("SELECT description FROM one_republic_songs WHERE one_republic_songs @@@ 'album:Native'");
        assert!(res.is_err());
    }

    #[pg_test]
    // Since the "search_tantivy" function cannout be tested directly from here,
    // we'll take advantage of the SPI to test the @@@ operator which has "search_tantivy" as the corresponding procedure
    fn test_search_tantivy_operator() {
        Spi::run(SETUP_SQL).expect("failed to create table and index");

        let res = Spi::get_one::<&str>(QUERY_SQL).expect("failed to get one");
        assert_eq!(res, Some("If I Lose Myself"));
    }
}

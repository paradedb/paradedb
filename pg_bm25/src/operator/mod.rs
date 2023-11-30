use pgrx::*;
use rustc_hash::{FxHashMap, FxHashSet};

#[pg_extern(immutable, parallel_safe)]
fn search_tantivy(element: AnyElement, query: &str, fcinfo: pg_sys::FunctionCallInfo) -> bool {
    let context = unsafe { (*fcinfo).context };
    let index_not_found: &str = "Could not find a \"USING bm25\" index on this table";

    let index_oid = unsafe {
        let planner_info = (context as *mut pg_sys::PlannerInfo)
            .as_ref()
            .expect(index_not_found);
        let rte_array = (*planner_info.simple_rte_array).as_ref().unwrap();
        let root_oid = rte_array.relid;

        let table = pg_sys::relation_open(root_oid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
        let table_relation = PgRelation::from_pg(table);
        let table_name = table_relation.name().to_string();

        match get_index_oid(&table_name, "bm25") {
            Ok(Some(oid)) => oid,
            _ => panic!("{}", index_not_found),
        }
    };

    let tid = if element.oid() == pg_sys::TIDOID {
        Some(item_pointer_to_u64(
            unsafe { pg_sys::ItemPointerData::from_datum(element.datum(), false) }.unwrap(),
        ))
    } else {
        panic!("{}", index_not_found);
    };

    match tid {
        Some(tid) => unsafe {
            let mut lookup_by_query = pg_func_extra(fcinfo, || {
                FxHashMap::<(pg_sys::Oid, Option<String>), FxHashSet<u64>>::default()
            });

            lookup_by_query
                .entry((index_oid, Some(String::from(query))))
                .or_insert_with(|| scan_index(query, index_oid))
                .contains(&tid)
        },
        None => false,
    }
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

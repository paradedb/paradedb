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

            #[cfg(any(feature = "pg10", feature = "pg11"))]
            let tid = {
                let htup = pg_sys::index_getnext(scan, pg_sys::ScanDirection_ForwardScanDirection);
                if htup.is_null() {
                    break;
                }
                item_pointer_to_u64(htup.as_ref().unwrap().t_self)
            };

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

// implementation of `restrict` partially derived from [0] [Apache 2.0 license]. Some useful
// references to understand this implementation:
// - [0] https://github.com/zombodb/zombodb/blob/ebf9e8c766c555fdafb80d2421eff9f820eba8c7/src/zdbquery/opclass.rs#L108
// - [1] https://github.com/postgres/postgres/blob/master/src/backend/utils/adt/selfuncs.c
// - [2] https://www.postgresql.org/docs/current/xoper-optimization.html#XOPER-RESTRICT
#[pg_extern(immutable, parallel_safe)]
fn restrict(planner_info: Internal, _oid: pg_sys::Oid, args: Internal, var_rel_id: i32) -> f64 {
    // Unless we can calculate otherwise, we assume that the baseline selectivity is double of
    // Postgres' default for the `eqsel` operator. According to [2], this doubled value is the
    // defualt selectivity for the `matchingsel` operator. We believe this is a decent approximation
    // for baseline selectivity.
    const DEFAULT_BASELINE_FREQ: f64 = 2.0 * pg_sys::DEFAULT_EQ_SEL;

    let root: *mut pg_sys::PlannerInfo =
        unsafe { planner_info.get_mut::<pg_sys::PlannerInfo>().unwrap() as *mut _ };

    let limit = unsafe { (*root).limit_tuples };

    if limit <= 0.0 {
        info!("no limit on query, returning default baseline selectivity {DEFAULT_BASELINE_FREQ}");
        return DEFAULT_BASELINE_FREQ;
    }

    let args = unsafe { args.get_mut::<pg_sys::List>().unwrap() as *mut _ };
    let args = unsafe { PgList::<pg_sys::Node>::from_pg(args) };
    let left = args.get_ptr(0);
    let right = args.get_ptr(1);
    if left.is_none() {
        panic!("left argument is null");
    } else if right.is_none() {
        panic!("right argument is null");
    }

    let left = left.unwrap();
    let mut heap_relation = None;

    if unsafe { is_a(left, pg_sys::NodeTag_T_Var) } {
        let mut ldata = pg_sys::VariableStatData::default();

        unsafe {
            pg_sys::examine_variable(root, left, var_rel_id, &mut ldata);

            let type_oid = ldata.vartype;
            let tce: PgBox<pg_sys::TypeCacheEntry> =
                PgBox::from_pg(pg_sys::lookup_type_cache(type_oid, 0));
            let heaprel_id = tce.typrelid;

            if heaprel_id == pg_sys::InvalidOid {
                heap_relation = None;
            } else {
                heap_relation = Some(PgRelation::with_lock(
                    heaprel_id,
                    pg_sys::AccessShareLock as pg_sys::LOCKMODE,
                ));
            }

            // free the ldata struct
            if !ldata.statsTuple.is_null() {
                (ldata.freefunc.unwrap())(ldata.statsTuple);
            }
        }
    }

    if let Some(heap_relation) = heap_relation {
        info!(
            "heap relation namespace {}, name {}",
            heap_relation.namespace(),
            heap_relation.name()
        );

        // -2 chosen as debugging sentinel since apparently `heap_relation.reltuples()` is returning -1 in some (all?) cases
        // https://github.com/paradedb/paradedb/issues/323
        let reltuples = heap_relation.reltuples().unwrap_or(-2f32) as f64;

        if reltuples > 0.0 {
            let result = (limit / reltuples).clamp(0.0, 1.0);
            info!("parsed limit of {limit} and computed tuple count {reltuples} --- returning clamped selectivity {result}");
            return result;
        }
    }

    info!("could not compute tuple count, returning default baseline selectivity {DEFAULT_BASELINE_FREQ}");
    DEFAULT_BASELINE_FREQ
}

extension_sql!(
    r#"
CREATE OPERATOR pg_catalog.@@@ (
    PROCEDURE = search_tantivy,
    RESTRICT = restrict,
    LEFTARG = anyelement,
    RIGHTARG = text
);

CREATE OPERATOR CLASS anyelement_bm25_ops DEFAULT FOR TYPE anyelement USING bm25 AS
    OPERATOR 1 pg_catalog.@@@(anyelement, text),
    STORAGE anyelement;

"#,
    name = "bm25_ops_anyelement_operator"
);

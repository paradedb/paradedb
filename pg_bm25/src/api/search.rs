use pgrx::{pg_sys::ItemPointerData, *};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::manager::get_current_executor_manager;
use crate::operator::scan_index;
use crate::parade_index::index::ParadeIndex;

#[pg_extern]
pub fn rank_bm25(ctid: Option<ItemPointerData>) -> f32 {
    match ctid {
        Some(ctid) => get_current_executor_manager()
            .get_score(ctid)
            .unwrap_or(0.0f32),
        None => 0.0f32,
    }
}

#[pg_extern]
pub fn highlight_bm25(
    ctid: Option<ItemPointerData>,
    index_name: String,
    field_name: String,
) -> String {
    let ctid = match ctid {
        Some(ctid) => ctid,
        _ => return "".into(),
    };
    let manager = get_current_executor_manager();
    let parade_index = ParadeIndex::from_index_name(index_name);
    let doc_address = manager
        .get_doc_address(ctid)
        .expect("could not lookup doc address in manager in highlight_bm25");
    let retrieved_doc = parade_index
        .searcher()
        .doc(doc_address)
        .expect("searcher could not retrieve doc by address in highlight_bm25");

    manager
        .get_highlight(&field_name, &retrieved_doc)
        .unwrap_or("".into())
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[pg_extern]
pub fn minmax_bm25(
    ctid: pg_sys::ItemPointerData,
    index_name: &str,
    query: &str,
    fcinfo: pg_sys::FunctionCallInfo,
) -> f32 {
    let indexrel =
        PgRelation::open_with_name_and_share_lock(index_name).expect("could not open index");
    let index_oid = indexrel.oid();
    let tid = Some(item_pointer_to_u64(ctid));

    match tid {
        Some(tid) => unsafe {
            let mut lookup_by_query = pg_func_extra(fcinfo, || {
                FxHashMap::<(pg_sys::Oid, Option<String>), FxHashSet<u64>>::default()
            });

            lookup_by_query
                .entry((index_oid, Some(String::from(query))))
                .or_insert_with(|| scan_index(query, index_oid))
                .contains(&tid);

            let max_score = get_current_executor_manager().get_max_score();
            let min_score = get_current_executor_manager().get_min_score();
            let raw_score = get_current_executor_manager()
                .get_score(ctid)
                .unwrap_or(0.0);

            if raw_score == 0.0 && min_score == max_score {
                return 0.0;
            }

            if min_score == max_score {
                return 1.0;
            }

            (raw_score - min_score) / (max_score - min_score)
        },
        None => 0.0,
    }
}

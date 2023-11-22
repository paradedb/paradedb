use pgrx::*;

use crate::index_access::utils::{
    categorize_tupdesc, get_parade_index, lookup_index_tupdesc, row_to_json,
};

#[allow(clippy::too_many_arguments)]
#[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
#[pg_guard]
pub unsafe extern "C" fn aminsert(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    _isnull: *mut bool,
    heap_tid: pg_sys::ItemPointer,
    _heap_relation: pg_sys::Relation,
    _check_unique: pg_sys::IndexUniqueCheck,
    _index_unchanged: bool,
    _index_info: *mut pg_sys::IndexInfo,
) -> bool {
    aminsert_internal(index_relation, values, heap_tid)
}

#[cfg(any(feature = "pg12", feature = "pg13"))]
#[pg_guard]
pub unsafe extern "C" fn aminsert(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    _isnull: *mut bool,
    heap_tid: pg_sys::ItemPointer,
    _heap_relation: pg_sys::Relation,
    _check_unique: pg_sys::IndexUniqueCheck,
    _index_info: *mut pg_sys::IndexInfo,
) -> bool {
    aminsert_internal(index_relation, values, heap_tid)
}

#[inline(always)]
unsafe fn aminsert_internal(
    index_relation: pg_sys::Relation,
    values: *mut pg_sys::Datum,
    heap_tid: pg_sys::ItemPointer,
) -> bool {
    let index_relation_ref: PgRelation = PgRelation::from_pg(index_relation);
    let index_name = index_relation_ref.name().to_string();

    let tupdesc = lookup_index_tupdesc(&index_relation_ref);
    let attributes = categorize_tupdesc(&tupdesc);
    let natts = tupdesc.natts as usize;
    let dropped = (0..tupdesc.natts as usize)
        .map(|i| tupdesc.get(i).unwrap().is_dropped())
        .collect::<Vec<bool>>();
    let values = std::slice::from_raw_parts(values, 1);
    let builder = row_to_json(values[0], &tupdesc, natts, &dropped, &attributes);

    // Insert row to parade index
    let mut parade_index = get_parade_index(index_name);
    parade_index.insert(*heap_tid, builder);

    true
}

#[cfg(feature = "pg_test")]
#[pgrx::pg_schema]
mod tests {
    use super::aminsert_internal;
    use pgrx::*;
    use shared::testing::SETUP_SQL;

    use crate::operator::get_index_oid;

    #[pg_test]
    fn test_aminsert_internal() {
        Spi::run(SETUP_SQL).expect("failed to create index and table");
        let oid = get_index_oid("idx_one_republic", "bm25")
            .expect("could not find oid for one_republic")
            .unwrap();

        let last_ctid = Spi::get_one::<pg_sys::ItemPointerData>(
            "SELECT ctid from one_republic_songs WHERE title = 'Apologize'",
        )
        .expect("failed to get last ctid");
        assert!(last_ctid.is_some());
        let ctid = &mut last_ctid.unwrap() as pg_sys::ItemPointer;

        let values = {
            let new_song = r#"
            {
                "title: "Love Runs Out",
                "album": "Native",
                "release_year": 2014,
                "genre": "Pop Rock",
                "description": "Energetic anthem about the determination to succeed in love and life.",
                "lyrics": "Ill be your light, your match, your burning sun,Ill be the bright and black that's making you run,And I feel alright, and we'll feel alright,'Cause we'll work it out, yeah we'll work it out."
            }
            "#;
            let song_json = JsonString(new_song.to_string());
            &mut song_json.into_datum().unwrap() as *mut pg_sys::Datum
        };

        unsafe {
            let index = pg_sys::index_open(oid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
            let res = aminsert_internal(index, values, ctid);
            assert!(res);
        };
    }
}

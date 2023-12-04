use pgrx::*;

#[pg_extern]
pub fn rank_bm25(_bm25_id: i64) -> f32 {
    // get_current_executor_manager()
    //     .get_score(bm25_id)
    //     .unwrap_or(0.0f32)
    0.0
}

#[pg_extern]
pub fn highlight_bm25(_bm25_id: i64, _index_name: String, _field_name: String) -> String {
    // let manager = get_current_executor_manager();
    // let parade_index = ParadeIndex::from_index_name(&index_name);
    // let doc_address = manager
    //     .get_doc_address(bm25_id)
    //     .expect("could not lookup doc address in manager in highlight_bm25");
    // let retrieved_doc = parade_index
    //     .searcher()
    //     .doc(doc_address)
    //     .expect("searcher could not retrieve doc by address in highlight_bm25");

    // manager
    //     .get_highlight(&field_name, &retrieved_doc)
    //     .unwrap_or("".into())
    "".to_string()
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[pg_extern]
pub fn minmax_bm25(
    bm25_id: i64,
    index_name: &str,
    query: &str,
    _fcinfo: pg_sys::FunctionCallInfo,
) -> f32 {
    // let indexrel =
    //     PgRelation::open_with_name_and_share_lock(index_name).expect("could not open index");
    // let index_oid = indexrel.oid();
    // let mut lookup_by_query = unsafe {
    //     pg_func_extra(fcinfo, || {
    //         FxHashMap::<(pg_sys::Oid, Option<String>), FxHashSet<u64>>::default()
    //     })
    // };

    // lookup_by_query
    //     .entry((index_oid, Some(String::from(query))))
    //     .or_insert_with(|| operator::scan_index(query, index_oid))
    //     .contains(&(bm25_id as u64));

    // let max_score = get_current_executor_manager().get_max_score();
    // let min_score = get_current_executor_manager().get_min_score();
    // let raw_score = get_current_executor_manager()
    //     .get_score(bm25_id)
    //     .unwrap_or(0.0);

    // if raw_score == 0.0 && min_score == max_score {
    //     return 0.0;
    // }

    // if min_score == max_score {
    //     return 1.0;
    // }

    // (raw_score - min_score) / (max_score - min_score)
    0.0
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::*;
    use shared::testing::SETUP_SQL;

    #[pg_test]
    fn test_rank_bm25() {
        Spi::run(SETUP_SQL).expect("failed to create index and table");
        let ctid = Spi::get_one::<pg_sys::ItemPointerData>(
            "SELECT ctid FROM one_republic_songs WHERE title = 'If I Lose Myself'",
        )
        .expect("could not get ctid");

        assert!(ctid.is_some());
        let ctid = ctid.unwrap();
        assert_eq!(ctid.ip_posid, 3);

        let query = "SELECT paradedb.rank_bm25(song_id) FROM one_republic_songs WHERE one_republic_songs @@@ 'lyrics:im AND description:song'";
        let rank = Spi::get_one::<f32>(query)
            .expect("failed to rank query")
            .unwrap();
        assert!(rank > 1.0);
    }

    #[pg_test]
    fn test_higlight() {
        Spi::run(SETUP_SQL).expect("failed to create index and table");

        let query = r#"
SELECT paradedb.highlight_bm25(song_id, 'idx_one_republic', 'lyrics')
FROM one_republic_songs
WHERE one_republic_songs @@@ 'lyrics:im:::max_num_chars=10';
        "#;

        let highlight = Spi::get_one::<&str>(query)
            .expect("failed to highlight lyrics")
            .unwrap();
        assert_eq!(highlight, "<b>Im</b> holding");
    }
}

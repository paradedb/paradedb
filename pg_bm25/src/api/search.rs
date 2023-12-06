use pgrx::{prelude::TableIterator, *};
use tantivy::{schema::FieldType, SnippetGenerator};

use crate::{
    index_access::utils::{get_parade_index, SearchConfig},
    parade_index::index::ParadeIndexKey,
};

#[pg_extern]
pub fn format_bm25_query(json: JsonB) -> String {
    let pgrx::JsonB(json_value) = json;
    let json_string = json_value.to_string();
    let search_config: SearchConfig =
        serde_json::from_value(json_value.clone()).expect("could not parse search config");

    info!("{search_config:#?}");

    let table = &search_config.table_name;
    let schema = &search_config.table_schema_name;
    let key = &search_config.key_field;

    let mut main_query = format!("SELECT * FROM {schema}.{table}");

    if let Some(highlight_field_name) = &search_config.highlight {
        main_query = format!(
            r#"
                {main_query}
                LEFT JOIN paradedb.highlight_bm25('{highlight_field_name}', '{json_string}') AS h
                ON {schema}.{table}.{key} = h.{key}
            "#
        )
    }

    main_query = format!("{main_query} WHERE ({schema}.{table}.ctid) @@@ '{json_string}'");

    info!("{}", main_query);

    main_query
}

#[pg_extern]
pub fn rank_bm25(
    config_json: JsonB,
) -> TableIterator<'static, (name!(id, i64), name!(rank_bm25, f32))> {
    let JsonB(search_config_json) = config_json;
    let search_config: SearchConfig =
        serde_json::from_value(search_config_json).expect("could not parse search config");
    let parade_index = get_parade_index(&search_config.index_name);

    let mut scan_state = parade_index.scan_state(&search_config);
    let top_docs = scan_state.search();

    let mut field_rows = Vec::new();
    for (score, doc_address) in top_docs.into_iter() {
        let document = scan_state
            .doc(doc_address)
            .unwrap_or_else(|err| panic!("error retrieving document for highlighting: {err:?}"));

        #[allow(unreachable_patterns)]
        let key = match parade_index.get_key_value(&document) {
            ParadeIndexKey::Number(k) => k,
            _ => unimplemented!("non-integer index keys are not yet implemented"),
        };
        field_rows.push((key, score));
    }
    TableIterator::new(field_rows)
}

#[pg_extern]
pub fn highlight_bm25(
    config_json: JsonB,
) -> TableIterator<'static, (name!(id, i64), name!(highlight_bm25, String))> {
    let JsonB(search_config_json) = config_json;
    let search_config: SearchConfig =
        serde_json::from_value(search_config_json).expect("could not parse search config");
    let parade_index = get_parade_index(&search_config.index_name);
    let schema = parade_index.schema();
    let function_schema = &search_config.schema_name;
    let field_name = search_config.highlight_field.as_ref().unwrap_or_else(|| {
        panic!("highlight_field parameter required for {function_schema}.highlight function")
    });
    let mut scan_state = parade_index.scan_state(&search_config);
    let top_docs = scan_state.search();

    let highlight_field = schema
        .get_field(&field_name)
        .unwrap_or_else(|err| panic!("error highlighting field {field_name}: {err:?}"));
    let highlight_field_entry = schema.get_field_entry(highlight_field);

    let mut snippet_generator = if let FieldType::Str(_) = highlight_field_entry.field_type() {
        SnippetGenerator::create(&parade_index.searcher(), &scan_state.query, highlight_field)
            .unwrap_or_else(|err| {
                panic!("failed to create snippet generator for field: {field_name}... {err}")
            })
    } else {
        panic!("can only highlight text fields")
    };

    if let Some(max_num_chars) = search_config.max_num_chars {
        snippet_generator.set_max_num_chars(max_num_chars);
    }

    let mut field_rows = Vec::new();
    for (_, doc_address) in top_docs {
        let document = scan_state
            .doc(doc_address)
            .unwrap_or_else(|err| panic!("error retrieving document for highlighting: {err:?}"));
        let snippet = snippet_generator.snippet_from_doc(&document);
        let html = snippet.to_html();

        #[allow(unreachable_patterns)]
        let key = match parade_index.get_key_value(&document) {
            ParadeIndexKey::Number(k) => k,
            _ => unimplemented!("non-integer index keys are not yet implemented"),
        };
        field_rows.push((key, html));
    }

    TableIterator::new(field_rows)
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[pg_extern]
pub fn minmax_bm25(
    _bm25_id: i64,
    _index_name: &str,
    _query: &str,
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

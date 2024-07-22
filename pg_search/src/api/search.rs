// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use crate::env::needs_commit;
use crate::index::state::{SearchAlias, SearchStateManager};
use crate::postgres::types::TantivyValue;
use crate::schema::SearchConfig;
use crate::writer::WriterDirectory;
use crate::{globals::WriterGlobal, index::SearchIndex};
use pgrx::{prelude::TableIterator, *};
use shared::postgres::transaction::Transaction;
use tantivy::TantivyDocument;

const DEFAULT_SNIPPET_PREFIX: &str = "<b>";
const DEFAULT_SNIPPET_POSTFIX: &str = "</b>";

#[pg_extern(name = "rank_bm25")]
pub fn rank_bm25(key: AnyElement, alias: default!(Option<String>, "NULL")) -> f32 {
    warning!("This function has been deprecated in favor of `score_bm25` since version 0.8.5");

    let key = unsafe { TantivyValue::try_from_anyelement(key).unwrap() };

    SearchStateManager::get_score(key, alias.map(SearchAlias::from))
        .expect("could not lookup doc address for search query")
}

#[pg_extern]
pub fn highlight(
    key: AnyElement,
    field: &str,
    prefix: default!(Option<String>, "NULL"),
    postfix: default!(Option<String>, "NULL"),
    max_num_chars: default!(Option<i32>, "NULL"),
    alias: default!(Option<String>, "NULL"),
) -> String {
    warning!("This function has been deprecated in favor of `snippet` since version 0.8.5");

    let key = unsafe {
        TantivyValue::try_from_anyelement(key)
            .expect("failed to convert key_field to Tantivy value")
    };

    let mut snippet = SearchStateManager::get_snippet(
        key,
        field,
        max_num_chars.map(|n| n as usize),
        alias.map(SearchAlias::from),
    )
    .expect("could not create snippet for highlighting");

    match (prefix, postfix) {
        (Some(prefix), Some(postfix)) => snippet.set_snippet_prefix_postfix(&prefix, &postfix),
        (None, Some(postfix)) => {
            snippet.set_snippet_prefix_postfix(DEFAULT_SNIPPET_PREFIX, &postfix)
        }
        (Some(prefix), None) => {
            snippet.set_snippet_prefix_postfix(&prefix, DEFAULT_SNIPPET_POSTFIX)
        }
        _ => snippet.set_snippet_prefix_postfix(DEFAULT_SNIPPET_PREFIX, DEFAULT_SNIPPET_POSTFIX),
    }

    snippet.to_html()
}

#[pg_extern]
pub fn minmax_bm25(
    config_json: JsonB,
    _key_type_dummy: Option<AnyElement>, // This ensures that postgres knows what the return type is
    key_oid: pgrx::pg_sys::Oid, // Have to pass oid as well because the dummy above will always by None
) -> TableIterator<'static, (name!(id, AnyElement), name!(rank_bm25, f32))> {
    warning!("`rank_hybrid` has been deprecated in favor of `score_hybrid` since version 0.8.5");

    let JsonB(search_config_json) = config_json;
    let search_config: SearchConfig =
        serde_json::from_value(search_config_json.clone()).expect("could not parse search config");
    let directory = WriterDirectory::from_index_name(&search_config.index_name);
    let search_index = SearchIndex::from_cache(&directory, &search_config.uuid)
        .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));

    let writer_client = WriterGlobal::client();
    let mut scan_state = search_index
        .search_state(
            &writer_client,
            &search_config,
            needs_commit(&search_config.index_name),
        )
        .unwrap();

    // Collect into a Vec to allow multiple iterations
    let top_docs: Vec<_> = scan_state.search_dedup(SearchIndex::executor()).collect();

    // Calculate min and max scores
    let (min_score, max_score) = top_docs
        .iter()
        .map(|(score, _)| score)
        .fold((f32::MAX, f32::MIN), |(min, max), bm25| {
            (min.min(*bm25), max.max(*bm25))
        });
    let score_range = max_score - min_score;

    // Now that we have min and max, iterate over the collected results
    let mut field_rows = Vec::new();
    for (score, doc_address) in top_docs {
        let key = unsafe {
            datum::AnyElement::from_polymorphic_datum(
                scan_state
                    .key_value(doc_address)
                    .try_into_datum(PgOid::from_untagged(key_oid))
                    .unwrap(),
                false,
                key_oid,
            )
            .unwrap()
        };
        let normalized_score = if score_range == 0.0 {
            1.0 // Avoid division by zero
        } else {
            (score - min_score) / score_range
        };

        field_rows.push((key, normalized_score));
    }
    TableIterator::new(field_rows)
}

#[pg_extern]
pub fn score_bm25(
    config_json: JsonB,
    _key_type_dummy: Option<AnyElement>, // This ensures that postgres knows what the return type is
    key_oid: pgrx::pg_sys::Oid, // Have to pass oid as well because the dummy above will always by None
) -> TableIterator<'static, (name!(id, AnyElement), name!(score_bm25, f32))> {
    let JsonB(search_config_json) = config_json;
    let search_config: SearchConfig =
        serde_json::from_value(search_config_json.clone()).expect("could not parse search config");
    let directory = WriterDirectory::from_index_name(&search_config.index_name);
    let search_index = SearchIndex::from_cache(&directory, &search_config.uuid)
        .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));

    let writer_client = WriterGlobal::client();
    let mut scan_state = search_index
        .search_state(
            &writer_client,
            &search_config,
            needs_commit(&search_config.index_name),
        )
        .expect("could not get scan state");

    let top_docs = scan_state
        .search_dedup(SearchIndex::executor())
        .map(|(score, doc_address)| {
            let key = unsafe {
                datum::AnyElement::from_polymorphic_datum(
                    scan_state
                        .key_value(doc_address)
                        .try_into_datum(PgOid::from_untagged(key_oid))
                        .expect("failed to convert key_field to datum"),
                    false,
                    key_oid,
                )
                .expect("null found in key_field")
            };
            (key, score)
        })
        .collect::<Vec<_>>();

    TableIterator::new(top_docs)
}

#[pg_extern]
pub fn snippet(
    config_json: JsonB,
    _key_type_dummy: Option<AnyElement>, // This ensures that postgres knows what the return type is
    key_oid: pgrx::pg_sys::Oid, // Have to pass oid as well because the dummy above will always by None
) -> TableIterator<
    'static,
    (
        name!(id, AnyElement),
        name!(snippet, String),
        name!(score_bm25, f32),
    ),
> {
    let JsonB(search_config_json) = config_json;
    let search_config: SearchConfig =
        serde_json::from_value(search_config_json.clone()).expect("could not parse search config");
    let directory = WriterDirectory::from_index_name(&search_config.index_name);
    let search_index = SearchIndex::from_cache(&directory, &search_config.uuid)
        .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));

    let writer_client = WriterGlobal::client();
    let mut scan_state = search_index
        .search_state(
            &writer_client,
            &search_config,
            needs_commit(&search_config.index_name),
        )
        .expect("could not get scan state");

    let highlight_field = search_config
        .highlight_field
        .expect("highlight_field is required");
    let mut snippet_generator = scan_state.snippet_generator(&highlight_field);
    if let Some(max_num_chars) = search_config.max_num_chars {
        snippet_generator.set_max_num_chars(max_num_chars)
    }

    let top_docs = scan_state
        .search_dedup(SearchIndex::executor())
        .map(|(score, doc_address)| {
            let key = unsafe {
                datum::AnyElement::from_polymorphic_datum(
                    scan_state
                        .key_value(doc_address)
                        .try_into_datum(PgOid::from_untagged(key_oid))
                        .expect("failed to convert key_field to datum"),
                    false,
                    key_oid,
                )
                .expect("null found in key_field")
            };

            let doc: TantivyDocument = scan_state
                .searcher
                .doc(doc_address)
                .expect("could not find document in searcher");

            let mut snippet = snippet_generator.snippet_from_doc(&doc);
            snippet.set_snippet_prefix_postfix(
                &search_config
                    .prefix
                    .clone()
                    .unwrap_or(DEFAULT_SNIPPET_PREFIX.to_string()),
                &search_config
                    .postfix
                    .clone()
                    .unwrap_or(DEFAULT_SNIPPET_POSTFIX.to_string()),
            );

            (key, snippet.to_html(), score)
        })
        .collect::<Vec<_>>();

    TableIterator::new(top_docs)
}

#[pg_extern]
fn drop_bm25_internal(index_name: &str) {
    let writer_client = WriterGlobal::client();

    // Drop the Tantivy data directory.
    SearchIndex::drop_index(&writer_client, index_name)
        .unwrap_or_else(|err| panic!("error dropping index {index_name}: {err:?}"));
}

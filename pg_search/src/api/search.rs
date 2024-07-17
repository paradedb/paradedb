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
use crate::{globals::WriterGlobal, index::SearchIndex, postgres::utils::get_search_index};
use pgrx::{prelude::TableIterator, *};

const DEFAULT_SNIPPET_PREFIX: &str = "<b>";
const DEFAULT_SNIPPET_POSTFIX: &str = "</b>";

#[pg_extern]
pub fn highlight(
    key: AnyElement,
    field: &str,
    prefix: default!(Option<String>, "NULL"),
    postfix: default!(Option<String>, "NULL"),
    max_num_chars: default!(Option<i32>, "NULL"),
    alias: default!(Option<String>, "NULL"),
) -> String {
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
pub fn rank_bm25(
    config_json: JsonB,
    _key_type_dummy: Option<AnyElement>, // This ensures that postgres knows what the return type is
    key_oid: pgrx::pg_sys::Oid, // Have to pass oid as well because the dummy above will always by None
) -> TableIterator<'static, (name!(id, AnyElement), name!(rank_bm25, f32))> {
    let JsonB(search_config_json) = config_json;
    let search_config: SearchConfig =
        serde_json::from_value(search_config_json.clone()).expect("could not parse search config");
    let search_index = get_search_index(&search_config.index_name);

    let writer_client = WriterGlobal::client();
    let mut scan_state = search_index
        .search_state(
            &writer_client,
            &search_config,
            needs_commit(&search_config.index_name),
        )
        .expect("could not get scan state");

    let top_docs = scan_state
        .search_dedup(search_index.executor)
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
fn drop_bm25_internal(index_name: &str) {
    let writer_client = WriterGlobal::client();

    // Drop the Tantivy data directory.
    SearchIndex::drop_index(&writer_client, index_name)
        .unwrap_or_else(|err| panic!("error dropping index {index_name}: {err}"));
}

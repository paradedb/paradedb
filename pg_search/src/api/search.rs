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
use crate::writer::{WriterClient, WriterDirectory};
use crate::{globals::WriterGlobal, index::SearchIndex, postgres::utils::get_search_index};
use pgrx::{prelude::TableIterator, *};

const DEFAULT_SNIPPET_PREFIX: &str = "<b>";
const DEFAULT_SNIPPET_POSTFIX: &str = "</b>";

macro_rules! rank_bm25_fn {
    ($func_name:ident, $key_type:ty) => {
        #[pg_extern(name = "rank_bm25")]
        pub fn $func_name(key: $key_type, alias: default!(Option<String>, "NULL")) -> f32 {
            rank_bm25_impl(TantivyValue::try_from(key).unwrap(), alias)
        }
    };
}

pub fn rank_bm25_impl(key: TantivyValue, alias: default!(Option<String>, "NULL")) -> f32 {
    SearchStateManager::get_score(key, alias.map(SearchAlias::from))
        .expect("could not lookup doc address for search query")
}

rank_bm25_fn!(rank_bool, bool);
rank_bm25_fn!(rank_i8, i8);
rank_bm25_fn!(rank_i16, i16);
rank_bm25_fn!(rank_i32, i32);
rank_bm25_fn!(rank_i64, i64);
rank_bm25_fn!(rank_f32, f32);
rank_bm25_fn!(rank_f64, f64);
rank_bm25_fn!(rank_numeric, pgrx::AnyNumeric);
rank_bm25_fn!(rank_string, String);
rank_bm25_fn!(rank_uuid, pgrx::Uuid);
rank_bm25_fn!(rank_date, pgrx::Date);
rank_bm25_fn!(rank_time, pgrx::Time);
rank_bm25_fn!(rank_timestamp, pgrx::Timestamp);
rank_bm25_fn!(rank_timetz, pgrx::TimeWithTimeZone);
rank_bm25_fn!(rank_timestamptz, pgrx::TimestampWithTimeZone);

macro_rules! highlight_fn {
    ($func_name:ident, $key_type:ty) => {
        #[pg_extern(name = "highlight")]
        pub fn $func_name(
            key: $key_type,
            field: &str,
            prefix: default!(Option<String>, "NULL"),
            postfix: default!(Option<String>, "NULL"),
            max_num_chars: default!(Option<i32>, "NULL"),
            alias: default!(Option<String>, "NULL"),
        ) -> String {
            highlight_impl(
                TantivyValue::try_from(key).unwrap(),
                field,
                prefix,
                postfix,
                max_num_chars,
                alias,
            )
        }
    };
}

highlight_fn!(highlight_i8, i8);
highlight_fn!(highlight_i16, i16);
highlight_fn!(highlight_i32, i32);
highlight_fn!(highlight_i64, i64);
highlight_fn!(highlight_f32, f32);
highlight_fn!(highlight_f64, f64);
highlight_fn!(highlight_numeric, pgrx::AnyNumeric);
highlight_fn!(highlight_string, String);
highlight_fn!(highlight_uuid, pgrx::Uuid);
highlight_fn!(highlight_date, pgrx::Date);
highlight_fn!(highlight_time, pgrx::Time);
highlight_fn!(highlight_timestamp, pgrx::Timestamp);
highlight_fn!(highlight_timetz, pgrx::TimeWithTimeZone);
highlight_fn!(highlight_timestamptz, pgrx::TimestampWithTimeZone);

pub fn highlight_impl(
    key: TantivyValue,
    field: &str,
    prefix: default!(Option<String>, "NULL"),
    postfix: default!(Option<String>, "NULL"),
    max_num_chars: default!(Option<i32>, "NULL"),
    alias: default!(Option<String>, "NULL"),
) -> String {
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

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[pg_extern]
pub fn minmax_bm25(
    config_json: JsonB,
) -> TableIterator<'static, (name!(id, String), name!(rank_bm25, f32))> {
    let JsonB(search_config_json) = config_json;
    let search_config: SearchConfig =
        serde_json::from_value(search_config_json).expect("could not parse search config");
    let search_index = get_search_index(&search_config.index_name);

    let writer_client = WriterGlobal::client();
    let mut scan_state = search_index
        .search_state(&writer_client, &search_config, needs_commit())
        .unwrap();

    // Collect into a Vec to allow multiple iterations
    let top_docs: Vec<_> = scan_state.search_dedup(search_index.executor).collect();

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
        let key = scan_state.key_value(doc_address);
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
fn drop_bm25_internal(index_name: &str) {
    let writer_client = WriterGlobal::client();
    let writer_directory = WriterDirectory::from_index_name(index_name);
    if needs_commit() {
        writer_client
            .lock()
            .expect("could not lock writer on drop_bm25")
            .request(crate::writer::WriterRequest::Commit {
                directory: writer_directory,
            })
            .expect("error committing existing transaction during drop_bm25");
    }
    // Drop the Tantivy data directory.
    SearchIndex::drop_index(&writer_client, index_name)
        .unwrap_or_else(|err| panic!("error dropping index {index_name}: {err}"));
}

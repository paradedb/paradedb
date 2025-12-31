// Copyright (c) 2023-2026 ParadeDB, Inc.
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

/// Contains our various "more_like_this" functions which live in the `pdb` schema.
#[pgrx::pg_schema]
mod pdb {
    use crate::api::HashMap;
    use crate::postgres::types::TantivyValue;
    use crate::query::SearchQueryInput;
    use pgrx::{default, pg_extern, AnyElement, PgOid};

    #[pg_extern(name = "more_like_this", immutable, parallel_safe)]
    pub fn more_like_this_empty() -> SearchQueryInput {
        panic!("more_like_this must be called with either key_value or document");
    }

    #[allow(clippy::too_many_arguments)]
    #[pg_extern(name = "more_like_this", immutable, parallel_safe)]
    pub fn more_like_this_fields(
        document: String,
        min_doc_frequency: default!(Option<i32>, "NULL"),
        max_doc_frequency: default!(Option<i32>, "NULL"),
        min_term_frequency: default!(Option<i32>, "NULL"),
        max_query_terms: default!(Option<i32>, "NULL"),
        min_word_length: default!(Option<i32>, "NULL"),
        max_word_length: default!(Option<i32>, "NULL"),
        boost_factor: default!(Option<f32>, "NULL"),
        stopwords: default!(Option<Vec<String>>, "NULL"),
    ) -> SearchQueryInput {
        let document: HashMap<String, tantivy::schema::OwnedValue> =
            json5::from_str(&document).expect("could not parse document_fields");

        SearchQueryInput::MoreLikeThis {
            min_doc_frequency: min_doc_frequency.map(|n| n as u64),
            max_doc_frequency: max_doc_frequency.map(|n| n as u64),
            min_term_frequency: min_term_frequency.map(|n| n as usize),
            max_query_terms: max_query_terms.map(|n| n as usize),
            min_word_length: min_word_length.map(|n| n as usize),
            max_word_length: max_word_length.map(|n| n as usize),
            boost_factor,
            stopwords,
            document: Some(document.into_iter().collect()),
            key_value: None,
            fields: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[pg_extern(name = "more_like_this", immutable, parallel_safe)]
    pub fn more_like_this_id(
        key_value: AnyElement,
        fields: default!(Option<Vec<String>>, "NULL"),
        min_doc_frequency: default!(Option<i32>, "NULL"),
        max_doc_frequency: default!(Option<i32>, "NULL"),
        min_term_frequency: default!(Option<i32>, "NULL"),
        max_query_terms: default!(Option<i32>, "NULL"),
        min_word_length: default!(Option<i32>, "NULL"),
        max_word_length: default!(Option<i32>, "NULL"),
        boost_factor: default!(Option<f32>, "NULL"),
        stopwords: default!(Option<Vec<String>>, "NULL"),
    ) -> SearchQueryInput {
        SearchQueryInput::MoreLikeThis {
            min_doc_frequency: min_doc_frequency.map(|n| n as u64),
            max_doc_frequency: max_doc_frequency.map(|n| n as u64),
            min_term_frequency: min_term_frequency.map(|n| n as usize),
            max_query_terms: max_query_terms.map(|n| n as usize),
            min_word_length: min_word_length.map(|n| n as usize),
            max_word_length: max_word_length.map(|n| n as usize),
            boost_factor,
            stopwords,
            fields: fields.map(|fields| fields.into_iter().collect()),
            key_value: unsafe {
                Some(
                    TantivyValue::try_from_datum(
                        key_value.datum(),
                        PgOid::from_untagged(key_value.oid()),
                    )
                    .unwrap_or_else(|err| panic!("could not read more_like_this key_value: {err}"))
                    .0,
                )
            },
            document: None,
        }
    }
}

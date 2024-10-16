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

use crate::postgres::options::SearchIndexCreateOptions;
use crate::query::SearchQueryInput;
use pgrx::{JsonB, PgRelation};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct SearchConfig {
    pub query: SearchQueryInput,
    pub index_name: String,
    pub index_oid: u32,
    pub table_oid: u32,
    pub database_oid: u32,
    pub key_field: String,
    pub offset_rows: Option<usize>,
    pub limit_rows: Option<usize>,
    pub max_num_chars: Option<usize>,
    pub highlight_field: Option<String>,
    pub prefix: Option<String>,
    pub postfix: Option<String>,
    pub stable_sort: Option<bool>,
    #[serde(default = "default_as_false")]
    pub need_scores: bool,
    pub uuid: String,
    pub order_by_field: Option<String>,
    pub order_by_direction: Option<String>,
    pub lenient_parsing: Option<bool>,
    pub conjunction_mode: Option<bool>,
}

fn default_as_false() -> bool {
    false
}

impl SearchConfig {
    pub fn contains_more_like_this(query: &SearchQueryInput) -> bool {
        match query {
            SearchQueryInput::Boolean {
                must,
                should,
                must_not,
            } => must
                .iter()
                .chain(should.iter())
                .chain(must_not.iter())
                .any(Self::contains_more_like_this),
            SearchQueryInput::Boost { query, .. } => Self::contains_more_like_this(query),
            SearchQueryInput::ConstScore { query, .. } => Self::contains_more_like_this(query),
            SearchQueryInput::DisjunctionMax { disjuncts, .. } => {
                disjuncts.iter().any(Self::contains_more_like_this)
            }
            SearchQueryInput::MoreLikeThis { .. } => true,
            _ => false,
        }
    }

    pub fn from_jsonb(JsonB(config_json_value): JsonB) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config_json_value)
    }

    /// Returns true if the [`SearchConfig`] instance is configured to sort fields in ascending order
    pub fn is_sort_ascending(&self) -> bool {
        match &self.order_by_direction {
            Some(direction) => direction.eq_ignore_ascii_case("asc"),
            None => true,
        }
    }
}

impl FromStr for SearchConfig {
    type Err = serde_path_to_error::Error<json5::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut deserializer = json5::Deserializer::from_str(s).expect("input is not valid json");
        serde_path_to_error::deserialize(&mut deserializer)
    }
}

pub type IndexRelation<'a> = &'a PgRelation;

impl From<(String, IndexRelation<'_>)> for SearchConfig {
    fn from(value: (String, IndexRelation)) -> Self {
        let (query_string, indexrel) = value;
        SearchConfig::from((
            SearchQueryInput::Parse {
                query_string,
                lenient: None,
                conjunction_mode: None,
            },
            indexrel,
        ))
    }
}

impl From<(SearchQueryInput, IndexRelation<'_>)> for SearchConfig {
    fn from(value: (SearchQueryInput, IndexRelation<'_>)) -> Self {
        let (query, indexrel) = value;
        let ops = indexrel.rd_options as *mut SearchIndexCreateOptions;
        let ops = unsafe { ops.as_ref().expect("indexrel.rd_options must not be null") };

        SearchConfig {
            query,
            index_name: indexrel.name().to_string(),
            index_oid: indexrel.oid().as_u32(),
            table_oid: indexrel.heap_relation().unwrap().oid().as_u32(),
            database_oid: crate::MyDatabaseId(),
            key_field: ops.get_key_field().unwrap().to_string(),
            offset_rows: None,
            limit_rows: None,
            max_num_chars: None,
            highlight_field: None,
            prefix: None,
            postfix: None,
            stable_sort: Some(false), // for speed
            need_scores: false,
            uuid: ops.get_uuid().unwrap(),
            order_by_field: None,
            order_by_direction: None,
            conjunction_mode: None,
            lenient_parsing: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::*;
    use tantivy::schema::{JsonObjectOptions, NumericOptions, TextOptions};

    use crate::schema::SearchFieldConfig;

    #[rstest]
    fn test_search_text_options() {
        let json = r#"{
            "indexed": true,
            "fast": false,
            "stored": true,
            "fieldnorms": true,
            "type": "default",
            "record": "basic",
            "normalizer": "raw"
        }"#;
        let config: serde_json::Value = serde_json::from_str(json).unwrap();
        let search_text_option: SearchFieldConfig =
            serde_json::from_value(serde_json::json!({"Text": config})).unwrap();
        let expected: TextOptions = search_text_option.into();

        let text_options: TextOptions = SearchFieldConfig::default_text().into();
        assert_eq!(expected.is_stored(), text_options.is_stored());
        assert_eq!(
            expected.get_fast_field_tokenizer_name(),
            text_options.get_fast_field_tokenizer_name()
        );

        let text_options = text_options.set_fast(Some("index"));
        assert_ne!(expected.is_fast(), text_options.is_fast());
    }

    #[rstest]
    fn test_search_numeric_options() {
        let json = r#"{
            "indexed": true,
            "stored": true,
            "fieldnorms": false,
            "fast": true
        }"#;
        let config: serde_json::Value = serde_json::from_str(json).unwrap();
        let expected: SearchFieldConfig =
            serde_json::from_value(serde_json::json!({"Numeric": config})).unwrap();
        let int_options: NumericOptions = SearchFieldConfig::default_numeric().into();

        assert_eq!(int_options, expected.into());
    }

    #[rstest]
    fn test_search_boolean_options() {
        let json = r#"{
            "indexed": true,
            "stored": true,
            "fieldnorms": false,
            "fast": true
        }"#;
        let config: serde_json::Value = serde_json::from_str(json).unwrap();
        let expected: SearchFieldConfig =
            serde_json::from_value(serde_json::json!({"Boolean": config})).unwrap();
        let int_options: NumericOptions = SearchFieldConfig::default_numeric().into();

        assert_eq!(int_options, expected.into());
    }

    #[rstest]
    fn test_search_jsonobject_options() {
        let json = r#"{
            "indexed": true,
            "fast": false,
            "stored": true,
            "expand_dots": true,
            "type": "default",
            "record": "basic",
            "normalizer": "raw"
        }"#;
        let config: serde_json::Value = serde_json::from_str(json).unwrap();
        let search_json_option: SearchFieldConfig =
            serde_json::from_value(serde_json::json!({"Json": config})).unwrap();
        let expected: JsonObjectOptions = search_json_option.into();

        let json_object_options: JsonObjectOptions = SearchFieldConfig::default_json().into();
        assert_eq!(expected.is_stored(), json_object_options.is_stored());
        assert_eq!(
            expected.get_fast_field_tokenizer_name(),
            json_object_options.get_fast_field_tokenizer_name()
        );
        assert_eq!(
            expected.is_expand_dots_enabled(),
            json_object_options.is_expand_dots_enabled()
        );

        let text_options = json_object_options.set_fast(Some("index"));
        assert_ne!(expected.is_fast(), text_options.is_fast());
    }
}

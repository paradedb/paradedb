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

use pgrx::JsonB;
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use std::str::FromStr;

use crate::{index::state::SearchAlias, query::SearchQueryInput};

#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct SearchConfig {
    pub query: SearchQueryInput,
    pub index_name: String,
    pub index_oid: u32,
    pub key_field: String,
    pub offset_rows: Option<usize>,
    pub limit_rows: Option<usize>,
    pub max_num_chars: Option<usize>,
    pub highlight_field: Option<String>,
    pub prefix: Option<String>,
    pub postfix: Option<String>,
    pub alias: Option<SearchAlias>,
    pub stable_sort: Option<bool>,
    pub uuid: String,
}

impl SearchConfig {
    pub fn from_jsonb(JsonB(config_json_value): JsonB) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config_json_value)
    }
}

impl FromStr for SearchConfig {
    type Err = serde_path_to_error::Error<json5::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut deserializer = json5::Deserializer::from_str(s).expect("input is not valid json");
        serde_path_to_error::deserialize(&mut deserializer)
    }
}
// Helpers to deserialize a comma-separated string, following all the rules
// of csv documents. This let's us easily use syntax like 1,2,3 or one,two,three
// in the SearchQuery input strings.

#[allow(unused)]
fn from_csv<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
{
    deserializer.deserialize_str(CSVVecVisitor::<T>::default())
}

/// Visits a string value of the form "v1,v2,v3" into a vector of bytes Vec<u8>
struct CSVVecVisitor<T: DeserializeOwned + std::str::FromStr>(std::marker::PhantomData<T>);

impl<T: DeserializeOwned + std::str::FromStr> Default for CSVVecVisitor<T> {
    fn default() -> Self {
        CSVVecVisitor(std::marker::PhantomData)
    }
}

impl<'de, T: DeserializeOwned + std::str::FromStr> serde::de::Visitor<'de> for CSVVecVisitor<T>
where
    <T as std::str::FromStr>::Err: std::fmt::Debug, // handle the parse error in a generic way
{
    type Value = Vec<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a str")
    }

    fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        // Treat the comma-separated string as a single record in a CSV.
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(s.as_bytes());

        // Try to get the record and collect its values into a vector.
        let mut output = Vec::new();
        for result in rdr.records() {
            match result {
                Ok(record) => {
                    for field in record.iter() {
                        output.push(
                            field
                                .parse::<T>()
                                .map_err(|_| E::custom("Failed to parse field"))?,
                        );
                    }
                }
                Err(e) => {
                    return Err(E::custom(format!(
                        "could not deserialize sequence value: {:?}",
                        e
                    )));
                }
            }
        }

        Ok(output)
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

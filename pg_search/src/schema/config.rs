// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::{Display, Formatter};
use tantivy::schema::{
    DateOptions, DateTimePrecision, IpAddrOptions, JsonObjectOptions, NumericOptions,
    TextFieldIndexing, TextOptions,
};
use tokenizers::{SearchNormalizer, SearchTokenizer};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
// TODO: re-enable this once we are okay with a breaking change
// #[serde(deny_unknown_fields)]
pub enum SearchFieldConfig {
    Text {
        #[serde(default = "default_as_true")]
        indexed: bool,
        #[serde(default)]
        fast: bool,
        #[serde(default = "default_as_true")]
        fieldnorms: bool,
        #[serde(default)]
        tokenizer: SearchTokenizer,
        #[serde(default = "default_as_freqs_and_positions")]
        record: IndexRecordOption,
        #[serde(default)]
        normalizer: SearchNormalizer,
        #[serde(default)]
        column: Option<String>,
    },
    Inet {
        #[serde(default = "default_as_true")]
        indexed: bool,
        #[serde(default = "default_as_true")]
        fast: bool,
    },
    Json {
        #[serde(default = "default_as_true")]
        indexed: bool,
        #[serde(default)]
        fast: bool,
        #[serde(default = "default_as_true")]
        fieldnorms: bool,
        #[serde(default = "default_as_true")]
        expand_dots: bool,
        #[serde(default)]
        tokenizer: SearchTokenizer,
        #[serde(default = "default_as_freqs_and_positions")]
        record: IndexRecordOption,
        #[serde(default)]
        normalizer: SearchNormalizer,
        #[serde(default)]
        column: Option<String>,
    },
    Range {
        #[serde(default = "default_as_true")]
        fast: bool,
    },
    Numeric {
        #[serde(default = "default_as_true")]
        indexed: bool,
        #[serde(default = "default_as_true")]
        fast: bool,
    },
    Boolean {
        #[serde(default = "default_as_true")]
        indexed: bool,
        #[serde(default = "default_as_true")]
        fast: bool,
    },
    Date {
        #[serde(default = "default_as_true")]
        indexed: bool,
        #[serde(default = "default_as_true")]
        fast: bool,
    },
}

impl SearchFieldConfig {
    pub fn text_from_json(value: serde_json::Value) -> Result<Self> {
        let mut config: Self = serde_json::from_value(json!({
            "Text": value
        }))?;

        match config {
            SearchFieldConfig::Text {
                ref tokenizer,
                ref mut fast,
                ..
            } => {
                if matches!(tokenizer, SearchTokenizer::Keyword) {
                    *fast = true;
                }
                Ok(config)
            }
            _ => Err(anyhow::anyhow!("Expected Text configuration")),
        }
    }

    pub fn inet_from_json(value: serde_json::Value) -> Result<Self> {
        let config: Self = serde_json::from_value(json!({
            "Inet": value
        }))?;

        match config {
            SearchFieldConfig::Inet { .. } => Ok(config),
            _ => Err(anyhow::anyhow!("Expected Inet configuration")),
        }
    }

    pub fn json_from_json(value: serde_json::Value) -> Result<Self> {
        let config: Self = serde_json::from_value(json!({
            "Json": value
        }))?;

        match config {
            SearchFieldConfig::Json { .. } => Ok(config),
            _ => Err(anyhow::anyhow!("Expected Json configuration")),
        }
    }

    pub fn range_from_json(value: serde_json::Value) -> Result<Self> {
        let config: Self = serde_json::from_value(json!({
            "Range": value
        }))?;

        match config {
            SearchFieldConfig::Range { .. } => Ok(config),
            _ => Err(anyhow::anyhow!("Expected Range configuration")),
        }
    }

    pub fn numeric_from_json(value: serde_json::Value) -> Result<Self> {
        let config: Self = serde_json::from_value(json!({
            "Numeric": value
        }))?;

        match config {
            SearchFieldConfig::Numeric { .. } => Ok(config),
            _ => Err(anyhow::anyhow!("Expected Numeric configuration")),
        }
    }

    pub fn boolean_from_json(value: serde_json::Value) -> Result<Self> {
        let config: Self = serde_json::from_value(json!({
            "Boolean": value
        }))?;

        match config {
            SearchFieldConfig::Boolean { .. } => Ok(config),
            _ => Err(anyhow::anyhow!("Expected Boolean configuration")),
        }
    }

    pub fn date_from_json(value: serde_json::Value) -> Result<Self> {
        let config: Self = serde_json::from_value(json!({
            "Date": value
        }))?;

        match config {
            SearchFieldConfig::Date { .. } => Ok(config),
            _ => Err(anyhow::anyhow!("Expected Date configuration")),
        }
    }

    pub fn alias(&self) -> Option<&str> {
        match self {
            Self::Text { column, .. } | Self::Json { column, .. } => column.as_deref(),
            _ => None,
        }
    }

    pub fn tokenizer(&self) -> Option<&SearchTokenizer> {
        match self {
            Self::Text { tokenizer, .. } | Self::Json { tokenizer, .. } => Some(tokenizer),
            _ => None,
        }
    }
}

impl SearchFieldConfig {
    pub fn from_json(value: serde_json::Value) -> Self {
        serde_json::from_value(value)
            .expect("value should be a valid SearchFieldConfig representation")
    }

    pub fn default_text() -> Self {
        Self::from_json(json!({"Text": {}}))
    }

    pub fn default_uuid() -> Self {
        let mut config = Self::from_json(json!({"Text": {}}));
        if let SearchFieldConfig::Text {
            ref mut tokenizer,
            ref mut fast,
            ..
        } = config
        {
            *tokenizer = SearchTokenizer::Keyword;
            *fast = true;
        }
        config
    }

    pub fn default_inet() -> Self {
        Self::from_json(json!({"Inet": {}}))
    }

    pub fn default_numeric() -> Self {
        Self::from_json(json!({"Numeric": {}}))
    }

    pub fn default_boolean() -> Self {
        Self::from_json(json!({"Boolean": {}}))
    }

    pub fn default_json() -> Self {
        Self::from_json(json!({"Json": {}}))
    }

    pub fn default_date() -> Self {
        Self::from_json(json!({"Date": {}}))
    }

    pub fn default_range() -> Self {
        Self::from_json(json!({"Json": {"fast": true}}))
    }
}

impl From<SearchFieldConfig> for TextOptions {
    fn from(config: SearchFieldConfig) -> Self {
        let mut text_options = TextOptions::default();
        match config {
            SearchFieldConfig::Text {
                indexed,
                fast,
                fieldnorms,
                tokenizer,
                record,
                normalizer,
                ..
            } => {
                if fast {
                    text_options = text_options.set_fast(Some(normalizer.name()));
                }
                if indexed {
                    let text_field_indexing = TextFieldIndexing::default()
                        .set_index_option(record.into())
                        .set_fieldnorms(fieldnorms)
                        .set_tokenizer(&tokenizer.name());

                    text_options = text_options.set_indexing_options(text_field_indexing);
                }
            }
            _ => panic!("attempted to convert non-text search field config to tantivy text config"),
        }
        text_options
    }
}

impl From<SearchFieldConfig> for IpAddrOptions {
    fn from(config: SearchFieldConfig) -> Self {
        let mut inet_options = IpAddrOptions::default();
        match config {
            SearchFieldConfig::Inet { indexed, fast, .. } => {
                if fast {
                    inet_options = inet_options.set_fast();
                }
                if indexed {
                    inet_options = inet_options.set_indexed();
                }
            }
            _ => {
                panic!(
                    "attempted to convert non-numeric search field config to tantivy ip addr config"
                )
            }
        }
        inet_options
    }
}

impl From<SearchFieldConfig> for NumericOptions {
    fn from(config: SearchFieldConfig) -> Self {
        let mut numeric_options = NumericOptions::default();
        match config {
            SearchFieldConfig::Numeric {
                indexed,
                fast,
                ..
            }
            // Following the example of Quickwit, which uses NumericOptions for boolean options.
            | SearchFieldConfig::Boolean { indexed, fast, .. } => {
                if fast {
                    numeric_options = numeric_options.set_fast();
                }
                if indexed {
                    numeric_options = numeric_options.set_indexed();
                }
            }
            _ => {
                panic!(
                    "attempted to convert non-numeric search field config to tantivy numeric config"
                )
            }
        }
        numeric_options
    }
}

impl From<SearchFieldConfig> for JsonObjectOptions {
    fn from(config: SearchFieldConfig) -> Self {
        let mut json_options = JsonObjectOptions::default();
        match config {
            SearchFieldConfig::Json {
                indexed,
                fast,
                fieldnorms,
                expand_dots,
                tokenizer,
                record,
                normalizer,
                ..
            } => {
                if fast {
                    json_options = json_options.set_fast(Some(normalizer.name()));
                }
                if expand_dots {
                    json_options = json_options.set_expand_dots_enabled();
                }
                if indexed {
                    let text_field_indexing = TextFieldIndexing::default()
                        .set_index_option(record.into())
                        .set_fieldnorms(fieldnorms)
                        .set_tokenizer(&tokenizer.name());

                    json_options = json_options.set_indexing_options(text_field_indexing);
                }
            }
            SearchFieldConfig::Range { .. } => {
                // Range must be indexed and fast to be searchable
                let text_field_indexing = TextFieldIndexing::default();
                json_options = json_options.set_indexing_options(text_field_indexing);
                json_options = json_options.set_fast(Some("raw"));
            }
            _ => {
                panic!("attempted to convert non-json search field config to tantivy json config")
            }
        }

        json_options
    }
}

impl From<SearchFieldConfig> for DateOptions {
    fn from(config: SearchFieldConfig) -> Self {
        let mut date_options = DateOptions::default();
        match config {
            SearchFieldConfig::Date { indexed, fast, .. } => {
                if fast {
                    date_options = date_options
                        .set_fast()
                        // Match Postgres' maximum allowed precision of microseconds
                        .set_precision(DateTimePrecision::Microseconds);
                }
                if indexed {
                    date_options = date_options.set_indexed();
                }
            }
            _ => {
                panic!("attempted to convert non-date search field config to tantivy date config")
            }
        }
        date_options
    }
}

#[allow(unused)] // used by serde
pub enum IndexRecordOptionSchema {
    Basic,
    WithFreqs,
    WithFreqsAndPositions,
}

#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct IndexRecordOption(tantivy::schema::IndexRecordOption);

#[allow(non_upper_case_globals)]
impl IndexRecordOption {
    pub const Basic: IndexRecordOption =
        IndexRecordOption(tantivy::schema::IndexRecordOption::Basic);
    pub const WithFreqs: IndexRecordOption =
        IndexRecordOption(tantivy::schema::IndexRecordOption::WithFreqs);
    pub const WithFreqsAndPositions: IndexRecordOption =
        IndexRecordOption(tantivy::schema::IndexRecordOption::WithFreqsAndPositions);
}

impl From<tantivy::schema::IndexRecordOption> for IndexRecordOption {
    #[inline]
    fn from(value: tantivy::schema::IndexRecordOption) -> Self {
        Self(value)
    }
}

impl From<IndexRecordOption> for tantivy::schema::IndexRecordOption {
    fn from(value: IndexRecordOption) -> Self {
        value.0
    }
}

impl Display for IndexRecordOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            tantivy::schema::IndexRecordOption::Basic => write!(f, "basic"),
            tantivy::schema::IndexRecordOption::WithFreqs => write!(f, "freq"),
            tantivy::schema::IndexRecordOption::WithFreqsAndPositions => write!(f, "position"),
        }
    }
}

fn default_as_true() -> bool {
    true
}

fn default_as_freqs_and_positions() -> IndexRecordOption {
    IndexRecordOption(tantivy::schema::IndexRecordOption::WithFreqsAndPositions)
}

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::{Display, Formatter};
use tantivy::schema::{
    DateOptions, DateTimePrecision, JsonObjectOptions, NumericOptions, TextFieldIndexing,
    TextOptions,
};
use tokenizers::manager::SearchTokenizerFilters;
use tokenizers::{SearchNormalizer, SearchTokenizer};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
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
        let config: Self = serde_json::from_value(json!({
            "Text": value
        }))?;

        match config {
            SearchFieldConfig::Text { .. } => Ok(config),
            _ => Err(anyhow::anyhow!("Expected Text configuration")),
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

    pub fn column(&self) -> Option<&str> {
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
        SearchFieldConfig::Text {
            indexed: true,
            fast: true,
            fieldnorms: false,

            // NB:  This should use the `SearchTokenizer::Keyword` tokenizer but for historical
            // reasons it uses the `SearchTokenizer::Raw` tokenizer but with the same filters
            // configuration as the `SearchTokenizer::Keyword` tokenizer.
            #[allow(deprecated)]
            tokenizer: SearchTokenizer::Raw(SearchTokenizerFilters::keyword().clone()),
            record: IndexRecordOption::Basic,
            normalizer: SearchNormalizer::Raw,
            column: None,
        }
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
        Self::from_json(json!({"Range": {}}))
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

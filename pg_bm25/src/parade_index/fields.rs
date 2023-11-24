use std::collections::HashMap;

use serde::*;
use tantivy::schema::*;

// Tokenizers
// Serde will pick a ParadeTokenizer variant based on the value of the
// "type" key, which needs to match one of the variant names below.
// The "type" field will not be present on the deserialized value.
#[derive(Default, Copy, Clone, Deserialize, Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ParadeTokenizer {
    #[serde(rename = "default")]
    #[default]
    Default,
    #[serde(rename = "raw")]
    Raw,
    #[serde(rename = "en_stem")]
    EnStem,
    #[serde(rename = "whitespace")]
    WhiteSpace,
    #[serde(rename = "chinese_compatible")]
    ChineseCompatible,
    #[serde(rename = "source_code")]
    SourceCode,
    #[serde(rename = "ngram")]
    Ngram {
        min_gram: usize,
        max_gram: usize,
        prefix_only: bool,
    },
    #[serde(rename = "lindera")]
    Lindera,
}

impl ParadeTokenizer {
    pub fn name(&self) -> String {
        match self {
            ParadeTokenizer::Default => "default".into(),
            ParadeTokenizer::Raw => "raw".into(),
            ParadeTokenizer::EnStem => "en_stem".into(),
            ParadeTokenizer::WhiteSpace => "whitespace".into(),
            ParadeTokenizer::ChineseCompatible => "chinese_compatible".into(),
            ParadeTokenizer::SourceCode => "source_code".into(),
            ParadeTokenizer::Ngram {
                min_gram,
                max_gram,
                prefix_only,
            } => format!("ngram_mingram:{min_gram}_maxgram:{max_gram}_prefixonly:{prefix_only}"),
            ParadeTokenizer::Lindera => "lindera".into(),
        }
    }
}

// Normalizers for fast fields
#[derive(Default, Copy, Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub enum ParadeNormalizer {
    #[serde(rename = "raw")]
    #[default]
    Raw,
    #[serde(rename = "lowercase")]
    Lowercase,
}

impl ParadeNormalizer {
    pub fn name(&self) -> &str {
        match self {
            ParadeNormalizer::Raw => "raw",
            ParadeNormalizer::Lowercase => "lowercase",
        }
    }
}

// Index record schema
#[allow(unused)]
#[derive(utoipa::ToSchema)]
pub enum IndexRecordOptionSchema {
    #[schema(rename = "basic")]
    Basic,
    #[schema(rename = "freq")]
    WithFreqs,
    #[schema(rename = "position")]
    WithFreqsAndPositions,
}

pub trait ToString {
    fn to_string(&self) -> String;
}

impl ToString for IndexRecordOption {
    fn to_string(&self) -> String {
        match self {
            IndexRecordOption::Basic => "basic".to_string(),
            IndexRecordOption::WithFreqs => "freq".to_string(),
            IndexRecordOption::WithFreqsAndPositions => "position".to_string(),
        }
    }
}

// Text options
#[derive(Copy, Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ParadeTextOptions {
    #[serde(default = "default_as_true")]
    indexed: bool,
    #[serde(default)]
    fast: bool,
    #[serde(default = "default_as_true")]
    stored: bool,
    #[serde(default = "default_as_true")]
    fieldnorms: bool,
    #[serde(default)]
    pub tokenizer: ParadeTokenizer,
    #[schema(value_type = IndexRecordOptionSchema)]
    #[serde(default = "default_as_freqs_and_positions")]
    record: IndexRecordOption,
    #[serde(default)]
    normalizer: ParadeNormalizer,
}

impl Default for ParadeTextOptions {
    fn default() -> Self {
        Self {
            indexed: true,
            fast: false,
            stored: true,
            fieldnorms: true,
            tokenizer: ParadeTokenizer::Default,
            record: IndexRecordOption::Basic,
            normalizer: ParadeNormalizer::Raw,
        }
    }
}

impl From<ParadeTextOptions> for TextOptions {
    fn from(parade_options: ParadeTextOptions) -> Self {
        let mut text_options = TextOptions::default();

        if parade_options.stored {
            text_options = text_options.set_stored();
        }
        if parade_options.fast {
            text_options = text_options.set_fast(Some(parade_options.normalizer.name()));
        }
        if parade_options.indexed {
            let text_field_indexing = TextFieldIndexing::default()
                .set_index_option(parade_options.record)
                .set_fieldnorms(parade_options.fieldnorms)
                .set_tokenizer(&parade_options.tokenizer.name());

            text_options = text_options.set_indexing_options(text_field_indexing);
        }

        text_options
    }
}

// Numeric options
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct ParadeNumericOptions {
    #[serde(default = "default_as_true")]
    indexed: bool,
    #[serde(default = "default_as_true")]
    fast: bool,
    #[serde(default = "default_as_true")]
    stored: bool,
}

impl Default for ParadeNumericOptions {
    fn default() -> Self {
        Self {
            indexed: true,
            fast: true,
            stored: true,
        }
    }
}

impl From<ParadeNumericOptions> for NumericOptions {
    fn from(parade_options: ParadeNumericOptions) -> Self {
        let mut numeric_options = NumericOptions::default();

        if parade_options.stored {
            numeric_options = numeric_options.set_stored();
        }
        if parade_options.fast {
            numeric_options = numeric_options.set_fast();
        }
        if parade_options.indexed {
            numeric_options = numeric_options.set_indexed();
        }

        numeric_options
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct ParadeBooleanOptions {
    #[serde(default = "default_as_true")]
    indexed: bool,
    #[serde(default = "default_as_true")]
    fast: bool,
    #[serde(default = "default_as_true")]
    stored: bool,
}

impl Default for ParadeBooleanOptions {
    fn default() -> Self {
        Self {
            indexed: true,
            fast: true,
            stored: true,
        }
    }
}

// Following the example of Quickwit, which uses NumericOptions for boolean options
impl From<ParadeBooleanOptions> for NumericOptions {
    fn from(parade_options: ParadeBooleanOptions) -> Self {
        let mut boolean_options = NumericOptions::default();

        if parade_options.stored {
            boolean_options = boolean_options.set_stored();
        }
        if parade_options.fast {
            boolean_options = boolean_options.set_fast();
        }
        if parade_options.indexed {
            boolean_options = boolean_options.set_indexed();
        }

        boolean_options
    }
}

// Json options
#[derive(Copy, Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ParadeJsonOptions {
    #[serde(default = "default_as_true")]
    indexed: bool,
    #[serde(default)]
    fast: bool,
    #[serde(default = "default_as_true")]
    stored: bool,
    #[serde(default = "default_as_true")]
    expand_dots: bool,
    #[serde(default)]
    pub tokenizer: ParadeTokenizer,
    #[schema(value_type = IndexRecordOptionSchema)]
    #[serde(default = "default_as_freqs_and_positions")]
    record: IndexRecordOption,
    #[serde(default)]
    normalizer: ParadeNormalizer,
}

impl Default for ParadeJsonOptions {
    fn default() -> Self {
        Self {
            indexed: true,
            fast: false,
            stored: true,
            expand_dots: true,
            tokenizer: ParadeTokenizer::Default,
            record: IndexRecordOption::Basic,
            normalizer: ParadeNormalizer::Raw,
        }
    }
}

impl From<ParadeJsonOptions> for JsonObjectOptions {
    fn from(parade_options: ParadeJsonOptions) -> Self {
        let mut json_options = JsonObjectOptions::default();

        if parade_options.stored {
            json_options = json_options.set_stored();
        }
        if parade_options.fast {
            json_options = json_options.set_fast(Some(parade_options.normalizer.name()));
        }
        if parade_options.expand_dots {
            json_options = json_options.set_expand_dots_enabled();
        }
        if parade_options.indexed {
            let text_field_indexing = TextFieldIndexing::default()
                .set_index_option(parade_options.record)
                .set_tokenizer(&parade_options.tokenizer.name());

            json_options = json_options.set_indexing_options(text_field_indexing);
        }

        json_options
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum ParadeOption {
    Text(ParadeTextOptions),
    Json(ParadeJsonOptions),
    Numeric(ParadeNumericOptions),
    Boolean(ParadeBooleanOptions),
}

pub type ParadeOptionMap = HashMap<String, ParadeOption>;
pub type ParadeFieldConfigSerialized = String;
pub type ParadeFieldConfigSerializedResult = serde_json::Result<ParadeFieldConfigSerialized>;
pub type ParadeFieldConfigDeserializedResult = serde_json::Result<ParadeOptionMap>;

// TODO: Enable DateTime and IP fields

fn default_as_true() -> bool {
    true
}

fn default_as_freqs_and_positions() -> IndexRecordOption {
    IndexRecordOption::WithFreqsAndPositions
}

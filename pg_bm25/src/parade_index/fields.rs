use serde::*;
use tantivy::schema::*;

// Tokenizers
#[derive(Default, Copy, Clone, Deserialize, Debug, Serialize, PartialEq, Eq)]
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
}

impl ParadeTokenizer {
    pub fn name(&self) -> &str {
        match self {
            ParadeTokenizer::Default => "default",
            ParadeTokenizer::Raw => "raw",
            ParadeTokenizer::EnStem => "en_stem",
            ParadeTokenizer::WhiteSpace => "whitespace",
            ParadeTokenizer::ChineseCompatible => "chinese_compatible",
            ParadeTokenizer::SourceCode => "source_code",
            ParadeTokenizer::Ngram {
                min_gram,
                max_gram,
                prefix_only,
            } => "ngram_mingram:{min_gram}_maxgram:{max_gram}_prefixonly:{prefix_only}",
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
#[derive(Copy, Clone, Debug, Deserialize, utoipa::ToSchema)]
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
    tokenizer: ParadeTokenizer,
    #[schema(value_type = IndexRecordOptionSchema)]
    #[serde(default)]
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
                .set_tokenizer(parade_options.tokenizer.name());

            text_options = text_options.set_indexing_options(text_field_indexing);
        }

        text_options
    }
}

// Numeric options
#[derive(Copy, Clone, Debug, Deserialize)]
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

#[derive(Copy, Clone, Debug, Deserialize)]
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
    #[serde(default)]
    record: IndexRecordOption,
    #[serde(default)]
    normalizer: ParadeNormalizer,
    #[serde(default)]
    ngram_prefix_only: bool,
    #[serde(default)]
    ngram_min: usize,
    #[serde(default)]
    ngram_max: usize,
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
            ngram_prefix_only: false,
            ngram_min: 0,
            ngram_max: 0,
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
                .set_tokenizer(parade_options.tokenizer.name());

            json_options = json_options.set_indexing_options(text_field_indexing);
        }

        json_options
    }
}

// TODO: Enable DateTime and IP fields

fn default_as_true() -> bool {
    true
}

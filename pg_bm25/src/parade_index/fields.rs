use serde::*;
use tantivy::schema::*;

// Tokenizers
// TODO: Custom tokenizers like CJK and ngrams
#[derive(Copy, Clone, Deserialize, Debug, PartialEq, Eq)]
pub enum ParadeTokenizer {
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "raw")]
    Raw,
    #[serde(rename = "en_stem")]
    EnStem,
    #[serde(rename = "whitespace")]
    WhiteSpace,
}

impl ParadeTokenizer {
    pub fn name(&self) -> &str {
        match self {
            ParadeTokenizer::Default => "default",
            ParadeTokenizer::Raw => "raw",
            ParadeTokenizer::EnStem => "en_stem",
            ParadeTokenizer::WhiteSpace => "whitespace",
        }
    }
}

impl Default for ParadeTokenizer {
    fn default() -> Self {
        ParadeTokenizer::Default
    }
}

// Normalizers for fast fields
#[derive(Copy, Clone, Deserialize, Debug, PartialEq, Eq)]
pub enum ParadeNormalizer {
    #[serde(rename = "raw")]
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

impl Default for ParadeNormalizer {
    fn default() -> Self {
        ParadeNormalizer::Raw
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
    #[schema(rename = "freqandposition")]
    WithFreqsAndPositions,
}

// Text options
#[derive(Copy, Clone, Debug, Deserialize, utoipa::ToSchema)]
pub struct ParadeTextOptions {
    #[serde(default)]
    indexed: bool,
    #[serde(default)]
    fast: bool,
    #[serde(default)]
    stored: bool,
    #[serde(default)]
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
    #[serde(default)]
    indexed: bool,
    #[serde(default)]
    fast: bool,
    #[serde(default)]
    stored: bool,
    #[serde(default)]
    coerce: bool
}

impl Default for ParadeNumericOptions {
    fn default() -> Self {
        Self {
            indexed: true,
            fast: true,
            stored: true,
            coerce: true,
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
        if parade_options.coerce {
            numeric_options = numeric_options.set_coerce();
        }

        numeric_options
    }
}

// TODO: Boolean, JSON options

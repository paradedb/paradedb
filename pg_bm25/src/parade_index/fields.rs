use crate::tokenizers::code::CodeTokenizer;
use crate::tokenizers::icu::ICUTokenizer;
use crate::tokenizers::lindera::{LinderaJapaneseTokenizer, LinderaKoreanTokenizer};
use crate::tokenizers::{cjk::ChineseTokenizer, lindera::LinderaChineseTokenizer};
use serde::*;
use std::collections::HashMap;
use tantivy::{
    schema::*,
    tokenizer::{
        AsciiFoldingFilter, Language, LowerCaser, NgramTokenizer, RawTokenizer, RemoveLongFilter,
        SimpleTokenizer, Stemmer, TextAnalyzer, WhitespaceTokenizer,
    },
};

pub const DEFAULT_REMOVE_TOKEN_LENGTH: usize = 255;
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
    #[serde(rename = "chinese_lindera")]
    ChineseLindera,
    #[serde(rename = "japanese_lindera")]
    JapaneseLindera,
    #[serde(rename = "korean_lindera")]
    KoreanLindera,
    #[serde(rename = "icu")]
    ICUTokenizer,
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
            ParadeTokenizer::ChineseLindera => "chinese_lindera".into(),
            ParadeTokenizer::JapaneseLindera => "japanese_lindera".into(),
            ParadeTokenizer::KoreanLindera => "korean_lindera".into(),
            ParadeTokenizer::ICUTokenizer => "icu".into(),
        }
    }
}

impl From<ParadeTokenizer> for TextAnalyzer {
    fn from(val: ParadeTokenizer) -> Self {
        match val {
            ParadeTokenizer::Default => TextAnalyzer::builder(SimpleTokenizer::default())
                .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                .filter(LowerCaser)
                .build(),
            ParadeTokenizer::WhiteSpace => TextAnalyzer::builder(WhitespaceTokenizer::default())
                .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                .filter(LowerCaser)
                .build(),
            ParadeTokenizer::EnStem => TextAnalyzer::builder(SimpleTokenizer::default())
                .filter(RemoveLongFilter::limit(40))
                .filter(LowerCaser)
                .filter(Stemmer::new(Language::English))
                .build(),
            ParadeTokenizer::Raw => TextAnalyzer::builder(RawTokenizer::default())
                .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                .build(),
            ParadeTokenizer::ChineseCompatible => TextAnalyzer::builder(ChineseTokenizer)
                .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                .filter(LowerCaser)
                .build(),
            ParadeTokenizer::SourceCode => TextAnalyzer::builder(CodeTokenizer::default())
                .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                .filter(LowerCaser)
                .filter(AsciiFoldingFilter)
                .build(),
            ParadeTokenizer::Ngram {
                min_gram,
                max_gram,
                prefix_only,
            } => {
                TextAnalyzer::builder(NgramTokenizer::new(min_gram, max_gram, prefix_only).unwrap())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build()
            }
            ParadeTokenizer::ChineseLindera => {
                TextAnalyzer::builder(LinderaChineseTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build()
            }
            ParadeTokenizer::JapaneseLindera => {
                TextAnalyzer::builder(LinderaJapaneseTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build()
            }
            ParadeTokenizer::KoreanLindera => {
                TextAnalyzer::builder(LinderaKoreanTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build()
            }
            ParadeTokenizer::ICUTokenizer => TextAnalyzer::builder(ICUTokenizer)
                .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                .filter(LowerCaser)
                .build(),
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
            fast: false,
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
            fast: false,
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

// TODO: Enable DateTime and IP fields

fn default_as_true() -> bool {
    true
}

fn default_as_freqs_and_positions() -> IndexRecordOption {
    IndexRecordOption::WithFreqsAndPositions
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {

    use tantivy::schema::{JsonObjectOptions, NumericOptions, TextOptions};

    use super::{
        default_as_true, ParadeBooleanOptions, ParadeJsonOptions, ParadeNormalizer,
        ParadeNumericOptions, ParadeTextOptions, ParadeTokenizer,
    };

    #[pgrx::pg_test]
    fn test_parade_tokenizer() {
        let tokenizer = ParadeTokenizer::Default;
        assert_eq!(tokenizer.name(), "default".to_string());

        let tokenizer = ParadeTokenizer::EnStem;
        assert_eq!(tokenizer.name(), "en_stem".to_string());

        let json = r#"{
            "type": "ngram",
            "min_gram": 20,
            "max_gram": 60,
            "prefix_only": true
        }"#;
        let tokenizer: ParadeTokenizer = serde_json::from_str(json).unwrap();
        assert_eq!(
            tokenizer,
            ParadeTokenizer::Ngram {
                min_gram: 20,
                max_gram: 60,
                prefix_only: true
            }
        );
    }

    #[pgrx::pg_test]
    fn test_parade_normalizer() {
        assert_eq!(ParadeNormalizer::Lowercase.name(), "lowercase");
        assert_ne!(ParadeNormalizer::Raw, ParadeNormalizer::Lowercase);
    }

    #[pgrx::pg_test]
    fn test_parade_text_options() {
        let json = r#"{
            "indexed": true,
            "fast": false,
            "stored": true,
            "fieldnorms": true,
            "type": "default",
            "record": "basic",
            "normalizer": "raw"
        }"#;
        let parade_text_option: ParadeTextOptions = serde_json::from_str(json).unwrap();
        let expected = TextOptions::from(parade_text_option);

        let text_options = TextOptions::from(ParadeTextOptions::default());
        assert_eq!(expected.is_stored(), text_options.is_stored());
        assert_eq!(
            expected.get_fast_field_tokenizer_name(),
            text_options.get_fast_field_tokenizer_name()
        );

        let text_options = text_options.set_fast(Some("index"));
        assert_ne!(expected.is_fast(), text_options.is_fast());
    }

    #[pgrx::pg_test]
    fn test_parade_numeric_options() {
        let json = r#"{
            "indexed": true,
            "stored": true,
            "fieldnorms": false,
            "fast": false
        }"#;
        let expected: NumericOptions = serde_json::from_str(json).unwrap();
        let int_options = NumericOptions::from(ParadeNumericOptions::default());

        assert_eq!(int_options, expected);
    }

    #[pgrx::pg_test]
    fn test_parade_boolean_options() {
        let json = r#"{
            "indexed": true,
            "stored": true,
            "fieldnorms": false,
            "fast": false
        }"#;
        let expected: NumericOptions = serde_json::from_str(json).unwrap();
        let int_options = NumericOptions::from(ParadeBooleanOptions::default());

        assert_eq!(int_options, expected);
    }

    #[pgrx::pg_test]
    fn test_parade_jsonobject_options() {
        let json = r#"{
            "indexed": true,
            "fast": false,
            "stored": true,
            "expand_dots": true,
            "type": "default",
            "record": "basic",
            "normalizer": "raw"
        }"#;
        let parade_json_option: ParadeJsonOptions = serde_json::from_str(json).unwrap();
        let expected = JsonObjectOptions::from(parade_json_option);

        let json_object_options = JsonObjectOptions::from(ParadeJsonOptions::default());
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

    #[pgrx::pg_test]
    fn test_default_as_true() {
        assert!(default_as_true())
    }
}

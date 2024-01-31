use serde::*;
use tantivy::tokenizer::{
    AsciiFoldingFilter, Language, LowerCaser, NgramTokenizer, RawTokenizer, RemoveLongFilter,
    SimpleTokenizer, Stemmer, TextAnalyzer, WhitespaceTokenizer,
};

use crate::tokenizers::code::CodeTokenizer;
#[cfg(feature = "icu")]
use crate::tokenizers::icu::ICUTokenizer;
use crate::tokenizers::lindera::{LinderaJapaneseTokenizer, LinderaKoreanTokenizer};
use crate::tokenizers::{cjk::ChineseTokenizer, lindera::LinderaChineseTokenizer};

pub const DEFAULT_REMOVE_TOKEN_LENGTH: usize = 255;
// Tokenizers
// Serde will pick a SearchTokenizer variant based on the value of the
// "type" key, which needs to match one of the variant names below.
// The "type" field will not be present on the deserialized value.
#[derive(Default, Copy, Clone, Deserialize, Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum SearchTokenizer {
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
    #[cfg(feature = "icu")]
    #[serde(rename = "icu")]
    ICUTokenizer,
}

impl SearchTokenizer {
    pub fn name(&self) -> String {
        match self {
            SearchTokenizer::Default => "default".into(),
            SearchTokenizer::Raw => "raw".into(),
            SearchTokenizer::EnStem => "en_stem".into(),
            SearchTokenizer::WhiteSpace => "whitespace".into(),
            SearchTokenizer::ChineseCompatible => "chinese_compatible".into(),
            SearchTokenizer::SourceCode => "source_code".into(),
            SearchTokenizer::Ngram {
                min_gram,
                max_gram,
                prefix_only,
            } => format!("ngram_mingram:{min_gram}_maxgram:{max_gram}_prefixonly:{prefix_only}"),
            SearchTokenizer::ChineseLindera => "chinese_lindera".into(),
            SearchTokenizer::JapaneseLindera => "japanese_lindera".into(),
            SearchTokenizer::KoreanLindera => "korean_lindera".into(),
            #[cfg(feature = "icu")]
            SearchTokenizer::ICUTokenizer => "icu".into(),
        }
    }
}

impl From<SearchTokenizer> for TextAnalyzer {
    fn from(val: SearchTokenizer) -> Self {
        match val {
            SearchTokenizer::Default => TextAnalyzer::builder(SimpleTokenizer::default())
                .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                .filter(LowerCaser)
                .build(),
            SearchTokenizer::WhiteSpace => TextAnalyzer::builder(WhitespaceTokenizer::default())
                .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                .filter(LowerCaser)
                .build(),
            SearchTokenizer::EnStem => TextAnalyzer::builder(SimpleTokenizer::default())
                .filter(RemoveLongFilter::limit(40))
                .filter(LowerCaser)
                .filter(Stemmer::new(Language::English))
                .build(),
            SearchTokenizer::Raw => TextAnalyzer::builder(RawTokenizer::default())
                .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                .build(),
            SearchTokenizer::ChineseCompatible => TextAnalyzer::builder(ChineseTokenizer)
                .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                .filter(LowerCaser)
                .build(),
            SearchTokenizer::SourceCode => TextAnalyzer::builder(CodeTokenizer::default())
                .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                .filter(LowerCaser)
                .filter(AsciiFoldingFilter)
                .build(),
            SearchTokenizer::Ngram {
                min_gram,
                max_gram,
                prefix_only,
            } => {
                TextAnalyzer::builder(NgramTokenizer::new(min_gram, max_gram, prefix_only).unwrap())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build()
            }
            SearchTokenizer::ChineseLindera => {
                TextAnalyzer::builder(LinderaChineseTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build()
            }
            SearchTokenizer::JapaneseLindera => {
                TextAnalyzer::builder(LinderaJapaneseTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build()
            }
            SearchTokenizer::KoreanLindera => {
                TextAnalyzer::builder(LinderaKoreanTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build()
            }
            #[cfg(feature = "icu")]
            ParadeTokenizer::ICUTokenizer => TextAnalyzer::builder(ICUTokenizer)
                .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                .filter(LowerCaser)
                .build(),
        }
    }
}

// Normalizers for fast fields
#[derive(Default, Copy, Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub enum SearchNormalizer {
    #[serde(rename = "raw")]
    #[default]
    Raw,
    #[serde(rename = "lowercase")]
    Lowercase,
}

impl SearchNormalizer {
    pub fn name(&self) -> &str {
        match self {
            SearchNormalizer::Raw => "raw",
            SearchNormalizer::Lowercase => "lowercase",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::schema::SearchFieldConfig;

    use super::*;
    use rstest::*;
    use tantivy::schema::{JsonObjectOptions, NumericOptions, TextOptions};

    #[rstest]
    fn test_search_tokenizer() {
        let tokenizer = SearchTokenizer::Default;
        assert_eq!(tokenizer.name(), "default".to_string());

        let tokenizer = SearchTokenizer::EnStem;
        assert_eq!(tokenizer.name(), "en_stem".to_string());

        let json = r#"{
            "type": "ngram",
            "min_gram": 20,
            "max_gram": 60,
            "prefix_only": true
        }"#;
        let tokenizer: SearchTokenizer = serde_json::from_str(json).unwrap();
        assert_eq!(
            tokenizer,
            SearchTokenizer::Ngram {
                min_gram: 20,
                max_gram: 60,
                prefix_only: true
            }
        );
    }

    #[rstest]
    fn test_search_normalizer() {
        assert_eq!(SearchNormalizer::Lowercase.name(), "lowercase");
        assert_ne!(SearchNormalizer::Raw, SearchNormalizer::Lowercase);
    }

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

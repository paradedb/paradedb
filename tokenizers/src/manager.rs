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
#![allow(deprecated)]

use std::fmt::Write;

#[cfg(feature = "icu")]
use crate::icu::ICUTokenizer;
use crate::{
    cjk::ChineseTokenizer,
    code::CodeTokenizer,
    lindera::{LinderaChineseTokenizer, LinderaJapaneseTokenizer, LinderaKoreanTokenizer},
    token_length::TokenLengthFilter,
    unicode_words::UnicodeWordsTokenizer,
};
use anyhow::Result;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use strum::AsRefStr;
use tantivy::tokenizer::{
    AlphaNumOnlyFilter, AsciiFoldingFilter, Language, LowerCaser, NgramTokenizer, RawTokenizer,
    RegexTokenizer, SimpleTokenizer, Stemmer, StopWordFilter, TextAnalyzer, WhitespaceTokenizer,
};
use tantivy_jieba;

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct SearchTokenizerFilters {
    pub remove_short: Option<usize>,
    pub remove_long: Option<usize>,
    pub lowercase: Option<bool>,
    pub stemmer: Option<Language>,
    pub stopwords_language: Option<Language>,
    pub stopwords: Option<Vec<String>>,
    pub alpha_num_only: Option<bool>,
    pub ascii_folding: Option<bool>,
    pub normalizer: Option<SearchNormalizer>,
}

impl SearchTokenizerFilters {
    /// Returns a [`SearchTokenizerFilter`] instance that effectively does not filter, or otherwise
    /// mutate tokens.
    ///
    /// This should be used for declaring the "key field" in an index.  It can be used for other
    /// text types that don't want tokenization too.
    pub const fn keyword() -> &'static Self {
        &SearchTokenizerFilters {
            remove_short: None,
            remove_long: None,
            lowercase: Some(false),
            stemmer: None,
            stopwords_language: None,
            stopwords: None,
            ascii_folding: None,
            alpha_num_only: None,
            normalizer: Some(SearchNormalizer::Raw),
        }
    }

    pub const fn keyword_deprecated() -> &'static Self {
        &SearchTokenizerFilters {
            remove_short: None,
            remove_long: Some(usize::MAX),
            lowercase: Some(false),
            stemmer: None,
            stopwords_language: None,
            stopwords: None,
            ascii_folding: None,
            alpha_num_only: None,
            normalizer: Some(SearchNormalizer::Raw),
        }
    }

    fn from_json_value(value: &serde_json::Value) -> Result<Self, anyhow::Error> {
        let mut filters = SearchTokenizerFilters::default();

        if let Some(remove_long) = value.get("remove_long") {
            filters.remove_long = Some(remove_long.as_u64().ok_or_else(|| {
                anyhow::anyhow!(
                    "a 'remove_long' value passed to the pg_search tokenizer configuration \
                     must be of type u64, found: {remove_long:#?}"
                )
            })? as usize);
        }
        if let Some(remove_short) = value.get("remove_short") {
            filters.remove_short = Some(remove_short.as_u64().ok_or_else(|| {
                anyhow::anyhow!(
                    "a 'remove_short' value passed to the pg_search tokenizer configuration \
                     must be of type u64, found: {remove_short:#?}"
                )
            })? as usize);
        }
        if let Some(lowercase) = value.get("lowercase") {
            filters.lowercase = Some(lowercase.as_bool().ok_or_else(|| {
                anyhow::anyhow!(
                    "a 'lowercase' value passed to the pg_search tokenizer configuration \
                     must be of type bool, found: {lowercase:#?}"
                )
            })?);
        };
        if let Some(stemmer) = value.get("stemmer") {
            filters.stemmer = Some(serde_json::from_value(stemmer.clone()).map_err(|_| {
                anyhow::anyhow!("stemmer tokenizer requires a valid 'stemmer' field")
            })?);
        }
        if let Some(stopwords_language) = value.get("stopwords_language") {
            filters.stopwords_language = Some(
                serde_json::from_value(stopwords_language.clone()).map_err(|e| {
                    anyhow::anyhow!(
                        "stopwords_language tokenizer requires a valid 'stopwords_language' field: {e}"
                    )
                })?,
            );
        }
        if let Some(stopwords) = value.get("stopwords") {
            filters.stopwords = Some(serde_json::from_value(stopwords.clone()).map_err(|_| {
                anyhow::anyhow!("stopwords tokenizer requires a valid 'stopwords' field")
            })?);
        }
        if let Some(alpha_num_only) = value.get("alpha_num_only") {
            filters.alpha_num_only = Some(alpha_num_only.as_bool().ok_or_else(|| {
                anyhow::anyhow!("ascii_folding tokenizer requires a valid 'alpha_num_only' field")
            })?);
        }
        if let Some(ascii_folding) = value.get("ascii_folding") {
            filters.ascii_folding = Some(ascii_folding.as_bool().ok_or_else(|| {
                anyhow::anyhow!("ascii_folding tokenizer requires a valid 'ascii_folding' field")
            })?);
        }

        Ok(filters)
    }

    fn name_suffix(&self) -> String {
        let mut buffer = String::new();
        let mut is_empty = true;

        fn sep(is_empty: bool) -> &'static str {
            if is_empty {
                ""
            } else {
                ","
            }
        }

        if let Some(value) = self.remove_short {
            write!(buffer, "{}remove_short={value}", sep(is_empty))
                .expect("Writing to String buffer should never fail");
            is_empty = false;
        }
        if let Some(value) = self.remove_long {
            write!(buffer, "{}remove_long={value}", sep(is_empty))
                .expect("Writing to String buffer should never fail");
            is_empty = false;
        }
        if let Some(value) = self.lowercase {
            write!(buffer, "{}lowercase={value}", sep(is_empty))
                .expect("Writing to String buffer should never fail");
            is_empty = false;
        }
        if let Some(value) = self.stemmer {
            write!(buffer, "{}stemmer={value:?}", sep(is_empty)).unwrap();
            is_empty = false;
        }
        if let Some(value) = self.stopwords_language.as_ref() {
            write!(buffer, "{}stopwords_language={value:?}", sep(is_empty)).unwrap();
            is_empty = false;
        }

        if let Some(value) = self.stopwords.as_ref() {
            write!(buffer, "{}stopwords={value:?}", sep(is_empty)).unwrap();
            is_empty = false;
        }
        if let Some(value) = self.alpha_num_only {
            write!(buffer, "{}alpha_num_only={value}", sep(is_empty)).unwrap();
            is_empty = false;
        }
        if let Some(value) = self.ascii_folding {
            write!(buffer, "{}ascii_folding={value}", sep(is_empty)).unwrap();
            is_empty = false;
        }

        if is_empty {
            "".into()
        } else {
            format!("[{buffer}]")
        }
    }

    fn token_length_filter(&self) -> Option<TokenLengthFilter> {
        match (self.remove_short, self.remove_long) {
            (None, None) => None,
            (remove_short, remove_long) => Some(TokenLengthFilter::new(remove_short, remove_long)),
        }
    }

    fn lower_caser(&self) -> Option<LowerCaser> {
        match self.lowercase {
            Some(false) => None, // Only disable if explicitly requested.
            _ => Some(LowerCaser),
        }
    }

    fn stemmer(&self) -> Option<Stemmer> {
        self.stemmer.map(Stemmer::new)
    }

    fn stopwords_language(&self) -> Option<StopWordFilter> {
        match self.stopwords_language {
            Some(language) => StopWordFilter::new(language),
            None => None,
        }
    }

    fn stopwords(&self) -> Option<StopWordFilter> {
        self.stopwords
            .as_ref()
            .map(|stopwords| StopWordFilter::remove(stopwords.clone()))
    }

    fn alpha_num_only(&self) -> Option<AlphaNumOnlyFilter> {
        match self.alpha_num_only {
            Some(true) => Some(AlphaNumOnlyFilter), // Only enable if explicitly requested.
            _ => None,
        }
    }

    fn ascii_folding(&self) -> Option<AsciiFoldingFilter> {
        match self.ascii_folding {
            Some(true) => Some(AsciiFoldingFilter), // Only enable if explicitly requested.
            _ => None,
        }
    }

    fn normalizer(&self) -> Option<SearchNormalizer> {
        self.normalizer
    }
}

macro_rules! add_filters {
    ($tokenizer:expr, $filters:expr $(, $extra_filter:expr )* $(,)?) => {{
        tantivy::tokenizer::TextAnalyzer::builder($tokenizer)
            .filter($filters.token_length_filter())
            .filter($filters.lower_caser())
            .filter($filters.stemmer())
            .filter($filters.stopwords_language())
            .filter($filters.stopwords())
            .filter($filters.ascii_folding())
            $(
                .filter($extra_filter)
            )*
            .filter($filters.alpha_num_only())
            .build()
    }};
}

// Serde will pick a SearchTokenizer variant based on the value of the
// "type" key, which needs to match one of the variant names below.
// The "type" field will not be present on the deserialized value.
//
// Ensure that new variants are added to `from_json_value`. We don't use serde_json to ser/de the
// SearchTokenizer, because our bincode serialization format is incompatible
// with the "tagged" format we use in our public API.
#[derive(Serialize, Clone, Debug, PartialEq, Eq, strum_macros::VariantNames, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum SearchTokenizer {
    Default(SearchTokenizerFilters),
    Keyword,
    #[deprecated(
        since = "0.19.0",
        note = "use the `SearchTokenizer::Keyword` variant instead"
    )]
    KeywordDeprecated,

    #[deprecated(
        since = "0.15.17",
        note = "use the `SearchTokenizer::Keyword` variant instead"
    )]
    Raw(SearchTokenizerFilters),
    LiteralNormalized(SearchTokenizerFilters),
    WhiteSpace(SearchTokenizerFilters),
    RegexTokenizer {
        pattern: String,
        filters: SearchTokenizerFilters,
    },
    ChineseCompatible(SearchTokenizerFilters),
    SourceCode(SearchTokenizerFilters),
    Ngram {
        min_gram: usize,
        max_gram: usize,
        prefix_only: bool,
        filters: SearchTokenizerFilters,
    },
    ChineseLindera(SearchTokenizerFilters),
    JapaneseLindera(SearchTokenizerFilters),
    KoreanLindera(SearchTokenizerFilters),
    #[cfg(feature = "icu")]
    #[strum(serialize = "icu")]
    ICUTokenizer(SearchTokenizerFilters),
    Jieba(SearchTokenizerFilters),

    Lindera(LinderaLanguage, SearchTokenizerFilters),
    UnicodeWords {
        remove_emojis: bool,
        filters: SearchTokenizerFilters,
    },
}

#[derive(Default, Serialize, Clone, Debug, PartialEq, Eq, strum_macros::VariantNames, AsRefStr)]
pub enum LinderaLanguage {
    #[default]
    Unspecified,
    Chinese,
    Japanese,
    Korean,
}

impl Default for SearchTokenizer {
    fn default() -> Self {
        Self::Default(SearchTokenizerFilters::default())
    }
}

impl SearchTokenizer {
    pub fn from_json_value(value: &serde_json::Value) -> Result<Self, anyhow::Error> {
        // We use the `type` field of a JSON object to distinguish the tokenizer variant.
        // Deserialized in this "tagged enum" fashion is not supported by bincode, which
        // we use elsewhere for serialization, so we manually parse the JSON object here.

        let tokenizer_type = value["type"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("a 'type' must be passed in pg_search tokenizer configuration, not found in: {value:#?}"))?;

        let filters = SearchTokenizerFilters::from_json_value(value)?;

        match tokenizer_type {
            "default" => Ok(SearchTokenizer::Default(filters)),
            "keyword" => Ok(SearchTokenizer::Keyword),
            #[allow(deprecated)]
            "raw" => Ok(SearchTokenizer::Raw(filters)),
            "literal_normalized" => Ok(SearchTokenizer::LiteralNormalized(filters)),
            "whitespace" => Ok(SearchTokenizer::WhiteSpace(filters)),
            "regex" => {
                let pattern: String =
                    serde_json::from_value(value["pattern"].clone()).map_err(|_| {
                        anyhow::anyhow!("regex tokenizer requires a string 'pattern' field")
                    })?;
                Ok(SearchTokenizer::RegexTokenizer { pattern, filters })
            }
            "chinese_compatible" => Ok(SearchTokenizer::ChineseCompatible(filters)),
            "source_code" => Ok(SearchTokenizer::SourceCode(filters)),
            "ngram" => {
                let min_gram: usize =
                    serde_json::from_value(value["min_gram"].clone()).map_err(|_| {
                        anyhow::anyhow!("ngram tokenizer requires an integer 'min_gram' field")
                    })?;
                let max_gram: usize =
                    serde_json::from_value(value["max_gram"].clone()).map_err(|_| {
                        anyhow::anyhow!("ngram tokenizer requires an integer 'max_gram' field")
                    })?;
                let prefix_only: bool = serde_json::from_value(value["prefix_only"].clone())
                    .map_err(|_| {
                        anyhow::anyhow!("ngram tokenizer requires a boolean 'prefix_only' field")
                    })?;
                Ok(SearchTokenizer::Ngram {
                    min_gram,
                    max_gram,
                    prefix_only,
                    filters,
                })
            }
            "chinese_lindera" => Ok(SearchTokenizer::ChineseLindera(filters)),
            "japanese_lindera" => Ok(SearchTokenizer::JapaneseLindera(filters)),
            "korean_lindera" => Ok(SearchTokenizer::KoreanLindera(filters)),
            #[cfg(feature = "icu")]
            "icu" => Ok(SearchTokenizer::ICUTokenizer(filters)),
            "jieba" => Ok(SearchTokenizer::Jieba(filters)),
            "unicode_words" => {
                let remove_emojis: bool = serde_json::from_value(value["remove_emojis"].clone())
                    .map_err(|_| {
                        anyhow::anyhow!(
                            "unicode_words tokenizer requires an integer 'remove_emojis' field"
                        )
                    })?;

                Ok(SearchTokenizer::UnicodeWords {
                    remove_emojis,
                    filters,
                })
            }
            _ => Err(anyhow::anyhow!(
                "unknown tokenizer type: {}",
                tokenizer_type
            )),
        }
    }

    pub fn to_tantivy_tokenizer(&self) -> Option<tantivy::tokenizer::TextAnalyzer> {
        let analyzer = match self {
            SearchTokenizer::Default(filters) => {
                add_filters!(SimpleTokenizer::default(), filters)
            }
            // the keyword tokenizer is a special case that does not have filters
            SearchTokenizer::Keyword => TextAnalyzer::builder(RawTokenizer::default()).build(),
            #[allow(deprecated)]
            SearchTokenizer::KeywordDeprecated => {
                TextAnalyzer::builder(RawTokenizer::default()).build()
            }
            SearchTokenizer::LiteralNormalized(filters) => {
                add_filters!(RawTokenizer::default(), filters)
            }
            SearchTokenizer::WhiteSpace(filters) => {
                add_filters!(WhitespaceTokenizer::default(), filters)
            }
            // this Tokenizer is deprecated because it's bugged. `filters.lower_caser()` provides defaults, but that is the
            // opposite of what the `raw` tokenizer should do.
            //
            // the decision was made to introduce the `keyword` tokenizer which does the correct thing
            // that is, doesn't mutate the input tokens
            #[allow(deprecated)]
            SearchTokenizer::Raw(filters) => {
                add_filters!(RawTokenizer::default(), filters)
            }
            SearchTokenizer::RegexTokenizer { pattern, filters } => {
                add_filters!(RegexTokenizer::new(pattern.as_str()).unwrap(), filters)
            }
            SearchTokenizer::Ngram {
                min_gram,
                max_gram,
                prefix_only,
                filters,
            } => add_filters!(
                NgramTokenizer::new(*min_gram, *max_gram, *prefix_only)
                    .expect("Ngram parameters should be valid parameters for NgramTokenizer"),
                filters
            ),
            SearchTokenizer::ChineseCompatible(filters) => {
                add_filters!(ChineseTokenizer, filters)
            }
            SearchTokenizer::SourceCode(filters) => {
                // for backwards compatibility, the source_code tokenizer defaults to ascii_folding
                // if it's not explicitly set
                if filters.ascii_folding().is_none() {
                    add_filters!(CodeTokenizer::default(), filters, AsciiFoldingFilter)
                } else {
                    add_filters!(CodeTokenizer::default(), filters)
                }
            }
            SearchTokenizer::ChineseLindera(filters)
            | SearchTokenizer::Lindera(LinderaLanguage::Chinese, filters) => {
                add_filters!(LinderaChineseTokenizer::default(), filters)
            }
            SearchTokenizer::JapaneseLindera(filters)
            | SearchTokenizer::Lindera(LinderaLanguage::Japanese, filters) => {
                add_filters!(LinderaJapaneseTokenizer::default(), filters)
            }
            SearchTokenizer::KoreanLindera(filters)
            | SearchTokenizer::Lindera(LinderaLanguage::Korean, filters) => {
                add_filters!(LinderaKoreanTokenizer::default(), filters)
            }
            #[cfg(feature = "icu")]
            SearchTokenizer::ICUTokenizer(filters) => {
                add_filters!(ICUTokenizer, filters)
            }
            SearchTokenizer::Jieba(filters) => {
                add_filters!(tantivy_jieba::JiebaTokenizer {}, filters)
            }
            SearchTokenizer::Lindera(LinderaLanguage::Unspecified, _) => {
                panic!("LinderaStyle::Unspecified is not supported")
            }
            SearchTokenizer::UnicodeWords {
                remove_emojis,
                filters,
            } => {
                add_filters!(UnicodeWordsTokenizer::new(*remove_emojis), filters)
            }
        };

        Some(analyzer)
    }

    fn filters(&self) -> &SearchTokenizerFilters {
        match self {
            SearchTokenizer::Default(filters) => filters,
            SearchTokenizer::Keyword => SearchTokenizerFilters::keyword(),
            #[allow(deprecated)]
            SearchTokenizer::KeywordDeprecated => SearchTokenizerFilters::keyword_deprecated(),
            #[allow(deprecated)]
            SearchTokenizer::Raw(filters) => filters,
            SearchTokenizer::LiteralNormalized(filters) => filters,
            SearchTokenizer::WhiteSpace(filters) => filters,
            SearchTokenizer::RegexTokenizer { filters, .. } => filters,
            SearchTokenizer::ChineseCompatible(filters) => filters,
            SearchTokenizer::SourceCode(filters) => filters,
            SearchTokenizer::Ngram { filters, .. } => filters,
            SearchTokenizer::ChineseLindera(filters) => filters,
            SearchTokenizer::JapaneseLindera(filters) => filters,
            SearchTokenizer::KoreanLindera(filters) => filters,
            SearchTokenizer::Lindera(_, filters) => filters,
            #[cfg(feature = "icu")]
            SearchTokenizer::ICUTokenizer(filters) => filters,
            SearchTokenizer::Jieba(filters) => filters,
            SearchTokenizer::UnicodeWords { filters, .. } => filters,
        }
    }

    pub fn normalizer(&self) -> Option<SearchNormalizer> {
        self.filters().normalizer()
    }
}

pub fn language_to_str(lang: &Language) -> &str {
    match lang {
        Language::Arabic => "Arabic",
        Language::Danish => "Danish",
        Language::Dutch => "Dutch",
        Language::English => "English",
        Language::Finnish => "Finnish",
        Language::French => "French",
        Language::German => "German",
        Language::Greek => "Greek",
        Language::Hungarian => "Hungarian",
        Language::Italian => "Italian",
        Language::Norwegian => "Norwegian",
        Language::Portuguese => "Portuguese",
        Language::Romanian => "Romanian",
        Language::Russian => "Russian",
        Language::Spanish => "Spanish",
        Language::Swedish => "Swedish",
        Language::Tamil => "Tamil",
        Language::Turkish => "Turkish",
    }
}

impl SearchTokenizer {
    pub fn name(&self) -> String {
        let filters_suffix = self.filters().name_suffix();
        match self {
            SearchTokenizer::Default(_filters) => format!("default{filters_suffix}"),
            SearchTokenizer::Keyword => format!("keyword{filters_suffix}"),
            #[allow(deprecated)]
            SearchTokenizer::KeywordDeprecated => format!("keyword{filters_suffix}"),
            #[allow(deprecated)]
            SearchTokenizer::Raw(_filters) => format!("raw{filters_suffix}"),
            SearchTokenizer::LiteralNormalized(_filters) => format!("literal_normalized{filters_suffix}"),
            SearchTokenizer::WhiteSpace(_filters) => format!("whitespace{filters_suffix}"),
            SearchTokenizer::RegexTokenizer { .. } => format!("regex{filters_suffix}"),
            SearchTokenizer::ChineseCompatible(_filters) => {
                format!("chinese_compatible{filters_suffix}")
            }
            SearchTokenizer::SourceCode(_filters) => format!("source_code{filters_suffix}"),
            SearchTokenizer::Ngram {
                min_gram,
                max_gram,
                prefix_only,
                filters: _,
            } => format!("ngram_mingram:{min_gram}_maxgram:{max_gram}_prefixonly:{prefix_only}{filters_suffix}"),
            SearchTokenizer::ChineseLindera(_filters) => format!("chinese_lindera{filters_suffix}"),
            SearchTokenizer::JapaneseLindera(_filters) => {
                format!("japanese_lindera{filters_suffix}")
            }
            SearchTokenizer::KoreanLindera(_filters) => format!("korean_lindera{filters_suffix}"),
            SearchTokenizer::Lindera(style, _filters) => match style {
                LinderaLanguage::Unspecified => panic!("LinderaStyle::Unspecified is not supported"),
                LinderaLanguage::Chinese => format!("chinese_lindera{filters_suffix}"),
                LinderaLanguage::Japanese => format!("japanese_lindera{filters_suffix}"),
                LinderaLanguage::Korean => format!("korean_lindera{filters_suffix}"),
            }
            #[cfg(feature = "icu")]
            SearchTokenizer::ICUTokenizer(_filters) => format!("icu{filters_suffix}"),
            SearchTokenizer::Jieba(_filters) => format!("jieba{filters_suffix}"),
            SearchTokenizer::UnicodeWords{remove_emojis, filters: _} => format!("remove_emojis:{remove_emojis}{filters_suffix}"),
        }
    }
}

impl<'de> Deserialize<'de> for SearchTokenizer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        SearchTokenizer::from_json_value(&value).map_err(de::Error::custom)
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
    use super::*;
    use rstest::*;

    #[rstest]
    fn test_search_tokenizer() {
        let tokenizer = SearchTokenizer::default();
        assert_eq!(tokenizer.name(), "default".to_string());

        let json = r#"{
            "type": "ngram",
            "min_gram": 20,
            "max_gram": 60,
            "prefix_only": true,
            "remove_long": 123,
            "lowercase": false
        }"#;

        let tokenizer =
            SearchTokenizer::from_json_value(&serde_json::from_str(json).unwrap()).unwrap();

        assert_eq!(
            tokenizer,
            SearchTokenizer::Ngram {
                min_gram: 20,
                max_gram: 60,
                prefix_only: true,
                filters: SearchTokenizerFilters {
                    remove_short: None,
                    remove_long: Some(123),
                    lowercase: Some(false),
                    stemmer: None,
                    stopwords_language: None,
                    stopwords: None,
                    ascii_folding: None,
                    normalizer: None,
                    alpha_num_only: None,
                }
            }
        );
    }

    #[rstest]
    fn test_regexizer() {
        let json = r#"{
            "type": "regex",
            "pattern": "a+b*",
            "remove_long": 100
        }"#;
        let tokenizer = SearchTokenizer::RegexTokenizer {
            pattern: "a+b*".to_string(),
            filters: SearchTokenizerFilters {
                remove_short: None,
                remove_long: Some(100),
                lowercase: None,
                stemmer: None,
                stopwords_language: None,
                stopwords: None,
                ascii_folding: None,
                normalizer: None,
                alpha_num_only: None,
            },
        };

        assert_eq!(
            tokenizer,
            SearchTokenizer::from_json_value(&serde_json::from_str(json).unwrap()).unwrap()
        );
    }

    #[rstest]
    fn test_search_normalizer() {
        assert_eq!(SearchNormalizer::Lowercase.name(), "lowercase");
        assert_ne!(SearchNormalizer::Raw, SearchNormalizer::Lowercase);
    }

    #[rstest]
    fn test_jieba_tokenizer_with_stopwords() {
        use tantivy::tokenizer::TokenStream;

        // Test Jieba tokenizer with custom stopwords including spaces and content words
        let json = r#"{
            "type": "jieba",
            "stopwords": [" ", "花朵", "公园"]
        }"#;

        let tokenizer =
            SearchTokenizer::from_json_value(&serde_json::from_str(json).unwrap()).unwrap();

        assert_eq!(
            tokenizer,
            SearchTokenizer::Jieba(SearchTokenizerFilters {
                remove_short: None,
                remove_long: None,
                lowercase: None,
                stemmer: None,
                stopwords_language: None,
                stopwords: Some(vec![
                    " ".to_string(),
                    "花朵".to_string(),
                    "公园".to_string()
                ]),
                ascii_folding: None,
                normalizer: None,
                alpha_num_only: None,
            })
        );

        // Test that the tokenizer is created successfully
        let mut analyzer = tokenizer.to_tantivy_tokenizer().unwrap();

        // Test tokenizing text with spaces and content words that should be filtered out
        let text = "我们 昨天 在 公园 里 看到 了 很多 美丽 的 花朵";
        let mut token_stream = analyzer.token_stream(text);

        let mut tokens = Vec::new();
        while token_stream.advance() {
            let token = token_stream.token();
            tokens.push(token.text.clone());
        }

        // Verify that custom stopwords are filtered out (spaces, 花朵, 公园)
        assert!(!tokens.contains(&" ".to_string()));
        assert!(!tokens.contains(&"花朵".to_string()));
        assert!(!tokens.contains(&"公园".to_string()));

        // Verify that other words are still present
        assert!(tokens.contains(&"我们".to_string()));
        assert!(tokens.contains(&"昨天".to_string()));
        assert!(tokens.contains(&"美丽".to_string()));
    }

    #[rstest]
    fn test_jieba_tokenizer_with_language_stopwords() {
        use tantivy::tokenizer::{Language, TokenStream};

        // Test Jieba tokenizer with language-based stopwords
        let json = r#"{
            "type": "jieba",
            "stopwords_language": "English"
        }"#;

        let tokenizer =
            SearchTokenizer::from_json_value(&serde_json::from_str(json).unwrap()).unwrap();

        assert_eq!(
            tokenizer,
            SearchTokenizer::Jieba(SearchTokenizerFilters {
                remove_short: None,
                remove_long: None,
                lowercase: None,
                stemmer: None,
                stopwords_language: Some(Language::English),
                stopwords: None,
                ascii_folding: None,
                normalizer: None,
                alpha_num_only: None,
            })
        );

        // Test that the tokenizer is created successfully
        let mut analyzer = tokenizer.to_tantivy_tokenizer().unwrap();

        // Test tokenizing mixed Chinese and English text
        let text = "我喜欢在 the library 里读书 and learning";
        let mut token_stream = analyzer.token_stream(text);

        let mut tokens = Vec::new();
        while token_stream.advance() {
            let token = token_stream.token();
            tokens.push(token.text.clone());
        }

        // Verify that English stopwords "the", "and" are filtered out
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"and".to_string()));

        // Verify that other words are still present
        assert!(tokens.contains(&"library".to_string()));
        assert!(tokens.contains(&"读书".to_string()));
        assert!(tokens.contains(&"learning".to_string()));
    }
}

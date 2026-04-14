// Copyright (c) 2023-2026 ParadeDB, Inc.
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

use crate::edge_ngram::{EdgeNgramTokenizer, TokenCharClass};
use crate::icu::ICUTokenizer;
use crate::ngram::NgramTokenizer;
use crate::{
    cjk::ChineseTokenizer,
    code::CodeTokenizer,
    lindera::{LinderaChineseTokenizer, LinderaJapaneseTokenizer, LinderaKoreanTokenizer},
    token_length::TokenLengthFilter,
    token_trim::TokenTrimFilter,
    unicode_words::UnicodeWordsTokenizer,
};

use crate::chinese_convert::{ChineseConvertTokenizer, ConvertMode};
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::AsRefStr;
use tantivy::tokenizer::{
    AlphaNumOnlyFilter, AsciiFoldingFilter, Language, LowerCaser, RawTokenizer, RegexTokenizer,
    SimpleTokenizer, Stemmer, StopWordFilter, TextAnalyzer, WhitespaceTokenizer,
};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct SearchTokenizerFilters {
    pub remove_short: Option<usize>,
    pub remove_long: Option<usize>,
    pub lowercase: Option<bool>,
    pub stemmer: Option<Language>,
    /// Supports one or more languages for stopwords filtering.
    /// Useful for documents containing multiple languages.
    pub stopwords_language: Option<Vec<Language>>,
    pub stopwords: Option<Vec<String>>,
    pub alpha_num_only: Option<bool>,
    pub ascii_folding: Option<bool>,
    pub trim: Option<bool>,
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
            trim: None,
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
            trim: None,
            normalizer: Some(SearchNormalizer::Raw),
        }
    }

    /// Parse stopwords_language from JSON - supports both a single string and an array of strings
    fn parse_stopwords_language(
        value: &serde_json::Value,
    ) -> Result<Option<Vec<Language>>, anyhow::Error> {
        match value {
            serde_json::Value::Null => Ok(None),
            serde_json::Value::String(s) => {
                let lang: Language = serde_json::from_value(serde_json::Value::String(s.clone()))
                    .map_err(|e| {
                    anyhow::anyhow!("stopwords_language tokenizer requires a valid language: {e}")
                })?;
                Ok(Some(vec![lang]))
            }
            serde_json::Value::Array(arr) => {
                let languages: Vec<Language> = arr
                    .iter()
                    .map(|v| {
                        serde_json::from_value(v.clone()).map_err(|e| {
                            anyhow::anyhow!(
                                "stopwords_language tokenizer requires valid languages: {e}"
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if languages.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(languages))
                }
            }
            _ => Err(anyhow::anyhow!(
                "stopwords_language must be a string or array of strings"
            )),
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
            filters.stopwords_language = Self::parse_stopwords_language(stopwords_language)?;
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
        if let Some(trim) = value.get("trim") {
            filters.trim = Some(trim.as_bool().ok_or_else(|| {
                anyhow::anyhow!(
                    "a 'trim' value passed to the pg_search tokenizer configuration \
                     must be of type bool, found: {trim:#?}"
                )
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
            if value.len() == 1 {
                write!(buffer, "{}stopwords_language={:?}", sep(is_empty), value[0]).unwrap();
            } else {
                write!(buffer, "{}stopwords_language={value:?}", sep(is_empty)).unwrap();
            }
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

    /// Returns StopWordFilters for all specified languages.
    /// Uses Tantivy's built-in StopWordFilter::new() for each language.
    fn stopwords_languages(&self) -> Vec<StopWordFilter> {
        self.stopwords_language
            .as_ref()
            .map(|languages| {
                languages
                    .iter()
                    .filter_map(|lang| StopWordFilter::new(*lang))
                    .collect()
            })
            .unwrap_or_default()
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

    fn trim_filter(&self) -> Option<TokenTrimFilter> {
        match self.trim {
            Some(true) => Some(TokenTrimFilter::new()), // Only enable if explicitly requested.
            _ => None,
        }
    }

    fn normalizer(&self) -> Option<SearchNormalizer> {
        self.normalizer
    }
}

macro_rules! add_filters {
    ($tokenizer:expr, $filters:expr $(, $extra_filter:expr )* $(,)?) => {{
        // Build the analyzer with static filters first
        let mut builder = tantivy::tokenizer::TextAnalyzer::builder($tokenizer)
            .filter($filters.token_length_filter())
            .filter($filters.trim_filter())
            .filter($filters.lower_caser())
            .filter($filters.stemmer())
            .filter($filters.stopwords())
            .filter($filters.ascii_folding())
            $(
                .filter($extra_filter)
            )*
            .filter($filters.alpha_num_only())
            // Convert to type-erased builder for dynamic filter application
            .dynamic();
        // Apply stopword language filters dynamically in a for loop
        for stopword_filter in $filters.stopwords_languages() {
            builder = builder.filter_dynamic(stopword_filter);
        }
        builder.build()
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
    #[strum(serialize = "default")]
    Simple(SearchTokenizerFilters),
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
        #[serde(default)]
        positions: bool,
        filters: SearchTokenizerFilters,
    },
    EdgeNgram {
        min_gram: usize,
        max_gram: usize,
        token_chars: Vec<String>,
        filters: SearchTokenizerFilters,
    },
    ChineseLinderaDeprecated(SearchTokenizerFilters),
    ChineseLindera {
        filters: SearchTokenizerFilters,
        keep_whitespace: bool,
    },
    JapaneseLinderaDeprecated(SearchTokenizerFilters),
    JapaneseLindera {
        filters: SearchTokenizerFilters,
        keep_whitespace: bool,
    },
    KoreanLinderaDeprecated(SearchTokenizerFilters),
    KoreanLindera {
        filters: SearchTokenizerFilters,
        keep_whitespace: bool,
    },
    #[strum(serialize = "icu")]
    ICUTokenizer(SearchTokenizerFilters),
    Jieba {
        chinese_convert: Option<ConvertMode>,
        filters: SearchTokenizerFilters,
    },
    LinderaDeprecated(LinderaLanguage, SearchTokenizerFilters),
    Lindera {
        language: LinderaLanguage,
        filters: SearchTokenizerFilters,
        keep_whitespace: bool,
    },
    UnicodeWordsDeprecated {
        remove_emojis: bool,
        filters: SearchTokenizerFilters,
    },
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
        Self::UnicodeWords {
            remove_emojis: false,
            filters: SearchTokenizerFilters::default(),
        }
    }
}

impl SearchTokenizer {
    // This is only used by the v1 api (prior to v0.20.0)
    pub fn from_json_value(value: &serde_json::Value) -> Result<Self, anyhow::Error> {
        // We use the `type` field of a JSON object to distinguish the tokenizer variant.
        // Deserialized in this "tagged enum" fashion is not supported by bincode, which
        // we use elsewhere for serialization, so we manually parse the JSON object here.

        let tokenizer_type = value["type"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("a 'type' must be passed in pg_search tokenizer configuration, not found in: {value:#?}"))?;

        let filters = SearchTokenizerFilters::from_json_value(value)?;

        match tokenizer_type {
            "default" => Ok(SearchTokenizer::Simple(filters)),
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
                let positions: bool = value
                    .get("positions")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Ok(SearchTokenizer::Ngram {
                    min_gram,
                    max_gram,
                    prefix_only,
                    positions,
                    filters,
                })
            }
            "edge_ngram" => {
                let min_gram: usize =
                    value.get("min_gram").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                let max_gram: usize =
                    value.get("max_gram").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
                let token_chars: Vec<String> = value
                    .get("token_chars")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_else(|| vec!["letter".to_string(), "digit".to_string()]);
                Ok(SearchTokenizer::EdgeNgram {
                    min_gram,
                    max_gram,
                    token_chars,
                    filters,
                })
            }
            "chinese_lindera" => Ok(SearchTokenizer::ChineseLinderaDeprecated(filters)),
            "japanese_lindera" => Ok(SearchTokenizer::JapaneseLinderaDeprecated(filters)),
            "korean_lindera" => Ok(SearchTokenizer::KoreanLinderaDeprecated(filters)),
            "icu" => Ok(SearchTokenizer::ICUTokenizer(filters)),
            "jieba" => {
                let chinese_convert: Option<ConvertMode> = if value["chinese_convert"].is_null() {
                    None
                } else {
                    Some(
                        serde_json::from_value(value["chinese_convert"].clone()).map_err(|_| {
                            anyhow::anyhow!(
                                "jieba tokenizer requires a string 'chinese_convert' field"
                            )
                        })?,
                    )
                };
                Ok(SearchTokenizer::Jieba {
                    chinese_convert,
                    filters,
                })
            }
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
            SearchTokenizer::Simple(filters) => {
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
                positions,
                filters,
            } => add_filters!(
                NgramTokenizer::new(*min_gram, *max_gram, *prefix_only, *positions)
                    .unwrap_or_else(|e| panic!("{}", e)),
                filters
            ),
            SearchTokenizer::EdgeNgram {
                min_gram,
                max_gram,
                token_chars,
                filters,
            } => {
                let classes = token_chars
                    .iter()
                    .map(|s| s.parse::<TokenCharClass>())
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap_or_else(|e| panic!("{}", e));
                add_filters!(
                    EdgeNgramTokenizer::new(*min_gram, *max_gram, classes)
                        .unwrap_or_else(|e| panic!("{}", e)),
                    filters
                )
            }
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
            SearchTokenizer::ChineseLinderaDeprecated(filters)
            | SearchTokenizer::LinderaDeprecated(LinderaLanguage::Chinese, filters) => {
                add_filters!(LinderaChineseTokenizer::new(true), filters)
            }
            SearchTokenizer::ChineseLindera {
                filters,
                keep_whitespace,
            }
            | SearchTokenizer::Lindera {
                language: LinderaLanguage::Chinese,
                filters,
                keep_whitespace,
            } => {
                add_filters!(LinderaChineseTokenizer::new(*keep_whitespace), filters)
            }
            SearchTokenizer::JapaneseLinderaDeprecated(filters)
            | SearchTokenizer::LinderaDeprecated(LinderaLanguage::Japanese, filters) => {
                add_filters!(LinderaJapaneseTokenizer::new(true), filters)
            }
            SearchTokenizer::JapaneseLindera {
                filters,
                keep_whitespace,
            }
            | SearchTokenizer::Lindera {
                language: LinderaLanguage::Japanese,
                filters,
                keep_whitespace,
            } => {
                add_filters!(LinderaJapaneseTokenizer::new(*keep_whitespace), filters)
            }
            SearchTokenizer::KoreanLinderaDeprecated(filters)
            | SearchTokenizer::LinderaDeprecated(LinderaLanguage::Korean, filters) => {
                add_filters!(LinderaKoreanTokenizer::new(true), filters)
            }
            SearchTokenizer::KoreanLindera {
                filters,
                keep_whitespace,
            }
            | SearchTokenizer::Lindera {
                language: LinderaLanguage::Korean,
                filters,
                keep_whitespace,
            } => {
                add_filters!(LinderaKoreanTokenizer::new(*keep_whitespace), filters)
            }
            SearchTokenizer::ICUTokenizer(filters) => {
                add_filters!(ICUTokenizer, filters)
            }
            SearchTokenizer::Jieba {
                chinese_convert,
                filters,
            } => {
                // If Chinese conversion is configured, perform the conversion before tokenization
                if let Some(convert_mode) = chinese_convert {
                    let base_tokenizer =
                        tantivy_jieba::JiebaTokenizer::with_ordinal_position_mode(true);
                    let convert_tokenizer =
                        ChineseConvertTokenizer::new(base_tokenizer, *convert_mode);
                    add_filters!(convert_tokenizer, filters)
                } else {
                    add_filters!(
                        tantivy_jieba::JiebaTokenizer::with_ordinal_position_mode(true),
                        filters
                    )
                }
            }
            SearchTokenizer::LinderaDeprecated(LinderaLanguage::Unspecified, _)
            | SearchTokenizer::Lindera {
                language: LinderaLanguage::Unspecified,
                ..
            } => {
                panic!("LinderaStyle::Unspecified is not supported")
            }
            SearchTokenizer::UnicodeWords {
                remove_emojis,
                filters,
            }
            | SearchTokenizer::UnicodeWordsDeprecated {
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
            SearchTokenizer::Simple(filters) => filters,
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
            SearchTokenizer::EdgeNgram { filters, .. } => filters,
            SearchTokenizer::ChineseLinderaDeprecated(filters) => filters,
            SearchTokenizer::ChineseLindera { filters, .. } => filters,
            SearchTokenizer::JapaneseLinderaDeprecated(filters) => filters,
            SearchTokenizer::JapaneseLindera { filters, .. } => filters,
            SearchTokenizer::KoreanLinderaDeprecated(filters) => filters,
            SearchTokenizer::KoreanLindera { filters, .. } => filters,
            SearchTokenizer::LinderaDeprecated(_, filters) => filters,
            SearchTokenizer::Lindera { filters, .. } => filters,
            SearchTokenizer::ICUTokenizer(filters) => filters,
            SearchTokenizer::Jieba { filters, .. } => filters,
            SearchTokenizer::UnicodeWordsDeprecated { filters, .. } => filters,
            SearchTokenizer::UnicodeWords { filters, .. } => filters,
        }
    }

    pub fn normalizer(&self) -> Option<SearchNormalizer> {
        self.filters().normalizer()
    }
}

pub static LANGUAGES: Lazy<HashMap<Language, &str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(Language::Arabic, "Arabic");
    map.insert(Language::Czech, "Czech");
    map.insert(Language::Danish, "Danish");
    map.insert(Language::Dutch, "Dutch");
    map.insert(Language::English, "English");
    map.insert(Language::Finnish, "Finnish");
    map.insert(Language::French, "French");
    map.insert(Language::German, "German");
    map.insert(Language::Greek, "Greek");
    map.insert(Language::Hungarian, "Hungarian");
    map.insert(Language::Italian, "Italian");
    map.insert(Language::Norwegian, "Norwegian");
    map.insert(Language::Polish, "Polish");
    map.insert(Language::Portuguese, "Portuguese");
    map.insert(Language::Romanian, "Romanian");
    map.insert(Language::Russian, "Russian");
    map.insert(Language::Spanish, "Spanish");
    map.insert(Language::Swedish, "Swedish");
    map.insert(Language::Tamil, "Tamil");
    map.insert(Language::Turkish, "Turkish");
    map
});

impl SearchTokenizer {
    pub fn name(&self) -> String {
        let filters_suffix = self.filters().name_suffix();
        match self {
            SearchTokenizer::Simple(_filters) => format!("default{filters_suffix}"),
            SearchTokenizer::Keyword => format!("keyword{filters_suffix}"),
            #[allow(deprecated)]
            SearchTokenizer::KeywordDeprecated => format!("keyword{filters_suffix}"),
            #[allow(deprecated)]
            SearchTokenizer::Raw(_filters) => format!("raw{filters_suffix}"),
            SearchTokenizer::LiteralNormalized(_filters) => {
                format!("literal_normalized{filters_suffix}")
            }
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
                positions,
            } => {
                let positions_suffix = if *positions { "_positions:true" } else { "" };
                format!(
                    "ngram_mingram:{min_gram}_maxgram:{max_gram}_prefixonly:{prefix_only}{positions_suffix}{filters_suffix}"
                )
            }
            SearchTokenizer::EdgeNgram {
                min_gram,
                max_gram,
                token_chars,
                filters: _,
            } => {
                let mut tc_sorted = token_chars.clone();
                tc_sorted.sort();
                let tc = tc_sorted.join(",");
                format!(
                    "edge_ngram_mingram:{min_gram}_maxgram:{max_gram}_tokenchars:{tc}{filters_suffix}"
                )
            }
            SearchTokenizer::ChineseLinderaDeprecated(_filters) => {
                format!("chinese_lindera{filters_suffix}")
            }
            SearchTokenizer::ChineseLindera {
                filters: _,
                keep_whitespace,
            } => {
                format!("chinese_lindera_keepwhitespace:{keep_whitespace}{filters_suffix}")
            }
            SearchTokenizer::JapaneseLinderaDeprecated(_filters) => {
                format!("japanese_lindera{filters_suffix}")
            }
            SearchTokenizer::JapaneseLindera {
                filters: _,
                keep_whitespace,
            } => {
                format!("japanese_lindera_keepwhitespace:{keep_whitespace}{filters_suffix}")
            }
            SearchTokenizer::KoreanLinderaDeprecated(_filters) => {
                format!("korean_lindera{filters_suffix}")
            }
            SearchTokenizer::KoreanLindera {
                filters: _,
                keep_whitespace,
            } => {
                format!("korean_lindera_keepwhitespace:{keep_whitespace}{filters_suffix}")
            }
            SearchTokenizer::LinderaDeprecated(style, _filters) => match style {
                LinderaLanguage::Unspecified => {
                    panic!("LinderaStyle::Unspecified is not supported")
                }
                LinderaLanguage::Chinese => format!("chinese_lindera{filters_suffix}"),
                LinderaLanguage::Japanese => format!("japanese_lindera{filters_suffix}"),
                LinderaLanguage::Korean => format!("korean_lindera{filters_suffix}"),
            },
            SearchTokenizer::Lindera {
                language,
                filters: _,
                keep_whitespace,
            } => match language {
                LinderaLanguage::Unspecified => {
                    panic!("LinderaStyle::Unspecified is not supported")
                }
                LinderaLanguage::Chinese => {
                    format!("chinese_lindera_keepwhitespace:{keep_whitespace}{filters_suffix}")
                }
                LinderaLanguage::Japanese => {
                    format!("japanese_lindera_keepwhitespace:{keep_whitespace}{filters_suffix}")
                }
                LinderaLanguage::Korean => {
                    format!("korean_lindera_keepwhitespace:{keep_whitespace}{filters_suffix}")
                }
            },
            SearchTokenizer::ICUTokenizer(_filters) => format!("icu{filters_suffix}"),
            SearchTokenizer::Jieba {
                chinese_convert,
                filters: _,
            } => {
                if let Some(chinese_convert) = chinese_convert {
                    format!("jieba{chinese_convert:?}{filters_suffix}")
                } else {
                    format!("jieba{filters_suffix}")
                }
            }
            SearchTokenizer::UnicodeWordsDeprecated {
                remove_emojis,
                filters: _,
            } => format!("remove_emojis:{remove_emojis}{filters_suffix}"),
            SearchTokenizer::UnicodeWords {
                remove_emojis,
                filters: _,
            } => format!("unicode_words_removeemojis:{remove_emojis}{filters_suffix}"),
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
        let tokenizer = SearchTokenizer::Simple(SearchTokenizerFilters::default());
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
                positions: false,
                filters: SearchTokenizerFilters {
                    remove_short: None,
                    remove_long: Some(123),
                    lowercase: Some(false),
                    stemmer: None,
                    stopwords_language: None,
                    stopwords: None,
                    ascii_folding: None,
                    trim: None,
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
                trim: None,
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
            SearchTokenizer::Jieba {
                chinese_convert: None,
                filters: SearchTokenizerFilters {
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
                    trim: None,
                    normalizer: None,
                    alpha_num_only: None,
                }
            }
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
            SearchTokenizer::Jieba {
                chinese_convert: None,
                filters: SearchTokenizerFilters {
                    remove_short: None,
                    remove_long: None,
                    lowercase: None,
                    stemmer: None,
                    stopwords_language: Some(vec![Language::English]),
                    stopwords: None,
                    ascii_folding: None,
                    trim: None,
                    normalizer: None,
                    alpha_num_only: None,
                }
            }
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

    #[rstest]
    fn test_jieba_tokenizer_with_trim_filter() {
        use tantivy::tokenizer::TokenStream;

        // Test Jieba tokenizer with trim filter
        let json = r#"{
            "type": "jieba",
            "trim": true
        }"#;

        let tokenizer =
            SearchTokenizer::from_json_value(&serde_json::from_str(json).unwrap()).unwrap();

        assert_eq!(
            tokenizer,
            SearchTokenizer::Jieba {
                chinese_convert: None,
                filters: SearchTokenizerFilters {
                    remove_short: None,
                    remove_long: None,
                    lowercase: None,
                    stemmer: None,
                    stopwords_language: None,
                    stopwords: None,
                    ascii_folding: None,
                    trim: Some(true),
                    normalizer: None,
                    alpha_num_only: None,
                }
            }
        );

        // Test that the tokenizer is created successfully
        let mut analyzer = tokenizer.to_tantivy_tokenizer().unwrap();

        // Test tokenizing text with spaces (which Jieba may produce as separate tokens)
        let text = "富裕 劳动力";
        let mut token_stream = analyzer.token_stream(text);

        let mut tokens = Vec::new();
        while token_stream.advance() {
            let token = token_stream.token();
            tokens.push(token.text.clone());
        }

        // Verify that space tokens are filtered out
        assert!(!tokens.contains(&" ".to_string()));
        assert!(!tokens.iter().any(|t| t.trim().is_empty()));

        // Verify that content words are still present
        assert!(tokens.contains(&"富裕".to_string()));
        assert!(tokens.contains(&"劳动".to_string()) || tokens.contains(&"劳动力".to_string()));
    }

    #[rstest]
    fn test_korean_lindera_tokenizer_with_trim_filter() {
        use tantivy::tokenizer::TokenStream;

        // Test Korean Lindera tokenizer with trim filter
        let json = r#"{
            "type": "korean_lindera",
            "trim": true
        }"#;

        let tokenizer =
            SearchTokenizer::from_json_value(&serde_json::from_str(json).unwrap()).unwrap();

        assert_eq!(
            tokenizer,
            SearchTokenizer::KoreanLinderaDeprecated(SearchTokenizerFilters {
                remove_short: None,
                remove_long: None,
                lowercase: None,
                stemmer: None,
                stopwords_language: None,
                stopwords: None,
                ascii_folding: None,
                trim: Some(true),
                normalizer: None,
                alpha_num_only: None,
            })
        );

        // Test that the tokenizer is created successfully
        let mut analyzer = tokenizer.to_tantivy_tokenizer().unwrap();

        // Test tokenizing Korean text with spaces
        // "아름다운 우리나라" (Beautiful our country)
        let text = "아름다운 우리나라";
        let mut token_stream = analyzer.token_stream(text);

        let mut tokens = Vec::new();
        while token_stream.advance() {
            let token = token_stream.token();
            tokens.push(token.text.clone());
        }

        // Verify that space tokens are filtered out
        assert!(!tokens.contains(&" ".to_string()));
        assert!(!tokens.iter().any(|t| t.trim().is_empty()));

        // Verify that Korean words are still present
        assert!(!tokens.is_empty());
    }

    #[rstest]
    fn test_deprecated_chinese_lindera_tokenizer_preserves_whitespace() {
        use tantivy::tokenizer::TokenStream;

        // Test Chinese Lindera tokenizer preserves whitespace by default
        // (backward compatible behavior)
        let json = r#"{
            "type": "chinese_lindera"
        }"#;

        let tokenizer =
            SearchTokenizer::from_json_value(&serde_json::from_str(json).unwrap()).unwrap();

        // Test that the tokenizer is created successfully
        let mut analyzer = tokenizer.to_tantivy_tokenizer().unwrap();

        // Test tokenizing text with spaces
        let text = "this is a test";
        let mut token_stream = analyzer.token_stream(text);

        let mut tokens = Vec::new();
        while token_stream.advance() {
            let token = token_stream.token();
            tokens.push(token.text.clone());
        }

        // Verify that space tokens are preserved (backward compatible behavior)
        assert!(tokens.contains(&" ".to_string()));

        // Verify that words are still present
        assert!(tokens.contains(&"this".to_string()));
        assert!(tokens.contains(&"is".to_string()));
        assert!(tokens.contains(&"a".to_string()));
        assert!(tokens.contains(&"test".to_string()));
    }

    #[rstest]
    fn test_trim_filter_with_multiple_tokenizers() {
        use tantivy::tokenizer::TokenStream;

        // Test that trim filter works across different tokenizers

        // Test 1: Chinese Lindera tokenizer with trim filter
        let json_lindera = r#"{
            "type": "chinese_lindera",
            "trim": true
        }"#;

        let tokenizer_lindera =
            SearchTokenizer::from_json_value(&serde_json::from_str(json_lindera).unwrap()).unwrap();
        let mut analyzer_lindera = tokenizer_lindera.to_tantivy_tokenizer().unwrap();

        let text_lindera = "富裕 劳动力";
        let mut token_stream_lindera = analyzer_lindera.token_stream(text_lindera);

        let mut tokens_lindera = Vec::new();
        while token_stream_lindera.advance() {
            let token = token_stream_lindera.token();
            tokens_lindera.push(token.text.clone());
        }

        // Verify no whitespace tokens
        assert!(!tokens_lindera.contains(&" ".to_string()));
        assert!(!tokens_lindera.iter().any(|t| t.trim().is_empty()));
        assert!(!tokens_lindera.is_empty());

        // Test 2: Chinese Compatible tokenizer with trim filter
        let json_chinese = r#"{
            "type": "chinese_compatible",
            "trim": true
        }"#;

        let tokenizer_chinese =
            SearchTokenizer::from_json_value(&serde_json::from_str(json_chinese).unwrap()).unwrap();
        let mut analyzer_chinese = tokenizer_chinese.to_tantivy_tokenizer().unwrap();

        let text_chinese = "中文 测试 文本";
        let mut token_stream_chinese = analyzer_chinese.token_stream(text_chinese);

        let mut tokens_chinese = Vec::new();
        while token_stream_chinese.advance() {
            let token = token_stream_chinese.token();
            tokens_chinese.push(token.text.clone());
        }

        // Verify no whitespace tokens
        assert!(!tokens_chinese.contains(&" ".to_string()));
        assert!(!tokens_chinese.iter().any(|t| t.trim().is_empty()));
    }

    /// Verify that a tantivy index whose schema references the old tokenizer name
    /// ("chinese_lindera") uses the deprecated tokenizer which preserves whitespace,
    /// while an index using the new name ("chinese_lindera_keepwhitespace:{false|true}")
    /// uses the new tokenizer which strips whitespace.
    #[rstest]
    #[case("chinese_lindera", true)]
    #[case("chinese_lindera_keepwhitespace:false", false)]
    #[case("chinese_lindera_keepwhitespace:true", true)]
    #[case("japanese_lindera", true)]
    #[case("japanese_lindera_keepwhitespace:false", false)]
    #[case("japanese_lindera_keepwhitespace:true", true)]
    #[case("korean_lindera", true)]
    #[case("korean_lindera_keepwhitespace:false", false)]
    #[case("korean_lindera_keepwhitespace:true", true)]
    fn test_schema_tokenizer_name_determines_tokenizer_variant(
        #[case] tokenizer_name: &str,
        #[case] should_find_space: bool,
    ) {
        use tantivy::collector::TopDocs;
        use tantivy::directory::RamDirectory;
        use tantivy::query::TermQuery;
        use tantivy::schema::{Schema, TextFieldIndexing, TextOptions, STORED};
        use tantivy::{Index, Term};

        // create all tokenizer variants
        let tokenizers = vec![
            SearchTokenizer::ChineseLinderaDeprecated(SearchTokenizerFilters::default()),
            SearchTokenizer::ChineseLindera {
                filters: SearchTokenizerFilters::default(),
                keep_whitespace: true,
            },
            SearchTokenizer::ChineseLindera {
                filters: SearchTokenizerFilters::default(),
                keep_whitespace: false,
            },
            SearchTokenizer::JapaneseLinderaDeprecated(SearchTokenizerFilters::default()),
            SearchTokenizer::JapaneseLindera {
                filters: SearchTokenizerFilters::default(),
                keep_whitespace: true,
            },
            SearchTokenizer::JapaneseLindera {
                filters: SearchTokenizerFilters::default(),
                keep_whitespace: false,
            },
            SearchTokenizer::KoreanLinderaDeprecated(SearchTokenizerFilters::default()),
            SearchTokenizer::KoreanLindera {
                filters: SearchTokenizerFilters::default(),
                keep_whitespace: true,
            },
            SearchTokenizer::KoreanLindera {
                filters: SearchTokenizerFilters::default(),
                keep_whitespace: false,
            },
        ];

        // Build schema with the given tokenizer name
        let text_indexing = TextFieldIndexing::default()
            .set_tokenizer(tokenizer_name)
            .set_index_option(tantivy::schema::IndexRecordOption::WithFreqsAndPositions);
        let text_options = TextOptions::default()
            .set_indexing_options(text_indexing)
            .set_stored();
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("content", text_options);
        schema_builder.add_text_field("id", STORED);
        let schema = schema_builder.build();

        // Create in-memory index and register tokenizers
        let directory = RamDirectory::create();
        let mut index = Index::create(directory, schema.clone(), Default::default()).unwrap();
        let tokenizer_manager = crate::create_tokenizer_manager(tokenizers);
        index.set_tokenizers(tokenizer_manager);

        let content_field = schema.get_field("content").unwrap();

        // Index a document containing spaces
        let mut writer = index.writer_with_num_threads(1, 15_000_000).unwrap();
        writer
            .add_document(tantivy::doc!(
                content_field => "foo and bar"
            ))
            .unwrap();
        writer.commit().unwrap();

        // Search for the space character as a raw term
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let space_term = Term::from_field_text(content_field, " ");
        let query = TermQuery::new(
            space_term,
            tantivy::schema::IndexRecordOption::WithFreqsAndPositions,
        );
        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(1).order_by_score())
            .unwrap();

        let found_space = !top_docs.is_empty();
        assert_eq!(found_space, should_find_space, "tokenizer (name={tokenizer_name}) whitespace did not keep/remove whitespace as expected");
    }
}

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

#[cfg(feature = "icu")]
use crate::icu::ICUTokenizer;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tantivy::tokenizer::Language;

// Serde will pick a SearchTokenizer variant based on the value of the
// "type" key, which needs to match one of the variant names below.
// The "type" field will not be present on the deserialized value.
//
// Ensure that new variants are added to the `to_json_value` and
// `from_json_value` methods. We don't use serde_json to ser/de the
// SearchTokenizer, because our bincode serialization format is incompatible
// with the "tagged" format we use in our public API.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub enum SearchTokenizer {
    #[default]
    Default,
    Raw,
    EnStem,
    Stem {
        language: Language,
    },
    Lowercase,
    WhiteSpace,
    RegexTokenizer {
        pattern: String,
    },
    ChineseCompatible,
    SourceCode,
    Ngram {
        min_gram: usize,
        max_gram: usize,
        prefix_only: bool,
    },
    ChineseLindera,
    JapaneseLindera,
    KoreanLindera,
    #[cfg(feature = "icu")]
    ICUTokenizer,
}

impl SearchTokenizer {
    pub fn to_json_value(&self) -> serde_json::Value {
        match self {
            SearchTokenizer::Default => json!({ "type": "default" }),
            SearchTokenizer::Raw => json!({ "type": "raw" }),
            SearchTokenizer::EnStem => json!({ "type": "en_stem" }),
            SearchTokenizer::Stem { language } => json!({ "type": "stem", "language": language }),
            SearchTokenizer::Lowercase => json!({ "type": "lowercase" }),
            SearchTokenizer::WhiteSpace => json!({ "type": "whitespace" }),
            SearchTokenizer::RegexTokenizer { pattern } => {
                json!({ "type": "regex_token", "pattern": pattern })
            }
            SearchTokenizer::ChineseCompatible => json!({ "type": "chinese_compatible" }),
            SearchTokenizer::SourceCode => json!({ "type": "source_code" }),
            SearchTokenizer::Ngram {
                min_gram,
                max_gram,
                prefix_only,
            } => json!({
                "type": "ngram",
                "min_gram": min_gram,
                "max_gram": max_gram,
                "prefix_only": prefix_only,
            }),
            SearchTokenizer::ChineseLindera => json!({ "type": "chinese_lindera" }),
            SearchTokenizer::JapaneseLindera => json!({ "type": "japanese_lindera" }),
            SearchTokenizer::KoreanLindera => json!({ "type": "korean_lindera" }),
            #[cfg(feature = "icu")]
            SearchTokenizer::ICUTokenizer => json!({ "type": "icu" }),
        }
    }

    pub fn from_json_value(value: &serde_json::Value) -> Result<Self, anyhow::Error> {
        // We use the `type` field of a JSON object to distinguish the tokenizer variant.
        // Deserialized in this "tagged enum" fashion is not supported by bincode, which
        // we use elsewhere for serialization, so we manually parse the JSON object here.

        let tokenizer_type = value["type"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("a 'type' must be passed in pg_search tokenizer configuration, not found in: {value:#?}"))?;

        match tokenizer_type {
            "default" => Ok(SearchTokenizer::Default),
            "raw" => Ok(SearchTokenizer::Raw),
            "en_stem" => Ok(SearchTokenizer::EnStem),
            "stem" => {
                let language: Language = serde_json::from_value(value["language"].clone())
                    .map_err(|_| {
                        anyhow::anyhow!("stem tokenizer requires a valid 'language' field")
                    })?;
                Ok(SearchTokenizer::Stem { language })
            }
            "lowercase" => Ok(SearchTokenizer::Lowercase),
            "whitespace" => Ok(SearchTokenizer::WhiteSpace),
            "regex_token" => {
                let pattern: String =
                    serde_json::from_value(value["pattern"].clone()).map_err(|_| {
                        anyhow::anyhow!("regex tokenizer requires a string 'pattern' field {value:#?}")
                    })?;
                Ok(SearchTokenizer::RegexTokenizer { pattern })
            }
            "chinese_compatible" => Ok(SearchTokenizer::ChineseCompatible),
            "source_code" => Ok(SearchTokenizer::SourceCode),
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
                })
            }
            "chinese_lindera" => Ok(SearchTokenizer::ChineseLindera),
            "japanese_lindera" => Ok(SearchTokenizer::JapaneseLindera),
            "korean_lindera" => Ok(SearchTokenizer::KoreanLindera),
            #[cfg(feature = "icu")]
            "icu" => Ok(SearchTokenizer::ICUTokenizer),
            _ => Err(anyhow::anyhow!(
                "unknown tokenizer type: {}",
                tokenizer_type
            )),
        }
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
        match self {
            SearchTokenizer::Default => "default".into(),
            SearchTokenizer::Raw => "raw".into(),
            SearchTokenizer::EnStem => "en_stem".into(),
            SearchTokenizer::Stem { language } => format!("stem_{}", language_to_str(language)),
            SearchTokenizer::Lowercase => "lowercase".into(),
            SearchTokenizer::WhiteSpace => "whitespace".into(),
            SearchTokenizer::RegexTokenizer { .. } => "regex_token".into(),
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
    fn test_regex_tokenizer() {
        let json = r#"{
        "type": "regex_token",
        "pattern": "a+b*"
    }"#;
        let tokenizer = SearchTokenizer::RegexTokenizer {
            pattern: "a+b*".to_string()
        };

        assert_eq!(tokenizer, SearchTokenizer::from_json_value(&serde_json::from_str(json).unwrap()).unwrap());
    }

    #[rstest]
    fn test_search_normalizer() {
        assert_eq!(SearchNormalizer::Lowercase.name(), "lowercase");
        assert_ne!(SearchNormalizer::Raw, SearchNormalizer::Lowercase);
    }
}

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

pub mod cjk;
pub mod code;
#[cfg(feature = "icu")]
pub mod icu;
pub mod lindera;
pub mod manager;

use cjk::ChineseTokenizer;
use code::CodeTokenizer;
use lindera::{LinderaChineseTokenizer, LinderaJapaneseTokenizer, LinderaKoreanTokenizer};
use tantivy::tokenizer::{
    AsciiFoldingFilter, Language, LowerCaser, NgramTokenizer, RawTokenizer, RemoveLongFilter,
    SimpleTokenizer, Stemmer, TextAnalyzer, TokenizerManager, WhitespaceTokenizer,
};
use tracing::info;

#[cfg(feature = "icu")]
use icu::ICUTokenizer;

pub use manager::{SearchNormalizer, SearchTokenizer};

pub const DEFAULT_REMOVE_TOKEN_LENGTH: usize = 255;

pub fn create_tokenizer_manager(search_tokenizers: Vec<&SearchTokenizer>) -> TokenizerManager {
    let tokenizer_manager = TokenizerManager::default();

    for search_tokenizer in search_tokenizers {
        let tokenizer_option = match search_tokenizer {
            SearchTokenizer::Default => Some(
                TextAnalyzer::builder(SimpleTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build(),
            ),
            SearchTokenizer::Raw => Some(
                TextAnalyzer::builder(RawTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .build(),
            ),
            SearchTokenizer::Lowercase => Some(
                TextAnalyzer::builder(RawTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build(),
            ),
            SearchTokenizer::WhiteSpace => Some(
                TextAnalyzer::builder(WhitespaceTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build(),
            ),
            SearchTokenizer::ChineseCompatible => Some(
                TextAnalyzer::builder(ChineseTokenizer)
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build(),
            ),
            SearchTokenizer::SourceCode => Some(
                TextAnalyzer::builder(CodeTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .filter(AsciiFoldingFilter)
                    .build(),
            ),
            SearchTokenizer::Ngram {
                min_gram,
                max_gram,
                prefix_only,
            } => Some(
                TextAnalyzer::builder(
                    NgramTokenizer::new(*min_gram, *max_gram, *prefix_only).unwrap(),
                )
                .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                .filter(LowerCaser)
                .build(),
            ),
            SearchTokenizer::ChineseLindera => Some(
                TextAnalyzer::builder(LinderaChineseTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build(),
            ),
            SearchTokenizer::JapaneseLindera => Some(
                TextAnalyzer::builder(LinderaJapaneseTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build(),
            ),
            SearchTokenizer::KoreanLindera => Some(
                TextAnalyzer::builder(LinderaKoreanTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build(),
            ),
            SearchTokenizer::EnStem => Some(
                TextAnalyzer::builder(SimpleTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .filter(Stemmer::new(Language::English))
                    .build(),
            ),
            SearchTokenizer::Stem { language } => Some(
                TextAnalyzer::builder(SimpleTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .filter(Stemmer::new(*language))
                    .build(),
            ),
            #[cfg(feature = "icu")]
            SearchTokenizer::ICUTokenizer => Some(
                TextAnalyzer::builder(ICUTokenizer)
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build(),
            ),
        };

        if let Some(text_analyzer) = tokenizer_option {
            info!(
                tokenizer_name = &search_tokenizer.name(),
                "registering tokenizer",
            );
            tokenizer_manager.register(&search_tokenizer.name(), text_analyzer);
        }
    }

    tokenizer_manager
}

pub fn create_normalizer_manager() -> TokenizerManager {
    let raw_tokenizer = TextAnalyzer::builder(RawTokenizer::default())
        .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
        .build();
    let lower_case_tokenizer = TextAnalyzer::builder(RawTokenizer::default())
        .filter(LowerCaser)
        .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
        .build();
    let tokenizer_manager = TokenizerManager::new();
    tokenizer_manager.register("raw", raw_tokenizer);
    tokenizer_manager.register("lowercase", lower_case_tokenizer);
    tokenizer_manager
}

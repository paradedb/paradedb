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

pub mod chinese_convert;
pub mod cjk;
pub mod code;
pub mod edge_ngram;
pub mod icu;
pub mod lindera;
pub mod manager;
pub mod ngram;
pub mod token_length;
pub mod token_trim;
mod unicode_words;

use std::collections::HashSet;

use tantivy::tokenizer::{LowerCaser, RawTokenizer, TextAnalyzer, TokenizerManager};
use tracing::debug;

pub use manager::{SearchNormalizer, SearchTokenizer};

pub fn create_tokenizer_manager(search_tokenizers: Vec<SearchTokenizer>) -> TokenizerManager {
    create_tokenizer_manager_with_legacy_names(search_tokenizers, false)
}

/// Create a manager while preserving shared legacy names from a persisted schema.
pub fn create_tokenizer_manager_with_legacy_names(
    search_tokenizers: Vec<SearchTokenizer>,
    preserve_legacy_trim_names: bool,
) -> TokenizerManager {
    let tokenizer_manager = TokenizerManager::default();
    register_tokenizers_into(
        &tokenizer_manager,
        search_tokenizers,
        preserve_legacy_trim_names,
    );
    tokenizer_manager
}

/// Register `search_tokenizers` into an existing manager. `TokenizerManager` registration
/// mutates its shared registry, so readers already holding the manager see the entries too.
pub fn register_tokenizers_into(
    tokenizer_manager: &TokenizerManager,
    search_tokenizers: Vec<SearchTokenizer>,
    preserve_legacy_trim_names: bool,
) {
    let current_names: HashSet<String> = search_tokenizers
        .iter()
        .map(|search_tokenizer| search_tokenizer.name().to_owned())
        .collect();

    for search_tokenizer in &search_tokenizers {
        let tokenizer_option = search_tokenizer.to_tantivy_tokenizer();

        if let Some(text_analyzer) = tokenizer_option {
            debug!(
                tokenizer_name = &search_tokenizer.name(),
                "registering tokenizer",
            );
            tokenizer_manager.register(&search_tokenizer.name(), text_analyzer);
        }

        if let Some(legacy_name) = search_tokenizer.legacy_name() {
            let should_register_legacy = preserve_legacy_trim_names
                || (!current_names.contains(&legacy_name)
                    && tokenizer_manager.get(&legacy_name).is_none());
            if should_register_legacy
                && let Some(text_analyzer) = search_tokenizer.to_tantivy_tokenizer()
            {
                tokenizer_manager.register(&legacy_name, text_analyzer);
            }
        }
    }
}

pub fn create_normalizer_manager() -> TokenizerManager {
    let tokenizer_manager = TokenizerManager::new();
    register_normalizers_into(&tokenizer_manager);
    tokenizer_manager
}

/// Register the fast-field normalizers into an existing manager; see
/// [`register_tokenizers_into`] for why registration rather than replacement.
pub fn register_normalizers_into(tokenizer_manager: &TokenizerManager) {
    let raw_tokenizer = TextAnalyzer::builder(RawTokenizer::default()).build();
    let lower_case_tokenizer = TextAnalyzer::builder(RawTokenizer::default())
        .filter(LowerCaser)
        .build();
    tokenizer_manager.register("raw", raw_tokenizer);
    tokenizer_manager.register("lowercase", lower_case_tokenizer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_settings_have_distinct_names_and_keep_the_legacy_alias_when_unambiguous() {
        let trimmed = SearchTokenizer::from_json_value(&serde_json::json!({
            "type": "jieba",
            "trim": true
        }))
        .unwrap();
        let untrimmed = SearchTokenizer::from_json_value(&serde_json::json!({
            "type": "jieba",
            "trim": false
        }))
        .unwrap();

        let manager = create_tokenizer_manager(vec![trimmed.clone()]);
        assert!(manager.get("jieba[trim=true]").is_some());
        assert!(manager.get("jieba").is_some());

        let manager = create_tokenizer_manager(vec![trimmed, untrimmed]);
        assert!(manager.get("jieba[trim=true]").is_some());
        assert!(manager.get("jieba").is_some());

        let manager = create_tokenizer_manager_with_legacy_names(
            vec![
                SearchTokenizer::from_json_value(&serde_json::json!({
                    "type": "jieba", "trim": true
                }))
                .unwrap(),
                SearchTokenizer::from_json_value(&serde_json::json!({
                    "type": "jieba", "trim": false
                }))
                .unwrap(),
            ],
            true,
        );
        assert!(manager.get("jieba[trim=true]").is_some());
        assert!(manager.get("jieba").is_some());
    }

    #[test]
    fn trim_override_does_not_replace_an_untrimmed_field_tokenizer() {
        use tantivy::tokenizer::TokenStream;

        let field_tokenizer = SearchTokenizer::Lindera {
            language: manager::LinderaLanguage::Korean,
            filters: manager::SearchTokenizerFilters {
                trim: Some(false),
                ..Default::default()
            },
            keep_whitespace: true,
            nfkc: false,
            reading_form: false,
        };
        let search_tokenizer = SearchTokenizer::Lindera {
            language: manager::LinderaLanguage::Korean,
            filters: manager::SearchTokenizerFilters {
                trim: Some(true),
                ..Default::default()
            },
            keep_whitespace: true,
            nfkc: false,
            reading_form: false,
        };

        let manager = create_tokenizer_manager_with_legacy_names(
            vec![field_tokenizer, search_tokenizer],
            false,
        );
        let mut analyzer = manager.get("korean_lindera_keepwhitespace:true").unwrap();
        let mut stream = analyzer.token_stream("아름다운 우리나라");
        let mut tokens = Vec::new();
        while stream.advance() {
            tokens.push(stream.token().text.clone());
        }

        assert!(tokens.contains(&" ".to_string()));
        assert!(
            manager
                .get("korean_lindera_keepwhitespace:true[trim=true]")
                .is_some()
        );
    }
}

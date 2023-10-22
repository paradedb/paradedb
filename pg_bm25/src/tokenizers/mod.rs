mod cjk;
mod code;

use std::collections::HashMap;

use crate::parade_index::fields::ParadeTokenizer;
use crate::tokenizers::code::CodeTokenizer;
use crate::{tokenizers::cjk::ChineseTokenizer, types::ParadeFieldConfig};

use tantivy::tokenizer::{
    AsciiFoldingFilter, LowerCaser, NgramTokenizer, RawTokenizer, RemoveLongFilter, TextAnalyzer,
    TokenizerManager,
};

pub const DEFAULT_REMOVE_TOKEN_LENGTH: usize = 255;

pub fn create_tokenizer_manager(
    field_configs: &HashMap<String, ParadeFieldConfig>,
) -> TokenizerManager {
    let tokenizer_manager = TokenizerManager::default();

    for (field_name, field_config) in field_configs {
        let parade_tokenizer = &field_config.json_options.tokenizer;

        let tokenizer_option = match parade_tokenizer {
            ParadeTokenizer::Raw => Some(
                TextAnalyzer::builder(RawTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .build(),
            ),
            ParadeTokenizer::ChineseCompatible => Some(
                TextAnalyzer::builder(ChineseTokenizer)
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build(),
            ),
            ParadeTokenizer::SourceCode => Some(
                TextAnalyzer::builder(CodeTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .filter(AsciiFoldingFilter)
                    .build(),
            ),
            ParadeTokenizer::Ngram {
                min_gram,
                max_gram,
                prefix_only,
            } => Some(
                TextAnalyzer::builder(
                    NgramTokenizer::new(min_gram.clone(), max_gram.clone(), prefix_only.clone())
                        .unwrap(),
                )
                .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                .filter(LowerCaser)
                .build(),
            ),
            _ => None,
        };

        if let Some(tokenizer) = tokenizer_option {
            let tokenizer_name = format!("{field_name}_{}", parade_tokenizer.name());
            tokenizer_manager.register(&tokenizer_name, tokenizer);
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

pub(crate) mod cjk;
pub(crate) mod code;
pub(crate) mod icu;
pub(crate) mod lindera;

use crate::parade_index::fields::{ParadeOption, ParadeOptionMap, ParadeTokenizer};
use crate::tokenizers::cjk::ChineseTokenizer;
use crate::tokenizers::code::CodeTokenizer;
use crate::tokenizers::icu::ICUTokenizer;
use crate::tokenizers::lindera::{
    LinderaChineseTokenizer, LinderaJapaneseTokenizer, LinderaKoreanTokenizer,
};
use serde_json::json;
use shared::plog;
use tantivy::tokenizer::{
    AsciiFoldingFilter, LowerCaser, NgramTokenizer, RawTokenizer, RemoveLongFilter, TextAnalyzer,
    TokenizerManager,
};

pub const DEFAULT_REMOVE_TOKEN_LENGTH: usize = 255;

pub fn create_tokenizer_manager(option_map: &ParadeOptionMap) -> TokenizerManager {
    let tokenizer_manager = TokenizerManager::default();

    for (field_name, field_options) in option_map.iter() {
        plog!(
            "attempting to create tokenizer",
            json!({ "field_name": field_name, "field_options": field_options })
        );

        let parade_tokenizer = match field_options {
            ParadeOption::Text(text_options) => text_options.tokenizer,
            ParadeOption::Json(json_options) => json_options.tokenizer,
            _ => continue,
        };

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
                    NgramTokenizer::new(min_gram, max_gram, prefix_only).unwrap(),
                )
                .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                .filter(LowerCaser)
                .build(),
            ),
            ParadeTokenizer::ChineseLindera => Some(
                TextAnalyzer::builder(LinderaChineseTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build(),
            ),
            ParadeTokenizer::JapaneseLindera => Some(
                TextAnalyzer::builder(LinderaJapaneseTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build(),
            ),
            ParadeTokenizer::KoreanLindera => Some(
                TextAnalyzer::builder(LinderaKoreanTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build(),
            ),
            ParadeTokenizer::ICUTokenizer => Some(
                TextAnalyzer::builder(ICUTokenizer)
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build(),
            ),
            _ => None,
        };

        if let Some(text_analyzer) = tokenizer_option {
            plog!(
                "registering tokenizer",
                json!({
                    "field_name": field_name,
                    "tokenizer_name": &parade_tokenizer.name()
                })
            );
            tokenizer_manager.register(&parade_tokenizer.name(), text_analyzer);
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

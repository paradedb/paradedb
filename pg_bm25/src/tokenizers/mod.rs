pub(crate) mod cjk;
pub(crate) mod code;
#[cfg(feature = "icu")]
pub(crate) mod icu;
pub(crate) mod lindera;

use crate::schema::SearchTokenizer;
use crate::schema::{SearchFieldConfig, SearchIndexSchema};
use crate::tokenizers::cjk::ChineseTokenizer;
use crate::tokenizers::code::CodeTokenizer;
use crate::tokenizers::lindera::{
    LinderaChineseTokenizer, LinderaJapaneseTokenizer, LinderaKoreanTokenizer,
};
use tantivy::tokenizer::{
    AsciiFoldingFilter, LowerCaser, NgramTokenizer, RawTokenizer, RemoveLongFilter, TextAnalyzer,
    TokenizerManager,
};
use tracing::info;

#[cfg(feature = "icu")]
use crate::tokenizers::icu::ICUTokenizer;

pub const DEFAULT_REMOVE_TOKEN_LENGTH: usize = 255;

pub fn create_tokenizer_manager(schema: &SearchIndexSchema) -> TokenizerManager {
    let tokenizer_manager = TokenizerManager::default();

    for search_field in &schema.fields {
        let field_config = &search_field.config;
        let field_name: &str = search_field.name.as_ref();
        info!(field_name, "attempting to create tokenizer");

        let search_tokenizer = match field_config {
            SearchFieldConfig::Text { tokenizer, .. }
            | SearchFieldConfig::Json { tokenizer, .. } => tokenizer,
            _ => continue,
        };

        let tokenizer_option = match search_tokenizer {
            SearchTokenizer::Raw => Some(
                TextAnalyzer::builder(RawTokenizer::default())
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
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
            #[cfg(feature = "icu")]
            SearchTokenizer::ICUTokenizer => Some(
                TextAnalyzer::builder(ICUTokenizer)
                    .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
                    .filter(LowerCaser)
                    .build(),
            ),
            _ => None,
        };

        if let Some(text_analyzer) = tokenizer_option {
            info!(
                field_name,
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

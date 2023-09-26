mod cjk;
mod code;

use crate::tokenizers::cjk::ChineseTokenizer;
use crate::tokenizers::code::CodeTokenizer;

use tantivy::tokenizer::{
    AsciiFoldingFilter, LowerCaser, RawTokenizer, RemoveLongFilter, TextAnalyzer, TokenizerManager,
};

pub const DEFAULT_REMOVE_TOKEN_LENGTH: usize = 255;

pub fn create_tokenizer_manager() -> TokenizerManager {
    let tokenizer_manager = TokenizerManager::default();

    let raw_tokenizer = TextAnalyzer::builder(RawTokenizer::default())
        .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
        .build();
    tokenizer_manager.register("raw", raw_tokenizer);
    let chinese_tokenizer = TextAnalyzer::builder(ChineseTokenizer)
        .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
        .filter(LowerCaser)
        .build();
    tokenizer_manager.register("chinese_compatible", chinese_tokenizer);
    tokenizer_manager.register(
        "source_code_default",
        TextAnalyzer::builder(CodeTokenizer::default())
            .filter(RemoveLongFilter::limit(DEFAULT_REMOVE_TOKEN_LENGTH))
            .filter(LowerCaser)
            .filter(AsciiFoldingFilter)
            .build(),
    );
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

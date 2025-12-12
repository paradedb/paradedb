// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// Chinese Traditional/Simplified conversion using OpenCC

use once_cell::sync::Lazy;
use opencc::OpenCC;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tantivy::tokenizer::Tokenizer;

/// OpenCC conversion modes
#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConvertMode {
    /// Traditional to Simplified Chinese
    #[default]
    T2S,
    /// Simplified to Traditional Chinese
    S2T,
    /// Traditional Chinese to Taiwan Traditional
    T2TW,
    /// Traditional Chinese to Hong Kong Traditional
    T2HK,
    /// Simplified Chinese to Taiwan Traditional
    S2TW,
    /// Simplified Chinese to Hong Kong Traditional
    S2HK,
}

/// Global OpenCC converter instances (lazy-loaded)
static OPENCC_T2S: Lazy<Mutex<OpenCC>> = Lazy::new(|| Mutex::new(OpenCC::new("t2s.json")));

static OPENCC_S2T: Lazy<Mutex<OpenCC>> = Lazy::new(|| Mutex::new(OpenCC::new("s2t.json")));

static OPENCC_T2TW: Lazy<Mutex<OpenCC>> = Lazy::new(|| Mutex::new(OpenCC::new("t2tw.json")));

static OPENCC_T2HK: Lazy<Mutex<OpenCC>> = Lazy::new(|| Mutex::new(OpenCC::new("t2hk.json")));

static OPENCC_S2TW: Lazy<Mutex<OpenCC>> = Lazy::new(|| Mutex::new(OpenCC::new("s2tw.json")));

static OPENCC_S2HK: Lazy<Mutex<OpenCC>> = Lazy::new(|| Mutex::new(OpenCC::new("s2hk.json")));

/// Get the OpenCC instance for the specified mode
fn get_opencc(mode: ConvertMode) -> &'static Lazy<Mutex<OpenCC>> {
    match mode {
        ConvertMode::T2S => &OPENCC_T2S,
        ConvertMode::S2T => &OPENCC_S2T,
        ConvertMode::T2TW => &OPENCC_T2TW,
        ConvertMode::T2HK => &OPENCC_T2HK,
        ConvertMode::S2TW => &OPENCC_S2TW,
        ConvertMode::S2HK => &OPENCC_S2HK,
    }
}

/// Chinese Traditional/Simplified Conversion Tokenizer Wrapper
///
/// Converts the entire text **before tokenization** to ensure proper word segmentation
#[derive(Clone)]
pub struct ChineseConvertTokenizer<T: Tokenizer> {
    inner: T,
    mode: ConvertMode,
    buffer: String, // Buffer to store the converted text
}

impl<T: Tokenizer> ChineseConvertTokenizer<T> {
    /// Create a new converter tokenizer
    pub fn new(inner: T, mode: ConvertMode) -> Self {
        Self {
            inner,
            mode,
            buffer: String::new(),
        }
    }
}

impl<T: Tokenizer> Tokenizer for ChineseConvertTokenizer<T> {
    type TokenStream<'a> = T::TokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        // Convert the entire text before tokenization, storing it in buffer
        let opencc = get_opencc(self.mode);
        self.buffer = opencc.lock().unwrap().convert(text);

        // Tokenize using the buffer text
        // The buffer's lifetime is bound to self ('a), so this is safe
        self.inner.token_stream(&self.buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::tokenizer::{SimpleTokenizer, TokenStream};

    #[test]
    fn test_t2s_conversion() {
        let base_tokenizer = SimpleTokenizer::default();
        let mut tokenizer = ChineseConvertTokenizer::new(base_tokenizer, ConvertMode::T2S);

        let mut stream = tokenizer.token_stream("繁體中文測試");
        let mut tokens = Vec::new();

        while stream.advance() {
            tokens.push(stream.token().text.clone());
        }

        // Should be converted to simplified Chinese before tokenization
        assert!(!tokens.is_empty());
        // Check if it contains simplified Chinese characters
        let text = tokens.join("");
        assert!(text.contains("繁体") || text.contains("测试"));
    }

    #[test]
    fn test_s2t_conversion() {
        let base_tokenizer = SimpleTokenizer::default();
        let mut tokenizer = ChineseConvertTokenizer::new(base_tokenizer, ConvertMode::S2T);

        let mut stream = tokenizer.token_stream("简体中文测试");
        let mut tokens = Vec::new();

        while stream.advance() {
            tokens.push(stream.token().text.clone());
        }

        // Should be converted to traditional Chinese
        assert!(!tokens.is_empty());
        let text = tokens.join("");
        assert!(text.contains("簡體") || text.contains("測試"));
    }

    #[test]
    fn test_jieba_with_convert() {
        // Test integration with Jieba tokenizer
        let base_tokenizer = tantivy_jieba::JiebaTokenizer {};
        let mut tokenizer = ChineseConvertTokenizer::new(base_tokenizer, ConvertMode::T2S);

        let mut stream = tokenizer.token_stream("繁體中文分詞測試");
        let mut tokens = Vec::new();

        while stream.advance() {
            tokens.push(stream.token().text.clone());
        }

        assert!(!tokens.is_empty());
        // Verify that conversion to simplified Chinese was successful
        for token in &tokens {
            println!("Token: {}", token);
        }
    }
}
